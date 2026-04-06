use std::sync::LazyLock;

use regex::Regex;

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
    position::{Column, Position},
    state::{CompletionEdit, TextBuffer, TextBufferHistory},
    utils::string_utils::{byte_index_to_char_index, char_index_to_byte_index},
};

static RE_SYMBOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+").unwrap());

pub fn register_misc_commands(cr: &mut CommandRegistry) {
    cr.register("stderr", |opt, _ctx| {
        eprintln!("{opt}");
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

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        let inverse_edits = ctx.state.completions.last_completion_inverse_edits.take();
        let inverse_edits = inverse_edits.unwrap_or_default();
        let mut new_inverse_edits = Vec::new();

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
                let item = &ctx.state.completions.items[selected_item_idx];
                // TODO extra edits
                let edit = CompletionEdit {
                    range: prefix_symbol_range,
                    text: item.edit.text.to_string(),
                };
                let inverse_edit = buffer.apply_edit(&edit)?;
                new_inverse_edits.push(inverse_edit);
            }
        }

        ctx.state.completions.last_completion_inverse_edits = Some(new_inverse_edits);

        let selections = buffer.view_selections(view_handle).unwrap();
        let sel = selections.primary();
        let cursor = sel.cursor;
        ctx.state.completions.prompt_suggestion_cursor_position = Some(cursor);

        ctx.queue.emit("buffer-modified", buffer.path_str());
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    // TODO Delete this when you are confident the completions stuff works well
    // //  vvv DEBUG DEBUG DEBUG DEBUG DEBUG DEBUG DEBUG vvv
    // cr.register(
    //     "completions-dbgload",
    //     focused_buffer_command(|_opt, ctx| {
    //         impl From<&str> for CompletionEdit {
    //             fn from(value: &str) -> Self {
    //                 Self {
    //                     range: (Position::ZERO, Position::ZERO),
    //                     text: value.to_string(),
    //                 }
    //             }
    //         }
    //         impl From<&str> for CompletionItem {
    //             fn from(value: &str) -> Self {
    //                 Self {
    //                     label: value.to_string(),
    //                     edit: value.into(),
    //                     extra_edits: vec![],
    //                 }
    //             }
    //         }
    //         ctx.state.completions.source_items.insert(
    //             CompletionSources::Dbg,
    //             vec!["foo".into(), "bar".into(), "spam".into(), "egg".into()],
    //         );
    //         ctx.queue.emit("completion-sources-modified", "");
    //         Ok(())
    //     }),
    // );
    // //  ^^^ DEBUG DEBUG DEBUG DEBUG DEBUG DEBUG DEBUG ^^^

    cr.register(
        "completions-reset",
        focused_buffer_command(|_opt, ctx| {
            let cursor = ctx.selections.primary().cursor;
            let should_reset =
                ctx.state.completions.prompt_suggestion_cursor_position != Some(cursor);

            if should_reset {
                ctx.state.completions.selected_item = 0;
                ctx.state.completions.last_completion_inverse_edits = None;
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

    cr.register(
        "completions-gather",
        focused_buffer_command(|_opt, ctx| {
            let cursor = ctx.selections.primary().cursor;
            if ctx.state.completions.prompt_suggestion_cursor_position == Some(cursor) {
                return Ok(());
            }
            // ctx.state.completions.selected_item = 0;
            // ctx.state.completions.last_completion_inverse_edits = None;

            let prefix_range = get_prefix_symbol_range(ctx.buffer, cursor);
            ctx.state.completions.original_symbol_start = prefix_range.0;
            let prefix = ctx.buffer.range_text(prefix_range)?;
            ctx.state.completions.prompt_suggestion_cursor_position = Some(cursor);

            let mut items = Vec::new();
            for (_source, source_items) in &ctx.state.completions.source_items {
                // if prefix.is_empty() {
                //     break;
                // }
                items.extend(
                    source_items
                        .iter()
                        .filter(|i| i.label.starts_with(&prefix))
                        // TODO Consider not just starts_with(), but also contains()
                        // but with lower "priority".
                        .cloned(),
                );
            }
            // FIXME sort appropriately (by most relevant kind first or something)
            items.sort_by(|a, b| a.label.cmp(&b.label));
            ctx.state.completions.items = items;
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
