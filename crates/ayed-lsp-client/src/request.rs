use serde_json::{Value, json};

use crate::types::{
    CompletionItem, DiagnosticTag, Position, TextDocumentIdentifier, TextDocumentPositionParams,
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum RequestType {
    #[default]
    Initialize,
    SuggestCompletion,
    ResolveCompletion,
    SignatureHelp,
    Hover,
    Definition,
}

#[derive(Debug)]
pub struct PendingRequest {
    pub id: i32,
    pub typ: RequestType,
    pub json: Value,
}

const JSON_RPC_VERSION: &str = "2.0";

pub fn build_initialize_request_json(request_id: i32) -> Value {
    json!({
        "jsonrpc": JSON_RPC_VERSION,
        "id": request_id,
        "method": "initialize",
        "params": {
            "processId": Value::Null,
            "capabilities": {
                "general": Value::Null,
                "workspace": Value::Null,
                "textDocument": {
                    "completion": {
                        "completionItem": {
                            "documentationFormat": ["plaintext", "markdown"],
                            "resolveSupport": {
                                "properties": [
                                    "additionalTextEdits",
                                    "documentation",
                                    // "detail", // don't async resolve this one for now, for simplicity.
                                ]
                            }
                        }
                    },
                    "signatureHelp": {
                        "signatureInformation": {
                            "documentationFormat": ["plaintext"],
                            "parameterInformation": {
                                "labelOffsetSupport": true,
                            },
                            "activeParameterSupport": true,
                        }
                    },
                    "publishDiagnosticsClientCapabilities": {
                        "relatedInformation": true,
                        "tagSupport": {
                            "valueSet": [DiagnosticTag::UNNECESSARY, DiagnosticTag::DEPRECATED],
                        },
                        "versionSupport": true,
                        "codeDescriptionSupport": true,
                        // "dataSupport": true,
                    }
                },
            },
            "rootUri": Value::Null,
            "workspaceFolders": [],
        },
    })
}

pub fn build_suggest_completion_request_json(
    request_id: i32,
    text_document: TextDocumentIdentifier,
    position: Position,
) -> Value {
    json!({
        "jsonrpc": JSON_RPC_VERSION,
        "id": request_id,
        "method": "textDocument/completion",
        "params": TextDocumentPositionParams {
            text_document: text_document,
            position: position,
        },
    })
}

pub fn build_resolve_completion_request_json(request_id: i32, item: &CompletionItem) -> Value {
    json!({
        "jsonrpc": JSON_RPC_VERSION,
        "id": request_id,
        "method": "completionItem/resolve",
        "params": item,
    })
}

pub fn build_signature_help_request_json(
    request_id: i32,
    text_document: TextDocumentIdentifier,
    position: Position,
) -> Value {
    json!({
        "jsonrpc": JSON_RPC_VERSION,
        "id": request_id,
        "method": "textDocument/signatureHelp",
        "params": TextDocumentPositionParams {
            text_document: text_document,
            position: position,
        },
    })
}

pub fn build_hover_request_json(
    request_id: i32,
    text_document: TextDocumentIdentifier,
    position: Position,
) -> Value {
    json!({
        "jsonrpc": JSON_RPC_VERSION,
        "id": request_id,
        "method": "textDocument/hover",
        "params": TextDocumentPositionParams {
            text_document: text_document,
            position: position,
        },
    })
}

pub fn build_definition_request_json(
    request_id: i32,
    text_document: TextDocumentIdentifier,
    position: Position,
) -> Value {
    json!({
        "jsonrpc": JSON_RPC_VERSION,
        "id": request_id,
        "method": "textDocument/definition",
        "params": TextDocumentPositionParams {
            text_document: text_document,
            position: position,
        },
    })
}
