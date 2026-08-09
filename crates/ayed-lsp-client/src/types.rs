use std::path::Path;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub uri: DocumentUri,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    pub sort_text: Option<String>,
    pub text_edit: TextEdit,
    pub additional_text_edits: Option<Vec<TextEdit>>,
    pub kind: Option<i32>,
    pub detail: Option<String>,
    pub documentation: Option<CompletionItemDocumentation>,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CompletionItemDocumentation {
    Markup(MarkupContent),
    String(String),
}

impl CompletionItemDocumentation {
    pub fn text(&self) -> &str {
        match self {
            Self::Markup(mc) => &mc.value,
            Self::String(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkupContent {
    kind: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishDiagnosticsParams {
    pub uri: DocumentUri,
    pub version: Option<i32>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<i32>, // type DiagnosticSeverity
    pub code: Option<DiagnosticCode>,
    pub code_description: Option<CodeDescription>,
    pub source: Option<String>,
    pub message: String,
    pub tags: Option<Vec<i32>>, // type DiagnosticTag
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
    // pub data: Option<Value>,
}

pub struct DiagnosticSeverity;
impl DiagnosticSeverity {
    pub const ERROR: i32 = 1;
    pub const WARNING: i32 = 2;
    pub const INFORMATION: i32 = 3;
    pub const HINT: i32 = 4;
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DiagnosticCode {
    Integer(i32),
    String(String),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDescription {
    pub href: String,
}

pub struct DiagnosticTag;
impl DiagnosticTag {
    pub const UNNECESSARY: i32 = 1;
    pub const DEPRECATED: i32 = 2;
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRelatedInformation {
    pub location: Location,
    pub message: String,
}

/// Id for client side completion items identification. Not in the spec.
#[derive(Debug, Default)]
pub struct CompletionItemId {
    pub idx: u32,
    pub generation: u32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureHelp {
    pub signatures: Vec<SignatureInformation>,
    pub active_signature: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureInformation {
    pub label: String,
    pub documentation: Option<Documentation>,
    pub parameters: Option<Vec<ParameterInformation>>,
    pub active_parameter: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParameterInformation {
    pub label: (u32, u32),
    pub documentation: Option<Documentation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
pub enum Documentation {
    String(String),
    MarkupContent { kind: String, value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
