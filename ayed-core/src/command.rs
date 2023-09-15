use crate::selection::Position;

#[derive(Debug, Clone)]
pub enum Command {
    Core(CoreCommand),
    Editor(EditorCommand),
}

impl From<CoreCommand> for Command {
    fn from(value: CoreCommand) -> Self {
        Command::Core(value)
    }
}

impl From<EditorCommand> for Command {
    fn from(value: EditorCommand) -> Self {
        Command::Editor(value)
    }
}

#[derive(Debug, Clone)]
pub enum CoreCommand {
    ShowModeLinePrompt,
    ShowWarpdrive,
    SetComboMode(String),
    SetEditorMode(&'static str),
    EditFile(String),
}

#[derive(Debug, Clone, Copy)]
pub enum EditorCommand {
    Noop, // Does nothing.

    // Text edit commands
    Insert(char),
    DeleteSelection,
    DeleteCursor,
    DeleteBeforeCursor,

    // Selection manipulation commands
    AnchorNext,
    AnchorDown,
    AnchorUp,

    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,

    MoveCursorTo(u32, u32),
    SetSelection { cursor: Position, anchor: Position },

    MoveCursorToLineStart,
    MoveCursorToLineStartSmart, // Flip flop between line start and first non white char.
    MoveCursorToLineEnd,
    MoveCursorToLineEndSmart, // Flip flop between line end and last char of line.

    DismissSecondarySelections,
    ShrinkSelectionToCursor,
    FlipSelection,
    FlipSelectionForward,
    FlipSelectionBackward,

    DuplicateSelectionAbove,
    DuplicateSelectionBelow,
}
