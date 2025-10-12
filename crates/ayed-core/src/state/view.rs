use crate::{
    position::{Column, Position, Row},
    slotmap::{Handle, SlotMap},
    utils::string_utils::{char_count, char_index_to_byte_index, char_index_to_byte_index_end},
};

use super::text_buffer::TextBuffer;

pub struct View {
    pub top_left: Position,
    pub buffer: Handle<TextBuffer>,
    pub virtual_buffer: Option<VirtualBuffer>,
}

impl View {
    pub fn render_view_line(
        &self,
        idx: Row,
        render: &mut String,
        buffers: &SlotMap<TextBuffer>,
    ) -> Option<()> {
        let idx = self.top_left.row + idx;
        if self.virtual_buffer.is_some() {
            self.render_virtual_line(idx, render, buffers)
        } else {
            self.render_true_line(idx, render, buffers)
        }
    }

    // TODO Maybe these different "position spaces" could have separate types/type aliases to make it clearer what
    // space a position is expressed in.
    pub fn map_true_position_to_view_position(&self, position: Position) -> Option<Position> {
        self.map_true_position_to_virtual_position(position)
            .map(|p| self.map_virtual_position_to_view_position(p))
    }

    pub fn map_view_position_to_true_position(&self, position: Position) -> Option<Position> {
        let vpos = self.map_view_position_to_virtual_position(position);
        self.map_virtual_position_to_true_position(vpos)
    }

    pub fn map_virtual_position_to_view_position(&self, position: Position) -> Position {
        position.local_to_pos(self.top_left)
    }

    pub fn map_view_position_to_virtual_position(&self, position: Position) -> Position {
        position.offset(self.top_left.to_offset())
    }

    pub fn map_true_position_to_virtual_position(&self, position: Position) -> Option<Position> {
        // FIXME PERF This is quite slow, can position ranges be cached somehow, instead of having to go through this
        // whole vbuffer every time?
        if let Some(vbuffer) = self.virtual_buffer.as_ref() {
            // FIXME The position could actually appear mutliple times in the
            // virtual buffer, if a same excerpt is used multiple times. This
            // function should return multiple positions.
            for vline_idx in 0..(vbuffer.lines.len() as Row) {
                let vline = vbuffer.lines.get(vline_idx as usize).unwrap();
                if vline.fragments.is_empty() {
                    continue;
                }
                let mut column = 0;
                for vfrag in &vline.fragments {
                    if let Some(column_offset) = vfrag.position_column_offset_within(position) {
                        return Some(Position::new(column + column_offset, vline_idx));
                    }
                    column += vfrag.char_count();
                }
            }
            None
        } else {
            Some(position)
        }
    }

    pub fn map_virtual_position_to_true_position(&self, position: Position) -> Option<Position> {
        if let Some(vbuffer) = self.virtual_buffer.as_ref() {
            let vline = vbuffer.lines.get(position.row as usize).unwrap();

            let mut column = 0;
            for vfrag in &vline.fragments {
                let after_vfrag_column = column + vfrag.char_count();

                match vfrag {
                    VirtualFragment::TrueLineExcerpt { row, from, .. } => {
                        if column <= position.column && position.column <= after_vfrag_column {
                            let true_row = *row;
                            let true_column = from + position.column - column;
                            return Some(Position::new(true_column, true_row));
                        }
                    }
                    _ => {}
                }

                column = after_vfrag_column;
            }
            None
        } else {
            Some(position)
        }
    }

    pub fn map_view_line_idx_to_line_number(&self, idx: Row) -> Option<Row> {
        let view_line = idx + self.top_left.row;

        let Some(vbuffer) = self.virtual_buffer.as_ref() else {
            return Some(view_line + 1);
        };

        let vline = vbuffer.lines.get(view_line as usize)?;
        for vfragment in &vline.fragments {
            match vfragment {
                VirtualFragment::TrueLineExcerpt { row, .. } => return Some(*row + 1),
                _ => (),
            }
        }
        None
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

    fn render_virtual_line(
        &self,
        idx: Row,
        render: &mut String,
        buffers: &SlotMap<TextBuffer>,
    ) -> Option<()> {
        render.clear();

        let vubffer = self.virtual_buffer.as_ref().unwrap();
        let Some(vline) = vubffer.lines.get(idx as usize) else {
            return None;
        };

        for vfrag in &vline.fragments {
            match vfrag {
                VirtualFragment::TrueLineExcerpt {
                    row,
                    from,
                    to,
                    ends_line,
                } => {
                    let buffer = buffers.get(self.buffer);
                    let Some(line) = buffer.line(*row) else {
                        render.push_str(&format!("$<bad vfrag: {row}>"));
                        continue;
                    };

                    if line.is_empty() && *ends_line {
                        render.push_str(" ");
                    } else {
                        let Some(excerpt) = buffer
                            .line(*row)
                            .and_then(|line| char_index_range_to_slice(line, *from, *to))
                        else {
                            render.push_str(&format!("$<bad vfrag: {row}:{from}-{to}>"));
                            continue;
                        };
                        render.push_str(excerpt)
                    }
                }
                VirtualFragment::Text(text) => {
                    render.push_str(&text);
                }
            }
        }
        Some(())
    }

    pub fn rebuild_line_wrap(&mut self, buffers: &SlotMap<TextBuffer>, wrap_column: Column) {
        if wrap_column == 0 {
            // Nonsense request
            return;
        }

        let buffer = buffers.get(self.buffer);
        let mut vbuffer = VirtualBuffer::default();

        for row in 0..buffer.line_count() {
            let mut line = buffer.line(row).unwrap();

            // Break line as long as you can.
            let mut i = 0;
            while let Some(idx) = char_index_to_byte_index(line, wrap_column.try_into().unwrap()) {
                let vline = VirtualLine {
                    fragments: vec![VirtualFragment::TrueLineExcerpt {
                        row,
                        from: wrap_column * i,
                        to: (wrap_column * (i + 1)).saturating_sub(1),
                        ends_line: false,
                    }],
                };
                vbuffer.lines.push(vline);

                line = &line[idx..];
                i += 1;
            }

            // Fill in remainder (or full line if it wasn't broken).
            if !line.is_empty() {
                let from = wrap_column * i;
                let line_char_count: Column = char_count(line).try_into().unwrap();
                let line_end = from + line_char_count;
                let to = line_end - 1;
                vbuffer.lines.push(VirtualLine {
                    fragments: vec![VirtualFragment::TrueLineExcerpt {
                        row,
                        from,
                        to,
                        ends_line: true,
                    }],
                });
            } else if i == 0 {
                // Handle empty lines.
                let from = wrap_column * i;
                let line_char_count: Column = char_count(line).try_into().unwrap();
                let line_end = from + line_char_count;
                let to = line_end - 1;
                vbuffer.lines.push(VirtualLine {
                    fragments: vec![VirtualFragment::TrueLineExcerpt {
                        row,
                        from,
                        to,
                        ends_line: true,
                    }],
                });
            }
        }

        self.virtual_buffer = Some(vbuffer);
    }
}

#[derive(Debug, Default)]
pub struct VirtualBuffer {
    pub lines: Vec<VirtualLine>,
}

#[derive(Debug, Default)]
pub struct VirtualLine {
    pub fragments: Vec<VirtualFragment>,
}

#[derive(Debug)]
pub enum VirtualFragment {
    TrueLineExcerpt {
        row: Row,
        from: Column,
        to: Column,
        ends_line: bool,
    },
    Text(String),
}

impl VirtualFragment {
    pub fn char_count(&self) -> Column {
        match self {
            Self::TrueLineExcerpt { from, to, .. } => to - *from,
            Self::Text(text) => char_count(text).try_into().unwrap(),
        }
    }

    pub fn contains_position(&self, position: Position) -> bool {
        match self {
            Self::TrueLineExcerpt {
                row,
                from,
                to,
                ends_line,
            } => {
                if position.row != *row {
                    return false;
                }
                if *ends_line {
                    position.column >= *from && position.column <= to.saturating_add(1)
                } else {
                    position.column >= *from && position.column <= *to
                }
            }
            Self::Text(_) => false,
        }
    }

    pub fn position_column_offset_within(&self, position: Position) -> Option<Column> {
        match self {
            Self::TrueLineExcerpt { from, .. } => {
                if !self.contains_position(position) {
                    return None;
                }
                Some(position.column - from)
            }
            Self::Text(_) => None,
        }
    }
}

fn char_index_range_to_slice(s: &str, from: Column, to: Column) -> Option<&str> {
    let start = char_index_to_byte_index(s, from.try_into().unwrap())?;
    let end = char_index_to_byte_index_end(s, to.try_into().unwrap())?;
    s.get(start as usize..end as usize)
}
