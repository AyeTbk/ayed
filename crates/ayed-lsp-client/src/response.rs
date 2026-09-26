use crate::types::{Diagnostic, DocumentUri, Location, TextEdit};

#[derive(Debug)]
pub enum Response {
    CompletionSuggestionsAvailable,
    CompletionSuggestionResolved {
        idx: u32,
    }, // TODO remove, this is dead code, i swaerr
    SignatureHelp {
        text: String,
    },
    HoverInfo {
        text: String,
    },
    GoToDefinitionInfo {
        locations: Vec<Location>,
    },
    FileDiagnostics {
        file: DocumentUri,
        diagnostics: Vec<Diagnostic>,
    },
    FormatDocumentEdits {
        file: DocumentUri,
        text_edits: Vec<TextEdit>,
    },
}
