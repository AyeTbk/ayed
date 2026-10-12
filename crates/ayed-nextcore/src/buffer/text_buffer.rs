use std::{cell::Cell, collections::HashMap, path::PathBuf};

use crate::{
    buffer::{BufferId, MarkerId, SelectionSet, TextEdit, ViewId},
    position::Position,
    util::id_map::IdMap,
};

// Invariants that must be respected for the lines:
// #1. There must always be at least one line. A line is a String the the lines vector.
// #2. All lines but the last end with the line separator, the '\n' character.
// Positions refer to lines in row and to codepoints (Rust chars) of said line in column.
pub struct TextBuffer {
    pub(super) id: BufferId,
    lines: Vec<String>,
    markers: IdMap<MarkerId, Position>,
    selections: HashMap<ViewId, SelectionSet>,

    history: (), // TODO eventually

    path: Option<PathBuf>,
    dirty: Cell<bool>,
    pub(crate) is_internal: bool,
}

impl TextBuffer {
    pub fn new_scratch() -> Self {
        Self::new_default()
    }

    pub fn new_from_path(filepath: impl Into<PathBuf>) -> Result<Self, String> {
        let mut this = Self::new_default();
        todo!("read from path, using iofacade");
        Ok(this)
    }

    fn new_default() -> Self {
        TextBuffer {
            id: Default::default(),
            lines: vec![String::new()],
            markers: Default::default(),
            selections: Default::default(),
            history: Default::default(),
            path: Default::default(),
            dirty: Default::default(),
            is_internal: Default::default(),
        }
    }

    pub fn apply_edit(&mut self, edit: TextEdit) -> TextEdit {
        todo!()
    }
}
