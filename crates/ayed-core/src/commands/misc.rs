use std::sync::LazyLock;

use regex::Regex;

use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
    position::{Column, Position},
    selection::Selection,
    state::{TextBuffer, TextBufferHistory},
    utils::string_utils::{byte_index_to_char_index, char_index_to_byte_index},
};

static RE_SYMBOL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+|[^\s\w]+").unwrap());

pub fn register_misc_commands(cr: &mut CommandRegistry) {
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

        register.content = buffer.selection_text(&selections.primary_selection);
        register.extra_content.clear();
        for selection in selections.extra_selections.iter() {
            register
                .extra_content
                .push(buffer.selection_text(selection));
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

    cr.register("suggestions-select", |opt, ctx| {
        if ctx.state.suggestions.items.is_empty() {
            return Ok(());
        }

        let opts = Options::new().flag("next").flag("previous").parse(opt)?;
        let next = opts.contains("next");
        let previous = opts.contains("previous");

        let cycling_from_original = ctx.state.suggestions.selected_item == 0;
        ctx.state.suggestions.selected_item += next as i32 - (previous as i32);
        let modulo = ctx.state.suggestions.items.len() as i32 + 1;
        ctx.state.suggestions.selected_item =
            ctx.state.suggestions.selected_item.rem_euclid(modulo);
        let selected_item_idx = i32::max(ctx.state.suggestions.selected_item - 1, 0) as usize;
        let cycling_to_original = ctx.state.suggestions.selected_item == 0;

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        // Select the symbols under cursor in order to delete and replace it later.
        for sel_idx in 0..sel_count {
            let selections = buffer.view_selections(view_handle).unwrap();
            let sel = selections.get(sel_idx).unwrap();
            let new_sel = selection_from_symbol_prefix_under_cursor(buffer, sel.cursor);
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            let sel = selections.get_mut(sel_idx).unwrap();
            *sel = new_sel;

            if cycling_from_original {
                // Gather original symbols
                ctx.state.suggestions.original_symbols.clear();

                let selections = buffer.view_selections(view_handle).unwrap();
                let sel = selections.get(sel_idx).unwrap();
                let original_sel = sel.with_end(sel.end().offset((-1, 0)));
                let original = buffer.selection_text(&original_sel);
                ctx.state.suggestions.original_symbols.push(original);
            }
        }

        // Delete symbols under cursors and replace with appropriate suggestion
        for sel_idx in 0..sel_count {
            let selections = buffer.view_selections(view_handle).unwrap();
            let sel = selections.get(sel_idx).unwrap();

            let delete_sel = sel.with_end(sel.end().offset((-1, 0)));
            buffer.delete_selection(&delete_sel)?;

            let text_to_insert;
            if cycling_to_original {
                text_to_insert = ctx.state.suggestions.original_symbols[sel_idx].as_str();
            } else {
                text_to_insert = ctx.state.suggestions.items[selected_item_idx].as_str();
            }
            let mut new_sel = buffer.insert_str_at(sel.start(), text_to_insert)?;
            new_sel = new_sel.with_end(new_sel.end().offset((1, 0)));

            ctx.state.suggestions.prompt_suggestion_cursor_position = Some(new_sel.end());

            let selections = buffer.view_selections_mut(view_handle).unwrap();
            *selections.get_mut(sel_idx).unwrap() = new_sel.shrunk_to_cursor();
        }

        ctx.queue.emit("buffer-modified", "");
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("suggestions-clear", |_opt, ctx| {
        // TODO In order for this to work properly, it would need to keep track of what
        // this position is over the modifications that happen in the buffer (in
        // particular, this makes the suggbox misbehave with multicursors before the
        // primary cursor).
        // TODO fix the above using a buffer mark when that's a thing.
        ctx.state.suggestions.prompt_suggestion_cursor_position = None;

        ctx.state.suggestions.items.clear();
        ctx.state.suggestions.selected_item = 0;

        Ok(())
    });

    cr.register(
        "suggestions-gather",
        focused_buffer_command(|_opt, ctx| {
            let source = ctx.state.config.get_entry_value("suggestions", "source")?;
            if source != "active-buffer" {
                return Err("only 'active-buffer' is supported as suggestion source".to_string());
            }

            let cursor = ctx.selections.primary().cursor;

            let should_prompt =
                Some(cursor) == ctx.state.suggestions.prompt_suggestion_cursor_position;

            if should_prompt && ctx.state.suggestions.selected_item != 0 {
                // Don't interfere with suggestions when user is selecting one.
                return Ok(());
            }

            let line = ctx.buffer.line(cursor.row).unwrap();

            let cursor_byte_idx =
                char_index_to_byte_index(line, cursor.column as _).unwrap_or(line.len());
            let mut maybe_symbol_prefix = None;
            let mut maybe_symbol_start_end = None;
            for matsh in RE_SYMBOL.find_iter(line) {
                if matsh.start() < cursor_byte_idx && matsh.end() >= cursor_byte_idx {
                    maybe_symbol_prefix =
                        Some((matsh.as_str(), &line[matsh.start()..cursor_byte_idx]));
                    maybe_symbol_start_end = Some((matsh.start(), matsh.end()));
                }
            }

            ctx.state.suggestions.items.clear();
            ctx.state.suggestions.selected_item = 0;

            if let Some((start_index, end_index)) = maybe_symbol_start_end {
                let start_column = byte_index_to_char_index(line, start_index).unwrap();
                let symbol_start = Position::new(start_column as Column, cursor.row);
                ctx.state.suggestions.original_symbol_start = symbol_start;

                let end_column = byte_index_to_char_index(line, end_index).unwrap();
                let prompt_position = Position::new(end_column as Column + 1, cursor.row);
                ctx.state.suggestions.prompt_suggestion_cursor_position = Some(prompt_position);
            }

            let Some((symbol, prefix)) = maybe_symbol_prefix else { return Ok(()) };
            // TODO bail if prefix hasnt changed (add prefix to suggs state)

            for i in 0..ctx.buffer.line_count() {
                let line = ctx.buffer.line(i).unwrap();
                for matsh in RE_SYMBOL.find_iter(line) {
                    let matsh_str = matsh.as_str();
                    if matsh_str.starts_with(prefix) && matsh_str != symbol {
                        let item = matsh_str.to_string();
                        if !ctx.state.suggestions.items.contains(&item) {
                            ctx.state.suggestions.items.push(item);
                        }
                    }
                }
            }

            Ok(())
        }),
    );
}

fn selection_from_symbol_prefix_under_cursor(buffer: &TextBuffer, cursor: Position) -> Selection {
    let row = cursor.row;
    let line = buffer.line(row).unwrap();
    let cursor_byte_idx = char_index_to_byte_index(line, cursor.column as _).unwrap();
    let mut maybe_selection = None;
    for matsh in RE_SYMBOL.find_iter(line) {
        if matsh.start() < cursor_byte_idx && matsh.end() >= cursor_byte_idx {
            let start_column = byte_index_to_char_index(line, matsh.start()).unwrap() as Column;
            // let end_column = byte_index_to_char_index(line, matsh.end()).unwrap() as Column;
            let end_column = cursor_byte_idx as Column;
            maybe_selection = Some(
                Selection::new()
                    .with_anchor((start_column, row).into())
                    .with_cursor((end_column, row).into()),
            )
        }
    }
    maybe_selection.unwrap_or_else(|| Selection::new().with_cursor(cursor).shrunk_to_cursor())
}
