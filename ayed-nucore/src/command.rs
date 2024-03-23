use std::collections::{HashMap, VecDeque};

use crate::{event::EventRegistry, state::State};

struct Command {
    func: Box<dyn Fn(&str, ExecuteCommandContext) -> Result<(), ()>>,
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
}

impl CommandRegistry {
    pub fn register(
        &mut self,
        name: impl Into<String>,
        func: Box<dyn Fn(&str, ExecuteCommandContext) -> Result<(), ()>>,
    ) {
        self.commands.insert(name.into(), Command { func });
    }

    pub fn execute_command(
        &self,
        name: &str,
        options: &str,
        ctx: ExecuteCommandContext,
    ) -> Result<(), ()> {
        let command = self.commands.get(name).ok_or(())?;
        (command.func)(options, ctx)
    }
}

pub struct ExecuteCommandContext<'a> {
    pub events: &'a mut EventRegistry,
    pub queue: &'a mut CommandQueue,
    pub state: &'a mut State,
}

#[derive(Default)]
pub struct CommandQueue {
    queue: VecDeque<(String, String)>,
}

impl CommandQueue {
    pub fn push(&mut self, command: impl Into<String>, options: impl Into<String>) {
        self.queue.push_front((command.into(), options.into()))
    }

    pub fn pop(&mut self) -> Option<(String, String)> {
        self.queue.pop_front()
    }

    pub fn extend_front(&mut self, iter: impl IntoIterator<Item = (String, String)>) {
        for (i, item) in iter.into_iter().enumerate() {
            self.queue.insert(i, item);
        }
    }
}
