use std::collections::BTreeMap;

use crate::{position::Position, state::text_buffer::TextEdit};

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
    pub source_items: BTreeMap<CompletionSource, CompletionSourceData>,
    /// The last completion inverse edits, kept to undo the edits when cycling through choices.
    pub last_completion_inverse_edits: Option<Vec<TextEdit>>,
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum CompletionSource {
    Buffer,
    Lsp,
}

#[derive(Debug, Default, Clone)]
pub struct CompletionSourceData {
    pub items: Vec<CompletionItem>,
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
    pub extra_edits: Vec<TextEdit>,
    pub kind: CompletionItemKind,
    pub source: CompletionSource,
    pub source_idx: u32,
    pub type_annotation: Option<String>,
    pub documentation: Option<String>,
}
