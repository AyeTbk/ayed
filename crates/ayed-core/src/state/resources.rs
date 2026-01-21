use std::path::Path;

use crate::slotmap::{Handle, SlotMap};

use super::{TextBuffer, View};

#[derive(Default)]
pub struct Resources {
    pub views: SlotMap<View>,
    pub buffers: SlotMap<TextBuffer>,
}

impl Resources {
    pub fn open_file(&mut self, absolute_path: &Path) -> Result<Handle<TextBuffer>, String> {
        debug_assert!(absolute_path.is_absolute());
        Ok(self.buffers.insert(TextBuffer::new_from_path(absolute_path)?))
    }

    pub fn open_scratch(&mut self) -> Handle<TextBuffer> {
        self.buffers.insert(TextBuffer::new_empty())
    }

    pub fn open_file_or_scratch(&mut self, absolute_path: &Path) -> Result<Handle<TextBuffer>, String> {
        debug_assert!(absolute_path.is_absolute());
        if let Ok(true) = std::fs::exists(absolute_path) {
            self.open_file(absolute_path)
        } else {
            let mut buffer = TextBuffer::new_empty();
            buffer.set_path(Some(absolute_path.to_path_buf()));
            Ok(self.buffers.insert(buffer))
        }
    }

    pub fn buffer_with_path(&self, absolute_path: &Path) -> Option<Handle<TextBuffer>> {
        debug_assert!(absolute_path.is_absolute());
        self.buffers
            .iter()
            .find(|(_, buf)| buf.path() == Some(absolute_path))
            .map(|(handle, _)| handle)
    }

    // FIXME this is inherently broken, there will eventually be the possibility of
    // buffers with multiple views
    pub fn view_with_buffer(&self, buffer: Handle<TextBuffer>) -> Option<Handle<View>> {
        self.views
            .iter()
            .find(|(_, view)| view.buffer == buffer)
            .map(|(handle, _)| handle)
    }
}
