use crate::types::{CompletionItem, Location};

#[derive(Debug)]
pub enum Response {
    CompletionSuggestions { items: Vec<CompletionItem> },
    HoverInfo { text: String },
    GotoDefinitionInfo { locations: Vec<Location> },
}
