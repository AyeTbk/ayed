use crate::range::Range;

#[derive(Default)]
pub struct SelectionSet {
    sels: Vec<Selection>,
}

pub struct Selection {
    range: Range,
    old_logical_cursor_column: Option<i32>,
}
