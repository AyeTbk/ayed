#[derive(Debug, Clone, Copy)]
pub enum Command {
    // Ayed commands
    ChangeMode(&'static str),
    ChangeModeArg(&'static str, usize),

    // Text edit commands
    Insert(char),
    DeleteSelection,
    DeleteCursor,
    DeleteBeforeCursor,

    // Selection manipulation commands
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorLeft,
    MoveCursorRight,
    DragCursorUp,
    DragCursorDown,
    DragCursorLeft,
    DragCursorRight,

    MoveCursorToLineStart,
    MoveCursorToLineEnd,
    DragCursorToLineStart,
    DragCursorToLineEnd,

    ShrinkSelectionToCursor,
    FlipSelection,
    FlipSelectionForward,
    FlipSelectionBackward,
}
