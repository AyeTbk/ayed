use std::collections::BTreeMap;

use crate::position::Position;

#[derive(Default)]
pub struct Completions {
    /// The filtered and ordered items that should be presented to the user.
    pub items: Vec<CompletionItem>,
    /// Selected item index, 1 based, where 0 means none.
    pub selected_item: i32,
    /// The start position of the primary cursor's original symbol.
    /// Used to position the box.
    pub original_symbol_start: Position,

    /// Unfiltered items from completion sources.
    pub source_items: BTreeMap<CompletionSource, Vec<CompletionItem>>,
    /// The last completion inverse edits, kept to undo the edits when cycling through choices.
    pub last_completion_inverse_edits: Option<Vec<CompletionEdit>>,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum CompletionSource {
    Buffer,
    Lsp,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum CompletionItemKind {
    Variable,
    Function,
    Member,
    Type,
    Interface,
    Module,
    Keyword,
    Plaintext,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    // TODO add filter text field
    pub text: String,
    pub extra_edits: Vec<CompletionEdit>,
    pub kind: CompletionItemKind,
    pub source: CompletionSource,
    pub type_annotation: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionEdit {
    /// A range that is [inclusive, exclusive[
    pub range: (Position, Position),
    pub text: String,
}
