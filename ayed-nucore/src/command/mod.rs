use std::collections::{HashMap, VecDeque};

use crate::{event::EventRegistry, state::State};

pub mod commands;

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
}

#[derive(Debug, Default)]
pub struct CommandQueue {
    queue: VecDeque<String>,
    scope_stack: Vec<Scope>,
}

impl CommandQueue {
    pub fn push(&mut self, command: impl Into<String>) {
        let command = command.into();
        if let Some(scope) = self.scope_stack.last_mut() {
            self.queue.insert(scope.remaining_commands as _, command);
            scope.remaining_commands += 1;
        } else {
            self.queue.push_back(command);
        }
    }

    pub fn pop(&mut self) -> Option<String> {
        if let Some(scope) = self.scope_stack.last_mut() {
            if scope.remaining_commands == 0 {
                loop {
                    let Some(scope) = self.scope_stack.last() else {
                        break;
                    };
                    if scope.remaining_commands != 0 {
                        break;
                    }
                    self.scope_stack.pop();
                }
            } else {
                scope.remaining_commands = scope.remaining_commands.saturating_sub(1);
            }
        }
        self.queue.pop_front()
    }

    pub fn extend_front(&mut self, iter: impl IntoIterator<Item = String>) {
        for (i, item) in iter.into_iter().enumerate() {
            self.queue.insert(i, item);
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.scope_stack.clear();
    }

    pub(crate) fn start_scope(&mut self) {
        self.scope_stack.push(Scope::default());
    }
}

#[derive(Debug, Default)]
struct Scope {
    remaining_commands: u32,
}

pub fn parse_command(command: &str) -> (&str, &str) {
    command.split_once(' ').unwrap_or((command, ""))
}
