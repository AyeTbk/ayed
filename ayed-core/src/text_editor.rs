use crate::{
    arena::{Arena, Handle},
    buffer::TextBuffer,
    command::EditorCommand,
    controls::TextBufferEdit,
    ui_state::UiPanel,
    utils::Position,
    utils::Rect,
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

    pub fn buffer_handle(&self) -> Handle<TextBuffer> {
        self.buffer
    }

    pub fn get_buffer<'a>(&self, buffers: &'a Arena<TextBuffer>) -> &'a TextBuffer {
        buffers.get(self.buffer)
    }

    pub fn get_buffer_mut<'a>(&self, buffers: &'a mut Arena<TextBuffer>) -> &'a mut TextBuffer {
        buffers.get_mut(self.buffer)
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

    pub fn execute_command(&mut self, command: EditorCommand, buffer: &mut TextBuffer) {
        self.check_current_mode();

        self.inner.execute_command(command, buffer);
    }

    pub fn render(&mut self, buffer: &TextBuffer) -> UiPanel {
        self.inner.render(buffer)
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
