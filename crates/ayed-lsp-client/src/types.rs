use std::path::Path;

use serde_derive::Serialize;

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

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}
