use std::collections::HashMap;

use crate::{
    buffer::{text_buffer::TextBuffer, view::View},
    impl_IdMapKey,
    util::id_map::IdMap,
};

pub mod text_buffer;
pub mod view;

pub type BufferStorage = IdMap<BufferId, TextBuffer>;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BufferId {
    id: u16,
}
impl_IdMapKey!(for BufferId);

pub type ViewStorage = IdMap<ViewId, View>;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ViewId {
    id: u32,
}
impl_IdMapKey!(for ViewId);
