use std::path::Path;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    pub sort_text: Option<String>,
    pub text_edit: TextEdit,
    pub additional_text_edits: Option<Vec<TextEdit>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextDocumentIdentifier {
    pub uri: DocumentUri,
}

impl TextDocumentIdentifier {
    pub fn new(absolute_filepath: &Path) -> Self {
        Self {
            uri: DocumentUri::new(absolute_filepath),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VersionedTextDocumentIdentifier {
    pub uri: DocumentUri,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
    pub uri: DocumentUri,
    pub language_id: String,
    pub version: i32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentPositionParams {
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentUri(pub String);

impl DocumentUri {
    pub fn new(absolute_filepath: &Path) -> Self {
        debug_assert!(absolute_filepath.is_absolute());
        Self(format!("file://{}", absolute_filepath.to_string_lossy()))
    }
}

pub struct LanguageId(Never);
enum Never {}
impl LanguageId {
    pub const RUST: &str = "rs";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}
