use crate::selection::Selection;

#[derive(Debug, Clone)]
pub enum Command {
    Core(CoreCommand),
    Editor(EditorCommand),
    ScriptedCommand(String),
}

impl Command {
    pub fn new(scripted_command: impl Into<String>) -> Self {
        Self::ScriptedCommand(scripted_command.into())
    }
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
    SetEditorMode(String),
    EditFile(String),
    WriteBuffer,
    WriteBufferQuit,
    Quit,
    ForceQuit,
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

    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,

    SetSelection(Selection),

    MoveCursorToLeftSymbol,
    MoveCursorToRightSymbol,
    SelectLeftSymbol,
    SelectRightSymbol,

    MoveCursorToLineStart,      // Go to column 0 of current line.
    MoveCursorToLineStartSmart, // Flip flop between line start and first non white char.
    MoveCursorToLineEnd, // Go to last column of current line. Set desired column to infinity.
    MoveCursorToLineEndSmart, // Flip flop between line end and last char of line.

    DismissSecondarySelections,
    ShrinkSelectionToCursor,
    FlipSelection,
    FlipSelectionForward,
    FlipSelectionBackward,

    DuplicateSelectionAbove,
    DuplicateSelectionBelow,
}
