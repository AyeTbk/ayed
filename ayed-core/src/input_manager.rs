//! Handles the hierarchical input to command conversion process.
// Hierarchy: global-editor-mode-[format]

use std::collections::HashMap;

use crate::{command::Command, input::Input, input_mapper::InputMapper, state::State};

pub struct InputManager {
    global_mapper: InputMapper,
    editor_mappers: HashMap<&'static str, EditorInputMapper>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            global_mapper: InputMapper::new(),
            editor_mappers: Default::default(),
        }
    }

    pub fn convert_input(&self, input: Input, state: &State) -> Vec<Command> {
        self.convert_input_with_editor_mode(
            input,
            state.active_editor_name,
            state.active_mode_name,
            state,
        )
    }

    pub fn convert_input_with_editor_mode(
        &self,
        input: Input,
        editor: &str,
        mode: &str,
        state: &State,
    ) -> Vec<Command> {
        let mut commands = self.global_mapper.convert_input(input, state);
        if commands.is_empty() {
            if let Some(editor_mapper) = self.editor_mappers.get(editor) {
                commands = editor_mapper.mapper.convert_input(input, state);
                if commands.is_empty() {
                    if let Some(mode_mappers) = editor_mapper.mode_mappers.get(mode) {
                        commands = mode_mappers.convert_input(input, state);
                    }
                }
            }
        }
        commands
    }
}

struct EditorInputMapper {
    mapper: InputMapper,
    mode_mappers: HashMap<&'static str, InputMapper>,
}

pub fn initialize_input_manager() -> InputManager {
    use Command::*;
    let mut manager = InputManager::new();

    manager.editor_mappers.insert("text", {
        let mut im = InputMapper::new();
        register_cursor_movement_inputs(&mut im).unwrap();
        EditorInputMapper {
            mapper: im,
            mode_mappers: vec![
                ("command", {
                    let mut im = InputMapper::new();
                    im.register("<tab>", ChangeMode("edit")).unwrap();

                    im.register("i", [FlipSelectionBackward, ChangeMode("edit")])
                        .unwrap();
                    im.register(
                        "<s-i>",
                        [FlipSelectionBackward, ChangeMode("edit"), AnchorNext],
                    )
                    .unwrap();
                    im.register(
                        "a",
                        [
                            FlipSelectionForward,
                            ChangeMode("edit"),
                            AnchorNext,
                            MoveCursorRight,
                        ],
                    )
                    .unwrap();
                    im.register(
                        "<s-a>",
                        [
                            FlipSelectionForward,
                            ChangeMode("edit"),
                            AnchorNext,
                            MoveCursorRight,
                        ],
                    )
                    .unwrap();

                    im.register(
                        // Insert line below and enter edit mode
                        "o",
                        [MoveCursorToLineEnd, Insert('\n'), ChangeMode("edit")],
                    )
                    .unwrap();
                    im.register(
                        // Insert line above and enter edit mode
                        "O",
                        [
                            MoveCursorToLineStart,
                            Insert('\n'),
                            MoveCursorUp,
                            ChangeMode("edit"),
                        ],
                    )
                    .unwrap();

                    im.register("d", DeleteSelection).unwrap();

                    im.register(
                        "x",
                        [
                            FlipSelectionForward,
                            AnchorNext,
                            MoveCursorToLineEnd,
                            FlipSelectionBackward,
                            AnchorNext,
                            MoveCursorToLineStart,
                            FlipSelectionForward,
                        ],
                    )
                    .unwrap();

                    im.register(";", ShrinkSelectionToCursor).unwrap();
                    im.register("<a-;>", DismissSecondarySelections).unwrap();
                    im.register("'", FlipSelection).unwrap();
                    im.register("<a-'>", FlipSelectionForward).unwrap();

                    im.register("<sa-c>", DuplicateSelectionAbove).unwrap();
                    im.register("<s-c>", DuplicateSelectionBelow).unwrap();

                    im
                }),
                ("edit", {
                    let mut im = InputMapper::new();
                    im.register("<tab>", ChangeMode("command")).unwrap();
                    im.register("<del>", DeleteCursor).unwrap();
                    im.register("<backspace>", DeleteBeforeCursor).unwrap();
                    im.register_char_insert();
                    im
                }),
            ]
            .into_iter()
            .collect(),
        }
    });

    manager.editor_mappers.insert("control", {
        EditorInputMapper {
            mapper: InputMapper::new(),
            mode_mappers: vec![("line", {
                let mut im = InputMapper::new();
                register_cursor_movement_inputs(&mut im).unwrap();
                im.register("<tab>", Noop).unwrap();
                im.register("<del>", DeleteCursor).unwrap();
                im.register("<backspace>", DeleteBeforeCursor).unwrap();
                im.register_char_insert();
                im
            })]
            .into_iter()
            .collect(),
        }
    });

    manager
}

fn register_cursor_movement_inputs(im: &mut InputMapper) -> Result<(), ()> {
    use Command::*;

    im.register("<up>", MoveCursorUp)?;
    im.register("<s-up>", [AnchorNext, MoveCursorUp])?;
    im.register("<down>", MoveCursorDown)?;
    im.register("<s-down>", [AnchorNext, MoveCursorDown])?;

    im.register("<left>", MoveCursorLeft)?;
    im.register("<s-left>", [AnchorNext, MoveCursorLeft])?;
    // im.register("<c-left>", MoveCursorToLeftSymbol)?;
    // im.register("<cs-left>", [AnchorNext, MoveCursorToLeftSymbol])?;
    im.register("<right>", MoveCursorRight)?;
    im.register("<s-right>", [AnchorNext, MoveCursorRight])?;
    // im.register("<c-right>", MoveCursorToRightSymbol)?;
    // im.register("<cs-right>", [AnchorNext, MoveCursorToRightSymbol])?;

    im.register("<home>", MoveCursorToLineStartSmart)?;
    im.register("<s-home>", [AnchorNext, MoveCursorToLineStartSmart])?;
    im.register("<a-home>", MoveCursorToLineStart)?;
    im.register("<sa-home>", [AnchorNext, MoveCursorToLineStart])?;
    im.register("<end>", MoveCursorToLineEndSmart)?;
    im.register("<s-end>", [AnchorNext, MoveCursorToLineEndSmart])?;
    im.register("<a-end>", MoveCursorToLineEnd)?;
    im.register("<sa-end>", [AnchorNext, MoveCursorToLineEnd])?;

    Ok(())
}
