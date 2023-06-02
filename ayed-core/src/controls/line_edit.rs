use crate::{
    buffer::TextBuffer,
    command::Command,
    input::Input,
    input_mapper::InputMap,
    selection::Position,
    state::State,
    text_mode::TextEditMode,
    ui_state::{Color, Span, Style, UiPanel},
    utils::Rect,
};

use super::TextBufferEdit;

pub struct LineEdit {
    editor: TextBufferEdit,
    buffer: TextBuffer,
}

impl LineEdit {
    pub fn new() -> Self {
        let buffer = TextBuffer::new_empty();

        let mut editor = TextBufferEdit::new();
        editor.set_rect(Rect::new(0, 0, 25, 1));

        Self { editor, buffer }
    }

    pub fn rect(&mut self) -> Rect {
        self.editor.rect()
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.editor.set_rect(rect);
    }

    pub fn input(&mut self, input: Input, state: &mut State) -> Option<String> {
        let commands = TextEditMode.convert_input_to_command(input, state);
        for command in commands {
            match command {
                Command::Insert('\n') => {
                    let mut line = String::new();
                    self.buffer.copy_line(0, &mut line).ok()?;
                    self.reset();
                    return Some(line);
                }
                _ => {
                    self.editor
                        .execute_command(command, &mut self.buffer, state);
                }
            }
        }

        None
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let mut editor_panel = self.editor.render(&self.buffer, state);

        editor_panel.position = self.rect().top_left();
        editor_panel.size = self.rect().size();

        for span in &mut editor_panel.spans {
            span.from.column_index += 1;
            span.to.column_index += 1;
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
                background_color: Some(Color::rgb(96, 32, 200)),
                invert: false,
            },
            importance: 1,
        });

        // Bg color
        editor_panel.spans.push(Span {
            from: Position::ZERO,
            to: Position::ZERO.with_moved_indices(0, self.rect().width as _),
            style: Style {
                foreground_color: None,
                background_color: Some(Color::rgb(40, 30, 50)),
                invert: false,
            },
            importance: 0,
        });

        editor_panel
    }

    fn reset(&mut self) {
        self.editor = TextBufferEdit::new();
        self.buffer = TextBuffer::new_empty();
    }
}
