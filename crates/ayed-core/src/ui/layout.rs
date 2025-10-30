use crate::position::{Offset, Position};

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0, 0, 1, 1)
    }
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn with_position_and_size(position: Position, size: Size) -> Self {
        Self {
            x: position.column.try_into().unwrap(),
            y: position.row.try_into().unwrap(),
            width: size.column,
            height: size.row,
        }
    }

    pub fn from_positions(a: Position, b: Position) -> Self {
        let top = i32::min(a.row, b.row);
        let bottom = i32::max(a.row, b.row);
        let left = i32::min(a.column, b.column);
        let right = i32::max(a.column, b.column);

        let width = (right - left + 1).try_into().unwrap();
        let height = (bottom - top + 1).try_into().unwrap();

        Self {
            x: left.try_into().unwrap(),
            y: top.try_into().unwrap(),
            width,
            height,
        }
    }

    pub fn top(&self) -> u32 {
        self.y
    }

    pub fn bottom(&self) -> u32 {
        (self.y + self.height).saturating_sub(1)
    }

    pub fn left(&self) -> u32 {
        self.x
    }

    pub fn right(&self) -> u32 {
        (self.x + self.width).saturating_sub(1)
    }

    pub fn top_left(&self) -> Position {
        (self.left() as _, self.top() as _).into()
    }

    pub fn top_right(&self) -> Position {
        (self.right() as _, self.top() as _).into()
    }

    pub fn bottom_right(&self) -> Position {
        (self.right() as _, self.bottom() as _).into()
    }

    pub fn size(&self) -> Size {
        (self.width, self.height).into()
    }

    pub fn contains_position(&self, position: Position) -> bool {
        self.top_left() <= position && position <= self.bottom_right()
    }

    pub fn intersection(&self, other: Rect) -> Option<Rect> {
        let top = u32::max(self.top(), other.top());
        let bottom = u32::min(self.bottom(), other.bottom());
        let left = u32::max(self.left(), other.left());
        let right = u32::min(self.right(), other.right());

        if top <= bottom && left <= right {
            Some(Rect::new(left, top, right - left + 1, bottom - top + 1))
        } else {
            None
        }
    }

    pub fn offset_from_position(&self, position: Position) -> Offset {
        let column_offset: i32 = if position.column < self.left() as i32 {
            position.column - self.left() as i32
        } else if position.column > self.right() as i32 {
            position.column - self.right() as i32
        } else {
            0
        };
        let row_offset: i32 = if position.row < self.top() as i32 {
            position.row - self.top() as i32
        } else if position.row > self.bottom() as i32 {
            position.row - self.bottom() as i32
        } else {
            0
        };
        Offset::new(column_offset, row_offset)
    }

    pub fn offset(&self, offset: impl Into<Offset>) -> Self {
        let offset = offset.into();
        Self {
            x: self.x.saturating_add_signed(offset.column),
            y: self.y.saturating_add_signed(offset.row),
            ..*self
        }
    }

    pub fn grown(&self, top: i32, bottom: i32, left: i32, right: i32) -> Self {
        Self {
            x: self.x.saturating_add_signed(-left),
            y: self.y.saturating_add_signed(-top),
            width: self.width.saturating_add_signed(right + left),
            height: self.height.saturating_add_signed(bottom + top),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub column: u32,
    pub row: u32,
}

impl Size {
    pub fn new(column: u32, row: u32) -> Self {
        Self { column, row }
    }
}

impl From<(u32, u32)> for Size {
    fn from(value: (u32, u32)) -> Self {
        Self {
            column: value.0,
            row: value.1,
        }
    }
}
