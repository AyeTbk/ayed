use crate::lsp;

pub enum Request {
    SuggestCompletion { file: File, position: Position },
    Hover { file: File, position: Position },
}

/// Absolute path to a file
pub struct File(pub String);

/// Position within a file, as a (column, row) tuple
pub struct Position(pub i32, pub i32);

pub fn make_lsp_request(
    id: i32,
    method: &'static str,
    params: Option<lsp::RequestParams>,
) -> lsp::Request {
    lsp::Request {
        jsonrpc: "2.0",
        id,
        method,
        params,
    }
}

pub fn make_lsp_notification(
    method: impl Into<String>,
    params: Option<lsp::NotificationParams>,
) -> lsp::Notification {
    lsp::Notification {
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
    }
}

pub fn convert_request_to_lsp(req: Request) -> lsp::Request {
    match req {
        Request::SuggestCompletion { file, position } => make_lsp_request(
            0,
            "textDocument/completion",
            Some(lsp::RequestParams::Completion(lsp::CompletionParams {
                position_params: lsp::TextDocumentPositionParams {
                    text_document: convert_file_to_lsp(file),
                    position: convert_position_to_lsp(position),
                },
            })),
        ),
        Request::Hover { file, position } => make_lsp_request(
            0,
            "textDocument/hover",
            Some(lsp::RequestParams::TextDocumentPosition(
                convert_file_position_to_lsp(file, position),
            )),
        ),
    }
}

pub fn convert_file_position_to_lsp(
    file: File,
    position: Position,
) -> lsp::TextDocumentPositionParams {
    lsp::TextDocumentPositionParams {
        text_document: convert_file_to_lsp(file),
        position: convert_position_to_lsp(position),
    }
}

pub fn convert_file_to_lsp(file: File) -> lsp::TextDocumentIdentifier {
    lsp::TextDocumentIdentifier {
        uri: format!("file://{}", file.0),
    }
}

pub fn convert_position_to_lsp(Position(column, row): Position) -> lsp::Position {
    lsp::Position {
        line: row,
        character: column,
    }
}
