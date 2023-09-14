use std::collections::HashMap;

use crate::{command::EditorCommand, input::Input, state::State};

pub trait InputMap {
    fn convert_input_to_command(&self, input: Input, state: &State) -> Vec<EditorCommand>;
}

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

    pub fn convert_input(&self, input: Input, _state: &State) -> Vec<EditorCommand> {
        let mut commands = Vec::new();

        if let Some(command) = self.mapping.get(&input).cloned() {
            commands.extend(command.to_commands());
        } else if self.do_char_insert {
            if let Some(ch) = input.char() {
                commands.push(EditorCommand::Insert(ch));
            }
        }

        commands
    }
}

#[derive(Debug, Clone)]
pub enum MappedCommand {
    Single(EditorCommand),
    Many(Vec<EditorCommand>),
}

impl MappedCommand {
    pub fn to_commands(self) -> Vec<EditorCommand> {
        match self {
            Self::Single(command) => vec![command],
            Self::Many(commands) => commands,
        }
    }
}

impl From<EditorCommand> for MappedCommand {
    fn from(command: EditorCommand) -> Self {
        Self::Single(command)
    }
}

impl<const N: usize> From<[EditorCommand; N]> for MappedCommand {
    fn from(commands: [EditorCommand; N]) -> Self {
        Self::Many(commands.to_vec())
    }
}
