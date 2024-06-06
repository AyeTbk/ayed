use crate::{
    position::Position,
    selection::Selections,
    slotmap::{Handle, SlotMap},
    utils::string_utils::{char_count, char_index_to_byte_index, char_index_to_byte_index_end},
    Ref,
};

use super::text_buffer::TextBuffer;

pub struct View {
    pub top_left: Position,
    pub buffer: Handle<TextBuffer>,
    pub selections: Ref<Selections>,
    pub virtual_buffer: Option<VirtualBuffer>,
}

impl View {
    pub fn render_view_line(
        &self,
        idx: u32,
        render: &mut String,
        buffers: &SlotMap<TextBuffer>,
    ) -> Option<()> {
        let idx = self.top_left.row.saturating_add(idx);
        if self.virtual_buffer.is_some() {
            self.render_virtual_line(idx, render, buffers)
        } else {
            self.render_true_line(idx, render, buffers)
        }
    }

    pub fn map_true_position_to_view_position(&self, position: Position) -> Option<Position> {
        // TODO this doesnt account for the vbuffer
        let (Some(column), Some(row)) = position.local_to(self.top_left) else {
            return None;
        };
        Some(Position::new(column, row))
    }

    pub fn map_view_line_idx_to_line_number(&self, _idx: u32) -> Option<u32> {
        // For an eventual line number bar
        todo!()
    }

    fn render_true_line(
        &self,
        row: u32,
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
        idx: u32,
        render: &mut String,
        buffers: &SlotMap<TextBuffer>,
    ) -> Option<()> {
        render.clear();

        let vubffer = self.virtual_buffer.as_ref().unwrap();
        let Some(vline) = vubffer.lines.get(idx as usize) else {
            return None;
        };

        for vspan in &vline.spans {
            match vspan {
                VirtualSpan::TrueLineExcerpt { row, from, to } => {
                    let buffer = buffers.get(self.buffer);
                    let Some(excerpt) = buffer
                        .line(*row)
                        .and_then(|line| char_index_range_to_slice(line, *from, *to))
                    else {
                        render.push_str(&format!("$<bad vspan: {row}:{from}-{to}>"));
                        continue;
                    };
                    render.push_str(excerpt)
                }
                VirtualSpan::Text(text) => {
                    render.push_str(&text);
                }
            }
        }
        Some(())
    }

    pub fn rebuild_line_wrap(&mut self, buffers: &SlotMap<TextBuffer>, wrap_column: u32) {
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
            while let Some(idx) = char_index_to_byte_index(line, wrap_column) {
                let vline = VirtualLine {
                    spans: vec![VirtualSpan::TrueLineExcerpt {
                        row,
                        from: wrap_column * i,
                        to: (wrap_column * (i + 1)).saturating_sub(1),
                    }],
                };
                vbuffer.lines.push(vline);

                line = &line[idx..];
                i += 1;
            }

            // Fill in remainder (or full line if it wasn't broken).
            if !line.is_empty() {
                let from = wrap_column * i;
                let to = from + char_count(line).saturating_sub(1);
                vbuffer.lines.push(VirtualLine {
                    spans: vec![VirtualSpan::TrueLineExcerpt { row, from, to }],
                });
            } else if i == 0 {
                // Handle empty lines.
                vbuffer.lines.push(VirtualLine {
                    spans: vec![VirtualSpan::Text(String::new())],
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
    pub spans: Vec<VirtualSpan>,
}

#[derive(Debug)]
pub enum VirtualSpan {
    TrueLineExcerpt { row: u32, from: u32, to: u32 },
    Text(String),
}

fn char_index_range_to_slice(s: &str, from: u32, to: u32) -> Option<&str> {
    let start = char_index_to_byte_index(s, from)?;
    let end = char_index_to_byte_index_end(s, to)?;
    s.get(start as usize..end as usize)
}
