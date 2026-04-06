use crate::types::CompletionItem;

#[derive(Debug)]
pub enum Response {
    CompletionSuggestions { items: Vec<CompletionItem> },
    HoverInfo { text: String },
}
