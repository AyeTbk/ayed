use crate::types::Location;

#[derive(Debug)]
pub enum Response {
    CompletionSuggestionsAvailable,
    CompletionSuggestionResolved { idx: u32 }, // TODO remove, this is dead code, i swaerr
    SignatureHelp { text: String },
    HoverInfo { text: String },
    GoToDefinitionInfo { locations: Vec<Location> },
}
