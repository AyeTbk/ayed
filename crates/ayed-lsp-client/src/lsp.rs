use serde_derive::{Deserialize, Serialize};

/// Used for disambiguating between server Responses and Notifications.
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub id: Option<i32>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: &'static str,
    pub id: i32,
    pub method: &'static str,
    pub params: Option<RequestParams>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestParams {
    #[serde(rename = "initialize")]
    Initialize(InitializeParams),
    #[serde(rename = "textDocument/completion")]
    Completion(CompletionParams),
    TextDocumentPosition(TextDocumentPositionParams),
    Other(serde_json::Value),
}

#[derive(Serialize, Deserialize)]
pub struct InitializeParams {
    pub process_id: Option<u32>,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "rootUri")]
    pub root_uri: Option<String>,
    #[serde(rename = "workspaceFolders")]
    pub workspace_folders: Vec<WorkspaceFolder>,
}

#[derive(Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub general: Option<ClientCapabilitiesGeneral>,
    pub workspace: Option<ClientCapabilitiesWorkspace>,
    #[serde(rename = "textDocument")]
    pub text_document: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct ClientCapabilitiesGeneral {
    #[serde(rename = "positionEncodings")]
    pub position_encodings: Vec<PositionEncodingKind>,
}

#[derive(Serialize, Deserialize)]
pub struct ClientCapabilitiesWorkspace {
    #[serde(rename = "workspaceFolders")]
    pub workspace_folders: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PositionEncodingKind {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "utf-16")]
    Utf16,
    #[serde(rename = "utf-32")]
    Utf32,
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceFolder {
    pub uri: String, // TODO Uri type?
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CompletionParams {
    #[serde(rename = "positionParams")]
    #[serde(flatten)]
    pub position_params: TextDocumentPositionParams,
}

#[derive(Serialize, Deserialize)]
pub struct TextDocumentPositionParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String, // TODO DocumentUri type?
}

#[derive(Serialize, Deserialize)]
pub struct Position {
    pub line: i32,
    pub character: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<NotificationParams>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NotificationParams {
    #[serde(rename = "initialized")]
    Initialized {},
    Other(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub id: i32,
    pub result: Option<serde_json::Value>,
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: Option<ServerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(rename = "positionEncoding")]
    pub position_encoding: PositionEncodingKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: Option<String>,
}
