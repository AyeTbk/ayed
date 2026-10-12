use crate::{impl_IdMapKey, util::id_map::IdMap};

pub use selection::{Selection, SelectionSet};
pub use text_buffer::TextBuffer;
pub use text_edit::TextEdit;
pub use view::View;

mod selection;
mod text_buffer;
mod text_edit;
mod view;

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

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MarkerId {
    id: u32,
}
impl_IdMapKey!(for MarkerId);
