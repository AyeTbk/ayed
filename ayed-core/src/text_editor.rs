use crate::{
    arena::Handle,
    buffer::Buffer,
    command::Command,
    core::{EditorContext, EditorContextMut},
    selection::{Position, Selection, SelectionBounds, Selections},
    ui_state::{Panel, Span, Style},
};

pub struct TextEditor {
    buffer: Handle<Buffer>,
    selections: Selections,
    viewport_top_left_position: Position,
}

impl TextEditor {
    pub fn new(buffer: Handle<Buffer>) -> Self {
        Self {
            buffer,
            selections: Selections::new(),
            viewport_top_left_position: Position::ZERO,
        }
    }

    pub fn viewport_content_panel(&self, ctx: &EditorContext) -> Panel {
        // Compute content
        let start_line_index = self.viewport_top_left_position.line_index;
        let after_end_line_index = start_line_index + ctx.viewport_size.1;
        let start_column_index = self.viewport_top_left_position.column_index;
        let line_slice_max_len = ctx.viewport_size.0;

        let mut panel_content = Vec::new();
        let content = ctx.buffers.get(self.buffer);

        for line_index in start_line_index..after_end_line_index {
            let full_line = if let Some(line) = content.line(line_index) {
                line
            } else {
                break;
            };

            let (start_column, end_column) = if start_column_index as usize >= full_line.len() {
                (0, 0)
            } else {
                let expected_end = start_column_index as usize + line_slice_max_len as usize;
                let end = expected_end.min(full_line.len());
                (start_column_index as usize, end)
            };
            dbg!((start_column, end_column));
            dbg!(full_line.len());

            let mut line = full_line[start_column..end_column].to_string();
            dbg!((line_slice_max_len, end_column));
            let line_visible_part_length = end_column - start_column;
            let padlen = line_slice_max_len as usize - line_visible_part_length;
            line.extend(" ".repeat(padlen).chars());

            panel_content.push(line);
        }

        panel_content.push(String::from("  "));

        // Compute spans
        let mut panel_spans = Vec::new();
        for selection in self.selections() {
            let from_relative_to_viewport = selection.from - self.viewport_top_left_position;
            let to_relative_to_viewport = selection.to - self.viewport_top_left_position;
            panel_spans.push(Span {
                from: from_relative_to_viewport,
                to: to_relative_to_viewport,
                style: Style {
                    foreground_color: None,
                    background_color: None,
                    invert: true,
                },
                importance: !0,
            });
        }

        // Wooowie done
        Panel {
            position: (0, 0),
            size: ctx.viewport_size,
            content: panel_content,
            spans: panel_spans,
        }
    }

    pub fn execute_command(&mut self, command: Command, ctx: &mut EditorContextMut) {
        let buffer = ctx.buffers.get_mut(self.buffer);

        match command {
            Command::Insert(c) => self.insert_char(c, buffer),
            Command::DeleteBeforeSelection => self.delete_before_selection(buffer),
            Command::DeleteSelection => self.delete_selection(buffer),
            Command::MoveSelectionUp => self.move_selection_vertically(-1, buffer),
            Command::MoveSelectionDown => self.move_selection_vertically(1, buffer),
            Command::MoveSelectionLeft => self.move_selection_horizontally(-1, buffer),
            Command::MoveSelectionRight => self.move_selection_horizontally(1, buffer),
        }

        self.adjust_viewport_to_primary_selection(ctx);
    }

    pub fn selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
        // FIXME this only shows selections as having a length of one
        self.selections.iter().map(|selection| SelectionBounds {
            from: selection.position.with_moved_indices(0, 0),
            to: selection.position.with_moved_indices(0, 1),
        })
    }

    fn insert_char(&mut self, ch: char, buffer: &mut Buffer) {
        for selection in self.selections.iter_mut() {
            if let Ok(new_position) = buffer.insert_char_at(ch, selection.position) {
                selection.position = new_position;
            }
        }
    }

    fn delete_before_selection(&mut self, buffer: &mut Buffer) {
        for selection in self.selections.iter_mut() {
            if selection.position == buffer.start_of_content_position() {
                // Can't delete before the beginning!
                continue;
            }
            let before_selection = buffer
                .moved_position_horizontally(selection.position, -1)
                .expect("wow?");
            buffer.delete_selection(Selection::new().with_position(before_selection));

            let new_selection = selection.with_position(before_selection);
            *selection = new_selection;
        }
    }

    fn delete_selection(&mut self, buffer: &mut Buffer) {
        for selection in self.selections.iter_mut() {
            buffer.delete_selection(*selection);
            *selection = selection.shrunk();
        }
    }

    fn move_selection_horizontally(&mut self, column_offset: i32, buffer: &Buffer) {
        for selection in self.selections.iter_mut() {
            let new_position = if let Some(moved_position) =
                buffer.moved_position_horizontally(selection.position, column_offset)
            {
                moved_position
            } else {
                if column_offset < 0 {
                    buffer.start_of_content_position()
                } else {
                    buffer.end_of_content_position()
                }
            };
            selection.position = new_position;
        }
    }

    fn move_selection_vertically(&mut self, line_offset: i32, buffer: &Buffer) {
        for selection in self.selections.iter_mut() {
            if let Some(moved_position) =
                buffer.moved_position_vertically(selection.position, line_offset)
            {
                selection.position = moved_position;
            }
        }
    }

    fn adjust_viewport_to_primary_selection(&mut self, ctx: &EditorContextMut) {
        let mut new_viewport_top_left_position = self.viewport_top_left_position;
        // Horizontal
        let vp_start_x = self.viewport_top_left_position.column_index;
        let vp_after_end_x = vp_start_x + ctx.viewport_size.0;
        let selection_x = self.selections.primary().position.column_index;

        if selection_x < vp_start_x {
            new_viewport_top_left_position.column_index = selection_x;
        } else if selection_x >= vp_after_end_x {
            new_viewport_top_left_position.column_index = selection_x - ctx.viewport_size.0 + 1;
        }

        // Vertical
        let vp_start_y = self.viewport_top_left_position.line_index;
        let vp_after_end_y = vp_start_y + ctx.viewport_size.1;
        let selection_y = self.selections.primary().position.line_index;

        if selection_y < vp_start_y {
            new_viewport_top_left_position.line_index = selection_y;
        } else if selection_y >= vp_after_end_y {
            new_viewport_top_left_position.line_index = selection_y - ctx.viewport_size.1 + 1;
        }

        self.viewport_top_left_position = new_viewport_top_left_position;
    }
}
