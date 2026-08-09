// Could there be a way to design a kind of "plugin" system
// or at least something modular, that would allow extending
// the editor in a less intrusive way? I feel the LSP
// functionalities would be a good candidate for that.

use std::path::{Path, PathBuf};

use ayed_lsp_client::{
    LspClient, Notification, Response,
    types::{
        DocumentUri, LanguageId, TextDocumentIdentifier, TextDocumentItem,
        VersionedTextDocumentIdentifier,
    },
};
use log::{debug, info};

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command},
    position::{Column, Position, Row},
    range::Range,
    selection::Selection,
    state::{
        CompletionItem, CompletionItemKind, CompletionSource, Diagnostic, DiagnosticKind,
        DiagnosticSource, TextEdit,
    },
};

pub fn register_lsp_commands(cr: &mut CommandRegistry) {
    cr.register("lsp-start", "nodoc", |_opt, ctx| {
        let Ok(server_command) = ctx.state.config.get_entry_value("lsp", "server-command") else {
            info!("no lsp server command set, skipping lsp-start");
            return Ok(());
        };

        if ctx.state.lsp_client.is_some() {
            return Ok(());
        }

        let mut client = LspClient::new(server_command, ctx.state.is_async_task_ready.clone());
        client.initialize();
        info!("lsp server '{server_command}' started");

        ctx.state.lsp_client = Some(client);
        Ok(())
    });

    cr.register("lsp-stop", "nodoc", |_opt, ctx| {
        if let Some(client) = ctx.state.lsp_client.take() {
            client.shutdown();
        }
        debug!("turned off");
        Ok(())
    });

    cr.register("lsp-poll", "nodoc", |_opt, ctx| {
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
                Response::CompletionSuggestionsAvailable => {
                    let items = client.completion_items().to_vec();
                    let items = lsp_completion_items_to_completion_items(items);

                    let sources = &mut ctx.state.completions.source_items;
                    let source_data = sources.entry(CompletionSource::Lsp).or_default();

                    source_data.items = items;

                    ctx.queue.emit("completion-sources-modified", "");
                }
                Response::CompletionSuggestionResolved { idx } => {
                    let resolved_item = client.completion_items()[idx as usize].clone();
                    // This sucks and I should rework this shit.
                    for item in &mut ctx.state.completions.items {
                        if item.source != CompletionSource::Lsp || item.source_idx != idx {
                            continue;
                        }
                        *item = lsp_completion_item_to_completion_item(idx as usize, resolved_item);
                        ctx.queue.push("completions-show-selected-documentation");
                        break;
                    }
                }
                Response::SignatureHelp { text } => {
                    ctx.state.modeline.set_message(text);
                }
                Response::HoverInfo { text } => {
                    ctx.state.hover_info = Some(text);
                }
                Response::GoToDefinitionInfo { locations } => {
                    let Some(location) = locations.into_iter().next() else { continue };

                    let filepath = lsp_uri_to_filepath(location.uri);
                    let range = lsp_range_to_range(location.range);
                    let sel = Selection::from_range(range);
                    let selstr = sel.to_string();
                    // FIXME Hardcoded value. This adjustment might not be necessary if I add a config for "minimum distance between cursor and view edge" kind of thing
                    let new_view_top = sel.start().row - 2;

                    ctx.queue.push(format!("edit {}", filepath.display()));
                    ctx.queue.push(format!("selections-set {}", selstr));
                    ctx.queue.push(format!("look-set-top {}", new_view_top));
                }
                Response::FileDiagnostics { file, diagnostics } => {
                    let filepath = lsp_uri_to_filepath(file);
                    let diags = lsp_diagnostics_to_diagnostics(diagnostics);
                    ctx.state
                        .diagnostics
                        .sources
                        .entry(DiagnosticSource::Lsp)
                        .or_default()
                        .insert(filepath, diags);

                    // FIXME this smells
                    ctx.queue.push("generate-highlights");
                }
            }
        }

        Ok(())
    });

    cr.register("lsp-doc-sync-open", "nodoc", |opt, ctx| {
        let opt = opt.raw();
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

    cr.register("lsp-doc-sync-change", "nodoc", |opt, ctx| {
        let opt = opt.raw();
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
            // FIXME PERF Send more granular updates
            new_content: buffer.content_to_string(),
        });

        Ok(())
    });

    cr.register("lsp-doc-sync-save", "nodoc", |opt, ctx| {
        let opt = opt.raw();
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

        client.queue_notification(Notification::TextDocumentDidSave {
            text_document: TextDocumentIdentifier {
                uri: DocumentUri::new(buffer_path),
            },
        });

        Ok(())
    });

    cr.register("lsp-doc-sync-close", "nodoc", |opt, ctx| {
        let opt = opt.raw();
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
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let Some(client) = &mut ctx.state.lsp_client else {
                return Err("lsp client not started".into());
            };

            let Some(path) = ctx.buffer.path() else {
                return Err("save the file before you can hover".into());
            };

            let cursor = ctx.selections.primary().cursor;

            client.queue_hover_request(
                TextDocumentIdentifier::new(path),
                position_to_lsp_position(cursor),
            );

            Ok(())
        }),
    );

    cr.register(
        "lsp-completions",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
            let selections_modified_source = opt.trim();
            if selections_modified_source == "completions-select" {
                return Ok(());
            }

            let Some(client) = &mut ctx.state.lsp_client else {
                return Ok(());
            };

            let Some(path) = ctx.buffer.path() else {
                return Err("save the file before you can ask for completions".into());
            };

            let cursor = ctx.selections.primary().cursor;

            client.queue_suggest_completion_request(
                TextDocumentIdentifier::new(path),
                position_to_lsp_position(cursor),
            );

            Ok(())
        }),
    );

    cr.register(
        "lsp-signature-help",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let Some(client) = &mut ctx.state.lsp_client else {
                return Ok(());
            };

            let Some(path) = ctx.buffer.path() else {
                return Err("save the file before you can goto".into());
            };

            let cursor = ctx.selections.primary().cursor;

            client.queue_signature_help_request(
                TextDocumentIdentifier::new(path),
                position_to_lsp_position(cursor),
            );

            Ok(())
        }),
    );

    cr.register(
        "lsp-goto",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let Some(client) = &mut ctx.state.lsp_client else {
                return Err("lsp client not started".into());
            };

            let Some(path) = ctx.buffer.path() else {
                return Err("save the file before you can goto".into());
            };

            let cursor = ctx.selections.primary().cursor;

            client.queue_definition_request(
                TextDocumentIdentifier::new(path),
                position_to_lsp_position(cursor),
            );

            Ok(())
        }),
    );
}

fn position_to_lsp_position(pos: Position) -> ayed_lsp_client::types::Position {
    ayed_lsp_client::types::Position {
        line: pos.row.clamp(0, Row::MAX).try_into().unwrap(),
        character: pos.column.clamp(0, Column::MAX).try_into().unwrap(),
    }
}

fn lsp_position_to_position(pos: ayed_lsp_client::types::Position) -> Position {
    Position {
        row: pos.line.clamp(0, Row::MAX as u32).try_into().unwrap(),
        column: pos
            .character
            .clamp(0, Column::MAX as u32)
            .try_into()
            .unwrap(),
    }
}

fn lsp_range_to_range(range: ayed_lsp_client::types::Range) -> Range {
    (
        lsp_position_to_position(range.start),
        lsp_position_to_position(range.end),
    )
        .into()
}

fn lsp_uri_to_filepath(uri: ayed_lsp_client::types::DocumentUri) -> PathBuf {
    let Some(path) = uri.0.strip_prefix("file://") else {
        unimplemented!("unknown lsp uri format: {uri:?}");
    };
    PathBuf::from(path)
}

fn lsp_diagnostics_to_diagnostics(
    diags: Vec<ayed_lsp_client::types::Diagnostic>,
) -> Vec<Diagnostic> {
    diags
        .into_iter()
        .map(lsp_diagnostic_to_diagnostic)
        .collect()
}

fn lsp_diagnostic_to_diagnostic(diag: ayed_lsp_client::types::Diagnostic) -> Diagnostic {
    use ayed_lsp_client::types::DiagnosticSeverity as LspSeverity;
    let kind = match diag.severity {
        Some(LspSeverity::ERROR) => DiagnosticKind::Error,
        Some(LspSeverity::WARNING) => DiagnosticKind::Warning,
        Some(LspSeverity::HINT) => DiagnosticKind::ExtraInfo,
        _ => DiagnosticKind::Lint,
    };

    let mut range = lsp_range_to_range(diag.range);
    if range.is_empty() {
        range.end = range.end.offset((1, 0));
    }

    let mut message = diag.message;
    if let Some(info) = diag.related_information {
        message.push('\n');
        message.push_str(&format!("{info:?}"));
    }

    Diagnostic {
        kind,
        range,
        message,
    }
}

fn lsp_completion_items_to_completion_items(
    items: Vec<ayed_lsp_client::types::CompletionItem>,
) -> Vec<CompletionItem> {
    // NOTE: dont filter anything out in this function!
    // Indices need to match with the raw lsp items.

    // FIXME sorting and filtering shouldnt happen in LSP commands,
    //          it should be happening in the more generalized completions code.

    // items.sort_by(|a, b| {
    //     fn get_key(e: &ayed_lsp_client::types::CompletionItem) -> &String {
    //         e.sort_text.as_ref().unwrap_or(&e.label)
    //     }
    //     get_key(a).cmp(get_key(b))
    // });
    let converted_items = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| lsp_completion_item_to_completion_item(i, item))
        .collect();
    converted_items
}

fn lsp_completion_item_to_completion_item(
    idx: usize,
    item: ayed_lsp_client::types::CompletionItem,
) -> CompletionItem {
    let extra_edits = item
        .additional_text_edits
        .map(|edits| {
            edits
                .into_iter()
                .map(lsp_text_edit_to_completion_edit)
                .collect()
        })
        .unwrap_or_default();
    let kind = item
        .kind
        .map(lsp_completion_item_kind_to_completion_item_kind)
        .unwrap_or(CompletionItemKind::Plaintext);
    let type_annotation = if matches!(item.kind, Some(2 | 3 | 4 | 5 | 6 | 10 | 12 | 21)) {
        item.detail
    } else {
        None
    };
    let documentation = item.documentation.map(|d| d.text().to_string());
    CompletionItem {
        label: item.label,
        text: item.text_edit.new_text,
        extra_edits,
        kind,
        source: CompletionSource::Lsp,
        source_idx: idx as u32,
        type_annotation,
        documentation,
    }
}

fn lsp_completion_item_kind_to_completion_item_kind(kind: i32) -> CompletionItemKind {
    use CompletionItemKind as CIK;
    match kind {
        14 => CIK::Keyword,
        2 | 3 | 4 | 10 => CIK::Function,
        6 | 21 => CIK::Variable,
        7 | 13 | 22 | 25 => CIK::Type,
        8 => CIK::Interface,
        5 | 20 => CIK::Member,
        9 | 17 | 19 => CIK::Module,
        1 | _ => CIK::Plaintext,
    }
}

fn lsp_text_edit_to_completion_edit(edit: ayed_lsp_client::types::TextEdit) -> TextEdit {
    TextEdit {
        range: lsp_range_to_range(edit.range),
        text: edit.new_text,
    }
}
