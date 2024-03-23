use crate::slotmap::SlotMap;

#[derive(Default)]
pub struct State {
    pub views: SlotMap<View>,
    pub buffers: SlotMap<Buffer>,
    pub selection_sets: SlotMap<SelectionSet>,
    pub selections: SlotMap<Selection>,
}

impl State {}

pub struct View {}
pub struct Buffer(pub String);
pub struct SelectionSet {}
pub struct Selection {}
