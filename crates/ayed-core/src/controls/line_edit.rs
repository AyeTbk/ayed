use crate::{
    command::EditorCommand,
    text_buffer::TextBuffer,
    ui_state::{Span, Style, UiPanel},
    utils::{Position, Rect},
};

use super::TextEdit;

pub struct LineEdit {
    edit: TextEdit,
    buffer: TextBuffer,
    buffer_string: String,
}

impl LineEdit {
    pub fn new() -> Self {
        let buffer = TextBuffer::new_empty();
        let mut edit = TextEdit::new();
        edit.set_rect(Rect::new(0, 0, 25, 1));

        Self {
            edit,
            buffer,
            buffer_string: Default::default(),
        }
    }

    pub fn rect(&self) -> Rect {
        self.edit.rect()
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.edit.set_rect(rect);
    }

    pub fn text(&self) -> &str {
        &self.buffer_string
    }

    pub fn execute_command(&mut self, command: EditorCommand) -> Option<String> {
        match command {
            EditorCommand::Insert('\n') => {
                let line = self.buffer.line(0)?.to_owned();
                self.reset();
                return Some(line);
            }
            EditorCommand::DeleteBeforeCursor if self.buffer.is_empty() => {
                // NOTE Returning an empty string essentially means the line edit
                // should be dismissed.
                // This is mostly just for the modeline prompt UX.
                return Some("".into());
            }
            _ => {
                self.edit.execute_command(command, &mut self.buffer);

                self.buffer
                    .line(0)
                    .clone()
                    .expect("there should always be at least one line");
            }
        }

        None
    }

    pub fn render(&mut self) -> UiPanel {
        let mut editor_panel = self.edit.render(&self.buffer, false);

        editor_panel.position = self.rect().top_left();
        editor_panel.size = self.rect().size();

        for span in &mut editor_panel.spans {
            span.from.column += 1;
            span.to.column += 1;
        }

        for line in &mut editor_panel.content {
            line.insert(0, '›');
        }

        // Prompt color
        editor_panel.spans.push(Span {
            from: Position::ZERO,
            to: Position::ZERO,
            style: Style {
                foreground_color: None,
                background_color: Some(crate::theme::colors::ACCENT_BRIGHT),
                ..Default::default()
            },
            priority: 1,
        });

        // Bg color
        editor_panel.spans.push(Span {
            from: Position::ZERO,
            to: Position::ZERO.with_moved_indices(self.rect().width as _, 0),
            style: Style {
                foreground_color: None,
                background_color: Some(crate::theme::colors::ACCENT),
                ..Default::default()
            },
            priority: 0,
        });

        editor_panel
    }

    fn reset(&mut self) {
        self.buffer = TextBuffer::new_empty();
        self.edit = TextEdit::new();
        self.buffer_string = Default::default();
    }
}
