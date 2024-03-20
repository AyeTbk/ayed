#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
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
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: u32,
    pub row: u32,
}

impl Position {
    pub const ZERO: Self = Self { column: 0, row: 0 };

    pub fn new(column: u32, row: u32) -> Self {
        Self { column, row }
    }

    pub fn offset(&self, offset: impl Into<Offset>) -> Self {
        let offset = offset.into();
        self.with_moved_indices(offset.column, offset.row)
    }

    pub fn with_moved_indices(&self, column_offset: i32, row_offset: i32) -> Self {
        let column = self.column.saturating_add_signed(column_offset);
        let row = self.row.saturating_add_signed(row_offset);
        Self { column, row }
    }

    pub fn with_row(&self, row: u32) -> Self {
        Self {
            column: self.column,
            row,
        }
    }

    pub fn with_column(&self, column: u32) -> Self {
        Self {
            column,
            row: self.row,
        }
    }

    pub fn offset_between(&self, other: &Self) -> Offset {
        self.to_offset() - other.to_offset()
    }

    pub fn to_offset(&self) -> Offset {
        Offset {
            column: self.column as i32,
            row: self.row as i32,
        }
    }
}

impl std::cmp::PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.row.cmp(&other.row) {
            std::cmp::Ordering::Equal => Some(self.column.cmp(&other.column)),
            o => Some(o),
        }
    }
}
impl std::cmp::Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Less)
    }
}

impl std::ops::Sub for Position {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            column: self.column.saturating_sub(rhs.column),
            row: self.row.saturating_sub(rhs.row),
        }
    }
}

impl From<(u32, u32)> for Position {
    fn from(value: (u32, u32)) -> Self {
        Self {
            column: value.0,
            row: value.1,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Offset {
    pub column: i32,
    pub row: i32,
}

impl Offset {
    pub const ZERO: Self = Self { column: 0, row: 0 };

    pub fn new(column_offset: i32, row_offset: i32) -> Self {
        Self {
            column: column_offset,
            row: row_offset,
        }
    }
}

impl std::ops::Add for Offset {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            column: self.column + rhs.column,
            row: self.row + rhs.row,
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
            column: self.column - rhs.column,
            row: self.row - rhs.row,
        }
    }
}
impl std::ops::Neg for Offset {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            column: -self.column,
            row: -self.row,
        }
    }
}

impl From<(i32, i32)> for Offset {
    fn from(value: (i32, i32)) -> Self {
        Self {
            column: value.0,
            row: value.1,
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
