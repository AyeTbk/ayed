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

    pub fn primary(&self) -> Selection {
        self.primary_selection
    }

    pub fn _merge_overlapping_selections(&self) -> Self {
        todo!()
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

    fn start_end(&self) -> (Position, Position) {
        let from;
        let to;
        // TODO rewrite using < and > of position
        if self.cursor.line_index != self.anchor.line_index {
            if self.cursor.line_index < self.anchor.line_index {
                from = self.cursor;
                to = self.anchor;
            } else {
                from = self.anchor;
                to = self.cursor;
            }
        } else {
            if self.cursor.column_index < self.anchor.column_index {
                from = self.cursor;
                to = self.anchor;
            } else {
                from = self.anchor;
                to = self.cursor;
            }
        }
        (from, to)
    }
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

    pub fn with_moved_indices(&self, line_offset: i32, column_offset: i32) -> Self {
        // FIXME? line_offset, column_offset  is like  y, x  instead of  x, y. It gets a bit confusing.
        let line_index = saturating_add_signed(self.line_index, line_offset);
        let column_index = saturating_add_signed(self.column_index, column_offset);
        Self {
            line_index,
            column_index,
        }
    }

    pub fn with_column_index(&self, column_index: u32) -> Self {
        Self {
            line_index: self.line_index,
            column_index,
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

fn saturating_add_signed(base: u32, signed: i32) -> u32 {
    if signed >= 0 {
        u32::saturating_add(base, signed as u32)
    } else {
        u32::saturating_sub(base, signed.unsigned_abs())
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturating_add_signed__saturates_down_to_zero() {
        let result = saturating_add_signed(1, -2);
        assert_eq!(result, 0);
    }

    #[test]
    fn saturating_add_signed__saturates_up_to_MAX() {
        let result = saturating_add_signed(std::u32::MAX - 1, 5);
        assert_eq!(result, std::u32::MAX);
    }

    #[test]
    fn saturating_add_signed__adds() {
        let result1 = saturating_add_signed(5, 45);
        let result2 = saturating_add_signed(24, -8);
        assert_eq!(result1, 50);
        assert_eq!(result2, 16);
    }
}
