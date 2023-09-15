use crate::{
    arena::Handle, buffer::TextBuffer, command::EditorCommand, controls::TextBufferEdit,
    selection::Position, state::State, ui_state::UiPanel, utils::Rect,
};

pub struct TextEditor {
    buffer: Handle<TextBuffer>,
    inner: TextBufferEdit,
    current_mode: String,
}

impl TextEditor {
    pub fn new(buffer: Handle<TextBuffer>) -> Self {
        let mut this = Self {
            buffer,
            inner: TextBufferEdit::new(),
            current_mode: String::new(),
        };
        this.check_current_mode();
        this
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.inner.set_rect(rect);
    }

    pub fn buffer(&self) -> Handle<TextBuffer> {
        self.buffer
    }

    pub fn mode(&self) -> &str {
        &self.current_mode
    }

    pub fn set_mode(&mut self, mode: String) {
        self.current_mode = mode;
    }

    pub fn view_top_left_position(&self) -> Position {
        self.inner.view_top_left_position()
    }

    pub fn execute_command(&mut self, command: EditorCommand, state: &mut State) {
        let mut fake_state = state.dummy_clone();
        // NOTE the fake_state only exists because I want to pass both the active buffer and
        // the whole state separately, which runs afoul of the borrow checker. The reason I
        // pass the buffer separately is purely for the LineEdit control, which reuses
        // TextEditor internally.
        // TODO Refactor all of this in a satisfactory manner.
        let buffer = state.buffers.get_mut(self.buffer);

        self.check_current_mode();

        self.inner.execute_command(command, buffer, &mut fake_state);
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let buffer = state.buffers.get(self.buffer);
        self.inner.render(buffer, state)
    }

    fn check_current_mode(&mut self) {
        match self.current_mode.as_str() {
            "command" => (),
            "edit" => (),
            _ => self.current_mode = String::from("command"),
        }

        self.inner.use_alt_cursor_style = self.current_mode == "edit";
    }
}
