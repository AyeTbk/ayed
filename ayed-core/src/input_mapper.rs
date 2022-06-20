use crate::{command::Command, core::EditorContextMut, input::Input};

pub trait InputMapper {
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut)
        -> Option<Command>;
}
