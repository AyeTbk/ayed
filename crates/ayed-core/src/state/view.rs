use crate::{
    position::{Position, Row},
    slotmap::{Handle, SlotMap},
};

use super::text_buffer::TextBuffer;

pub struct View {
    pub top_left: Position,
    pub buffer: Handle<TextBuffer>,
}

impl View {
    pub fn render_view_line(
        &self,
        idx: Row,
        render: &mut String,
        buffers: &SlotMap<TextBuffer>,
    ) -> Option<()> {
        let idx = self.top_left.row + idx;
        self.render_true_line(idx, render, buffers)
    }

    // TODO Maybe these different "position spaces" could have separate types/type aliases to make it clearer what
    // space a position is expressed in.
    pub fn map_true_position_to_view_position(&self, position: Position) -> Position {
        position.local_to_pos(self.top_left)
    }

    pub fn map_view_position_to_true_position(&self, position: Position) -> Position {
        position.offset(self.top_left.to_offset())
    }

    pub fn map_view_line_idx_to_line_number(&self, idx: Row) -> Option<Row> {
        let view_line = idx + self.top_left.row;
        return Some(view_line + 1);
    }

    fn render_true_line(
        &self,
        row: Row,
        render: &mut String,
        buffers: &SlotMap<TextBuffer>,
    ) -> Option<()> {
        render.clear();

        let buffer = buffers.get(self.buffer);
        let Some(line) = buffer.line(row) else {
            return None;
        };
        render.push_str(line);
        Some(())
    }
}
