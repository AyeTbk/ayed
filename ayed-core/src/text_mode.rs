use crate::{command::Command, core::EditorContextMut, input::Input, input_mapper::InputMapper};

pub struct TextCommandMode;

impl TextCommandMode {
    pub const NAME: &'static str = "text-command";
}

impl InputMapper for TextCommandMode {
    fn convert_input_to_command(
        &self,
        input: Input,
        _ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        let command = match input {
            Input::Char('\t') | Input::Char('i') => Command::ChangeMode(TextEditMode::NAME),
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

impl TextEditMode {
    pub const NAME: &'static str = "text-edit";
}

impl InputMapper for TextEditMode {
    fn convert_input_to_command(
        &self,
        input: Input,
        _ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        let command = match input {
            Input::Char('\t') => Command::ChangeMode(TextCommandMode::NAME),
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
