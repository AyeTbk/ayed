use std::{collections::BTreeSet, path::Path};

use regex::Regex;

use crate::{
    command::{
        CommandRegistry,
        helpers::{ErrorExt, alias, focused_buffer_command, register_selection_movement},
        options::Options,
    },
    config::ConfigState,
    position::{Column, Offset, Position, Row},
    selection::{Selection, Selections},
    state::View,
    utils::{
        path::PathExt,
        string_utils::{
            byte_index_to_char_index, char_count,
            ops::{is_whitespace, take_while},
        },
    },
};

pub fn register_editor_commands(cr: &mut CommandRegistry) {
    cr.register(
        "buffer-write",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
            let path = if opt.is_empty() {
                None
            } else {
                let normalized_path = ctx.state.normalize_path(Path::new(opt));
                Some(normalized_path)
            };

            if let Some(path) = path {
                ctx.buffer.set_path(path);
            }

            ctx.buffer.write()?;

            if let Some(path) = ctx.buffer.path() {
                ctx.queue
                    .push(format!("buffer-saved {}", path.to_string_lossy()));

                let denormalized_path = ctx.state.denormalize_path(path);
                ctx.queue
                    .push(format!("message written to {denormalized_path:?}",));
            }

            Ok(())
        }),
    );
    cr.register("w", "nodoc", alias("buffer-write"));
    cr.register("wq", "nodoc", |_opt, ctx| {
        ctx.queue.push(format!("buffer-write"));
        ctx.queue.push(format!("quit"));
        Ok(())
    });

    cr.register(
        "buffer-close",
        Options::new().doc("Closes active buffer.").flag("force"),
        |opt, ctx| {
            let force = opt.contains("force");

            // Validity checks
            let Some(buffer_handle) = ctx.state.active_editor_buffer(&ctx.resources) else {
                return Err("no currently open buffer".into());
            };
            let buffer = ctx.resources.buffers.get(buffer_handle);
            if buffer.is_dirty() && !force {
                return Err(format!("there are unsaved changes"));
            }
            let path = buffer
                .path()
                .unwrap_or(Path::new(""))
                .to_str_or_err()?
                .to_string();

            ctx.queue.emit("buffer-closed", &path);

            ctx.queue.push(format!("buffer-close__part2 {path}"));

            Ok(())
        },
    );

    cr.register("buffer-close__part2", "nodoc", |opt, ctx| {
        let opt = opt.raw();
        let Some(buffer_handle) = ctx.resources.buffer_with_path(Path::new(opt)) else {
            return Err(format!("big oof, no such buffer: {opt}"));
        };

        // Cleanup buffer resource
        ctx.resources.buffers.remove(buffer_handle);
        ctx.state.per_buffer.remove(&buffer_handle);

        // Cleanup views resources
        let mut views_to_cleanup = Vec::new();
        for (view_handle, view) in ctx.resources.views.iter() {
            if view.buffer == buffer_handle {
                views_to_cleanup.push(view_handle);
            }
        }
        for view_handle in views_to_cleanup {
            ctx.resources.views.remove(view_handle);
        }

        ctx.state.active_editor_view = ctx.resources.views.keys().next();

        if ctx.state.active_editor_view.is_none() {
            ctx.queue.push("edit --scratch");
        }

        Ok(())
    });

    cr.register("buffer", "nodoc", |opt, ctx| {
        let name = opt.remainder();
        let Some(buffer_handle) = ctx.resources.buffer_with_name(name) else {
            return Err(format!("no buffer named '{name}'"));
        };

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
                    .set_view_selections(view, Selections::new());

                view
            }
        };

        ctx.state.active_editor_view = Some(view_handle);
        Ok(())
    });

    cr.register(
        "edit",
        Options::new().doc("nodoc").flag("scratch"),
        |opt, ctx| {
            let scratch = opt.contains("scratch");
            let mut position = Position::ZERO;
            let path = if opt.remainder().is_empty() {
                "".into()
            } else {
                let mut path_str = opt.remainder();
                // Parse :line:column notation - TODO make this better, and probably a seperate function too
                if let Some((stem1, suffix1)) = path_str.rsplit_once(':') {
                    path_str = stem1;
                    if let Some((stem2, suffix2)) = path_str.rsplit_once(':') {
                        path_str = stem2;
                        position.column = suffix1.parse::<Column>().unwrap() - 1;
                        position.row = suffix2.parse::<Row>().unwrap() - 1;
                    } else {
                        position.row = suffix1.parse::<Column>().unwrap() - 1;
                    }
                }
                ctx.state.normalize_path(Path::new(path_str))
            };

            let buffer_handle;
            let buffer_opened_path: Option<&Path>;
            if path.as_os_str().is_empty() && scratch {
                buffer_handle = ctx.resources.open_scratch();
                buffer_opened_path = Some(&path);
            } else {
                match ctx.resources.buffer_with_path(&path) {
                    Some(handle) => {
                        buffer_handle = handle;
                        buffer_opened_path = None;
                    }
                    None => {
                        buffer_handle = ctx.resources.open_file_or_scratch(&path)?;
                        buffer_opened_path = Some(&path);
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
                        .set_view_selections(view, Selections::new());

                    view
                }
            };

            ctx.state.active_editor_view = Some(view_handle);

            let buffer = ctx.resources.buffers.get(buffer_handle);
            position = buffer.limit_position_to_content(position);
            if let Some(format) = buffer.forced_format.as_ref() {
                ctx.queue.set_state(ConfigState::FORMAT, format);
            }

            // The state must be updated before 'buffer-opened' is emitted so that
            // hooked commands may behave correctly.
            ctx.queue
                .set_state(ConfigState::FILE, path.to_str_or_err()?);

            let sel = Selection::new().with_start_and_end(position, position);
            ctx.queue.push(format!("selections-set {}", sel));

            if let Some(path) = buffer_opened_path {
                ctx.queue.emit("buffer-opened", path.to_str_or_err()?);
            }

            Ok(())
        },
    );
    cr.register("e", "nodoc", alias("edit"));

    cr.register(
        "format-set",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let format = opt.raw().trim();
            ctx.buffer.forced_format = if format.is_empty() {
                None
            } else {
                ctx.queue.set_state(ConfigState::FORMAT, format);
                Some(format.to_string())
            };
            Ok(())
        }),
    );

    cr.register(
        "look",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
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

    cr.register(
        "look-set-top",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
            let new_row = i32::from_str_radix(opt.trim(), 10).map_err(|e| e.to_string())?;
            ctx.view.top_left.row = new_row;
            ctx.view.top_left.row = i32::max(0, ctx.view.top_left.row);
            Ok(())
        }),
    );

    cr.register("look-keep-primary-cursor-in-view", "nodoc", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view(&ctx.panels) {
            let view_rect = ctx
                .state
                .focused_view_content_rect(&ctx.panels, &ctx.resources)
                .unwrap();
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

    register_selection_movement(cr, "move", Options::new().doc("nodoc"), |opt, ctx| {
        let opt = opt.remainder();
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

        let mut selection = ctx.selection;

        let horizontal_move = offset.column != 0;
        if horizontal_move {
            let new_cursor = ctx
                .buffer
                .move_position_horizontally(selection.cursor, offset.column)
                .unwrap_or(selection.cursor);

            selection = selection.with_anchor(new_cursor).with_cursor(new_cursor);
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
                .move_logical_position_vertically(desired_logpos, offset.row, &ctx.state.config)
                .unwrap_or(logpos);
            let new_cursor = ctx
                .buffer
                .map_logical_position_to_true_position(moved_logpos, &ctx.state.config);

            selection = selection
                .with_anchor(new_cursor)
                .with_provisional_cursor(new_cursor);
        }

        Ok(Some(selection))
    });

    register_selection_movement(
        cr,
        "move-to-char",
        Options::new().doc("nodoc").flag("before"),
        |opt, ctx| {
            let before = opt.contains("before");
            let opt = opt.remainder();
            // FIXME in order to support options with pending commands,
            // hook arg substitution needs to be supported.
            let Some(ch) = opt.chars().next() else {
                return Err("missing target char".to_string());
            };
            let mut selection = ctx.selection;
            let cursor = selection.cursor;
            let start_row = cursor.row;

            let mut found_position = None;
            let mut start_column = cursor.column + 1;
            'find_pos: for row_i in start_row..ctx.buffer.line_count() {
                let Some(line) = ctx.buffer.line(row_i) else { break };

                // Find ch in line
                for (column, chr) in line.chars().enumerate().skip(start_column as _) {
                    if chr == ch {
                        found_position = Some(Position::new(column as i32, row_i));
                        break 'find_pos;
                    }
                }

                start_column = 0;
            }

            if let Some(pos) = found_position {
                let offset = if before { -1 } else { 0 };
                let new_cursor = ctx
                    .buffer
                    .move_position_horizontally(pos, offset)
                    .unwrap_or(pos);
                let sel = selection.with_cursor(new_cursor).shrunk_to_cursor();
                selection = sel;
            }

            Ok(Some(selection))
        },
    );

    register_selection_movement(
        cr,
        "move-to-edge",
        Options::new().doc("nodoc"),
        |opt, ctx| {
            enum Edge {
                LineStart,
                LinePastIndent,
                LineEnd,
                LinePastEnd,
                BufferStart,
                BufferEnd,
            }

            let edge = match opt.remainder().trim() {
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

            let mut selection = ctx.selection;
            let mut cursor = selection.cursor;
            let mut should_magnetize_to_infinity_and_beyond = false;

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
                    should_magnetize_to_infinity_and_beyond = true;
                    cursor = cursor.with_column(new_column);
                }
                Edge::BufferStart => {
                    cursor = Position::new(0, 0);
                }
                Edge::BufferEnd => {
                    cursor = ctx.buffer.end_position();
                }
            }

            let mut sel = selection.with_cursor(cursor).with_anchor(cursor);
            if should_magnetize_to_infinity_and_beyond {
                sel = sel.magnetized_to_infinity_and_beyond();
            }
            selection = sel;

            Ok(Some(selection))
        },
    );

    register_selection_movement(
        cr,
        "move-regex",
        Options::new().doc("nodoc").flag("reversed").flag("line"),
        |opt, ctx| {
            let reversed = opt.contains("reversed");
            let stay_within_line = opt.contains("line");
            let pattern = opt.remainder();

            let regex = Regex::new(pattern).map_err(|e| e.to_string())?;

            let mut selection = ctx.selection;
            let cursor = selection.cursor;
            let mut row = cursor.row;
            let mut search_start_column = cursor.column;

            'line: loop {
                let Some(line) = ctx.buffer.line(row) else { break 'line };
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

                    selection = selection
                        .with_cursor(Position::new(new_cursor_column, row))
                        .with_anchor(new_anchor);

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
                        row_is_out_of_bounds = row >= ctx.buffer.line_count();
                    };
                    if row_is_out_of_bounds {
                        break 'line;
                    }
                }
            }

            Ok(Some(selection))
        },
    );

    cr.register(
        "line",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let row_number = opt.raw().parse::<i32>().map_err(|e| e.to_string())?;
            let row = (row_number - 1).clamp(0, ctx.buffer.last_row());
            let sels = Selections::new_with(Selection::with_position(Position::new(0, row)), &[]);
            ctx.buffer.set_view_selections(ctx.view_handle, sels);

            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "select-regex",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            // TODO Design a command to execute another command with modeline written args

            // For every selection, find matches to the regex pattern to make new selections out of.
            // If this results in no selections, don't overwrite the selections and err out,

            let pattern = opt.raw();
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
            let mut sels = Selections::new();
            sels.primary_selection = new_selections.remove(0);
            sels.extra_selections = new_selections;
            ctx.buffer.set_view_selections(ctx.view_handle, sels);

            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "insert-char",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
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
                ctx.buffer.insert_char_at(the_char, sel.cursor)?;
            }

            ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "insert-str",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
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
                ctx.buffer.insert_str_at(&the_str, sel.cursor)?;
            }

            ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "replace",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let opt = opt.raw();
            let Some(ch) = opt.chars().next() else {
                return Err("missing replacement char".to_string());
            };

            let sel_count = ctx.selections.count();

            for sel_idx in (0..sel_count).rev() {
                let mut selections = ctx.buffer.view_selections(ctx.view_handle).unwrap().clone();
                let Some(sel) = selections.get_mut(sel_idx) else {
                    continue;
                };

                let delete_sel = sel.clone();
                let after_sel = delete_sel.end().offset((1, 0));
                *sel = sel.shrunk_to_cursor();
                ctx.buffer.set_view_selections(ctx.view_handle, selections);

                // TODO Should these operations somehow be made "transactionally"?
                // like, if it fails, the buffer isnt left unclean?
                // Could TextEdit based buffer modifications make this simple-ish?
                ctx.buffer.insert_char_at(ch, after_sel)?;
                ctx.buffer.delete_selection(&delete_sel)?;
            }

            ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "delete",
        "nodoc",
        focused_buffer_command(|opt, ctx| {
            let contains_cursor = opt.contains("-c");

            let sel_count = ctx.selections.count();

            let selections = ctx.buffer.view_selections(ctx.view_handle).unwrap().clone();
            for sel_idx in (0..sel_count).rev() {
                let Some(mut sel) = selections.get(sel_idx) else {
                    continue;
                };
                if contains_cursor {
                    sel = sel.shrunk_to_cursor();
                }
                ctx.buffer.delete_selection(&sel)?;
            }

            ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "delete-around",
        Options::new().doc("nodoc").flag("c").flag("p").flag("n"),
        focused_buffer_command(|opt, ctx| {
            let contains_cursor = opt.contains("c");
            let contains_previous = opt.contains("p");
            let contains_next = opt.contains("n");
            let (delete_before, delete_after) = if !contains_next && !contains_previous {
                (true, true)
            } else {
                (contains_previous, contains_next)
            };

            let sel_count = ctx.selections.count();

            let selections = ctx.buffer.view_selections(ctx.view_handle).unwrap().clone();
            for sel_idx in (0..sel_count).rev() {
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

            ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register(
        "indent",
        Options::new()
            .doc("nodoc")
            .flag("more")
            .flag("less")
            .flag("reindent")
            .flag("auto")
            .flag("auto-dedent"), // To be used with auto, to check whether to dedent or no considering the current line
        focused_buffer_command(|opt, ctx| {
            let mut more = opt.contains("more");
            let less = opt.contains("less");
            let reindent = opt.contains("reindent");
            let auto = opt.contains("auto");
            let auto_dedent = opt.contains("auto-dedent");
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
                    .insert_str_at(new_indentation, Position::new(0, row))?;
            }

            ctx.queue.emit("buffer-modified", ctx.buffer.path_str());
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );

    cr.register("__auto-indent-shim", "nodoc", |opt, ctx| {
        let opt = opt.raw();
        match opt {
            // TODO find a way to not have to check for both of these (like avoid the raw string)
            "\n" | r"\n" => ctx.queue.push("indent --auto"),
            // FIXME find a proper and cleaner way to handle nesting aware auto-indent/dedent
            "}" => ctx.queue.push("indent --auto --auto-dedent"),
            _ => (),
        }
        Ok(())
    });

    cr.register("selections-merge-overlapping", "nodoc", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view(&ctx.panels) {
            let view = ctx.resources.views.get_mut(view_handle);
            let buffer = ctx.resources.buffers.get_mut(view.buffer);
            let selections = buffer.view_selections(view_handle).unwrap();
            buffer.set_view_selections(view_handle, selections.overlapping_selections_merged());
        }

        Ok(())
    });

    cr.register("selections-dismiss-extras", "nodoc", |_opt, ctx| {
        if let Some(view_handle) = ctx.state.focused_view(&ctx.panels) {
            let view = ctx.resources.views.get_mut(view_handle);
            let buffer = ctx.resources.buffers.get_mut(view.buffer);
            let mut selections = buffer.view_selections(view_handle).unwrap().clone();
            selections.dismiss_extras();
            buffer.set_view_selections(view_handle, selections);
        }

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selections-set", "nodoc", |opt, ctx| {
        let opt = opt.raw();
        let Some(view_handle) = ctx.state.focused_view(&ctx.panels) else {
            return Ok(());
        };

        let view = ctx.resources.views.get_mut(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        buffer.set_view_selections(view_handle, Selections::parse(&opt)?);

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("selections-shrink", "nodoc", |_opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view(&ctx.panels) else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let mut selections = buffer.view_selections(view_handle).unwrap().clone();
        for selection in selections.iter_mut() {
            *selection = selection.shrunk_to_cursor();
        }
        buffer.set_view_selections(view_handle, selections);

        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register(
        "selections-flip",
        Options::new().doc("nodoc").flag("forward").flag("backward"),
        |opt, ctx| {
            let forward = opt.contains("forward");
            let backward = opt.contains("backward");

            let Some(view_handle) = ctx.state.focused_view(&ctx.panels) else {
                return Ok(());
            };
            let view = ctx.resources.views.get(view_handle);
            let buffer = ctx.resources.buffers.get_mut(view.buffer);
            let mut selections = buffer.view_selections(view_handle).unwrap().clone();
            let mut anything_changed = false;
            for selection in selections.iter_mut() {
                let flipped_sel = if forward {
                    selection.flipped_forward()
                } else if backward {
                    selection.flipped_forward().flipped()
                } else {
                    selection.flipped()
                };

                if *selection != flipped_sel {
                    anything_changed = true;
                }

                *selection = flipped_sel;
            }
            buffer.set_view_selections(view_handle, selections);

            if anything_changed {
                ctx.queue.emit("selections-modified", "");
            }

            Ok(())
        },
    );

    cr.register(
        "selections-duplicate",
        Options::new().doc("nodoc").flag("up").flag("down"),
        |opt, ctx| {
            let up = opt.contains("up");
            let down = opt.contains("down");

            let row_offset = -(up as i8) + (down as i8);
            let offset = Offset::new(0, row_offset as i32);

            let Some(view_handle) = ctx.state.focused_view(&ctx.panels) else {
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

            buffer.set_view_selections(view_handle, selections);

            ctx.queue.emit("selections-modified", "");

            Ok(())
        },
    );

    cr.register(
        "selections-rotate",
        Options::new().doc("nodoc").flag("reversed"),
        focused_buffer_command(|opt, ctx| {
            let reversed = opt.contains("reversed");
            let rotate_amount = if reversed { -1 } else { 1 };

            let mut selections = ctx.buffer.view_selections(ctx.view_handle).unwrap().clone();
            selections.rotate(rotate_amount);
            ctx.buffer.set_view_selections(ctx.view_handle, selections);

            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );
}
