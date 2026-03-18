use serde_json::{Value, json};

use crate::types::{TextDocumentIdentifier, TextDocumentItem, VersionedTextDocumentIdentifier};

#[derive(Debug)]
pub enum Notification {
    Initialized,
    TextDocumentDidOpen {
        text_document: TextDocumentItem,
    },
    TextDocumentDidChange {
        text_document: VersionedTextDocumentIdentifier,
        new_content: String,
    },
    TextDocumentDidClose {
        text_document: TextDocumentIdentifier,
    },
}

pub fn convert_notification_to_json(notif: Notification) -> Value {
    const JSON_RPC_VERSION: &str = "2.0";
    use Notification as N;
    match notif {
        N::Initialized => json!({
            "jsonrpc": JSON_RPC_VERSION,
            "method": "initialized",
        }),
        N::TextDocumentDidOpen { text_document } => json!({
            "jsonrpc": JSON_RPC_VERSION,
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": text_document,
            },
        }),
        N::TextDocumentDidChange {
            text_document,
            new_content,
        } => json!({
            "jsonrpc": JSON_RPC_VERSION,
            "method": "textDocument/didChange",
            "params": {
                "textDocument": text_document,
                "contentChanges": [{
                    "text": new_content,
                }],
            },
        }),
        N::TextDocumentDidClose { text_document } => json!({
            "jsonrpc": JSON_RPC_VERSION,
            "method": "textDocument/didClose",
            "params": {
                "textDocument": text_document,
            },
        }),
    }
}
