use crate::selection::Position;

#[derive(Debug, Clone, Copy)]
pub enum Command {
    // Ayed commands
    ChangeMode(&'static str),

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
    MoveCursorToLineEnd,

    ShrinkSelectionToCursor,
    FlipSelection,
    FlipSelectionForward,
    FlipSelectionBackward,

    DuplicateSelectionAbove,
    DuplicateSelectionBelow,
}
