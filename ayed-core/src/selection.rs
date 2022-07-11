#[derive(Debug, Clone, Copy)]
pub struct SelectionBounds {
    pub from: Position,
    pub to: Position,
}

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
}

impl Selection {
    pub fn new() -> Self {
        Self {
            cursor: Position::ZERO,
            anchor: Position::ZERO,
        }
    }

    pub fn with_position(&self, position: Position) -> Self {
        Self {
            cursor: position,
            anchor: position,
        }
    }

    pub fn shrunk(&self) -> Self {
        let mut this = *self;
        this.anchor = this.cursor;
        this
    }

    pub fn _length(&self) -> u32 {
        todo!("FIXME move this elsewhere, a selection cannot know its length")
    }

    pub fn cursor(&self) -> Position {
        self.cursor
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

    pub fn start_end(&self) -> (Position, Position) {
        let from;
        let to;
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

    pub fn bounds(&self) -> SelectionBounds {
        let (from, mut to) = self.start_end();
        to.column_index += 1;
        SelectionBounds { from, to }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line_index: u32,
    pub column_index: u32,
}

impl Position {
    pub const ZERO: Self = Self {
        line_index: 0,
        column_index: 0,
    };

    pub fn with_moved_indices(&self, line_offset: i32, column_offset: i32) -> Self {
        // FIXME line_offset, column_offset  is like  y, x  instead of  x, y. It gets a bit confusing.
        let line_index = saturating_add_signed(self.line_index, line_offset);
        let column_index = saturating_add_signed(self.column_index, column_offset);
        Self {
            line_index,
            column_index,
        }
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
