use std::collections::HashMap;

use crate::position::Position;

#[derive(Default)]
pub struct Completions {
    /// The filtered and ordered items that should be presented to the user.
    pub items: Vec<CompletionItem>,
    /// Selected item index, 1 based, where 0 means none.
    pub selected_item: i32,
    /// The position in the buffer where the cursor should be for the
    /// suggestion box to show up. Used to show/hide the box when appropriate.
    pub prompt_suggestion_cursor_position: Option<Position>,
    /// The start position of the primary cursor's original symbol.
    /// Used to position the box.
    pub original_symbol_start: Position,

    /// Unfiltered items from completion sources.
    pub source_items: HashMap<CompletionSources, Vec<CompletionItem>>,
    /// The last completion inverse edits, kept to undo the edits when cycling through choices.
    pub last_completion_inverse_edits: Option<Vec<CompletionEdit>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionSources {
    Dbg,
    Lsp,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    // TODO add filter text field
    pub edit: CompletionEdit, // TODO This might just need to be a String, because the range will be ignored
    pub extra_edits: Vec<CompletionEdit>,
}

#[derive(Debug, Clone)]
pub struct CompletionEdit {
    /// A range that is [inclusive, exclusive[
    pub range: (Position, Position),
    pub text: String,
}
