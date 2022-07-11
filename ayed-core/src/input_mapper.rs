use std::collections::HashMap;

use crate::{command::Command, core::EditorContextMut, input::Input};

pub trait InputMap {
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut)
        -> Option<Command>;
}

#[derive(Default)]
pub struct InputMapper {
    do_char_insert: bool,
    mapping: HashMap<Input, Command>,
}

impl InputMapper {
    pub fn register_char_insert(&mut self) {
        self.do_char_insert = true;
    }

    pub fn register(&mut self, input: &str, command: Command) -> Result<(), ()> {
        let input = Input::try_parse(input)?;
        self.register_raw(input, command);
        Ok(())
    }

    pub fn register_raw(&mut self, input: Input, command: Command) {
        self.mapping.insert(input.normalized(), command.into());
    }
}

impl InputMap for InputMapper {
    fn convert_input_to_command(
        &self,
        input: Input,
        _ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        let mapped_command = self.mapping.get(&input).copied();
        if mapped_command.is_some() {
            mapped_command
        } else if self.do_char_insert {
            Some(Command::Insert(input.char()?))
        } else {
            None
        }
    }
}
