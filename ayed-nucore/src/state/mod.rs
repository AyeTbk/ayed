use crate::{
    config::Config,
    slotmap::{Handle, SlotMap},
    ui::Size,
};

mod text_buffer;
pub use text_buffer::TextBuffer;

mod view;
pub use view::View;

#[derive(Default)]
pub struct State {
    pub views: SlotMap<View>,
    pub active_view: Option<Handle<View>>,
    pub buffers: SlotMap<TextBuffer>,
    pub config: Config,
    pub modeline_err: Option<String>,
    pub quit_requested: bool,
    pub viewport_size: Size,
}

impl State {
    pub fn open_file(&mut self, path: &str) -> Result<Handle<TextBuffer>, String> {
        Ok(self.buffers.insert(TextBuffer::new_from_path(path)?))
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
