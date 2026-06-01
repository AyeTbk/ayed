use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    sync::LazyLock,
};

use log::debug;
use regex::Regex;

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
    position::{Column, Position},
    state::{
        CompletionEdit, CompletionItem, CompletionItemKind, CompletionSource, TextBuffer,
        TextBufferHistory,
    },
    utils::string_utils::{byte_index_to_char_index, char_index_to_byte_index},
};

static RE_SYMBOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w[\w!\-]*").unwrap());
static RE_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\.|::|,)(\s*)").unwrap());

pub fn register_misc_commands(cr: &mut CommandRegistry) {
    cr.register("stderr", |opt, _ctx| {
        debug!("{opt}");
        Ok(())
    });

    cr.register("pending-command-run", |opt, ctx| {
        let Some(cmd) = ctx.state.config.state_value("pending-command") else {
            return Err("pending-command not set!".to_string());
        };
        ctx.queue.push(format!("{cmd} {opt}"));
        Ok(())
    });

    cr.register("history-save", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);

        let history_entry = ctx.state.edit_histories.entry(view.buffer);
        use std::collections::hash_map::Entry;
        match history_entry {
            Entry::Occupied(mut history) => {
                history.get_mut().save_state(buffer);
            }
            Entry::Vacant(history) => {
                history.insert(TextBufferHistory::new(buffer));
            }
        }

        Ok(())
    });

    cr.register("history-undo", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);

        let undid = ctx
            .state
            .edit_histories
            .get_mut(&view.buffer)
            .is_some_and(|history| history.undo(buffer));

        if undid {
            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");
        } else {
            ctx.queue.push("message no remaining history");
        }

        Ok(())
    });

    cr.register("yank", |_opt, ctx| {
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
        focused_buffer_command(|opt, mut ctx| {
            let opts = Options::new().flag("before").parse(opt)?;
            let before = opts.contains("before");

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
                    ctx.buffer.insert_char_at(ctx.buffer.end_position(), '\n')?;
                    text = text.strip_suffix('\n').unwrap_or(text);
                }

                let inserted_sel = ctx.buffer.insert_str_at(insert_at, text)?;
                *sel = inserted_sel;
            }

            let sels = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
            *sels = ctx.selections;

            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register("completions-select", |opt, ctx| {
        if ctx.state.completions.items.is_empty() {
            return Ok(());
        }

        let opts = Options::new().flag("next").flag("previous").parse(opt)?;
        let next = opts.contains("next");
        let previous = opts.contains("previous");

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
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        let inverse_edits = ctx.state.completions.last_completion_inverse_edits.take();
        let inverse_edits = inverse_edits.unwrap_or_default();
        let mut new_inverse_edits = Vec::new();

        let item = &ctx.state.completions.items[selected_item_idx];

        for sel_idx in 0..sel_count {
            if !cycling_from_original {
                let reverse_edit = inverse_edits.get(sel_idx).unwrap();
                buffer.apply_edit(reverse_edit)?;
                // TODO extra edits too
            }

            let selections = buffer.view_selections(view_handle).unwrap();
            let sel = selections.get(sel_idx).unwrap();

            if !cycling_to_original {
                let prefix_symbol_range = get_prefix_symbol_range(buffer, sel.cursor);
                // TODO extra edits
                let edit = CompletionEdit {
                    range: prefix_symbol_range,
                    text: item.text.to_string(),
                };
                let inverse_edit = buffer.apply_edit(&edit)?;
                new_inverse_edits.push(inverse_edit);
            }
        }

        ctx.state.completions.last_completion_inverse_edits = Some(new_inverse_edits);

        // Show selected item documentation in the hover info panel, if possible.
        if cycling_to_original {
            ctx.state.hover_info = None;
        } else {
            ctx.state.hover_info = item.documentation.clone();
        }

        ctx.queue.emit("buffer-modified", buffer.path_str());
        ctx.queue.emit("selections-modified", "completions-select");

        Ok(())
    });

    // Check that completion menu should be displayed. Gathers completions if so. Clears them otherwise.
    cr.register(
        "completions-check",
        focused_buffer_command(|opt, ctx| {
            let selections_modified_source = opt.trim();
            if selections_modified_source == "completions-select" {
                return Ok(());
            }

            let cursor = ctx.selections.primary().cursor;
            let prefix_range = get_prefix_symbol_range(ctx.buffer, cursor);

            ctx.queue.push("completions-clear"); // TODO check if this should even be a command rather than a fn call.

            let prefix_exists = prefix_range.0 != prefix_range.1;
            if prefix_exists || position_follows_a_separator(ctx.buffer, cursor) {
                ctx.queue.push("completions-gather"); // TODO check if this should even be a command rather than a fn call.
            }
            Ok(())
        }),
    );

    cr.register(
        "completions-clear",
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
        focused_buffer_command(|_opt, ctx| {
            let cursor = ctx.selections.primary().cursor;

            let prefix_range = get_prefix_symbol_range(ctx.buffer, cursor);
            ctx.state.completions.original_symbol_start = prefix_range.0;
            let prefix = ctx.buffer.range_text(prefix_range)?;

            let mut items_by_labels: HashMap<String, HashMap<CompletionItemKind, CompletionItem>> =
                HashMap::new();
            for (_source, source_items) in &ctx.state.completions.source_items {
                // TODO hashmap of {label, hashmap of {kind, item}}
                // fill things up naively, then check over every label entries
                // for anything that has plaintext kind. For them, if there are
                // also other kinds, remove plaintext kind.
                for i in source_items {
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
        focused_buffer_command(|opt, ctx| {
            let selections_modified_source = opt.trim();
            if selections_modified_source == "completions-select" {
                return Ok(());
            }

            let content = ctx.buffer.content_to_string();
            let mut symbols = BTreeSet::new();
            let mut completions = Vec::new();
            for matsh in RE_SYMBOL.find_iter(&content) {
                let symbol = matsh.as_str();
                if symbol.len() < 3 || symbols.contains(symbol) {
                    continue;
                }
                symbols.insert(symbol.to_string());
                completions.push(CompletionItem {
                    label: symbol.to_string(),
                    text: symbol.to_string(),
                    extra_edits: Vec::new(),
                    kind: CompletionItemKind::Plaintext,
                    source: CompletionSource::Buffer,
                    type_annotation: None,
                    documentation: None,
                });
            }

            ctx.state
                .completions
                .source_items
                .insert(CompletionSource::Buffer, completions);

            ctx.queue.emit("completion-sources-modified", "");

            Ok(())
        }),
    );
}

fn get_prefix_symbol_range(buffer: &TextBuffer, cursor: Position) -> (Position, Position) {
    let row = cursor.row;
    let line = buffer.line(row).unwrap();
    let cursor_byte_idx = char_index_to_byte_index(line, cursor.column as _).unwrap();
    let mut maybe_range = None;
    for matsh in RE_SYMBOL.find_iter(line) {
        if matsh.start() < cursor_byte_idx && matsh.end() >= cursor_byte_idx {
            let start_column = byte_index_to_char_index(line, matsh.start()).unwrap() as Column;
            let end_column = cursor_byte_idx as Column;
            maybe_range = Some(((start_column, row).into(), (end_column, row).into()))
        }
    }
    maybe_range.unwrap_or((cursor, cursor))
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
