// Could there be a way to design a kind of "plugin" system
// or at least something modular, that would allow extending
// the editor in a less intrusive way? I feel the LSP
// functionalities would be a good candidate for that.

use std::path::Path;

use ayed_lsp_client::{
    LspClient, Notification, Request, Response,
    types::{
        DocumentUri, LanguageId, Position, TextDocumentIdentifier, TextDocumentItem,
        VersionedTextDocumentIdentifier,
    },
};

use crate::command::{CommandRegistry, helpers::focused_buffer_command};

pub fn register_lsp_commands(cr: &mut CommandRegistry) {
    cr.register("lsp-start", |_opt, ctx| {
        if ctx.state.lsp_client.is_some() {
            return Err("client already running".into());
        }

        let mut client = LspClient::new();
        client.initialize();

        ctx.state.lsp_client = Some(client);
        Ok(())
    });

    cr.register("lsp-stop", |_opt, ctx| {
        if let Some(client) = ctx.state.lsp_client.take() {
            client.shutdown();
        }
        dbg!("turned off");
        Ok(())
    });

    cr.register("lsp-poll", |_opt, ctx| {
        let Some(client) = &mut ctx.state.lsp_client else {
            return Ok(());
        };

        client.tick();

        if client.is_just_initialized() {
            // Inform server of pre-opened buffers
            for (_, buffer) in ctx.resources.buffers.iter() {
                let Some(path) = buffer.path() else { continue };
                client.queue_notification(Notification::TextDocumentDidOpen {
                    text_document: TextDocumentItem {
                        uri: DocumentUri::new(path),
                        language_id: LanguageId::RUST.to_string(),
                        version: buffer.content_version(),
                        text: buffer.content_to_string(),
                    },
                });
            }
        }

        for response in client.receive_responses() {
            match response {
                Response::HoverInfo { text } => {
                    ctx.state.hover_info = Some(text);
                }
                Response::CompletionSuggestions { items } => {
                    ctx.state.completions.items = items;
                }
            }
        }

        Ok(())
    });

    cr.register("lsp-doc-sync-open", |opt, ctx| {
        let Some(client) = &mut ctx.state.lsp_client else {
            return Ok(());
        };

        let buffer_path = Path::new(opt);
        if opt.is_empty() {
            return Ok(());
        }

        let Some(buffer_handle) = ctx.resources.buffer_with_path(buffer_path) else {
            return Err(format!("no buffer with path '{}'", opt));
        };
        let buffer = ctx.resources.buffers.get(buffer_handle);

        client.queue_notification(Notification::TextDocumentDidOpen {
            text_document: TextDocumentItem {
                uri: DocumentUri::new(buffer_path),
                language_id: LanguageId::RUST.to_string(),
                version: buffer.content_version.get(),
                text: buffer.content_to_string(),
            },
        });

        Ok(())
    });

    cr.register("lsp-doc-sync-change", |opt, ctx| {
        let Some(client) = &mut ctx.state.lsp_client else {
            return Ok(());
        };

        let buffer_path = Path::new(opt);
        if opt.is_empty() {
            return Ok(());
        }

        let Some(buffer_handle) = ctx.resources.buffer_with_path(buffer_path) else {
            return Err(format!("no buffer with path '{}'", opt));
        };
        let buffer = ctx.resources.buffers.get(buffer_handle);

        client.queue_notification(Notification::TextDocumentDidChange {
            text_document: VersionedTextDocumentIdentifier {
                uri: DocumentUri::new(buffer_path),
                version: buffer.content_version.get(),
            },
            new_content: buffer.content_to_string(),
        });

        Ok(())
    });

    cr.register("lsp-doc-sync-close", |opt, ctx| {
        let Some(client) = &mut ctx.state.lsp_client else {
            return Ok(());
        };

        let buffer_path = Path::new(opt);
        if opt.is_empty() {
            return Ok(());
        }

        if ctx.resources.buffer_with_path(buffer_path).is_none() {
            return Err(format!("no buffer with path '{}'", opt));
        };

        client.queue_notification(Notification::TextDocumentDidClose {
            text_document: TextDocumentIdentifier {
                uri: DocumentUri::new(buffer_path),
            },
        });

        Ok(())
    });

    cr.register(
        "lsp-hover",
        focused_buffer_command(|_opt, ctx| {
            let Some(client) = &mut ctx.state.lsp_client else {
                return Err("lsp client not started".into());
            };

            let Some(path) = ctx.buffer.path() else {
                return Err("save the file before you can hover".into());
            };

            let cursor = ctx.selections.primary().cursor;

            client.queue_request(Request::Hover {
                text_document: TextDocumentIdentifier::new(path),
                position: cursor.into(),
            });

            Ok(())
        }),
    );

    cr.register(
        "lsp-completions",
        focused_buffer_command(|_opt, ctx| {
            let Some(client) = &mut ctx.state.lsp_client else {
                return Err("lsp client not started".into());
            };

            let Some(path) = ctx.buffer.path() else {
                return Err("save the file before you can ask for completions".into());
            };

            let cursor = ctx.selections.primary().cursor;

            client.queue_request(Request::SuggestCompletion {
                text_document: TextDocumentIdentifier::new(path),
                position: cursor.into(),
            });

            Ok(())
        }),
    );
}

impl From<crate::position::Position> for Position {
    fn from(value: crate::position::Position) -> Self {
        Self {
            line: value.row.try_into().unwrap(),
            character: value.column.try_into().unwrap(),
        }
    }
}
