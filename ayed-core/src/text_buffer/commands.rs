use std::collections::HashMap;

use super::{first_non_whitespace_column_of_line, SelectionsId, TextBuffer};
use crate::{scripted_command::ScriptedCommand, state::State, utils::Position};

pub fn register_commands(commands: &mut HashMap<String, ScriptedCommand>) {
    register_command!(commands, move_cursor_up);
    register_command!(commands, move_cursor_down);
    register_command!(commands, move_cursor_left);
    register_command!(commands, move_cursor_right);
    register_command!(commands, move_cursor_to_near_symbol);
}

pub fn move_cursor_up(state: &mut State, args: &str) -> Result<(), String> {
    let anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_up_impl(buffer, selections_id, anchored);
    Ok(())
}
pub fn move_cursor_up_impl(buffer: &mut TextBuffer, selections_id: SelectionsId, anchored: bool) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_up(selection.desired_cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = sel.with_provisional_cursor(moved_cursor);
        if !anchored {
            *sel = sel.with_provisional_anchor(moved_cursor);
        }
    }

    buffer.merge_overlapping_selections();
}

pub fn move_cursor_down(state: &mut State, args: &str) -> Result<(), String> {
    let anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_down_impl(buffer, selections_id, anchored);
    Ok(())
}
pub fn move_cursor_down_impl(buffer: &mut TextBuffer, selections_id: SelectionsId, anchored: bool) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_down(selection.desired_cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = sel.with_provisional_cursor(moved_cursor);
        if !anchored {
            *sel = sel.with_provisional_anchor(moved_cursor);
        }
    }

    buffer.merge_overlapping_selections();
}

pub fn move_cursor_left(state: &mut State, args: &str) -> Result<(), String> {
    let anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_left_impl(buffer, selections_id, anchored);
    Ok(())
}
pub fn move_cursor_left_impl(buffer: &mut TextBuffer, selections_id: SelectionsId, anchored: bool) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_left(selection.cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = sel.with_cursor(moved_cursor);
        if !anchored {
            *sel = sel.with_anchor(moved_cursor);
        }
    }

    buffer.merge_overlapping_selections();
}

pub fn move_cursor_right(state: &mut State, args: &str) -> Result<(), String> {
    let anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_right_impl(buffer, selections_id, anchored);
    Ok(())
}
pub fn move_cursor_right_impl(
    buffer: &mut TextBuffer,
    selections_id: SelectionsId,
    anchored: bool,
) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_right(selection.cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = sel.with_cursor(moved_cursor);
        if !anchored {
            *sel = sel.with_anchor(moved_cursor);
        }
    }

    buffer.merge_overlapping_selections();
}

pub fn move_cursor_to_near_symbol(state: &mut State, args: &str) -> Result<(), String> {
    let anchored = args.split_whitespace().any(|arg| arg == "anchor");
    let select = args.split_whitespace().any(|arg| arg == "select");
    let previous = args.split_whitespace().any(|arg| arg == "previous");
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_to_near_symbol_impl(buffer, selections_id, anchored, select, !previous);
    Ok(())
}
pub fn move_cursor_to_near_symbol_impl(
    buffer: &mut TextBuffer,
    selections_id: SelectionsId,
    anchored: bool,
    select_symbol: bool,
    next_instead_of_previous: bool,
) {
    let re_symbol = regex::Regex::new(r"\w+|[\{\}\[\]\(\)<>]|[^\w\s[\{\}\[\]\(\)<>]]+").unwrap();

    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();

        let cursor = selection.cursor();
        let mut cursor_column = cursor.column as i64;
        let line_indices: Vec<u32> = if next_instead_of_previous {
            (cursor.row..buffer.line_count()).collect()
        } else {
            (0..=cursor.row).rev().collect()
        };

        'line: for line_index in line_indices {
            let Some(line) = buffer.line(line_index) else {
                break;
            };

            let mut symbols_edges: Vec<(u32, u32)> = re_symbol
                .find_iter(&line)
                .map(|m| {
                    (
                        m.start().try_into().unwrap(),
                        m.end().saturating_sub(1).try_into().unwrap(),
                    )
                })
                .collect();
            if !next_instead_of_previous {
                symbols_edges.reverse();
            }

            let candidate_cursor_and_anchor_columns = symbols_edges
                .into_iter()
                .skip_while(|&(start, end)| {
                    (next_instead_of_previous && cursor_column >= end as i64)
                        || (!next_instead_of_previous && cursor_column <= start as i64)
                })
                .map(|(start, end)| {
                    if next_instead_of_previous {
                        (end, start)
                    } else {
                        (start, end)
                    }
                })
                .next();

            if let Some((curosr_column, anchor_column)) = candidate_cursor_and_anchor_columns {
                let new_cursor = Position::new(curosr_column, line_index);
                let new_anchor = if anchored {
                    selection.anchor()
                } else if select_symbol {
                    Position::new(anchor_column, line_index)
                } else {
                    new_cursor
                };

                let selection = buffer
                    .get_selection_mut(selections_id, selection_idx)
                    .unwrap();
                *selection = selection.with_cursor(new_cursor).with_anchor(new_anchor);

                break 'line;
            }

            cursor_column = if next_instead_of_previous {
                -1
            } else {
                u32::MAX as _
            };
        }
    }

    buffer.merge_overlapping_selections();
}

pub fn move_cursor_to_line_edge(state: &mut State, args: &str) -> Result<(), String> {
    let anchored = args.split_whitespace().any(|arg| arg == "anchor");
    let to_start = args.split_whitespace().any(|arg| arg == "start");
    let smart = args.split_whitespace().any(|arg| arg == "smart");
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_to_line_edge_impl(buffer, selections_id, anchored, to_start, smart);
    Ok(())
}
pub fn move_cursor_to_line_edge_impl(
    buffer: &mut TextBuffer,
    selections_id: SelectionsId,
    anchored: bool,
    to_start: bool,
    smart: bool,
) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();

        let cursor = selection.cursor();
        let column = if to_start {
            if smart {
                let first_non_whitespace_column =
                    first_non_whitespace_column_of_line(buffer, cursor.row).unwrap_or(0);
                if cursor.column != first_non_whitespace_column {
                    first_non_whitespace_column
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            let line_last_column = buffer.line_char_count(cursor.row).unwrap_or(0);
            if smart {
                let column_before_terminator = buffer
                    .line_char_count(cursor.row)
                    .unwrap_or(0)
                    .saturating_sub(1);
                if cursor.column != column_before_terminator {
                    column_before_terminator
                } else {
                    line_last_column
                }
            } else {
                line_last_column
            }
        };

        let selection = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *selection = selection
            .with_cursor(cursor.with_column(column))
            .with_desired_cursor_column_index(std::u32::MAX);
        if !anchored {
            *selection = selection.with_anchor(cursor.with_column(column));
        }
    }

    buffer.merge_overlapping_selections();
}

pub fn duplicate_selection_impl(buffer: &mut TextBuffer, selections_id: SelectionsId, above: bool) {
    let mut new_selections = Vec::new();
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();

        let selection_line_count: i32 = selection.line_span().try_into().unwrap();
        let line_offset = if above { -1 } else { 1 } * selection_line_count;
        let dupe_cursor = selection.cursor().with_moved_indices(0, line_offset);
        let dupe_anchor = selection.anchor().with_moved_indices(0, line_offset);
        let dupe = selection
            .with_provisional_cursor(dupe_cursor)
            .with_provisional_anchor(dupe_anchor);
        // Try to put the cursor and anchor at their desired column indices.
        let dupe = dupe
            .with_provisional_cursor(dupe.desired_cursor())
            .with_provisional_anchor(dupe.desired_anchor());
        let dupe = buffer.limit_selection_to_content(&dupe);

        new_selections.push(dupe);
    }

    let selections_set = buffer.get_selections_mut(selections_id);
    for (i, selection) in new_selections.into_iter().enumerate() {
        let index = selections_set.add(selection);
        if i == 0 {
            selections_set.change_primary(index);
        }
    }

    buffer.merge_overlapping_selections();
}
