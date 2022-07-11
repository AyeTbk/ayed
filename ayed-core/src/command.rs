#[derive(Debug, Clone, Copy)]
pub enum Command {
    // Ayed commands
    ChangeMode(&'static str),

    // Text edit commands
    Insert(char),
    DeleteBeforeSelection,
    DeleteSelection,

    // Selection manipulation commands
    MoveSelectionUp,
    MoveSelectionDown,
    MoveSelectionLeft,
    MoveSelectionRight,
}
