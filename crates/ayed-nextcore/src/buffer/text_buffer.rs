use std::collections::HashMap;

use crate::{buffer::BufferId, impl_IdMapKey, position::Position};

// Invariants that must be respected for the lines:
// #1. There must always be at least one line. A line is a String the the lines vector.
// #2. All lines but the last end with the line separator, the '\n' character.
// Positions refer to lines in row and to codepoints (Rust chars) of said line in column.
pub struct TextBuffer {
    pub(super) id: BufferId,
    lines: Vec<String>,
    markers: HashMap<MarkerId, Position>,
    selections: HashMap<ViewId, SelectionSet>,
    bleh: (),
    // HOWTO selections?
    // Buffer owns the SelectionSets, keyed by ViewId.
    // Some getter could return the Ranges given the key.
}
