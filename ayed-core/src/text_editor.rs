use crate::{
    arena::Handle, buffer::TextBuffer, command::EditorCommand, controls::TextBufferEdit,
    selection::Position, state::State, ui_state::UiPanel, utils::Rect,
};

pub struct TextEditor {
    buffer: Handle<TextBuffer>,
    inner: TextBufferEdit,
    is_command_mode: bool, // FIXME the current mode *should* be on a per editor basis, but is currently global.
}

impl TextEditor {
    pub fn new(buffer: Handle<TextBuffer>) -> Self {
        Self {
            buffer,
            inner: TextBufferEdit::new(),
            is_command_mode: true,
        }
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.inner.set_rect(rect);
    }

    pub fn buffer(&self) -> Handle<TextBuffer> {
        self.buffer
    }

    pub fn view_top_left_position(&self) -> Position {
        self.inner.view_top_left_position()
    }

    pub fn execute_command(&mut self, command: EditorCommand, state: &mut State) {
        let mut fake_state = state.dummy_clone(); // NOTE what is fake state used for?? i dont remember
        let buffer = state.buffers.get_mut(self.buffer);

        self.inner.execute_command(command, buffer, &mut fake_state);
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let buffer = state.buffers.get(self.buffer);
        self.inner.render(buffer, state)
    }
}
