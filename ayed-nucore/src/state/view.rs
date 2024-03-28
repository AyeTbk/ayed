use crate::{position::Position, selection::Selections, slotmap::Handle, Ref};

use super::text_buffer::TextBuffer;

pub struct View {
    pub top_left: Position,
    pub buffer: Handle<TextBuffer>,
    pub selections: Ref<Selections>,
}
