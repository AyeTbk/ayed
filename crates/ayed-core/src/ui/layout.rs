use crate::position::{Offset, Position};

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Default for Rect {
    fn default() -> Self {
        Self::new(0, 0, 1, 1)
    }
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
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

        let width = (right - left).saturating_add(1);
        let height = (bottom - top).saturating_add(1);

        Self {
            x: left,
            y: top,
            width,
            height,
        }
    }

    pub fn top(&self) -> i32 {
        self.y
    }

    pub fn bottom(&self) -> i32 {
        self.y.saturating_add(self.height).saturating_sub(1)
    }

    pub fn left(&self) -> i32 {
        self.x
    }

    pub fn right(&self) -> i32 {
        self.x.saturating_add(self.width).saturating_sub(1)
    }

    pub fn top_left(&self) -> Position {
        (self.left(), self.top()).into()
    }

    pub fn top_right(&self) -> Position {
        (self.right(), self.top()).into()
    }

    pub fn bottom_right(&self) -> Position {
        (self.right(), self.bottom()).into()
    }

    pub fn size(&self) -> Size {
        (self.width, self.height).into()
    }

    pub fn contains_position(&self, position: Position) -> bool {
        self.left() <= position.column
            && position.column <= self.right()
            && self.top() <= position.row
            && position.row <= self.bottom()
    }

    pub fn intersection(&self, other: Rect) -> Option<Rect> {
        let top = i32::max(self.top(), other.top());
        let bottom = i32::min(self.bottom(), other.bottom());
        let left = i32::max(self.left(), other.left());
        let right = i32::min(self.right(), other.right());

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
            x: self.x.saturating_add(offset.column),
            y: self.y.saturating_add(offset.row),
            ..*self
        }
    }

    pub fn grown(&self, top: i32, bottom: i32, left: i32, right: i32) -> Self {
        Self {
            x: self.x.saturating_add(-left),
            y: self.y.saturating_add(-top),
            width: self.width.saturating_add(right + left),
            height: self.height.saturating_add(bottom + top),
        }
    }

    pub fn cells(&self) -> impl Iterator<Item = Position> {
        let (start_y, end_y) = (self.y, self.bottom());
        let (start_x, end_x) = (self.x, self.right());
        let mut y = start_y;
        let mut x = start_x;
        std::iter::from_fn(move || {
            if y > end_y {
                return None;
            }
            let cell_x = x;
            let cell_y = y;
            x += 1;
            if x > end_x {
                x = start_x;
                y += 1
            }
            return Some(Position::new(cell_x, cell_y));
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub column: i32,
    pub row: i32,
}

impl Size {
    pub fn new(column: i32, row: i32) -> Self {
        Self { column, row }
    }
}

impl From<(i32, i32)> for Size {
    fn from(value: (i32, i32)) -> Self {
        Self {
            column: value.0,
            row: value.1,
        }
    }
}

impl From<(u32, u32)> for Size {
    fn from(value: (u32, u32)) -> Self {
        Self {
            column: value.0.try_into().unwrap(),
            row: value.1.try_into().unwrap(),
        }
    }
}
