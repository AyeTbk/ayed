use std::collections::{HashMap, VecDeque};

use crate::{event::EventRegistry, panels::Panels, state::State};

pub mod commands;
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

    pub fn execute_command(
        &self,
        command: &str,
        ctx: ExecuteCommandContext,
    ) -> Result<Result<(), String>, String> {
        let (name, options) = parse_command(command);
        let command = self
            .commands
            .get(name)
            .ok_or_else(|| format!("unknown command '{name}'"))?;
        Ok((command.func)(options, ctx))
    }
}

pub struct ExecuteCommandContext<'a> {
    pub events: &'a mut EventRegistry,
    pub queue: &'a mut CommandQueue,
    pub state: &'a mut State,
    pub panels: &'a mut Panels,
}

#[derive(Debug)]
pub struct CommandQueue {
    queue: VecDeque<String>,
    scope_stack: Vec<Scope>,
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self {
            queue: VecDeque::default(),
            scope_stack: vec![Scope::default()],
        }
    }
}

impl CommandQueue {
    pub fn set_state(&mut self, state: &str, value: &str) {
        self.push(format!("state-set {state} {value}"));
    }

    pub fn push(&mut self, command: impl Into<String>) {
        let command = command.into();
        self.queue
            .insert(self.current_scope().remaining_commands as _, command);
        self.current_scope_mut().remaining_commands += 1;
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
        self.scope_stack.push(Default::default())
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
