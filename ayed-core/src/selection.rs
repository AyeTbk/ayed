use std::ops::RangeInclusive;

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

    pub fn set(&mut self, index: usize, selection: Selection) {
        if index == 0 {
            self.primary_selection = selection;
        } else {
            self.extra_selections[index - 1] = selection;
        }
    }

    pub fn add(&mut self, selection: Selection) {
        // TODO maybe add the selection at a sensible location instead of just added at the end
        self.extra_selections.push(selection);
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
    desired_column_index: u32,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            cursor: Position::ZERO,
            anchor: Position::ZERO,
            desired_column_index: 0,
        }
    }

    pub fn with_position(&self, position: Position) -> Self {
        Self {
            cursor: position,
            anchor: position,
            desired_column_index: position.column_index,
        }
    }

    pub fn with_cursor(&self, cursor: Position) -> Self {
        Self {
            cursor,
            anchor: self.anchor,
            desired_column_index: cursor.column_index,
        }
    }

    pub fn with_provisional_cursor(&self, cursor: Position) -> Self {
        Self {
            cursor,
            anchor: self.anchor,
            desired_column_index: self.desired_column_index,
        }
    }

    pub fn with_anchor(&self, anchor: Position) -> Self {
        Self {
            cursor: self.cursor,
            anchor,
            desired_column_index: self.desired_column_index,
        }
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

    pub fn shrunk_to_cursor(&self) -> Self {
        let mut this = *self;
        this.anchor = this.cursor;
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
            desired_column_index: self.anchor.column_index,
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
        self.cursor.with_column_index(self.desired_column_index)
    }

    pub fn anchor(&self) -> Position {
        self.anchor
    }

    pub fn start(&self) -> Position {
        self.start_end().0
    }

    pub fn end(&self) -> Position {
        self.start_end().1
    }

    pub fn is_forward(&self) -> bool {
        self.anchor < self.cursor
    }

    pub fn line_span(&self) -> RangeInclusive<u32> {
        self.start().line_index..=self.end().line_index
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
                desired_column_index: self.desired_column_index,
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

    fn cursor_is_at_start(&self) -> bool {
        self.cursor <= self.anchor
    }

    fn start_end(&self) -> (Position, Position) {
        if self.cursor <= self.anchor {
            (self.cursor, self.anchor)
        } else {
            (self.anchor, self.cursor)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EditInfo {
    Deleted(DeletedEditInfo),
    AddedOne(Position),
    LineSplit(Position), // position points where the char (now first char of the new line) was.
}

impl From<DeletedEditInfo> for EditInfo {
    fn from(value: DeletedEditInfo) -> Self {
        Self::Deleted(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeletedEditInfo {
    pub pos1_line_index: u32,
    pub pos1_before_delete_start_column_index: i64, // Can be -1
    pub pos2: Position,                             // Position after deleted content end
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line_index: u32,
    pub column_index: u32,
}

impl Position {
    pub const ZERO: Self = Self {
        line_index: 0,
        column_index: 0,
    };

    pub fn new(line_index: u32, column_index: u32) -> Self {
        Self {
            line_index,
            column_index,
        }
    }

    pub fn offset(&self, offset: Offset) -> Self {
        self.with_moved_indices(offset.line_offset, offset.column_offset)
    }

    pub fn with_moved_indices(&self, line_offset: i32, column_offset: i32) -> Self {
        // FIXME? line_offset, column_offset  is like  y, x  instead of  x, y. It gets a bit confusing.
        let line_index = self.line_index.saturating_add_signed(line_offset);
        let column_index = self.column_index.saturating_add_signed(column_offset);
        Self {
            line_index,
            column_index,
        }
    }

    pub fn with_line_index(&self, line_index: u32) -> Self {
        Self {
            line_index,
            column_index: self.column_index,
        }
    }

    pub fn with_column_index(&self, column_index: u32) -> Self {
        Self {
            line_index: self.line_index,
            column_index,
        }
    }

    pub fn offset_between(&self, other: &Self) -> Offset {
        self.to_offset() - other.to_offset()
    }

    pub fn to_offset(&self) -> Offset {
        Offset {
            line_offset: self.line_index as i32,
            column_offset: self.column_index as i32,
        }
    }
}

impl std::cmp::PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.line_index.cmp(&other.line_index) {
            std::cmp::Ordering::Equal => Some(self.column_index.cmp(&other.column_index)),
            o => Some(o),
        }
    }
}
impl std::cmp::Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl std::ops::Sub for Position {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            column_index: self.column_index - rhs.column_index,
            line_index: self.line_index - rhs.line_index,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Offset {
    pub line_offset: i32,
    pub column_offset: i32,
}

impl Offset {
    pub const ZERO: Self = Self {
        line_offset: 0,
        column_offset: 0,
    };

    pub fn new(line_offset: i32, column_offset: i32) -> Self {
        Self {
            line_offset,
            column_offset,
        }
    }
}

impl std::ops::Add for Offset {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            column_offset: self.column_offset + rhs.column_offset,
            line_offset: self.line_offset + rhs.line_offset,
        }
    }
}
impl std::ops::AddAssign for Offset {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl std::ops::Sub for Offset {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            column_offset: self.column_offset - rhs.column_offset,
            line_offset: self.line_offset - rhs.line_offset,
        }
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selections__overlapping_selections_merged__no_overlap() {
        let pos0 = Position::new(0, 0);
        let pos1 = Position::new(0, 2);
        let pos2 = Position::new(1, 0);
        let pos3 = Position::new(3, 79);

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
        let pos1 = Position::new(0, 2);
        let pos2 = Position::new(0, 15);
        let pos3 = Position::new(1, 0);
        let pos4 = Position::new(3, 79);

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
