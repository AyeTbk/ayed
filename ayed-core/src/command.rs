#[derive(Debug, Clone, Copy)]
pub enum Command {
    // Ayed commands
    ChangeMode(&'static str),

    // Text edit commands
    Insert(char),
    DeleteSelection,
    DeleteBeforeSelection, // TODO change to DeleteBeforeCursor

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

    FlipSelection,
    FlipSelectionForward,
    FlipSelectionBackward,
}
