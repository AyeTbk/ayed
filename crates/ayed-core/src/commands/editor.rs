use std::collections::BTreeSet;

use regex::Regex;

use crate::{
    command::{
        CommandRegistry,
        helpers::{ErrorExt, alias, focused_buffer_command},
        options::Options,
    },
    config::ConfigState,
    position::{Column, Offset, Position},
    selection::{Selection, Selections},
    state::View,
    utils::string_utils::{
        byte_index_to_char_index, char_count,
        ops::{is_whitespace, take_while},
    },
};

pub fn register_editor_commands(cr: &mut CommandRegistry) {
    cr.register(
        "buffer-write",
        focused_buffer_command(|opt, ctx| {
            let path = if opt.is_empty() { None } else { Some(opt) };

            if let Some(path) = path {
                ctx.buffer.set_path(path);
            }

            ctx.buffer.write()?;

            ctx.queue.push(format!(
                "message written to {}",
                ctx.buffer.path().unwrap_or_default()
            ));

            Ok(())
        }),
    );
    cr.register("w", alias("buffer-write"));
    cr.register("wq", |_opt, ctx| {
        ctx.queue.push(format!("buffer-write"));
        ctx.queue.push(format!("quit"));
        Ok(())
    });

    cr.register("edit", |opt, ctx| {
        let opts = Options::new().flag("scratch").parse(opt)?;
        let scratch = opts.contains("scratch");
        let path = opts.remainder();

        let buffer_handle;
        let buffer_opened_path: Option<&str>;
        if path.is_empty() && scratch {
            buffer_handle = ctx.resources.open_scratch();
            buffer_opened_path = Some(path);
            ctx.queue.emit("buffer-opened", "");
        } else {
            match ctx.resources.buffer_with_path(path) {
                Some(handle) => {
                    buffer_handle = handle;
                    buffer_opened_path = None;
                }
                None => {
                    buffer_handle = ctx.resources.open_file_or_scratch(path)?;
                    buffer_opened_path = Some(path);
                }
            }
        }

        let view_handle = match ctx.resources.view_with_buffer(buffer_handle) {
            Some(handle) => handle,
            None => {
                let view = ctx.resources.views.insert(View {
                    top_left: Position::ZERO,
                    buffer: buffer_handle,
                });

                ctx.resources
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
    cr.register("e", alias("edit"));

    cr.register(
        "look",
        focused_buffer_command(|opt, ctx| {
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

            ctx.view.top_left = ctx.view.top_left.offset(offset);

            ctx.view.top_left.column = i32::max(0, ctx.view.top_left.column);
            ctx.view.top_left.row = i32::max(0, ctx.view.top_left.row);

            Ok(())
        }),
    );

    cr.register("look-keep-primary-cursor-in-view", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view() {
            let view_rect = ctx.state.focused_view_rect(&ctx.resources);
            let view = ctx.resources.views.get_mut(view_handle);
            let cursor = {
                let buffer = ctx.resources.buffers.get(view.buffer);
                let selections = buffer.view_selections(view_handle).unwrap();
                selections.primary().cursor
            };
            let offset = view_rect.offset_from_position(cursor);
            view.top_left = view.top_left.offset(offset);
        }

        Ok(())
    });

    cr.register(
        "move",
        focused_buffer_command(|opt, mut ctx| {
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

            for selection in ctx.selections.iter_mut() {
                let horizontal_move = offset.column != 0;
                if horizontal_move {
                    let new_cursor = ctx
                        .buffer
                        .move_position_horizontally(selection.cursor, offset.column)
                        .unwrap_or(selection.cursor);

                    *selection = if anchored {
                        selection.with_cursor(new_cursor)
                    } else {
                        selection.with_anchor(new_cursor).with_cursor(new_cursor)
                    };
                } else {
                    let logpos = ctx
                        .buffer
                        .map_true_position_to_logical_position(selection.cursor, &ctx.state.config);
                    if selection.old_logical_cursor_column.is_none() {
                        selection.old_logical_cursor_column = Some(logpos.column);
                    }
                    let desired_logpos = if let Some(column) = selection.old_logical_cursor_column {
                        logpos.with_column(column)
                    } else {
                        logpos
                    };
                    let moved_logpos = ctx
                        .buffer
                        .move_logical_position_vertically(
                            desired_logpos,
                            offset.row,
                            &ctx.state.config,
                        )
                        .unwrap_or(logpos);
                    let new_cursor = ctx
                        .buffer
                        .map_logical_position_to_true_position(moved_logpos, &ctx.state.config);

                    *selection = if anchored {
                        selection.with_provisional_cursor(new_cursor)
                    } else {
                        selection
                            .with_anchor(new_cursor)
                            .with_provisional_cursor(new_cursor)
                    };
                }
            }

            let sels = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
            *sels = ctx.selections;

            // TODO make this automatic maybe?, keep track of it in TextBuffer
            // Same for buffer-modified??
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "move-to-edge",
        focused_buffer_command(|opt, mut ctx| {
            enum Edge {
                LineStart,
                LinePastIndent,
                LineEnd,
                LinePastEnd,
                BufferStart,
                BufferEnd,
            }

            let opts = Options::new()
                .flag("anchored")
                .flag("reanchored")
                .parse(opt)?;
            let anchored = opts.contains("anchored");
            let reanchored = opts.contains("reanchored"); // Behaves like kakoune

            let edge = match opts.remainder().trim() {
                "line-start" => Edge::LineStart,
                "line-past-indent" => Edge::LinePastIndent,
                "line-end" => Edge::LineEnd,
                "line-past-end" => Edge::LinePastEnd,
                "buffer-start" => Edge::BufferStart,
                "buffer-end" => Edge::BufferEnd,
                rem => {
                    return Err(format!("edge unknown '{rem}'"));
                }
            };

            for selection in ctx.selections.iter_mut() {
                let original_cursor = selection.cursor;
                let mut cursor = selection.cursor;

                match edge {
                    Edge::LineStart => cursor = cursor.with_column(0),
                    Edge::LinePastIndent => {
                        let line = ctx.buffer.line(cursor.row).expect("cursor should be valid");
                        if let Some(not_indent_idx) = line.find(|c| !is_whitespace(c)) {
                            let indent = &line[..not_indent_idx];
                            let new_column = indent.chars().count() as Column;
                            cursor = cursor.with_column(new_column);
                        }
                    }
                    Edge::LineEnd => {
                        let maybe_column = ctx.buffer.line_char_count(cursor.row);
                        let new_column = i32::max(
                            maybe_column
                                .expect("cursor should be valid")
                                .saturating_sub(1),
                            0,
                        );
                        cursor = cursor.with_column(new_column);
                    }
                    Edge::LinePastEnd => {
                        let maybe_column = ctx.buffer.line_char_count(cursor.row);
                        let new_column = maybe_column.expect("cursor should be valid");
                        cursor = cursor.with_column(new_column);
                    }
                    Edge::BufferStart => {
                        cursor = Position::new(0, 0);
                    }
                    Edge::BufferEnd => {
                        cursor = ctx.buffer.end_position();
                    }
                }

                let mut sel = selection.with_cursor(cursor);
                if reanchored {
                    sel = sel.with_anchor(original_cursor);
                } else if !anchored {
                    sel = sel.with_anchor(cursor);
                }
                *selection = sel;
            }

            let sels = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
            *sels = ctx.selections;

            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

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

        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let mut selections = buffer.view_selections(view_handle).unwrap().clone();
        let regex = Regex::new(pattern).map_err(|e| e.to_string())?;

        for selection in selections.iter_mut() {
            let cursor = selection.cursor;
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
                        let mut new_anchor = Position::new(new_anchor_column, row);
                        // Limit new anchor to where the cursor was
                        if reversed {
                            if new_anchor > cursor {
                                new_anchor = cursor;
                            }
                        } else {
                            if new_anchor < cursor {
                                new_anchor = cursor;
                            }
                        }
                        *selection = selection.with_anchor(new_anchor);
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

    cr.register(
        "select-regex",
        focused_buffer_command(|opt, ctx| {
            // TODO Design a command to execute another command with modeline written args

            // For every selection, find matches to the regex pattern to make new selections out of.
            // If this results in no selections, don't overwrite the selections and err out,

            let pattern = opt;
            let re_pattern = Regex::new(pattern).or_strerr()?;

            let mut new_selections = Vec::new();
            for sel in ctx.selections.iter() {
                let start_idx = ctx.buffer.map_position_to_byte_index(sel.start()).unwrap();
                let Some(text) = ctx.buffer.selection_text(sel) else { continue };
                for matsh in re_pattern.find_iter(&text) {
                    let start = ctx
                        .buffer
                        .map_byte_index_to_position(start_idx + matsh.start(), false)
                        .unwrap();
                    let end = ctx
                        .buffer
                        .map_byte_index_to_position(start_idx + matsh.end(), true)
                        .unwrap();

                    let new_sel = if sel.is_forward() {
                        Selection::new().with_anchor(start).with_cursor(end)
                    } else {
                        Selection::new().with_cursor(start).with_anchor(end)
                    };
                    new_selections.push(new_sel);
                }
            }

            if new_selections.is_empty() {
                return Err("no selections left".to_string());
            }
            let sels = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
            sels.primary_selection = new_selections.remove(0);
            sels.extra_selections = new_selections;

            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "insert-char",
        focused_buffer_command(|opt, ctx| {
            let the_char = if opt == r"\n" {
                '\n'
            } else {
                opt.chars()
                    .next()
                    .ok_or_else(|| format!("not a char: {opt}"))?
            };

            let sel_count = ctx.selections.count();

            for sel_idx in 0..sel_count {
                let Some(sel) = ctx
                    .buffer
                    .view_selections(ctx.view_handle)
                    .unwrap()
                    .get(sel_idx)
                else {
                    continue;
                };
                ctx.buffer.insert_char_at(sel.cursor, the_char)?;
            }

            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "insert-str",
        focused_buffer_command(|opt, ctx| {
            let the_str = opt.replace(r"\n", "\n");

            let sel_count = ctx.selections.count();

            for sel_idx in 0..sel_count {
                let Some(sel) = ctx
                    .buffer
                    .view_selections(ctx.view_handle)
                    .unwrap()
                    .get(sel_idx)
                else {
                    continue;
                };
                ctx.buffer.insert_str_at(sel.cursor, &the_str)?;
            }

            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "delete",
        focused_buffer_command(|opt, ctx| {
            let contains_cursor = opt.contains("-c");

            let sel_count = ctx.selections.count();

            for sel_idx in (0..sel_count).rev() {
                let selections = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
                let Some(mut sel) = selections.get(sel_idx) else {
                    continue;
                };
                if contains_cursor {
                    sel = sel.shrunk_to_cursor();
                }
                ctx.buffer.delete_selection(&sel)?;
            }

            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "delete-around",
        focused_buffer_command(|opt, ctx| {
            let contains_cursor = opt.contains("-c");
            let contains_previous = opt.contains("-p");
            let contains_next = opt.contains("-n");
            let (delete_before, delete_after) = if !contains_next && !contains_previous {
                (true, true)
            } else {
                (contains_previous, contains_next)
            };

            let sel_count = ctx.selections.count();

            for sel_idx in (0..sel_count).rev() {
                let selections = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
                let Some(mut sel) = selections.get(sel_idx) else {
                    continue;
                };

                if contains_cursor {
                    sel = sel.shrunk_to_cursor();
                }

                if delete_after {
                    let from = sel.end();
                    let Some(at) = ctx.buffer.move_position_horizontally(from, 1) else {
                        continue;
                    };
                    ctx.buffer.delete_at(at)?;
                }
                if delete_before {
                    let from = sel.start();
                    let Some(at) = ctx.buffer.move_position_horizontally(from, -1) else {
                        continue;
                    };
                    ctx.buffer.delete_at(at)?;
                }
            }

            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "indent",
        focused_buffer_command(|opt, ctx| {
            let opts = Options::new()
                .flag("more")
                .flag("less")
                .flag("reindent")
                .flag("auto")
                .flag("auto-dedent") // To be used with auto, to check whether to dedent or no considering the current line
                .parse(opt)?;
            let mut more = opts.contains("more");
            let less = opts.contains("less");
            let reindent = opts.contains("reindent");
            let auto = opts.contains("auto");
            let auto_dedent = opts.contains("auto-dedent");
            if !(more || less || reindent || auto) {
                more = true;
            }

            let mut affected_lines = BTreeSet::new();
            for sel in ctx.selections.iter() {
                for line_sel in sel.split_lines() {
                    affected_lines.insert(line_sel.cursor.row);
                }
            }

            let indent_size = ctx.state.config.get_editor().indent_size;
            for row in affected_lines {
                let Some(line) = ctx.buffer.line(row) else { continue };

                let mut level_mod = (more as i32) - (less as i32);

                if auto && auto_dedent {
                    if line.trim() == "}" {
                        level_mod -= 1;
                    } else {
                        continue;
                    }
                }

                let indentation = take_while(line, is_whitespace).0;
                let indent_char_count = char_count(indentation) as i32;
                let new_indent_char_count = if auto {
                    let Some(prev_line) = ctx.buffer.line(row - 1) else { continue };
                    if let Some('{') = prev_line.trim_end().chars().last() {
                        level_mod += 1;
                    }
                    char_count(take_while(prev_line, is_whitespace).0) as i32
                } else {
                    indent_char_count
                };
                let new_indent_level = i32::max(new_indent_char_count / indent_size + level_mod, 0);
                let new_indent_char_count = new_indent_level * indent_size;
                let new_indentation = " ".repeat(new_indent_char_count as usize);

                if indent_char_count > 0 {
                    let indent_sel = Selection::new()
                        .with_anchor(Position::new(0, row))
                        .with_cursor(Position::new(indent_char_count - 1, row));
                    ctx.buffer.delete_selection(&indent_sel)?;
                }

                ctx.buffer
                    .insert_str_at(Position::new(0, row), &new_indentation)?;
            }

            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register("__auto-indent-shim", |opt, ctx| {
        match opt {
            // TODO find a way to not have to check for both of these (like avoid the raw string)
            "\n" | r"\n" => ctx.queue.push("indent --auto"),
            // FIXME find a proper and cleaner way to handle nesting aware auto-indent/dedent
            "}" => ctx.queue.push("indent --auto --auto-dedent"),
            _ => (),
        }
        Ok(())
    });

    cr.register("selections-merge-overlapping", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view() {
            let view = ctx.resources.views.get_mut(view_handle);
            let buffer = ctx.resources.buffers.get_mut(view.buffer);
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            *selections = selections.overlapping_selections_merged()
        }

        Ok(())
    });

    cr.register("selections-dismiss-extras", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view() {
            let view = ctx.resources.views.get_mut(view_handle);
            let buffer = ctx.resources.buffers.get_mut(view.buffer);
            let selections = buffer.view_selections_mut(view_handle).unwrap();
            selections.dismiss_extras();
        }

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selections-set", |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.resources.views.get_mut(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let selections = buffer.view_selections_mut(view_handle).unwrap();
        *selections = Selections::parse(&opt)?;

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selection-shrink", |_opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let selections = buffer.view_selections_mut(view_handle).unwrap();
        for selection in selections.iter_mut() {
            *selection = selection.shrunk_to_cursor();
        }

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selections-flip", |opt, ctx| {
        let opts = Options::new().flag("forward").flag("backward").parse(opt)?;
        let forward = opts.contains("forward");
        let backward = opts.contains("backward");

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
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

    cr.register("selections-duplicate", |opt, ctx| {
        let opts = Options::new().flag("up").flag("down").parse(opt)?;
        let up = opts.contains("up");
        let down = opts.contains("down");

        let row_offset = -(up as i8) + (down as i8);
        let offset = Offset::new(0, row_offset as i32);

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let mut selections = buffer.view_selections(view_handle).unwrap().clone();

        let make_dupe = |sel: Selection, offset| {
            buffer.limit_selection_to_content(
                &sel.with_anchor(sel.anchor.offset(offset))
                    .with_cursor(sel.cursor.offset(offset)),
            )
        };

        let mut new_extra_sels = Vec::new();

        for (i, &sel) in selections.iter().enumerate() {
            if i != 0 {
                new_extra_sels.push(sel);
            }
            new_extra_sels.push(make_dupe(sel, offset));
        }
        selections.extra_selections = new_extra_sels;
        selections.rotate(1);

        *buffer.view_selections_mut(view_handle).unwrap() = selections;

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register(
        "selections-rotate",
        focused_buffer_command(|opt, ctx| {
            let opts = Options::new().flag("reversed").parse(opt)?;
            let reversed = opts.contains("reversed");
            let rotate_amount = if reversed { -1 } else { 1 };

            let selections = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
            selections.rotate(rotate_amount);

            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );
}
