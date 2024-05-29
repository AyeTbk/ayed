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
            x: position.column,
            y: position.row,
            width: size.column,
            height: size.row,
        }
    }

    pub fn from_positions(a: Position, b: Position) -> Self {
        let top = u32::min(a.row, b.row);
        let bottom = u32::max(a.row, b.row);
        let left = u32::min(a.column, b.column);
        let right = u32::max(a.column, b.column);

        let width = right - left + 1;
        let height = bottom - top + 1;

        Self {
            x: left,
            y: top,
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
        (self.x, self.y).into()
    }

    pub fn bottom_right(&self) -> Position {
        (self.right(), self.bottom()).into()
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
        let column_offset: i32 = if position.column < self.left() {
            (position.column as i64 - self.left() as i64)
                .try_into()
                .unwrap()
        } else if position.column > self.right() {
            (position.column as i64 - self.right() as i64)
                .try_into()
                .unwrap()
        } else {
            0
        };
        let row_offset: i32 = if position.row < self.top() {
            (position.row as i64 - self.top() as i64)
                .try_into()
                .unwrap()
        } else if position.row > self.bottom() {
            (position.row as i64 - self.bottom() as i64)
                .try_into()
                .unwrap()
        } else {
            0
        };
        Offset::new(column_offset, row_offset)
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
