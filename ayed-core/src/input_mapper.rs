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
    description: HashMap<Input, String>,
    insert_order: HashMap<Input, u32>,
    insert_count: u32,
}

impl InputMapper {
    pub fn new() -> Self {
        Self {
            do_char_insert: false,
            mapping: Default::default(),
            description: Default::default(),
            insert_order: Default::default(),
            insert_count: 0,
        }
    }

    pub fn register_char_insert(&mut self) {
        self.do_char_insert = true;
    }

    pub fn register(&mut self, input: &str, command: impl Into<MappedCommand>) -> Result<(), ()> {
        let input = Input::parse(input)?;
        self.register_input(input, command, None);
        Ok(())
    }

    pub fn register_with_description(
        &mut self,
        input: &str,
        command: impl Into<MappedCommand>,
        description: &str,
    ) -> Result<(), ()> {
        let input = Input::parse(input)?;
        self.register_input(input, command, description.to_string());
        Ok(())
    }

    pub fn register_input(
        &mut self,
        input: Input,
        command: impl Into<MappedCommand>,
        description: impl Into<Option<String>>,
    ) {
        let norm_input = input.normalized();
        self.mapping.insert(norm_input, command.into());
        if let Some(desc) = description.into() {
            self.description.insert(norm_input, desc);
        }
        self.insert_order.insert(norm_input, self.insert_count);
        self.insert_count += 1;
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

    pub fn ordered_inputs(&self) -> Vec<(Input, Option<String>)> {
        let mut inputs = self
            .mapping
            .keys()
            .map(|input| (*input, self.description.get(input).cloned()))
            .collect::<Vec<_>>();
        inputs.sort_by_key(|(input, _)| self.insert_order.get(input).copied().unwrap_or(u32::MAX));
        inputs
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
