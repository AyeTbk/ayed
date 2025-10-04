use crate::position::Position;

#[derive(Default)]
pub struct Suggestions {
    /// The position in the buffer where the cursor should be for the
    /// suggestion box to show up. Used to show hide the box when appropriate.
    pub prompt_suggestion_cursor_position: Option<Position>,
    pub items: Vec<String>,
    /// Selected item index, 1 based, where 0 means none.
    pub selected_item: i32,
    /// The original symbols for all the active view cursors.
    pub original_symbols: Vec<String>,
    /// The start position of the primary cursor's original symbol.
    pub original_symbol_start: Position,
}
