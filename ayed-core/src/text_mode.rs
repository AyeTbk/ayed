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
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut) -> Vec<Command> {
        use Command::*;

        let mut im = InputMapper::default();
        im.register("<tab>", ChangeMode(TextEditMode::NAME))
            .unwrap();

        im.register("i", [FlipSelectionBackward, ChangeMode(TextEditMode::NAME)])
            .unwrap();
        im.register(
            "a",
            [
                FlipSelectionForward,
                DragCursorRight,
                ChangeModeArg(TextEditMode::NAME, 1),
            ],
        )
        .unwrap();

        im.register(
            // Insert line above and enter edit mode
            "o",
            [
                MoveCursorToLineEnd,
                Insert('\n'),
                ChangeMode(TextEditMode::NAME),
            ],
        )
        .unwrap();
        im.register(
            // Insert line above and enter edit mode
            "O",
            [
                MoveCursorToLineStart,
                Insert('\n'),
                MoveCursorUp,
                ChangeMode(TextEditMode::NAME),
            ],
        )
        .unwrap();

        im.register("d", DeleteSelection).unwrap();

        im.register("<up>", MoveCursorUp).unwrap();
        im.register("<down>", MoveCursorDown).unwrap();
        im.register("<left>", MoveCursorLeft).unwrap();
        im.register("<c-left>", MoveCursorToLineStart).unwrap();
        im.register("<home>", MoveCursorToLineStart).unwrap();
        im.register("<right>", MoveCursorRight).unwrap();
        im.register("<c-right>", MoveCursorToLineEnd).unwrap();
        im.register("<end>", MoveCursorToLineEnd).unwrap();

        im.register("<s-up>", DragCursorUp).unwrap();
        im.register("<s-down>", DragCursorDown).unwrap();
        im.register("<s-left>", DragCursorLeft).unwrap();
        im.register("<s-right>", DragCursorRight).unwrap();

        im.register("<a-;>", FlipSelection).unwrap();

        im.convert_input_to_command(input, ctx)
    }
}

pub struct TextEditMode;

impl TextEditMode {
    pub const NAME: &'static str = "text-edit";
}

impl InputMap for TextEditMode {
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut) -> Vec<Command> {
        use Command::*;

        let mut im = InputMapper::default();

        im.register("<tab>", ChangeMode(TextCommandMode::NAME))
            .unwrap();

        im.register_char_insert();
        im.register("<ret>", Insert('\n')).unwrap();
        im.register("<backspace>", DeleteBeforeSelection).unwrap();
        im.register("<del>", DeleteSelection).unwrap();

        im.register("<up>", MoveCursorUp).unwrap();
        im.register("<down>", MoveCursorDown).unwrap();
        im.register("<left>", MoveCursorLeft).unwrap();
        im.register("<home>", MoveCursorToLineStart).unwrap();
        im.register("<right>", MoveCursorRight).unwrap();
        im.register("<end>", MoveCursorToLineEnd).unwrap();

        im.convert_input_to_command(input, ctx)
    }
}
