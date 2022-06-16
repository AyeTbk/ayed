pub enum Command {
    Insert(char),
    DeleteBeforeSelection,
    DeleteSelection,
    MoveSelectionUp,
    MoveSelectionDown,
    MoveSelectionLeft,
    MoveSelectionRight,
}
