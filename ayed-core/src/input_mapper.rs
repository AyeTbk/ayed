use crate::{command::Command, input::Input};

pub trait InputMapper {
    fn convert_input_to_command(&self, input: Input, ctx: &InputContext) -> Command;
}

#[derive(Default)]
pub struct InputContext {
    _mode: (),
}

#[derive(Default)]
pub struct InputMapperImpl {
    //
}

impl InputMapper for InputMapperImpl {
    fn convert_input_to_command(&self, input: Input, _ctx: &InputContext) -> Command {
        match input {
            Input::Char(ch) => Command::Insert(ch),
            Input::Return => Command::Insert('\n'),
            Input::Up => Command::MoveSelectionUp,
            Input::Down => Command::MoveSelectionDown,
            Input::Left => Command::MoveSelectionLeft,
            Input::Right => Command::MoveSelectionRight,
            _ => todo!(),
        }
    }
}
