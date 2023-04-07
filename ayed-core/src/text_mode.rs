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
            // Insert line below and enter edit mode
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

        register_cursor_movement_inputs(&mut im).unwrap();

        im.register("<;>", ShrinkSelectionToCursor).unwrap();
        im.register("<a-;>", FlipSelection).unwrap();

        im.register("<sa-c>", DuplicateSelectionAbove).unwrap();
        im.register("C", DuplicateSelectionBelow).unwrap();

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

        register_cursor_movement_inputs(&mut im).unwrap();

        im.register("<del>", DeleteCursor).unwrap();
        im.register("<backspace>", DeleteBeforeCursor).unwrap();

        im.convert_input_to_command(input, ctx)
    }
}

fn register_cursor_movement_inputs(im: &mut InputMapper) -> Result<(), ()> {
    use Command::*;

    im.register("<up>", MoveCursorUp)?;
    im.register("<s-up>", DragCursorUp)?;
    im.register("<down>", MoveCursorDown)?;
    im.register("<s-down>", DragCursorDown)?;

    im.register("<left>", MoveCursorLeft)?;
    im.register("<s-left>", DragCursorLeft)?;
    im.register("<c-left>", MoveCursorToLeftSymbol)?;
    im.register("<cs-left>", DragCursorToLeftSymbol)?;
    im.register("<home>", MoveCursorToLineStart)?;
    im.register("<s-home>", DragCursorToLineStart)?;

    im.register("<right>", MoveCursorRight)?;
    im.register("<s-right>", DragCursorRight)?;
    im.register("<c-right>", MoveCursorToRightSymbol)?;
    im.register("<cs-right>", DragCursorToRightSymbol)?;
    im.register("<end>", MoveCursorToLineEnd)?;
    im.register("<s-end>", DragCursorToLineEnd)?;

    Ok(())
}
