use crate::{
    arena::{Arena, Handle},
    buffer::TextBuffer,
    command::EditorCommand,
    controls::TextBufferEdit,
    highlight::{Highlight, HighlightPosition},
    ui_state::{Span, UiPanel},
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

    pub fn render(&mut self, buffer: &TextBuffer, highlights: &[Highlight]) -> UiPanel {
        let mut panel = self.inner.render(buffer);
        panel.spans.extend(
            highlights
                .iter()
                .cloned()
                .flat_map(|h| self.convert_highlight_to_span(h)),
        );
        panel
    }

    fn check_current_mode(&mut self) {
        match self.current_mode.as_str() {
            "command" => (),
            "edit" => (),
            _ => self.current_mode = String::from("command"),
        }

        self.inner.use_alt_cursor_style = self.current_mode == "edit";
    }

    fn convert_highlight_to_span(&self, highlight: Highlight) -> Option<Span> {
        let maybe_from_to = match highlight.position {
            HighlightPosition::Panel { from, to } => Some((from, to)),
            HighlightPosition::Content { from, to } => {
                let top_left = self.view_top_left_position();

                let maybe_from = if from < top_left {
                    None
                } else {
                    Some(from - top_left)
                };
                let maybe_to = if to < top_left {
                    None
                } else {
                    Some(to - top_left)
                };

                match (maybe_from, maybe_to) {
                    (Some(from), Some(to)) => Some((from, to)),
                    (None, Some(to)) => Some((Position::ZERO, to)),
                    _ => None,
                }
            }
        };

        maybe_from_to.map(|(from, to)| Span {
            from,
            to,
            style: highlight.style,
            priority: highlight.priority,
        })
    }
}
