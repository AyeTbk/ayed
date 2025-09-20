use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Option<RequestParams>,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestParams {
    initialize(InitializeParams),
}

#[derive(Serialize, Deserialize)]
pub struct InitializeParams {
    pub process_id: Option<u32>,
    pub capabilities: ClientCapabilities,
    pub workspace_folders: Vec<WorkspaceFolder>,
}

#[derive(Serialize, Deserialize)]
pub struct ClientCapabilities {
    pub general: Option<ClientCapabilitiesGeneral>,
}

#[derive(Serialize, Deserialize)]
pub struct ClientCapabilitiesGeneral {
    #[serde(rename = "positionEncodings")]
    pub position_encodings: Vec<PositionEncodingKind>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: u64,
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
