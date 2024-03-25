use crate::{selection::Selections, slotmap::Handle, Ref};

use super::text_buffer::TextBuffer;

pub struct View {
    pub buffer: Handle<TextBuffer>,
    pub selections: Ref<Selections>,
}
