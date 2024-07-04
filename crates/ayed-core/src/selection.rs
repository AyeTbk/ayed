use crate::utils::Position;

#[derive(Debug, Clone)]
pub struct Selections {
    primary_selection: Selection,
    extra_selections: Vec<Selection>,
}

impl Selections {
    pub fn new() -> Self {
        Self {
            primary_selection: Selection::new(),
            extra_selections: Vec::new(),
        }
    }

    pub fn new_with(primary: Selection, extra: &[Selection]) -> Self {
        Self {
            primary_selection: primary,
            extra_selections: extra.to_owned(),
        }
    }

    pub fn primary(&self) -> Selection {
        self.primary_selection
    }

    pub fn change_primary(&mut self, idx_of_new_primary: usize) {
        if idx_of_new_primary == 0 {
            return;
        }
        std::mem::swap(
            &mut self.primary_selection,
            &mut self.extra_selections[idx_of_new_primary - 1],
        );
    }

    pub fn clear_extras(&mut self) {
        self.extra_selections.clear();
    }

    pub fn overlapping_selections_merged(&self) -> Self {
        let mut selections = self.extra_selections.clone();
        selections.insert(0, self.primary_selection);

        let mut i = 0;
        while i < selections.len() {
            let mut selection = selections.remove(i);
            let mut j = i;
            while j < selections.len() {
                let other = selections.remove(j);
                if let Some(merged) = selection.merged_with(&other) {
                    selection = merged;
                } else {
                    selections.insert(j, other);
                    j += 1;
                }
            }
            selections.insert(i, selection);
            i += 1;
        }

        let merged_primary_selection = selections.remove(0);
        Self {
            primary_selection: merged_primary_selection,
            extra_selections: selections,
        }
    }

    pub fn count(&self) -> usize {
        1 + self.extra_selections.len()
    }

    pub fn get(&self, index: usize) -> Option<Selection> {
        if index == 0 {
            Some(self.primary_selection)
        } else {
            self.extra_selections.get(index - 1).copied()
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Selection> {
        if index == 0 {
            Some(&mut self.primary_selection)
        } else {
            self.extra_selections.get_mut(index - 1)
        }
    }

    pub fn add(&mut self, selection: Selection) -> usize {
        // TODO maybe add the selection at a sensible location instead of just added at the end
        let len = self.extra_selections.len();
        self.extra_selections.push(selection);
        len + 1
    }

    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        Some(&self.primary_selection)
            .into_iter()
            .chain(self.extra_selections.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Selection> {
        Some(&mut self.primary_selection)
            .into_iter()
            .chain(self.extra_selections.iter_mut())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    cursor: Position,
    anchor: Position,
    desired_cursor_column_index: u32,
    desired_anchor_column_index: u32,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            cursor: Position::ZERO,
            anchor: Position::ZERO,
            desired_cursor_column_index: 0,
            desired_anchor_column_index: 0,
        }
    }

    pub fn with_position(position: Position) -> Self {
        Self {
            cursor: position,
            anchor: position,
            desired_cursor_column_index: position.column,
            desired_anchor_column_index: position.column,
        }
    }

    pub fn with_cursor(&self, cursor: Position) -> Self {
        Self {
            cursor,
            desired_cursor_column_index: cursor.column,
            ..*self
        }
    }

    pub fn with_provisional_cursor(&self, cursor: Position) -> Self {
        Self { cursor, ..*self }
    }

    pub fn with_desired_cursor_column_index(&self, desired_cursor_column_index: u32) -> Self {
        Self {
            desired_cursor_column_index,
            ..*self
        }
    }

    pub fn with_anchor(&self, anchor: Position) -> Self {
        Self {
            anchor,
            desired_anchor_column_index: anchor.column,
            ..*self
        }
    }

    pub fn with_provisional_anchor(&self, anchor: Position) -> Self {
        Self { anchor, ..*self }
    }

    pub fn with_start(&self, start: Position) -> Self {
        if self.is_forward() {
            self.with_anchor(start)
        } else {
            self.with_cursor(start)
        }
    }

    pub fn with_end(&self, end: Position) -> Self {
        if self.is_forward() {
            self.with_cursor(end)
        } else {
            self.with_anchor(end)
        }
    }

    pub fn with_start_and_end(&self, start: Position, end: Position) -> Self {
        if self.is_forward() {
            self.with_anchor(start).with_cursor(end)
        } else {
            self.with_cursor(start).with_anchor(end)
        }
    }

    pub fn shrunk_to_cursor(&self) -> Self {
        let mut this = *self;
        this.anchor = this.cursor;
        this.desired_anchor_column_index = this.desired_cursor_column_index;
        this
    }

    pub fn shrunk_to_start(&self) -> Self {
        let mut this = *self;
        this.cursor = self.start();
        this.anchor = this.cursor;
        this
    }

    pub fn flipped(&self) -> Self {
        Self {
            cursor: self.anchor,
            anchor: self.cursor,
            desired_cursor_column_index: self.desired_anchor_column_index,
            desired_anchor_column_index: self.desired_cursor_column_index,
        }
    }

    pub fn flipped_forward(&self) -> Self {
        if !self.is_forward() {
            self.flipped()
        } else {
            *self
        }
    }

    pub fn cursor(&self) -> Position {
        self.cursor
    }

    pub fn desired_cursor(&self) -> Position {
        self.cursor.with_column(self.desired_cursor_column_index)
    }

    pub fn anchor(&self) -> Position {
        self.anchor
    }

    pub fn desired_anchor(&self) -> Position {
        self.anchor.with_column(self.desired_anchor_column_index)
    }

    pub fn to_desired(&self) -> Self {
        Self {
            cursor: self.cursor,
            anchor: self.anchor,
            desired_cursor_column_index: self.cursor.column,
            desired_anchor_column_index: self.anchor.column,
        }
    }

    pub fn start(&self) -> Position {
        self.start_end().0
    }

    pub fn end(&self) -> Position {
        self.start_end().1
    }

    pub fn start_end(&self) -> (Position, Position) {
        if self.cursor < self.anchor {
            (self.cursor, self.anchor)
        } else {
            (self.anchor, self.cursor)
        }
    }

    pub fn is_forward(&self) -> bool {
        self.anchor <= self.cursor
    }

    pub fn merged_with(&self, other: &Self) -> Option<Self> {
        if self.overlaps_with(other) {
            let start = self.start().min(other.start());
            let end = self.end().max(other.end());
            let (cursor, anchor) = if self.cursor_is_at_start() {
                (start, end)
            } else {
                (end, start)
            };
            Some(Self {
                cursor,
                anchor,
                desired_cursor_column_index: self.desired_cursor_column_index,
                desired_anchor_column_index: self.desired_anchor_column_index,
            })
        } else {
            None
        }
    }

    pub fn overlaps_with(&self, other: &Self) -> bool {
        (self.start() <= other.start() && self.end() >= other.start())
            || (other.start() <= self.start() && other.end() >= self.start())
    }

    pub fn contains(&self, position: Position) -> bool {
        self.start() <= position && position <= self.end()
    }

    pub fn split_lines(&self) -> impl Iterator<Item = Selection> {
        let (self_start, self_end) = self.start_end();
        let line_count = self_end.row.saturating_sub(self_start.row) + 1;
        let mut i = 0;
        std::iter::from_fn(move || {
            if i >= line_count {
                return None;
            }

            let start_column = if i == 0 { self_start.column } else { 0 };
            let end_column = if i == line_count - 1 {
                self_end.column
            } else {
                u32::MAX
            };
            let line = self_start.row + i;
            i += 1;
            Some(Selection::new().with_start_and_end(
                Position::new(start_column, line),
                Position::new(end_column, line),
            ))
        })
    }

    pub fn line_span(&self) -> u32 {
        let (start, end) = self.start_end();
        end.row.saturating_sub(start.row).saturating_add(1)
    }

    fn cursor_is_at_start(&self) -> bool {
        self.cursor <= self.anchor
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selections__overlapping_selections_merged__no_overlap() {
        let pos0 = Position::new(0, 0);
        let pos1 = Position::new(2, 0);
        let pos2 = Position::new(0, 1);
        let pos3 = Position::new(79, 3);

        let selections = Selections::new_with(
            Selection::new().with_anchor(pos0).with_cursor(pos1),
            &[Selection::new().with_anchor(pos2).with_cursor(pos3)],
        );

        let merged = selections.overlapping_selections_merged();

        assert_eq!(
            merged.primary_selection.anchor,
            selections.primary_selection.anchor
        );
        assert_eq!(
            merged.primary_selection.cursor,
            selections.primary_selection.cursor
        );
        assert_eq!(merged.extra_selections.len(), 1);
        assert_eq!(
            merged.extra_selections[0].anchor,
            selections.extra_selections[0].anchor
        );
        assert_eq!(
            merged.extra_selections[0].cursor,
            selections.extra_selections[0].cursor
        );
    }

    #[test]
    fn selections__overlapping_selections_merged__multiple_merged_in_one() {
        let pos0 = Position::new(0, 0);
        let pos1 = Position::new(2, 0);
        let pos2 = Position::new(15, 0);
        let pos3 = Position::new(0, 1);
        let pos4 = Position::new(79, 3);

        let selections = Selections::new_with(
            Selection::new().with_anchor(pos0).with_cursor(pos2),
            &[
                Selection::new().with_anchor(pos1).with_cursor(pos3),
                Selection::new().with_anchor(pos3).with_cursor(pos4),
            ],
        );

        let merged = selections.overlapping_selections_merged();

        assert_eq!(merged.primary_selection.anchor, pos0);
        assert_eq!(merged.primary_selection.cursor, pos4);
        assert!(merged.extra_selections.is_empty());
    }
}
