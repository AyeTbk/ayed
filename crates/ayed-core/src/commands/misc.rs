use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    sync::LazyLock,
};

use regex::Regex;

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options}, position::{Column, Position}, range::Range, selection::Selection, state::{
        CompletionItem, CompletionItemKind, CompletionSource, CompletionSourceData, TextBuffer,
        TextEdit,
    }, utils::string_utils::{byte_index_to_char_index, char_index_to_byte_index},
};

static RE_SYMBOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w[\w!\-]*").unwrap());
static RE_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\.|::|,)(\s*)").unwrap());

pub fn register_misc_commands(cr: &mut CommandRegistry) {
    cr.register("stderr", "nodoc", |opt, _ctx| {
        let opt = opt.raw();
        log::debug!("{opt}");
        Ok(())
    });

    cr.register("pending-command-run", "nodoc", |opt, ctx| {
        let opt = opt.raw();
        let Some(cmd) = ctx.state.config.state_value("pending-command") else {
            return Err("pending-command not set!".to_string());
        };
        ctx.queue.push(format!("{cmd} {opt}"));
        Ok(())
    });

    cr.register(
        "undo",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let undid = ctx.buffer.undo()?;
            if !undid {
                ctx.queue.push("message  nothing to undo");
            } else {
                ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
                ctx.queue.emit("selections-modified", "");
            }
            Ok(())
        }),
    );

    cr.register(
        "redo",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let redid = ctx.buffer.redo()?;
            if !redid {
                ctx.queue.push("message  nothing to redo");
            } else {
                ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
                ctx.queue.emit("selections-modified", "");
            }
            Ok(())
        }),
    );

    cr.register(
        "history-checkpoint",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            ctx.buffer.checkpoint();
            Ok(())
        }),
    );

    cr.register("yank", "nodoc", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get(view.buffer);

        let selections = buffer.view_selections(view_handle).unwrap();
        let register = &mut ctx.state.register;

        register.content = buffer
            .selection_text(&selections.primary_selection)
            .unwrap();
        register.extra_content.clear();
        for selection in selections.extra_selections.iter() {
            register
                .extra_content
                .push(buffer.selection_text(selection).unwrap());
        }

        let sel_count = register.extra_content.len() + 1;
        ctx.queue.push(format!(
            "message yanked {} selection{}",
            sel_count,
            if sel_count != 1 { "s" } else { "" }
        ));

        Ok(())
    });

    cr.register(
        "paste",
        Options::new().doc("nodoc").flag("before"),
        focused_buffer_command(|opt, mut ctx| {
            let before = opt.contains("before");

            // TODO fix this paste command, it behaves weird.

            let enumerated_sels = ctx.selections.iter_mut().enumerate().collect::<Vec<_>>();
            for (i, sel) in enumerated_sels.into_iter().rev() {
                let mut text = ctx
                    .state
                    .register
                    .iter()
                    .cycle()
                    .nth(i)
                    .expect("register.iter is never empty");

                let line_pasting_mode = text.ends_with('\n');

                let insert_at = if line_pasting_mode {
                    if before {
                        // Line start of selection start
                        Position::new(0, sel.start().row)
                    } else {
                        // Line start of row after selection end row
                        Position::new(0, sel.end().row + 1)
                    }
                } else {
                    if before {
                        sel.start()
                    } else {
                        if sel.end() == ctx.buffer.end_position() {
                            Position::new(0, sel.end().row + 1)
                        } else {
                            ctx.buffer
                                .move_position_horizontally(sel.end(), 1)
                                .unwrap_or(sel.end())
                        }
                    }
                };

                if insert_at > ctx.buffer.end_position() {
                    ctx.buffer.insert_char_at('\n', ctx.buffer.end_position())?;
                    text = text.strip_suffix('\n').unwrap_or(text);
                }

                let inserted_sel = ctx.buffer.insert_str_at(text, insert_at)?;
                *sel = inserted_sel;
            }

            ctx.buffer
                .set_view_selections(ctx.view_handle, ctx.selections);

            ctx.buffer.overwrite_history_current_selections_after();

            ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "completions-select",
        Options::new().doc("nodoc").flag("next").flag("previous"),
        |opt, ctx| {
            if ctx.state.completions.items.is_empty() {
                return Ok(());
            }

            let next = opt.contains("next");
            let previous = opt.contains("previous");

            let cycling_from_original = ctx.state.completions.selected_item == 0;
            ctx.state.completions.selected_item += next as i32 - (previous as i32);
            let modulo = ctx.state.completions.items.len() as i32 + 1;
            ctx.state.completions.selected_item =
                ctx.state.completions.selected_item.rem_euclid(modulo);
            let selected_item_idx = i32::max(ctx.state.completions.selected_item - 1, 0) as usize;
            let cycling_to_original = ctx.state.completions.selected_item == 0;

            let Some(view_handle) = ctx.state.focused_view(&ctx.panels) else {
                return Ok(());
            };
            let view = ctx.resources.views.get_mut(view_handle);
            let buffer = ctx.resources.buffers.get_mut(view.buffer);
            let sel_count = buffer.view_selections(view_handle).unwrap().count();

            let inverse_edits = ctx.state.completions.last_completion_inverse_edits.take();
            let inverse_edits = inverse_edits.unwrap_or_default();
            let mut new_inverse_edits = Vec::new();

            let item = &ctx.state.completions.items[selected_item_idx];

            // Stuff to adjust panel's position // FIXME should be replaced by a system where the buffer keeps track of positions, you know the idea
            let current_panel_row = ctx.state.completions.original_symbol_start.row;
            let mut panel_row_fix = 0;

            if !cycling_from_original {
                for inverse_edit in inverse_edits.iter().rev() {
                    panel_row_fix += buffer.line_delta_above_row(inverse_edit, current_panel_row);
                    buffer.apply_edit(inverse_edit)?;
                }
            }

            for sel_idx in 0..sel_count {
                let selections = buffer.view_selections(view_handle).unwrap();
                let sel = selections.get(sel_idx).unwrap();

                if !cycling_to_original {
                    let prefix_symbol_range = get_prefix_symbol_range(buffer, sel.cursor);
                    let edit = TextEdit {
                        range: prefix_symbol_range,
                        text: item.text.to_string(),
                    };
                    let inverse_edit = buffer.apply_edit(&edit)?;
                    new_inverse_edits.push(inverse_edit);

                    // Apply extra edits only for the first selection, assuming
                    // they are only used for importing required modules/stuff.
                    if sel_idx == 0 {
                        for extra_edit in &item.extra_edits {
                            panel_row_fix +=
                                buffer.line_delta_above_row(extra_edit, current_panel_row);
                            let extra_inverse_edit = buffer.apply_edit(&extra_edit)?;
                            new_inverse_edits.push(extra_inverse_edit);
                        }
                    }
                }
            }

            ctx.state.completions.original_symbol_start.row += panel_row_fix;
            view.top_left.row += panel_row_fix;

            ctx.state.completions.last_completion_inverse_edits = Some(new_inverse_edits);

            ctx.queue.push("completions-show-selected-documentation");

            ctx.queue.emit("buffer-modified", buffer.path_str());
            ctx.queue.emit("selections-modified", "completions-select");

            Ok(())
        },
    );

    // FIXME this should probably be hooked to a completions-on-select event as opposed to being queued directly.
    cr.register(
        "completions-show-selected-documentation",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let hover_info;
            let selected_item_idx = ctx.state.completions.selected_item - 1;
            if selected_item_idx == -1 {
                hover_info = None;
            } else {
                let selected_item = &ctx.state.completions.items[selected_item_idx as usize];
                hover_info = selected_item.documentation.clone();
            }

            ctx.state.hover_info = hover_info;

            Ok(())
        }),
    );

    // Check that completion menu should be displayed. Gathers completions if so. Clears them otherwise.
    cr.register(
        "completions-check",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
            let selections_modified_source = opt.trim();
            if selections_modified_source == "completions-select" {
                return Ok(());
            }

            let cursor = ctx.selections.primary().cursor;
            let prefix_range = get_prefix_symbol_range(ctx.buffer, cursor);

            ctx.queue.push("completions-clear"); // TODO check if this should even be a command rather than a fn call.

            let prefix_exists = !prefix_range.is_empty();
            if prefix_exists || position_follows_a_separator(ctx.buffer, cursor) {
                ctx.queue.push("completions-gather"); // TODO check if this should even be a command rather than a fn call.
            }
            Ok(())
        }),
    );

    cr.register(
        "completions-clear",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            ctx.state.completions.items.clear();
            ctx.state.completions.selected_item = 0;
            ctx.state.completions.last_completion_inverse_edits = None;
            Ok(())
        }),
    );

    // Gathers completions from the various sources, filling the list of active completion items.
    cr.register(
        "completions-gather",
        "nodoc",
        focused_buffer_command(|_opt, ctx| {
            let cursor = ctx.selections.primary().cursor;

            let prefix_range = get_prefix_symbol_range(ctx.buffer, cursor);
            ctx.state.completions.original_symbol_start = prefix_range.start;
            let prefix = ctx.buffer.range_text(prefix_range);

            let mut items_by_labels: HashMap<String, HashMap<CompletionItemKind, CompletionItem>> =
                HashMap::new();
            for (_source, source_data) in &ctx.state.completions.source_items {
                // TODO hashmap of {label, hashmap of {kind, item}}
                // fill things up naively, then check over every label entries
                // for anything that has plaintext kind. For them, if there are
                // also other kinds, remove plaintext kind.
                for i in &source_data.items {
                    // TODO Consider not just starts_with(), but also contains()
                    // but with lower "priority".
                    // Make case insensitive but higher priority if case matches.
                    if !i.label.starts_with(&prefix) {
                        continue;
                    }
                    if i.text == prefix {
                        continue;
                    }

                    items_by_labels
                        .entry(i.label.clone())
                        .or_default()
                        .insert(i.kind, i.clone());
                }
            }

            // Remove plaintext items when there are alternatives of other kinds.
            for (_, kinds) in &mut items_by_labels {
                if kinds.len() > 1 {
                    kinds.remove(&CompletionItemKind::Plaintext);
                }
            }

            let mut items = items_by_labels
                .into_iter()
                .flat_map(|(_, v)| v.into_iter())
                .map(|(_, v)| v)
                .collect::<Vec<_>>();

            items.sort_by(|a, b| {
                let cmp = a.kind.cmp(&b.kind);
                if !matches!(cmp, Ordering::Equal) {
                    return cmp;
                }
                let cmp = a.source.cmp(&b.source);
                if !matches!(cmp, Ordering::Equal) {
                    return cmp;
                }
                let cmp = a.label.cmp(&b.label);
                cmp
            });

            ctx.state.completions.selected_item =
                i32::clamp(ctx.state.completions.selected_item, 0, items.len() as _);
            ctx.state.completions.items = items;
            Ok(())
        }),
    );

    cr.register(
        "completions-source-buffer",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
            let selections_modified_source = opt.trim();
            if selections_modified_source == "completions-select" {
                return Ok(());
            }

            let content = ctx.buffer.content_to_string();
            let mut symbols = BTreeSet::new();
            let mut items = Vec::new();
            for matsh in RE_SYMBOL.find_iter(&content) {
                let symbol = matsh.as_str();
                if symbol.len() < 3 || symbols.contains(symbol) {
                    continue;
                }
                symbols.insert(symbol.to_string());
                items.push(CompletionItem {
                    label: symbol.to_string(),
                    text: symbol.to_string(),
                    extra_edits: Vec::new(),
                    kind: CompletionItemKind::Plaintext,
                    source: CompletionSource::Buffer,
                    source_idx: 0,
                    type_annotation: None,
                    documentation: None,
                });
            }

            ctx.state
                .completions
                .source_items
                .insert(CompletionSource::Buffer, CompletionSourceData { items });

            ctx.queue.emit("completion-sources-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "diagnostics-hover",
        "Show the message of the diagnostic under the primary cursor.",
        focused_buffer_command(|_, ctx| {
            let Some(path) = ctx.buffer.path() else { return Ok(()) };
            let diags = ctx.state.diagnostics.for_file(path);
            let cursor = ctx.selections.primary().cursor;

            for diag in diags {
                if diag.range.contains(cursor) {
                    ctx.state.hover_info = Some(diag.message.clone());
                }
            }
            Ok(())
        }),
    );

    cr.register(
        "diagnostics-move-to",
        Options::new()
            .doc("Move to the next/previous diagnostic in the buffer.")
            .flag("next")
            .flag("previous"),
        focused_buffer_command(|opt, ctx| {
            let previous = opt.contains("previous");
            let next = opt.contains("next") || !previous;

            let Some(path) = ctx.buffer.path() else { return Ok(()) };
            let diags = ctx.state.diagnostics.for_file(path);

            let cursor = ctx.selections.primary().cursor;

            let mut nearest_before_cursor = None;
            let mut nearest_after_cursor = None;
            for diag in diags {
                if diag.range.contains(cursor) {
                    continue;
                }

                if diag.range.end <= cursor {
                    let dist = cursor - diag.range.end;
                    if nearest_before_cursor.is_none() {
                        nearest_before_cursor = Some((dist, diag));
                    }
                    if let Some((nearest_dist, nearest_diag)) = &mut nearest_before_cursor {
                        if dist < *nearest_dist {
                            *nearest_dist = dist;
                            *nearest_diag = diag;
                        }
                    }
                } else if diag.range.start > cursor {
                    let dist = diag.range.start - cursor;
                    if nearest_after_cursor.is_none() {
                        nearest_after_cursor = Some((dist, diag));
                    }
                    if let Some((nearest_dist, nearest_diag)) = &mut nearest_after_cursor {
                        if dist < *nearest_dist {
                            *nearest_dist = dist;
                            *nearest_diag = diag;
                        }
                    }
                }
            }

            let mut nearest_diag = None;
            if let Some(nearest) = nearest_before_cursor && previous {
                nearest_diag = Some(nearest.1);
            }
            if let Some(nearest) = nearest_after_cursor && next {
                nearest_diag = Some(nearest.1);
            }
            if let Some(nearest_diag) = nearest_diag {
                let sel = Selection::with_position(nearest_diag.range.start);
                ctx.queue.push(format!("selections-set {}", sel));
            } else {
                return Err("no further diagnostics found".to_string());
            }

            Ok(())
        }),
    );
}

fn get_prefix_symbol_range(buffer: &TextBuffer, cursor: Position) -> Range {
    let row = cursor.row;
    let line = buffer.line(row).unwrap();
    let cursor_byte_idx = char_index_to_byte_index(line, cursor.column as _).unwrap();
    let mut maybe_range = None;
    for matsh in RE_SYMBOL.find_iter(line) {
        if matsh.start() < cursor_byte_idx && matsh.end() >= cursor_byte_idx {
            let start_column = byte_index_to_char_index(line, matsh.start()).unwrap() as Column;
            let end_column = cursor_byte_idx as Column;
            maybe_range = Some(Range::from((
                (start_column, row).into(),
                (end_column, row).into(),
            )))
        }
    }
    maybe_range.unwrap_or(Range::from((cursor, cursor)))
}

fn position_follows_a_separator(buffer: &TextBuffer, cursor: Position) -> bool {
    let row = cursor.row;
    let line = buffer.line(row).unwrap();
    let cursor_byte_idx = char_index_to_byte_index(line, cursor.column as _).unwrap();
    for capture in RE_SEPARATOR.captures_iter(line) {
        let matsh = capture.get(2).expect("ws expected in group 2");
        if matsh.start() <= cursor_byte_idx && matsh.end() >= cursor_byte_idx {
            return true;
        }
    }
    return false;
}
