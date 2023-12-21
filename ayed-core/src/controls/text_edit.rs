use crate::{
    command::EditorCommand,
    selection::Selection,
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
    anchor_next_tracker: AnchorNextTracker,
}

impl TextEdit {
    pub fn new() -> Self {
        Self {
            rect: Rect::new(0, 0, 25, 25),
            view_top_left_position: Position::ZERO,
            selections_id: SelectionsId::default(),
            anchor_next_tracker: AnchorNextTracker::new(),
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

    pub fn view_rect(&self) -> Rect {
        Rect::with_position_and_size(self.view_top_left_position, self.rect.size())
    }

    pub fn selections_id(&self) -> SelectionsId {
        self.selections_id
    }

    pub fn execute_command(&mut self, command: EditorCommand, buffer: &mut TextBuffer) {
        let is_anchored = self.anchor_next_tracker.is_anchored();

        use EditorCommand::*;
        match command {
            Noop => (),
            //
            Insert(chr) => {
                buffer.insert_char(self.selections_id, chr);
                // if not anchored:
                // self.shrink_selections(buffer);
            }
            DeleteSelection => {
                buffer.delete(self.selections_id);
            }
            DeleteCursor => {
                for selection in buffer.get_selections(self.selections_id).clone().iter() {
                    buffer.delete_selection(Selection::with_position(selection.cursor()));
                }
            }
            DeleteBeforeCursor => {
                for selection in buffer.get_selections(self.selections_id).clone().iter() {
                    let before_cursor = buffer.move_position_left(selection.cursor());
                    if before_cursor != selection.cursor() {
                        buffer.delete_selection(Selection::with_position(before_cursor));
                    }
                }
            }
            //
            AnchorNext => self.anchor_next_tracker.start(),
            MoveCursorUp => move_cursor_up_impl(buffer, self.selections_id, is_anchored),
            MoveCursorDown => move_cursor_down_impl(buffer, self.selections_id, is_anchored),
            MoveCursorLeft => move_cursor_left_impl(buffer, self.selections_id, is_anchored),
            MoveCursorRight => move_cursor_right_impl(buffer, self.selections_id, is_anchored),
            _ => (),
        }

        self.anchor_next_tracker.tick();
        self.adjust_viewport_to_primary_selection(buffer);
    }

    pub fn render(&mut self, buffer: &TextBuffer, use_alt_selection_style: bool) -> UiPanel {
        self.adjust_viewport_to_primary_selection(buffer); // this is here to keep the cursor in view when resizing the window

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
                let line_chars_count_in_view =
                    char_count(&line).saturating_sub(self.view_top_left_position.column);
                let padding_len = size.column.saturating_sub(line_chars_count_in_view);
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
            let mut nil_line = String::from("~");
            nil_line.extend(
                std::iter::once(' ')
                    .cycle()
                    .take(size.column.saturating_sub(1) as usize),
            );
            content.push(nil_line);
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

        spans.extend(self.compute_selections_spans(buffer, use_alt_selection_style));

        UiPanel {
            position: self.rect.top_left(),
            size,
            content,
            spans,
        }
    }

    pub fn compute_selections_spans(&self, buffer: &TextBuffer, alt_style: bool) -> Vec<Span> {
        let mut spans = Vec::new();

        let view_offset = -self.view_top_left_position.to_offset();

        for (i, selection) in buffer.get_selections(self.selections_id).iter().enumerate() {
            let is_primary = i == 0;
            let cursor_color = if alt_style {
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
            let selection_color = if alt_style {
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
            let cursor = selection.cursor();
            if self.view_rect().contains_position(cursor) {
                spans.push(Span {
                    from: cursor.offset(view_offset),
                    to: cursor.offset(view_offset),
                    style: Style {
                        foreground_color: cursor_color,
                        invert: true,
                        ..Default::default()
                    },
                    priority: 255,
                });
            }

            for line_selection in selection.split_lines() {
                let line_selection = buffer.limit_selection_to_content(&line_selection);
                let (from, to) = line_selection.start_end();

                let Some(line_sel_rect) =
                    Rect::from_positions(from, to).intersection(self.view_rect())
                else {
                    continue;
                };

                let (from, to) = (
                    line_sel_rect.top_left().offset(view_offset),
                    line_sel_rect.bottom_right().offset(view_offset),
                );

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

    fn adjust_viewport_to_primary_selection(&mut self, buffer: &TextBuffer) {
        let mut new_viewport_top_left_position = self.view_top_left_position;
        let primary_cursor = buffer.get_selections(self.selections_id).primary().cursor();

        // Horizontal
        let vp_start_x = self.view_top_left_position.column;
        let vp_after_end_x = vp_start_x as u64 + self.rect.width as u64;
        let selection_x = primary_cursor.column;

        if selection_x < vp_start_x {
            new_viewport_top_left_position.column = selection_x;
        } else if selection_x as u64 >= vp_after_end_x {
            new_viewport_top_left_position.column = selection_x - self.rect.width + 1;
        }

        // Vertical
        let vp_start_y = self.view_top_left_position.row;
        let vp_after_end_y = vp_start_y as u64 + self.rect.height as u64;
        let selection_y = primary_cursor.row;

        if selection_y < vp_start_y {
            new_viewport_top_left_position.row = selection_y;
        } else if selection_y as u64 >= vp_after_end_y {
            new_viewport_top_left_position.row = selection_y - self.rect.height + 1;
        }

        self.view_top_left_position = new_viewport_top_left_position;
    }
}

struct AnchorNextTracker {
    counter: u8,
}

impl AnchorNextTracker {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn is_anchored(&self) -> bool {
        self.counter != 0
    }

    pub fn start(&mut self) {
        self.counter = 2;
    }

    pub fn tick(&mut self) {
        self.counter = self.counter.saturating_sub(1);
    }
}
