use std::collections::HashMap;

use crate::{
    command::{Command, EditorCommand},
    input::Input,
    state::State,
};

#[derive(Default)]
pub struct InputMapper {
    do_char_insert: bool,
    mapping: HashMap<Input, MappedCommand>,
}

impl InputMapper {
    pub fn new() -> Self {
        Self {
            do_char_insert: false,
            mapping: Default::default(),
        }
    }

    pub fn register_char_insert(&mut self) {
        self.do_char_insert = true;
    }

    pub fn register(&mut self, input: &str, command: impl Into<MappedCommand>) -> Result<(), ()> {
        let input = Input::parse(input)?;
        self.register_input(input, command);
        Ok(())
    }

    pub fn register_input(&mut self, input: Input, command: impl Into<MappedCommand>) {
        self.mapping.insert(input.normalized(), command.into());
    }

    pub fn convert_input(&self, input: Input, _state: &State) -> Vec<Command> {
        let mut commands = Vec::new();

        if let Some(command) = self.mapping.get(&input).cloned() {
            commands.extend(command.to_commands());
        } else if self.do_char_insert {
            if let Some(ch) = input.char() {
                commands.push(EditorCommand::Insert(ch).into());
            }
        }

        commands
    }

    pub fn iter(&self) -> impl Iterator<Item = Input> + '_ {
        self.mapping.iter().map(|(k, _)| *k)
    }
}

#[derive(Debug, Clone)]
pub enum MappedCommand {
    Single(Command),
    Many(Vec<Command>),
}

impl MappedCommand {
    pub fn to_commands(self) -> Vec<Command> {
        match self {
            Self::Single(command) => vec![command],
            Self::Many(commands) => commands,
        }
    }
}

impl<T: Into<Command>> From<T> for MappedCommand {
    fn from(command: T) -> Self {
        Self::Single(command.into())
    }
}

impl<T: Into<Command>, const N: usize> From<[T; N]> for MappedCommand {
    fn from(commands: [T; N]) -> Self {
        Self::Many(commands.into_iter().map(Into::into).collect())
    }
}
