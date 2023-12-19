use crate::{
    arena::{Arena, Handle},
    command::EditorCommand,
    controls::TextEdit,
    highlight::{Highlight, HighlightPosition},
    panels::line_numbers::LineNumbers,
    text_buffer::{SelectionsId, TextBuffer},
    ui_state::{Span, UiPanel},
    utils::Position,
    utils::Rect,
};

pub struct TextEditor {
    buffer: Handle<TextBuffer>,
    rect: Rect,
    inner: TextEdit,
    line_numbers: LineNumbers,
    current_mode: String,
}

impl TextEditor {
    pub fn new(buffer: Handle<TextBuffer>) -> Self {
        let mut this = Self {
            buffer,
            rect: Rect::new(0, 0, 1, 1),
            inner: TextEdit::new(),
            line_numbers: LineNumbers::new(),
            current_mode: String::new(),
        };
        this.check_current_mode();
        this
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn inner_rect(&mut self) -> Rect {
        self.inner.rect()
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

    pub fn selections_id(&self) -> SelectionsId {
        self.inner.selections_id()
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

    pub fn render(&mut self, buffer: &TextBuffer, highlights: &[Highlight]) -> Vec<UiPanel> {
        // TODO split the details this into separate methods

        // Update inner rects (this is here because access to the buffer is needed)
        self.line_numbers
            .set_line_data(buffer.line_count(), self.view_top_left_position().row);
        let lines_w = self.line_numbers.needed_width();
        let lines_rect = Rect {
            x: self.rect.x,
            y: self.rect.y,
            width: lines_w,
            height: self.rect.height,
        };
        self.line_numbers.set_rect(lines_rect);
        let inner_rect = Rect {
            x: self.rect.x + lines_w,
            y: self.rect.y,
            width: self.rect.width.saturating_sub(lines_w),
            height: self.rect.height,
        };
        self.inner.set_rect(inner_rect);

        // Render stuff
        let mut panel = self.inner.render(buffer);
        panel.spans.extend(
            highlights
                .iter()
                .cloned()
                .flat_map(|h| self.convert_highlight_to_span(h)),
        );
        let line_numbers = self.line_numbers.render();
        vec![panel, line_numbers]
    }

    fn check_current_mode(&mut self) {
        match self.current_mode.as_str() {
            "command" => (),
            "edit" => (),
            _ => self.current_mode = String::from("command"),
        }

        // self.inner.use_alt_cursor_style = self.current_mode == "edit"; // TODO make this work again
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
