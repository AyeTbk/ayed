#[derive(Debug)]
pub enum Response {
    CompletionSuggestions { items: Vec<String> },
    HoverInfo { text: String },
}
