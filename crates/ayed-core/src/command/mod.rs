use std::collections::{HashMap, VecDeque};

use crate::{
    panels::Panels,
    state::{Resources, State},
};

pub mod helpers;
pub mod options;

struct Command {
    func: Box<dyn Fn(&str, ExecuteCommandContext) -> Result<(), String>>,
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    pub fn register(
        &mut self,
        name: impl Into<String>,
        func: impl (Fn(&str, ExecuteCommandContext) -> Result<(), String>) + 'static,
    ) {
        self.commands.insert(
            name.into(),
            Command {
                func: Box::new(func),
            },
        );
    }

    pub fn register_event(&mut self, name: impl Into<String>) {
        self.register(name, |_, _| Ok(()));
    }

    pub fn execute_command(
        &self,
        command: &str,
        ctx: ExecuteCommandContext,
    ) -> Result<Result<(), String>, String> {
        let (name, options) = parse_command(command);
        let command = if let Some(command) = self.commands.get(name) {
            command
        } else {
            if self.is_hardcoded_event(command) {
                return Ok(Ok(()));
            } else {
                return Err(format!("unknown command '{name}'"));
            }
        };
        Ok((command.func)(options, ctx))
    }

    fn is_hardcoded_event(&self, command: &str) -> bool {
        let (name, _) = parse_command(command);
        name.starts_with("state-modified:")
            || name.starts_with("state-before-modified:")
            || name.starts_with("state-after-modified:")
    }
}

pub struct ExecuteCommandContext<'a> {
    pub queue: &'a mut CommandQueue,
    pub state: &'a mut State,
    pub resources: &'a mut Resources,
    pub panels: &'a mut Panels,
}

#[derive(Debug)]
pub struct CommandQueue {
    queue: VecDeque<String>,
    scope_stack: Vec<Scope>,
    debug_log: String,
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self {
            queue: VecDeque::default(),
            scope_stack: vec![Scope::default()],
            debug_log: String::new(),
        }
    }
}

impl CommandQueue {
    pub fn take_debug_log(&mut self) -> String {
        let log = format!("Command log:\n{}", self.debug_log);
        self.debug_log.clear();
        log
    }

    pub fn set_state(&mut self, state: &str, value: &str) {
        self.push(format!("state-set {state} {value}"));
    }

    pub fn push(&mut self, command: impl Into<String>) {
        let command = command.into();
        self.queue
            .insert(self.current_scope().remaining_commands as _, command);
        self.current_scope_mut().remaining_commands += 1;
    }

    // TODO remove this? I added it mostly to easy the removal of the EventRegistry.
    pub fn emit(&mut self, command: impl Into<String>, also_concat_this: &str) {
        let mut cmd = command.into();
        cmd.push(' ');
        cmd.push_str(also_concat_this);
        self.push(cmd);
    }

    pub fn pop(&mut self) -> Option<String> {
        if self.current_scope().remaining_commands == 0 {
            loop {
                let scope = self.current_scope();
                if scope.remaining_commands > 0 {
                    break;
                }
                if self.scope_stack.len() == 1 {
                    // Don't pop the first scope, at least one should always exist.
                    break;
                }
                self.scope_stack.pop();
            }
        }

        if let Some(command) = self.queue.pop_front() {
            self.current_scope_mut().remaining_commands -= 1;

            let scope_stack_depth = self.scope_stack.len();
            let indent = "  ".repeat(scope_stack_depth);
            self.debug_log += &format!("{indent}{command}\n");

            Some(command)
        } else {
            None
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = String>) {
        for item in iter.into_iter() {
            self.push(item)
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();

        self.scope_stack.clear();
        self.scope_stack.push(Default::default());

        self.debug_log.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub(crate) fn start_scope(&mut self) {
        self.scope_stack.push(Scope::default());
    }

    fn current_scope(&self) -> &Scope {
        self.scope_stack
            .last()
            .expect("there should always be a scope")
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scope_stack
            .last_mut()
            .expect("there should always be a scope")
    }
}

#[derive(Debug, Default)]
struct Scope {
    remaining_commands: u32,
}

pub fn parse_command(command: &str) -> (&str, &str) {
    command.split_once(' ').unwrap_or((command, ""))
}
