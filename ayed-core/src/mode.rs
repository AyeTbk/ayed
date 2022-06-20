use crate::{
    command::Command,
    input::Input,
    input_mapper::{InputContext, InputMapper},
};

pub struct TextCommandMode;

impl InputMapper for TextCommandMode {
    fn convert_input_to_command(&self, input: Input, _ctx: &InputContext) -> Option<Command> {
        let command = match input {
            Input::Char('\t') => Command::ChangeMode("text-edit"),
            Input::Up => Command::MoveSelectionUp,
            Input::Down => Command::MoveSelectionDown,
            Input::Left => Command::MoveSelectionLeft,
            Input::Right => Command::MoveSelectionRight,
            _ => return None,
        };
        Some(command)
    }
}

pub struct TextEditMode;

impl InputMapper for TextEditMode {
    fn convert_input_to_command(&self, input: Input, _ctx: &InputContext) -> Option<Command> {
        let command = match input {
            Input::Char('\t') => Command::ChangeMode("text-command"),
            Input::Char(ch) => Command::Insert(ch),
            Input::Return => Command::Insert('\n'),
            Input::Backspace => Command::DeleteBeforeSelection,
            Input::Delete => Command::DeleteSelection,
            Input::Up => Command::MoveSelectionUp,
            Input::Down => Command::MoveSelectionDown,
            Input::Left => Command::MoveSelectionLeft,
            Input::Right => Command::MoveSelectionRight,
            _ => todo!(),
        };
        Some(command)
    }
}
