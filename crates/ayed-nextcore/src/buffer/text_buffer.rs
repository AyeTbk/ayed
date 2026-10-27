use std::{
    cell::Cell,
    path::{Path, PathBuf},
};

use crate::{
    buffer::{BufferId, MarkerId, TextEdit, View, ViewId},
    position::{LPosition, PPosition, Position},
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
    views: IdMap<u16, View>,

    history: (), // TODO eventually

    path: Option<PathBuf>,
    dirty: Cell<bool>,
    /// Version for the buffer's content. Must increment for every change, including undos. For LSP.
    content_version: Cell<i32>,
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
            views: Default::default(),
            history: Default::default(),
            path: Default::default(),
            dirty: Default::default(),
            content_version: Default::default(),
            is_internal: Default::default(),
        }
    }

    pub fn create_view(&mut self) -> ViewId {
        let id = self.views.insert(View::default());
        ViewId {
            id,
            buffer: self.id,
        }
    }

    pub fn delete_view(&mut self, view: ViewId) {
        debug_assert!(view.buffer == self.id);
        self.views.remove(view.id);
    }

    pub fn write(&self) -> Result<(), String> {
        todo!()
    }

    fn write_to(&self, path: &Path) -> Result<(), String> {
        todo!()
    }

    fn write_content<W: std::io::Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        todo!()
    }

    pub fn reload(&mut self) -> Result<(), String> {
        todo!()
    }

    pub fn has_changed_in_storage(&self) -> Result<bool, String> {
        todo!()
    }

    pub fn map_pposition_to_lposition(&self, from: PPosition, config: &()) -> LPosition {
        todo!()
    }

    pub fn map_lposition_to_pposition(&self, from: LPosition, config: &()) -> PPosition {
        todo!()
    }

    pub fn apply_edit(&mut self, edit: TextEdit) -> TextEdit {
        todo!()
    }

    fn mark_dirty(&self) {
        self.dirty.set(true);
        self.content_version.update(|n| n + 1);
    }
}
