use crate::slotmap::{Handle, SlotMap};

use super::{TextBuffer, View};

#[derive(Default)]
pub struct Resources {
    pub views: SlotMap<View>,
    pub buffers: SlotMap<TextBuffer>,
}

impl Resources {
    pub fn open_file(&mut self, path: &str) -> Result<Handle<TextBuffer>, String> {
        Ok(self.buffers.insert(TextBuffer::new_from_path(path)?))
    }

    pub fn open_scratch(&mut self) -> Handle<TextBuffer> {
        self.buffers.insert(TextBuffer::new_empty())
    }

    pub fn open_file_or_scratch(&mut self, path: &str) -> Result<Handle<TextBuffer>, String> {
        if let Ok(true) = std::fs::exists(path) {
            self.open_file(path)
        } else {
            let mut buffer = TextBuffer::new_empty();
            buffer.set_path(path);
            Ok(self.buffers.insert(buffer))
        }
    }

    pub fn buffer_with_path(&self, path: &str) -> Option<Handle<TextBuffer>> {
        self.buffers
            .iter()
            .find(|(_, buf)| buf.path() == Some(path))
            .map(|(handle, _)| handle)
    }

    pub fn view_with_buffer(&self, buffer: Handle<TextBuffer>) -> Option<Handle<View>> {
        self.views
            .iter()
            .find(|(_, view)| view.buffer == buffer)
            .map(|(handle, _)| handle)
    }
}
