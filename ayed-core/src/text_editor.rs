use crate::{
    arena::Handle,
    buffer::Buffer,
    command::Command,
    core::EditorContext,
    selection::{Position, Selection, SelectionBounds, Selections},
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

    pub fn viewport_content_string(&self, output: &mut Vec<String>, ctx: &EditorContext) {
        let start_line_index = self.viewport_top_left_position.line_index;
        let end_line_index = start_line_index + ctx.viewport_size.1;
        let start_column_index = self.viewport_top_left_position.column_index;
        let line_slice_max_len = ctx.viewport_size.0;

        output.clear();

        let content = ctx.buffers.get(self.buffer);

        for line_index in start_line_index..=end_line_index {
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
            let sliced_line = &full_line[start_column..end_column];
            output.push(sliced_line.to_string());
        }
    }

    pub fn execute_command(&mut self, command: Command, ctx: &mut EditorContext) {
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

    fn selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
        // FIXME this only shows selections as hacing a length
        self.selections.iter().map(|selection| SelectionBounds {
            from: selection.position.with_moved_indices(0, 0),
            to: selection.position.with_moved_indices(0, 1),
        })
    }
}
