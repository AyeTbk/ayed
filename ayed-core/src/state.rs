use crate::{
    arena::{Arena, Handle},
    buffer::TextBuffer,
    mode_line::ModeLineInfos,
};

#[derive(Debug)]
pub struct State {
    pub buffers: Arena<TextBuffer>,
    pub active_buffer_handle: Handle<TextBuffer>,
    pub viewport_size: (u32, u32),
    pub mode_line_infos: ModeLineInfos,
    //
    pub active_combo_mode_name: Option<&'static str>,
    pub active_editor_name: &'static str,
    pub active_mode_name: &'static str,
}

impl State {
    pub fn active_buffer(&self) -> &TextBuffer {
        self.buffers.get(self.active_buffer_handle)
    }

    pub fn active_buffer_mut(&mut self) -> &mut TextBuffer {
        self.buffers.get_mut(self.active_buffer_handle)
    }

    pub fn dummy_clone(&self) -> Self {
        Self {
            buffers: Default::default(),
            mode_line_infos: Default::default(),
            ..*self // FIXME this is really bad, the buffer_handle is copied but the new State doesnt have any buffer
        }
    }
}
