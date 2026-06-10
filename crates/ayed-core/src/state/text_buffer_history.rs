use std::collections::HashMap;

use crate::{selection::Selections, slotmap::Handle, state::TextEdit};

use super::View;

// Undo/redo stack.
// Edit history and motion history are separate.
// Edit history happens in edit groups, the boundaries of which are
// defined by "recording" checkpoints. Edit groups have a snapshot
// of all selections before and after the edits of the group. Groups
// as a whole are undo/redo steps.
// Motion history is separate. Dont bother with it for now. Make
// edit history work, and motion history can be tacked on later.
// Unlike edits, motions arent grouped or checkpointed.
// Undoing edits may or may not undo motions, figure out what feels
// best to use.
// TODO motion history

type AllSelections = HashMap<Handle<View>, Selections>;

#[derive(Debug, Default)]
pub struct TextBufferHistory {
    pub current_group_edge_idx: usize,
    pub edit_groups: Vec<EditGroup>,
}

impl TextBufferHistory {
    pub fn record_edit(
        &mut self,
        edit: TextEdit,
        selections_before: &AllSelections,
        selections_after: &AllSelections,
    ) {
        // Get rid of potential redo groups.
        self.edit_groups.drain(self.current_group_edge_idx..);

        // If the current edit group doesn't exist, or exists but is complete,
        // add a new group and make it the current.
        if matches!(
            self.current_edit_group().map(|g| g.is_complete),
            None | Some(true)
        ) {
            self.edit_groups.push(Default::default());
            self.current_group_edge_idx = self.current_group_edge_idx + 1;
        }

        let edit_group = self
            .current_edit_group_mut()
            .expect("none case should be handled above");

        if edit_group.edits.is_empty() {
            edit_group.selections_before = selections_before.clone();
        }
        edit_group.selections_after = selections_after.clone();
        edit_group.edits.push(edit);
    }

    pub fn record_checkpoint(&mut self) {
        let Some(edit_group) = self.current_edit_group_mut() else {
            return;
        };
        if edit_group.is_complete {
            return;
        }
        edit_group.is_complete = true;
    }

    pub fn can_undo(&self) -> bool {
        self.current_group_edge_idx != 0
    }

    pub fn can_redo(&self) -> bool {
        self.current_group_edge_idx != self.edit_groups.len()
    }

    fn current_edit_group(&self) -> Option<&EditGroup> {
        let idx = self.current_edit_group_idx()?;
        Some(&self.edit_groups[idx])
    }

    pub fn current_edit_group_mut(&mut self) -> Option<&mut EditGroup> {
        let idx = self.current_edit_group_idx()?;
        Some(&mut self.edit_groups[idx])
    }

    fn current_edit_group_idx(&self) -> Option<usize> {
        (self.current_group_edge_idx).checked_sub(1)
    }
}

#[derive(Debug, Default)]
pub struct EditGroup {
    pub edits: Vec<TextEdit>,
    pub selections_before: AllSelections,
    pub selections_after: AllSelections,
    pub is_complete: bool,
}
