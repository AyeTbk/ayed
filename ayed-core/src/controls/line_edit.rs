use crate::{
    buffer::TextBuffer,
    command::EditorCommand,
    ui_state::{Span, Style, UiPanel},
    utils::{Position, Rect},
};

use super::TextBufferEdit;

pub struct LineEdit {
    editor: TextBufferEdit,
    buffer: TextBuffer,
    buffer_string: String,
}

impl LineEdit {
    pub fn new() -> Self {
        let buffer = TextBuffer::new_empty();

        let mut editor = TextBufferEdit::new();
        editor.set_rect(Rect::new(0, 0, 25, 1));

        Self {
            editor,
            buffer,
            buffer_string: Default::default(),
        }
    }

    pub fn rect(&mut self) -> Rect {
        self.editor.rect()
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.editor.set_rect(rect);
    }

    pub fn text(&self) -> &str {
        &self.buffer_string
    }

    pub fn execute_command(&mut self, command: EditorCommand) -> Option<String> {
        match command {
            EditorCommand::Insert('\n') => {
                let mut line = String::new();
                self.buffer.copy_line(0, &mut line).ok()?;
                self.reset();
                return Some(line);
            }
            EditorCommand::DeleteBeforeCursor if self.buffer.is_empty() => {
                // NOTE This is mostly just for the modeline prompt UX.
                return Some("".into());
            }
            _ => {
                self.editor.execute_command(command, &mut self.buffer);

                self.buffer
                    .copy_line(0, &mut self.buffer_string)
                    .expect("buffer invariant 1 says there should always be at least one line");
            }
        }

        None
    }

    pub fn render(&mut self) -> UiPanel {
        let mut editor_panel = self.editor.render(&self.buffer);

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
        self.editor = TextBufferEdit::new();
        self.buffer = TextBuffer::new_empty();
        self.buffer_string = Default::default();
    }
}
