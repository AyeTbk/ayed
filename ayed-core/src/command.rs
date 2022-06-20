pub enum Command {
    ChangeMode(&'static str),
    Insert(char),
    DeleteBeforeSelection,
    DeleteSelection,
    MoveSelectionUp,
    MoveSelectionDown,
    MoveSelectionLeft,
    MoveSelectionRight,
}
