use crate::{
    command::Command,
    core::EditorContextMut,
    input::Input,
    input_mapper::{InputMap, InputMapper},
};

pub struct TextCommandMode;

impl TextCommandMode {
    pub const NAME: &'static str = "text-command";
}

impl InputMap for TextCommandMode {
    fn convert_input_to_command(
        &self,
        input: Input,
        ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        use Command::*;

        let mut im = InputMapper::default();
        im.register("<tab>", ChangeMode(TextEditMode::NAME))
            .unwrap();
        im.register("i", ChangeMode(TextEditMode::NAME)).unwrap();

        im.register("<up>", MoveSelectionUp).unwrap();
        im.register("<down>", MoveSelectionDown).unwrap();
        im.register("<left>", MoveSelectionLeft).unwrap();
        im.register("<right>", MoveSelectionRight).unwrap();

        im.convert_input_to_command(input, ctx)
    }
}

pub struct TextEditMode;

impl TextEditMode {
    pub const NAME: &'static str = "text-edit";
}

impl InputMap for TextEditMode {
    fn convert_input_to_command(
        &self,
        input: Input,
        ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        use Command::*;

        let mut im = InputMapper::default();

        im.register("<tab>", ChangeMode(TextCommandMode::NAME))
            .unwrap();

        im.register_char_insert();
        im.register("<ret>", Insert('\n')).unwrap();
        im.register("<backspace>", DeleteBeforeSelection).unwrap();
        im.register("<del>", DeleteSelection).unwrap();

        im.register("<up>", MoveSelectionUp).unwrap();
        im.register("<down>", MoveSelectionDown).unwrap();
        im.register("<left>", MoveSelectionLeft).unwrap();
        im.register("<right>", MoveSelectionRight).unwrap();

        im.convert_input_to_command(input, ctx)
    }
}
