use crate::{command::Command, input::Input};

pub trait InputMapper {
    fn convert_input_to_command(&self, input: Input, ctx: &InputContext) -> Option<Command>;
}

#[derive(Default)]
pub struct InputContext {
    _mode: (),
}
