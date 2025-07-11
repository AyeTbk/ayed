use regex::Regex;

use crate::{
    config::ConfigState,
    panels::FocusedPanel,
    position::{Column, Offset, Position},
    selection::Selections,
    state::{TextBuffer, TextBufferHistory, View},
    utils::string_utils::byte_index_to_char_index,
};

use super::{CommandRegistry, options::Options};

pub fn register_builtin_commands(cr: &mut CommandRegistry) {
    cr.register("quit", |_opt, ctx| {
        for (_, view) in ctx.state.views.iter() {
            let buffer = ctx.state.buffers.get(view.buffer);
            if buffer.is_dirty() {
                return Err(format!("there are unsaved changes"));
            }
        }
        ctx.state.quit_requested = true;
        Ok(())
    });
    cr.register("quit!", |_opt, ctx| {
        ctx.state.quit_requested = true;
        Ok(())
    });
    // TODO add alias capabilities to command registry
    cr.register("q", |_opt, ctx| {
        ctx.queue.push("quit");
        Ok(())
    });
    cr.register("q!", |_opt, ctx| {
        ctx.queue.push("quit!");
        Ok(())
    });

    cr.register("buffer-write", |opt, ctx| {
        let path = if opt.is_empty() { None } else { Some(opt) };

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        if let Some(path) = path {
            buffer.set_path(path);
        }

        buffer.write()?;

        ctx.queue.push(format!(
            "message written to {}",
            buffer.path().unwrap_or_default()
        ));

        Ok(())
    });
    cr.register("w", |opt, ctx| {
        ctx.queue.push(format!("buffer-write {opt}"));
        Ok(())
    });

    cr.register("wq", |_opt, ctx| {
        ctx.queue.push(format!("buffer-write"));
        ctx.queue.push(format!("quit"));
        Ok(())
    });

    cr.register("error", |opt, _ctx| Err(opt.to_string()));

    cr.register("message", |opt, ctx| {
        ctx.state.modeline.set_message(opt.to_string());
        Ok(())
    });

    cr.register("state-set", |opt, ctx| {
        let (state, rest) = opt
            .split_once(|ch: char| ch.is_ascii_whitespace())
            .ok_or_else(|| format!("bad options `{}`", opt))?;

        let state = state.trim();
        let value = rest.trim();

        ctx.queue
            .emit(format!("state-before-modified:{state}"), value);

        ctx.queue.push(format!("state-set__part2 {opt}"));

        Ok(())
    });
    cr.register("state-set__part2", |opt, ctx| {
        let (state, rest) = opt
            .split_once(|ch: char| ch.is_ascii_whitespace())
            .ok_or_else(|| format!("bad options `{}`", opt))?;

        let state = state.trim();
        let value = rest.trim();

        ctx.state.config.set_state(state, value);
        ctx.queue.emit(format!("state-modified:{state}"), value);

        Ok(())
    });
    cr.register("ss", |opt, ctx| {
        ctx.queue.push(format!("state-set {opt}"));
        Ok(())
    });

    cr.register("panel-focus", |opt, ctx| {
        let panel_name = opt
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("missing panel name"))?;

        // Cleanup if needed
        match ctx.state.focused_panel {
            FocusedPanel::Warpdrive => {
                ctx.panels.warpdrive.clear_state();
            }
            FocusedPanel::Modeline(view_handle) => {
                let buffer_handle = ctx.state.views.get(view_handle).buffer;
                ctx.state.views.remove(view_handle);
                ctx.state.buffers.remove(buffer_handle);
            }
            _ => (),
        }

        match panel_name {
            "editor" => {
                ctx.state.focused_panel = FocusedPanel::Editor;
            }
            "modeline" => {
                let buffer = ctx.state.buffers.insert(TextBuffer::new_empty());
                let view = ctx.state.views.insert(View {
                    top_left: Position::ZERO,
                    buffer,
                    virtual_buffer: None,
                });
                ctx.state
                    .buffers
                    .get_mut(buffer)
                    .add_view_selections(view, Selections::new());

                // TODO the modeline view and buffer handles could just be stored in the panel maybe?
                // It would avoid having to cleanup and recreate them (but would still need to clear the buffer).
                ctx.state.focused_panel = FocusedPanel::Modeline(view);
            }
            "warpdrive" => {
                ctx.state.focused_panel = FocusedPanel::Warpdrive;
            }
            _ => return Err(format!("unknown panel '{opt}'")),
        }

        ctx.queue.set_state("panel", panel_name);

        Ok(())
    });

    cr.register("modeline-exec", |_opt, ctx| {
        let FocusedPanel::Modeline(view_handle) = ctx.state.focused_panel else {
            return Err("modeline not focused".into());
        };

        let buffer_handle = ctx.state.views.get(view_handle).buffer;
        let line = ctx.state.buffers.get(buffer_handle).first_line();

        ctx.queue.push("panel-focus editor");
        if !line.is_empty() {
            ctx.queue.push(line);
        }

        Ok(())
    });

    cr.register("edit", |opt, ctx| {
        let opts = Options::new().flag("scratch").parse(opt)?;
        let scratch = opts.contains("scratch");
        let path = opts.remainder();

        let buffer_handle;
        let buffer_opened_path: Option<&str>;
        if path.is_empty() && scratch {
            buffer_handle = ctx.state.open_scratch();
            buffer_opened_path = Some(path);
            ctx.queue.emit("buffer-opened", "");
        } else {
            match ctx.state.buffer_with_path(path) {
                Some(handle) => {
                    buffer_handle = handle;
                    buffer_opened_path = None;
                }
                None => {
                    buffer_handle = ctx.state.open_file_or_scratch(path)?;
                    buffer_opened_path = Some(path);
                }
            }
        }

        let view_handle = match ctx.state.view_with_buffer(buffer_handle) {
            Some(handle) => handle,
            None => {
                let view = ctx.state.views.insert(View {
                    top_left: Position::ZERO,
                    buffer: buffer_handle,
                    virtual_buffer: None,
                });

                ctx.state
                    .buffers
                    .get_mut(buffer_handle)
                    .add_view_selections(view, Selections::new());

                view
            }
        };

        ctx.state.active_editor_view = Some(view_handle);

        // The state must be updated before 'buffer-opened' is emitted so that
        // hooked commands may behave correctly.
        ctx.queue.set_state(ConfigState::FILE, path);

        if let Some(path) = buffer_opened_path {
            ctx.queue.emit("buffer-opened", path);
        }

        Ok(())
    });

    cr.register("look", |opt, ctx| {
        let mut offset = Offset::new(0, 0);
        for ch in opt.chars() {
            match ch {
                'u' => offset.row -= 1,
                'd' => offset.row += 1,
                'l' => offset.column -= 1,
                'r' => offset.column += 1,
                _ => return Err(format!("invalid option: {ch}")),
            }
        }

        if let Some(view_handle) = ctx.state.focused_view() {
            let view = ctx.state.views.get_mut(view_handle);
            view.top_left = view.top_left.offset(offset);
        }

        Ok(())
    });

    cr.register("look-keep-primary-cursor-in-view", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view() {
            let view_rect = ctx.state.focused_view_rect();
            let view = ctx.state.views.get_mut(view_handle);
            let cursor = {
                let buffer = ctx.state.buffers.get(view.buffer);
                let selections = buffer.view_selections(view_handle).unwrap();
                selections.primary().cursor()
            };
            let Some(view_cursor) = view.map_true_position_to_virtual_position(cursor) else {
                return Ok(());
            };
            let offset = view_rect.offset_from_position(view_cursor);
            view.top_left = view.top_left.offset(offset);
        }

        Ok(())
    });

    cr.register("move", |opt, ctx| {
        let Some(ch) = opt.chars().next() else {
            return Err(format!("missing option: (u, d, l, r)"));
        };
        let offset = match ch.to_ascii_lowercase() {
            'u' => Offset::new(0, -1),
            'd' => Offset::new(0, 1),
            'l' => Offset::new(-1, 0),
            'r' => Offset::new(1, 0),
            _ => return Err(format!("invalid option: {opt}")),
        };
        let anchored = opt.contains("anchored");

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let mut selections = buffer.view_selections(view_handle).unwrap().clone();

        for selection in selections.iter_mut() {
            let horizontal_move = offset.column != 0;
            if horizontal_move {
                let new_cursor = buffer
                    .move_position_horizontally(selection.cursor(), offset.column)
                    .unwrap_or(selection.cursor());

                *selection = if anchored {
                    selection.with_cursor(new_cursor)
                } else {
                    selection.with_anchor(new_cursor).with_cursor(new_cursor)
                };
            } else {
                let limited_cursor =
                    buffer.limit_position_to_content(selection.desired_cursor().offset(offset));
                *selection = if anchored {
                    selection.with_provisional_cursor(limited_cursor)
                } else {
                    selection
                        .with_anchor(limited_cursor)
                        .with_provisional_cursor(limited_cursor)
                }
            }
        }

        *buffer.view_selections_mut(view_handle).unwrap() = selections;

        // TODO make this automatic maybe?, keep track of it in TextBuffer
        // Same for buffer-modified??
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("move-regex", |opt, ctx| {
        let opts = Options::new()
            .flag("reversed")
            .flag("anchored")
            .flag("line")
            .parse(opt)?;
        let reversed = opts.contains("reversed");
        let anchored = opts.contains("anchored");
        let stay_within_line = opts.contains("line");
        let pattern = opts.remainder();

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);
        let mut selections = buffer.view_selections(view_handle).unwrap().clone();
        let regex = Regex::new(pattern).map_err(|e| e.to_string())?;

        for selection in selections.iter_mut() {
            let cursor = selection.cursor();
            let mut row = cursor.row;
            let mut search_start_column = cursor.column;

            'line: loop {
                let Some(line) = buffer.line(row) else { break 'line };
                let mut matches = regex.find_iter(line).collect::<Vec<_>>();
                if reversed {
                    matches.reverse();
                }
                let mut needle: Option<(Column, Column)> = None;
                for matsh in matches {
                    // The following `unwrap`s can't fail since the match was found in `line`.
                    let start = byte_index_to_char_index(line, matsh.start()).unwrap();
                    let one_past_end = byte_index_to_char_index(line, matsh.end()).unwrap();
                    // The following `unwrap`s may fail under extreme circumstances.
                    let start: Column = start.try_into().unwrap();
                    let one_past_end: Column = one_past_end.try_into().unwrap();
                    let end = if start == one_past_end {
                        // Don't adjust end if the match is zero width, like for the regex $
                        one_past_end
                    } else {
                        one_past_end - 1
                    };

                    let (new_anchor_column, new_cursor_column) =
                        if reversed { (end, start) } else { (start, end) };

                    let match_happens_too_early = if reversed {
                        search_start_column <= start
                    } else {
                        search_start_column >= end
                    };
                    if match_happens_too_early {
                        continue;
                    }

                    needle = Some((new_anchor_column, new_cursor_column));
                    break;
                }

                if let Some((new_anchor_column, new_cursor_column)) = needle {
                    if !anchored {
                        *selection = selection.with_anchor(Position::new(new_anchor_column, row));
                    }
                    *selection = selection.with_cursor(Position::new(new_cursor_column, row));

                    // Found the match for this selection, onto the next!
                    break 'line;
                } else if stay_within_line {
                    // Couldn't find a match on this line for this selection, skip.
                    break 'line;
                } else {
                    // Couldn't find a match on this line for this selection, go
                    // to next line and try again.
                    let row_is_out_of_bounds;
                    if reversed {
                        row -= 1;
                        search_start_column = Column::MAX;
                        row_is_out_of_bounds = row < 0;
                    } else {
                        row += 1;
                        search_start_column = -1;
                        row_is_out_of_bounds = row >= buffer.line_count();
                    };
                    if row_is_out_of_bounds {
                        break 'line;
                    }
                }
            }
        }

        *buffer.view_selections_mut(view_handle).unwrap() = selections;

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("insert-char", |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let the_char = if opt == r"\n" {
            '\n'
        } else {
            opt.chars()
                .next()
                .ok_or_else(|| format!("not a char: {opt}"))?
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        for sel_idx in (0..sel_count).rev() {
            let Some(sel) = buffer.view_selections(view_handle).unwrap().get(sel_idx) else {
                continue;
            };
            buffer.insert_char_at(sel.cursor(), the_char)?;
        }

        ctx.queue.emit("buffer-modified", "");
        Ok(())
    });

    cr.register("delete", |opt, ctx| {
        let contains_cursor = opt.contains("-c");

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        for sel_idx in (0..sel_count).rev() {
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            let Some(mut sel) = selections.get(sel_idx) else {
                continue;
            };
            if contains_cursor {
                sel = sel.shrunk_to_cursor();
            }
            buffer.delete_selection(&sel)?;
        }

        ctx.queue.emit("buffer-modified", "");
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("delete-around", |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let contains_cursor = opt.contains("-c");
        let contains_previous = opt.contains("-p");
        let contains_next = opt.contains("-n");
        let (delete_before, delete_after) = if !contains_next && !contains_previous {
            (true, true)
        } else {
            (contains_previous, contains_next)
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        for sel_idx in (0..sel_count).rev() {
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            let Some(mut sel) = selections.get(sel_idx) else {
                continue;
            };

            if contains_cursor {
                sel = sel.shrunk_to_cursor();
            }

            if delete_after {
                let from = sel.end();
                let Some(at) = buffer.move_position_horizontally(from, 1) else {
                    continue;
                };
                buffer.delete_at(at)?;
            }
            if delete_before {
                let from = sel.start();
                let Some(at) = buffer.move_position_horizontally(from, -1) else {
                    continue;
                };
                buffer.delete_at(at)?;
            }
        }

        ctx.queue.emit("buffer-modified", "");
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selections-merge-overlapping", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view() {
            let view = ctx.state.views.get_mut(view_handle);
            let buffer = ctx.state.buffers.get_mut(view.buffer);
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            *selections = selections.overlapping_selections_merged()
        }

        Ok(())
    });

    cr.register("selections-dismiss-extras", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view() {
            let view = ctx.state.views.get_mut(view_handle);
            let buffer = ctx.state.buffers.get_mut(view.buffer);
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            selections.dismiss_extras();
        }

        Ok(())
    });

    cr.register("selections-set", |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get_mut(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);
        let selections = buffer.view_selections_mut(view_handle).unwrap();
        *selections = Selections::parse(&opt)?;

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selection-shrink", |_opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);
        let selections = buffer.view_selections_mut(view_handle).unwrap();
        for selection in selections.iter_mut() {
            *selection = selection.shrunk_to_cursor();
        }

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selection-flip", |opt, ctx| {
        let opts = Options::new().flag("forward").flag("backward").parse(opt)?;
        let forward = opts.contains("forward");
        let backward = opts.contains("backward");

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);
        let selections = buffer.view_selections_mut(view_handle).unwrap();
        for selection in selections.iter_mut() {
            *selection = if forward {
                selection.flipped_forward()
            } else if backward {
                selection.flipped_forward().flipped()
            } else {
                selection.flipped()
            };
        }

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    // FIXME rename this command to duplicate-selection or something
    cr.register("dupe", |opt, ctx| {
        // FIXME the primary selection should become the newly created selection
        let row_offset = match opt.chars().next() {
            Some('u') => -1,
            Some('d') => 1,
            _ => return Err(format!("invalid option: {opt}")),
        };
        let offset = Offset::new(0, row_offset);

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);
        let mut selections = buffer.view_selections(view_handle).unwrap().clone();
        let dupes = selections
            .iter()
            .map(|sel| {
                buffer.limit_selection_to_content(
                    &sel.with_provisional_anchor(sel.desired_anchor().offset(offset))
                        .with_provisional_cursor(sel.desired_cursor().offset(offset)),
                )
            })
            .collect::<Vec<_>>();
        for dupe in dupes {
            selections.add(dupe);
        }

        *buffer.view_selections_mut(view_handle).unwrap() = selections;

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("history-save", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

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
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let undid = ctx
            .state
            .edit_histories
            .get_mut(&view.buffer)
            .is_some_and(|history| history.undo(buffer));

        if undid {
            ctx.queue.emit("buffer-modified", "");
        } else {
            ctx.queue.push("message no remaining history");
        }

        Ok(())
    });

    cr.register("yank", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get(view.buffer);

        let selections = buffer.view_selections(view_handle).unwrap();
        let register = &mut ctx.state.register;

        register.content = buffer.selection_text(&selections.primary_selection);
        register.extra_content.clear();
        for selection in selections.extra_selections.iter() {
            register
                .extra_content
                .push(buffer.selection_text(selection));
        }

        Ok(())
    });

    cr.register("paste", |opt, ctx| {
        let opts = Options::new().flag("before").parse(opt)?;
        let before = opts.contains("before");

        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let sel_count = buffer.view_selections(view_handle).unwrap().count();
        for sel_idx in (0..sel_count).rev() {
            let selections = buffer.view_selections(view_handle).unwrap();
            let Some(sel) = selections.get(sel_idx) else {
                continue;
            };
            let text = ctx
                .state
                .register
                .iter()
                .cycle()
                .nth(sel_idx)
                .expect("register.iter is never empty");
            let insert_at = if before {
                sel.start()
            } else {
                buffer
                    .move_position_horizontally(sel.end(), 1)
                    .unwrap_or(sel.end())
            };

            let inserted_sel = buffer.insert_str_at(insert_at, text)?;

            let selections = buffer.view_selections_mut(view_handle).unwrap();
            let Some(sel) = selections.get_mut(sel_idx) else {
                continue;
            };
            *sel = inserted_sel;
        }

        ctx.queue.emit("buffer-modified", "");
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

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
        let selected_item_idx = (ctx.state.suggestions.selected_item - 1).max(0) as usize;
        let cycling_to_original = ctx.state.suggestions.selected_item == 0;

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);
        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        if cycling_from_original {
            ctx.state.suggestions.original_symbols.clear();
            for sel_idx in 0..sel_count {
                let selections = buffer.view_selections(view_handle).unwrap();
                let sel = selections.get(sel_idx).unwrap();
                let original = buffer.selection_text(&sel);
                ctx.state.suggestions.original_symbols.push(original);
            }
        }

        for sel_idx in 0..sel_count {
            let selections = buffer.view_selections(view_handle).unwrap();
            let sel = selections.get(sel_idx).unwrap();

            buffer.delete_selection(&sel)?;

            let text_to_insert;
            if cycling_to_original {
                text_to_insert = ctx.state.suggestions.original_symbols[sel_idx].as_str();
            } else {
                text_to_insert = ctx.state.suggestions.items[selected_item_idx].as_str();
            }
            let new_sel = buffer.insert_str_at(sel.start(), text_to_insert)?;

            let selections = buffer.view_selections_mut(view_handle).unwrap();
            *selections.get_mut(sel_idx).unwrap() = new_sel;
        }

        ctx.queue.emit("buffer-modified", "");
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("suggestions-gather", |_opt, ctx| {
        // Look at symbol under primary cursor (or right before)
        // If it's the same as old one saved in state, bail.
        // Else, clear suggs and gather new from configured source
        ctx.state.suggestions.items.clear();
        ctx.state.suggestions.selected_item = 0;

        ctx.queue
            .push("error 1qaz do this here! suggestions-gather");

        Ok(())
    });

    cr.register("vbuf-clear", |_opt, ctx| {
        // FIXME I dont believe the vbuffer should be handled directly
        // by commands like this. Its settings should be set (with
        // states) and the changes should take effect automatically.
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get_mut(view_handle);
        view.virtual_buffer = None;

        Ok(())
    });
    cr.register("vbuf-line-wrap-rebuild", |_opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get_mut(view_handle);
        view.rebuild_line_wrap(
            &ctx.state.buffers,
            ctx.state.editor_rect.size().column.try_into().unwrap(),
        );

        Ok(())
    });
}
