use crate::{
    command::EditorCommand,
    text_buffer::{
        char_count,
        commands::{
            move_cursor_down_impl, move_cursor_left_impl, move_cursor_right_impl,
            move_cursor_up_impl,
        },
        SelectionsId, TextBuffer,
    },
    ui_state::{Color, Span, Style, UiPanel},
    utils::{Position, Rect},
};

pub struct TextEdit {
    rect: Rect,
    view_top_left_position: Position,
    selections_id: SelectionsId,
}

impl TextEdit {
    pub fn new() -> Self {
        Self {
            rect: Rect::new(0, 0, 25, 25),
            view_top_left_position: Position::ZERO,
            selections_id: SelectionsId::default(),
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn view_top_left_position(&self) -> Position {
        self.view_top_left_position
    }

    pub fn selections_id(&self) -> SelectionsId {
        self.selections_id
    }

    pub fn execute_command(&mut self, command: EditorCommand, buffer: &mut TextBuffer) {
        use EditorCommand::*;
        match command {
            Noop => (),
            Insert(chr) => {
                buffer.insert_char(self.selections_id, chr).unwrap();
                // if not anchored:
                // self.shrink_selections(buffer);
            }
            MoveCursorUp => move_cursor_up_impl(buffer, self.selections_id),
            MoveCursorDown => move_cursor_down_impl(buffer, self.selections_id),
            MoveCursorLeft => move_cursor_left_impl(buffer, self.selections_id),
            MoveCursorRight => move_cursor_right_impl(buffer, self.selections_id),
            _ => (),
        }
    }

    pub fn render(&mut self, buffer: &TextBuffer) -> UiPanel {
        let size = self.rect.size();
        let mut spans = Vec::new();
        let mut content = buffer
            .lines()
            .skip(self.view_top_left_position.row as usize)
            .take(size.row as usize)
            .map(|line| {
                let line_chars_in_view = line
                    .chars()
                    .skip(self.view_top_left_position.column as usize)
                    .take(size.column as usize);
                let padding_len = size.column.saturating_sub(char_count(&line));
                let padding_chars = std::iter::once(' ').cycle().take(padding_len as usize);
                let mut buf = String::new();
                buf.extend(line_chars_in_view);
                buf.extend(padding_chars);
                buf
            })
            .collect::<Vec<_>>();

        let content_lines_count = content.len() as u32;
        let non_content_lines_count =
            (self.view_top_left_position.row + size.row).saturating_sub(content_lines_count);
        for i in 0..non_content_lines_count {
            let row = content_lines_count + i;
            content.push("~".into());
            spans.push(Span {
                from: Position::new(0, row),
                to: Position::new(0, row),
                style: Style {
                    foreground_color: Some(Color::rgb(155, 100, 200)),
                    ..Default::default()
                },
                ..Default::default()
            })
        }

        spans.extend(self.compute_selections_spans(buffer));

        UiPanel {
            position: self.rect.top_left(),
            size,
            content,
            spans,
        }
    }

    pub fn compute_selections_spans(&self, buffer: &TextBuffer) -> Vec<Span> {
        let mut spans = Vec::new();

        let use_alt_cursor_style = false;

        for (i, selection) in buffer.get_selections(self.selections_id).iter().enumerate() {
            let is_primary = i == 0;
            let cursor_color = if use_alt_cursor_style {
                if is_primary {
                    Some(Color::RED)
                } else {
                    Some(Color::rgb(170, 10, 10))
                }
            } else {
                if is_primary {
                    Some(Color::WHITE)
                } else {
                    Some(Color::rgb(180, 180, 180))
                }
            };
            let selection_color = if use_alt_cursor_style {
                if is_primary {
                    Some(Color::rgb(100, 32, 96))
                } else {
                    Some(Color::rgb(80, 26, 76))
                }
            } else {
                if is_primary {
                    Some(Color::rgb(18, 72, 150))
                } else {
                    Some(Color::rgb(12, 52, 100))
                }
            };

            // Cursor span
            spans.push(Span {
                from: selection.cursor(),
                to: selection.cursor(),
                style: Style {
                    foreground_color: cursor_color,
                    invert: true,
                    ..Default::default()
                },
                priority: 255,
            });

            for line_selection in selection.split_lines() {
                let line_selection = buffer.limit_selection_to_content(&line_selection);
                let (from, to) = line_selection.start_end();

                // Selection span on line
                spans.push(Span {
                    from,
                    to,
                    style: Style {
                        foreground_color: Some(Color::rgb(200, 200, 200)),
                        background_color: selection_color,
                        ..Default::default()
                    },
                    priority: 254,
                });
            }
        }
        spans
    }
}
