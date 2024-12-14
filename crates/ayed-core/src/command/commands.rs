use std::cell::RefCell;

use regex::Regex;

use crate::{
    config::ConfigState,
    event::EventRegistry,
    panels::FocusedPanel,
    position::{Offset, Position},
    selection::Selections,
    state::{TextBuffer, View},
    utils::string_utils::{byte_index_to_char_index, char_index_to_byte_index},
    Ref,
};

use super::{options::Options, CommandRegistry};

pub fn register_builtin_commands(cr: &mut CommandRegistry, _ev: &mut EventRegistry) {
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

        buffer.write_atomic()?;

        Ok(())
    });
    cr.register("w", |opt, ctx| {
        ctx.queue.push(format!("buffer-write {opt}"));
        Ok(())
    });

    cr.register("error", |opt, _ctx| Err(opt.to_string()));

    cr.register("state-set", |opt, ctx| {
        let (state, rest) = opt
            .split_once(|ch: char| ch.is_ascii_whitespace())
            .ok_or_else(|| format!("bad options `{}`", opt))?;

        let state = state.trim();
        let value = rest.trim();

        ctx.state.config.set_state(state, value);
        ctx.events.emit(format!("state-modified:{state}"), value);

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
                let selections = Ref::new(RefCell::new(Selections::new()));

                let buffer = ctx.state.buffers.insert(TextBuffer::new_empty());
                ctx.state
                    .buffers
                    .get_mut(buffer)
                    .add_selections(&selections);

                let view = ctx.state.views.insert(View {
                    top_left: Position::ZERO,
                    buffer,
                    selections,
                    virtual_buffer: None,
                });

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
        if path.is_empty() && scratch {
            buffer_handle = ctx.state.open_scratch();
            ctx.events.emit("buffer-opened", "");
        } else {
            match ctx.state.buffer_with_path(path) {
                Some(handle) => buffer_handle = handle,
                None => {
                    buffer_handle = ctx.state.open_file_or_scratch(path)?;
                    ctx.events.emit("buffer-opened", path);
                }
            }
        }

        let view_handle = match ctx.state.view_with_buffer(buffer_handle) {
            Some(handle) => handle,
            None => {
                let selections = Ref::new(RefCell::new(Selections::new()));

                ctx.state
                    .buffers
                    .get_mut(buffer_handle)
                    .add_selections(&selections);

                ctx.state.views.insert(View {
                    top_left: Position::ZERO,
                    buffer: buffer_handle,
                    selections,
                    virtual_buffer: None,
                })
            }
        };

        ctx.state.active_editor_view = Some(view_handle);

        ctx.queue.set_state(ConfigState::FILE, path);

        Ok(())
    });

    cr.register("selections-merge-overlapping", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view() {
            let view = ctx.state.views.get_mut(view_handle);
            let mut selections = view.selections.borrow_mut();
            *selections = selections.overlapping_selections_merged()
        }

        Ok(())
    });

    cr.register("selections-set", |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get_mut(view_handle);
        let mut selections = view.selections.borrow_mut();
        *selections = Selections::parse(&opt)?;

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
            let cursor = view.selections.borrow().primary().cursor();
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

        let mut selections = view.selections.borrow().clone();

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

        *view.selections.borrow_mut() = selections;

        // FIXME these are being done often in this file, they probably should be hooked
        // to some events, like buffer-modified and selections-modified.
        ctx.queue.push("selections-merge-overlapping");
        ctx.queue.push("look-keep-primary-cursor-in-view");

        Ok(())
    });

    cr.register("move-regex", |opt, ctx| {
        let opts = Options::new()
            .flag("reversed")
            .flag("anchored")
            .flag("keepline")
            .parse(opt)?;
        let next = !opts.contains("reversed");
        let anchored = opts.contains("anchored");
        let keepline = opts.contains("keepline");
        let pattern = opts.remainder();

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let regex = Regex::new(pattern).map_err(|e| e.to_string())?;
        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);
        let mut selections = view.selections.borrow().clone();

        for selection in selections.iter_mut() {
            let mut begin_column = selection.cursor().column;
            let mut row = selection.cursor().row;
            // TODO implement cycling through the whole file.
            while let Some(line) = buffer.line(row) {
                let start_index = if next {
                    char_index_to_byte_index(line, begin_column).unwrap()
                } else {
                    if let Some(index) = char_index_to_byte_index(line, begin_column) {
                        index
                    } else {
                        if line.is_empty() {
                            if keepline {
                                break;
                            }
                            if row == 0 {
                                break;
                            }
                            row = row - 1;
                            continue;
                        } else {
                            line.len() - 1
                        }
                    }
                };

                let mut matches = regex.find_iter(line).collect::<Vec<_>>();
                if !next {
                    matches.reverse();
                }
                let maybe_match = matches
                    .into_iter()
                    .skip_while(|m| {
                        // NOTE: this stinks, if positions allowed negative rows/columns, this could be simpler.

                        if next {
                            let cursor_at_line_start_fix =
                                !(row == selection.cursor().row && selection.cursor().column == 0);
                            if start_index == 0 && cursor_at_line_start_fix {
                                m.start() < start_index
                            } else {
                                m.start() <= start_index
                            }
                        } else {
                            let line_end = line.len().saturating_sub(1);
                            let cursor_at_line_end_fix = !(row == selection.cursor().row
                                && selection.cursor().column as usize == line_end);
                            if start_index == line_end && cursor_at_line_end_fix {
                                m.start() > start_index
                            } else {
                                m.end() > start_index
                            }
                        }
                    })
                    .next();
                if let Some(matsh) = maybe_match {
                    let start_column = byte_index_to_char_index(line, matsh.start()).unwrap();
                    let end_column = if matsh.is_empty() {
                        start_column
                    } else {
                        byte_index_to_char_index(line, matsh.end().saturating_sub(1)).unwrap()
                    };
                    *selection = if next {
                        selection.with_cursor(Position::new(end_column, row))
                    } else {
                        selection.with_cursor(Position::new(start_column, row))
                    };
                    if !anchored {
                        *selection = if next {
                            selection.with_anchor(Position::new(start_column, row))
                        } else {
                            selection.with_anchor(Position::new(end_column, row))
                        };
                    }
                    break;
                }

                if keepline {
                    break;
                }
                if next {
                    row += 1;
                    begin_column = 0;
                } else {
                    if row == 0 {
                        break;
                    }
                    row = row - 1;
                    begin_column = u32::MAX;
                }
            }
        }

        *view.selections.borrow_mut() = selections;

        ctx.queue.push("selections-merge-overlapping");
        ctx.queue.push("look-keep-primary-cursor-in-view");
        Ok(())
    });

    // FIXME rename this command to duplicate-selection or something
    cr.register("dupe", |opt, ctx| {
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
        let buffer = ctx.state.buffers.get(view.buffer);
        let mut selections = view.selections.borrow().clone();
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

        *view.selections.borrow_mut() = selections;
        ctx.queue.push("selections-merge-overlapping");

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

        let sel_count = {
            // Using a block to limit the borrow to this line.
            view.selections.borrow().count()
        };
        for sel_idx in (0..sel_count).rev() {
            let Some(sel) = view.selections.borrow().get(sel_idx) else {
                continue;
            };
            buffer.insert_char_at(sel.cursor(), the_char)?;
        }

        ctx.events.emit("buffer-modified", "");
        Ok(())
    });

    cr.register("delete", |_opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let sel_count = {
            // Using a block to limit the borrow to this line.
            view.selections.borrow().count()
        };
        for sel_idx in (0..sel_count).rev() {
            let Some(sel) = view.selections.borrow().get(sel_idx) else {
                continue;
            };
            buffer.delete_selection(&sel)?;
        }

        ctx.queue.push("selections-merge-overlapping");
        ctx.events.emit("buffer-modified", "");
        Ok(())
    });

    cr.register("delete-around", |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let contains_previous = opt.contains("-p");
        let contains_next = opt.contains("-n");
        let (delete_before, delete_after) = if !contains_next && !contains_previous {
            (true, true)
        } else {
            (contains_previous, contains_next)
        };

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let sel_count = {
            // Using a block to limit the borrow to this line.
            view.selections.borrow().count()
        };
        for sel_idx in (0..sel_count).rev() {
            let Some(sel) = view.selections.borrow().get(sel_idx) else {
                continue;
            };

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

        ctx.queue.push("selections-merge-overlapping");
        ctx.events.emit("buffer-modified", "");
        Ok(())
    });

    cr.register("selection-shrink", |_opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.state.views.get(view_handle);
        for selection in view.selections.borrow_mut().iter_mut() {
            *selection = selection.shrunk_to_cursor();
        }

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
        for selection in view.selections.borrow_mut().iter_mut() {
            *selection = if forward {
                selection.flipped_forward()
            } else if backward {
                selection.flipped_forward().flipped()
            } else {
                selection.flipped()
            };
        }

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
        view.rebuild_line_wrap(&ctx.state.buffers, ctx.state.editor_size.column);

        Ok(())
    });
}
