use std::path::Path;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub uri: DocumentUri,
    pub range: Range,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    pub sort_text: Option<String>,
    pub text_edit: TextEdit,
    pub additional_text_edits: Option<Vec<TextEdit>>,
    pub kind: Option<i32>,
    pub detail: Option<String>,
    pub documentation: Option<Value>,
}

pub fn extract_completion_item_documentation(value: Option<Value>) -> Option<String> {
    let extract_string = |v: Value| {
        let Value::String(s) = v else { return None; };
        return Some(s);
    };
    match value? {
        Value::String(s) => Some(s),
        Value::Object(mut map) => extract_string(map.remove("value")?),
        _ => None,
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}
