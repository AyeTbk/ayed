//! Handles the hierarchical input to command conversion process.
// Hierarchy:
// combo|global-editor-mode

use std::collections::HashMap;

use crate::{
    command::{Command, CoreCommand, EditorCommand},
    input::Input,
    input_mapper::InputMapper,
    state::State,
};

pub struct InputManager {
    global_mapper: InputMapper,
    combo_mappers: HashMap<String, InputMapper>,
    editor_mappers: HashMap<String, EditorInputMapper>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            global_mapper: InputMapper::new(),
            combo_mappers: Default::default(),
            editor_mappers: Default::default(),
        }
    }

    pub fn combo_mapping(&self, combo: &str) -> Vec<(Input, String)> {
        if let Some(combo_mapper) = self.combo_mappers.get(combo) {
            combo_mapper
                .ordered_inputs()
                .into_iter()
                .map(|(input, desc)| (input, desc.unwrap_or_default()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn convert_input(&self, input: Input, state: &State) -> Vec<Command> {
        self.convert_input_with_editor_mode(
            input,
            &state.active_editor_name,
            &state.active_mode_name,
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
        if let Some(combo_mode) = &state.active_combo_mode_name {
            // Combos dont cascade down. Failure to convert shouldn't recover. The combo panel should just be dismissed.
            if let Some(combo_mapper) = self.combo_mappers.get(combo_mode) {
                return combo_mapper.convert_input(input, state);
            } else {
                return vec![];
            }
        }

        let mut commands = self.global_mapper.convert_input(input, state);
        if commands.is_empty() {
            if let Some(editor_mapper) = self.editor_mappers.get(editor) {
                commands = editor_mapper.mapper.convert_input(input, state);
                if commands.is_empty() {
                    if let Some(mode_mappers) = editor_mapper.mode_mappers.get(mode) {
                        return mode_mappers.convert_input(input, state);
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
    use CoreCommand::*;
    use EditorCommand::*;
    let mut manager = InputManager::new();

    let set_edit_mode = || SetEditorMode("edit".into());
    let set_command_mode = || SetEditorMode("command".into());

    // Global
    manager
        .global_mapper
        .register(":", ShowModeLinePrompt)
        .unwrap();

    // Editors
    manager.editor_mappers.insert("text".into(), {
        let mut im = InputMapper::new();
        register_cursor_movement_inputs(&mut im).unwrap();
        EditorInputMapper {
            mapper: im,
            mode_mappers: vec![
                ("command", {
                    let mut im = InputMapper::new();
                    im.register("w", ShowWarpdrive).unwrap();
                    im.register("<space>", SetComboMode("user".into())).unwrap();
                    im.register("<tab>", set_edit_mode()).unwrap();

                    im.register("i", [Editor(FlipSelectionBackward), Core(set_edit_mode())])
                        .unwrap();
                    im.register(
                        "<s-i>",
                        [
                            Editor(FlipSelectionBackward),
                            Core(set_edit_mode()),
                            Editor(AnchorNext),
                        ],
                    )
                    .unwrap();
                    im.register(
                        "a",
                        [
                            Editor(FlipSelectionForward),
                            Core(set_edit_mode()),
                            Editor(AnchorNext),
                            Editor(MoveCursorRight),
                        ],
                    )
                    .unwrap();
                    im.register(
                        "<s-a>",
                        [
                            Editor(FlipSelectionForward),
                            Core(set_edit_mode()),
                            Editor(AnchorNext),
                            Editor(MoveCursorRight),
                        ],
                    )
                    .unwrap();

                    im.register(
                        // Insert line below and enter edit mode
                        "o",
                        [
                            Editor(MoveCursorToLineEnd),
                            Editor(Insert('\n')),
                            Core(set_edit_mode()),
                        ],
                    )
                    .unwrap();
                    im.register(
                        // Insert line above and enter edit mode
                        "O",
                        [
                            Editor(MoveCursorToLineStart),
                            Editor(Insert('\n')),
                            Editor(MoveCursorUp),
                            Core(set_edit_mode()),
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
                    im.register("<tab>", set_command_mode()).unwrap();
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

    manager.editor_mappers.insert("control".into(), {
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

    // Combos
    manager.combo_mappers.insert("user".into(), {
        let mut im = InputMapper::new();
        im.register_with_description("s", WriteBuffer, "save buffer")
            .unwrap();
        im.register_with_description("f", SetComboMode("file".into()), "file commands")
            .unwrap();
        im.register_with_description("e", Noop, "nothing").unwrap();
        im.register_with_description("<s-e>", Noop, "also nothing")
            .unwrap();
        im.register_with_description(
            "c",
            [
                MoveCursorToLineStartSmart,
                Insert('/'),
                Insert('/'),
                Insert(' '),
            ],
            "janky comment line",
        )
        .unwrap();
        im
    });
    manager.combo_mappers.insert("file".into(), {
        let mut im = InputMapper::new();
        im.register_with_description("s", Noop, "does nothing")
            .unwrap();
        im
    });

    manager
}

fn register_cursor_movement_inputs(im: &mut InputMapper) -> Result<(), ()> {
    use EditorCommand::*;

    im.register("<up>", MoveCursorUp)?;
    im.register("<s-up>", [AnchorNext, MoveCursorUp])?;
    im.register("<down>", MoveCursorDown)?;
    im.register("<s-down>", [AnchorNext, MoveCursorDown])?;

    im.register("<left>", MoveCursorLeft)?;
    im.register("<s-left>", [AnchorNext, MoveCursorLeft])?;
    im.register("<c-left>", MoveCursorToLeftSymbol)?;
    im.register("<cs-left>", [AnchorNext, MoveCursorToLeftSymbol])?;
    im.register("<right>", MoveCursorRight)?;
    im.register("<s-right>", [AnchorNext, MoveCursorRight])?;
    im.register("<c-right>", MoveCursorToRightSymbol)?;
    im.register("<cs-right>", [AnchorNext, MoveCursorToRightSymbol])?;

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
