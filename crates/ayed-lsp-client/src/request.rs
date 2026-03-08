use serde_json::{Value, json};

use crate::types::{Position, TextDocumentIdentifier, TextDocumentPositionParams};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestType {
    Initialize,
    SuggestCompletion,
    Hover,
}

pub enum Request {
    Initialize,
    SuggestCompletion {
        text_document: TextDocumentIdentifier,
        position: Position,
    },
    Hover {
        text_document: TextDocumentIdentifier,
        position: Position,
    },
}

impl Request {
    pub fn typ(&self) -> RequestType {
        match self {
            Self::Initialize => RequestType::Initialize,
            Self::SuggestCompletion { .. } => RequestType::SuggestCompletion,
            Self::Hover { .. } => RequestType::Hover,
        }
    }
}

pub fn convert_request_to_json(req: Request, request_id: i32) -> Value {
    const JSON_RPC_VERSION: &str = "2.0";
    use Request as R;
    match req {
        R::Initialize => json!({
            "jsonrpc": JSON_RPC_VERSION,
            "id": request_id,
            "method": "initialize",
            "params": {
                "processId": Value::Null,
                "capabilities": {
                    "general": Value::Null,
                    "workspace": Value::Null,
                    "text_document": Value::Null,
                },
                "root_uri": Value::Null,
                "workspace_folders": [],
            },
        }),
        R::SuggestCompletion {
            text_document,
            position,
        } => json!({
            "jsonrpc": JSON_RPC_VERSION,
            "id": request_id,
            "method": "textDocument/completion",
            "params": TextDocumentPositionParams {
                text_document: text_document,
                position: position,
            },
        }),
        R::Hover {
            text_document,
            position,
        } => json!({
            "jsonrpc": JSON_RPC_VERSION,
            "id": request_id,
            "method": "textDocument/hover",
            "params": TextDocumentPositionParams {
                text_document: text_document,
                position: position,
            },
        }),
    }
}
