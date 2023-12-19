use std::collections::HashMap;

use super::{SelectionsId, TextBuffer};
use crate::{scripted_command::ScriptedCommand, selection::Selection, state::State};

pub fn register_commands(commands: &mut HashMap<String, ScriptedCommand>) {
    register_command!(commands, move_cursor_up);
    register_command!(commands, move_cursor_down);
    register_command!(commands, move_cursor_left);
    register_command!(commands, move_cursor_right);
}

pub fn move_cursor_up(state: &mut State, args: &str) -> Result<(), String> {
    let _anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_up_impl(buffer, selections_id);
    Ok(())
}
pub fn move_cursor_up_impl(buffer: &mut TextBuffer, selections_id: SelectionsId) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_up(selection.desired_cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = selection
            .with_provisional_cursor(moved_cursor)
            .with_provisional_anchor(moved_cursor);
    }
}

pub fn move_cursor_down(state: &mut State, args: &str) -> Result<(), String> {
    let _anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_down_impl(buffer, selections_id);
    Ok(())
}
pub fn move_cursor_down_impl(buffer: &mut TextBuffer, selections_id: SelectionsId) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_down(selection.desired_cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = selection
            .with_provisional_cursor(moved_cursor)
            .with_provisional_anchor(moved_cursor);
    }
}

pub fn move_cursor_left(state: &mut State, args: &str) -> Result<(), String> {
    let _anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_left_impl(buffer, selections_id);
    Ok(())
}
pub fn move_cursor_left_impl(buffer: &mut TextBuffer, selections_id: SelectionsId) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_left(selection.cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = Selection::with_position(moved_cursor);
    }
}

pub fn move_cursor_right(state: &mut State, args: &str) -> Result<(), String> {
    let _anchored = matches!(args.split_whitespace().next(), Some("anchor"));
    let selections_id = state.editors.active_editor().selections_id();
    let buffer = state.buffers.active_buffer_mut();
    move_cursor_right_impl(buffer, selections_id);
    Ok(())
}
pub fn move_cursor_right_impl(buffer: &mut TextBuffer, selections_id: SelectionsId) {
    for selection_idx in 0..buffer.get_selections(selections_id).count() {
        let selection = buffer.get_selection(selections_id, selection_idx).unwrap();
        let moved_cursor = buffer.move_position_right(selection.cursor());
        let sel = buffer
            .get_selection_mut(selections_id, selection_idx)
            .unwrap();
        *sel = Selection::with_position(moved_cursor);
    }
}
