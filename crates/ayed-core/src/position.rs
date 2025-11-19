pub type Column = i32;
pub type Row = i32;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: Column,
    pub row: Row,
}

impl Position {
    pub const ZERO: Self = Self { column: 0, row: 0 };
    pub const MAX: Self = Self {
        column: i32::MAX,
        row: i32::MAX,
    };

    pub fn new(column: Column, row: Row) -> Self {
        Self { column, row }
    }

    pub fn parse(src: &str) -> Result<Self, String> {
        let mut parts = src.trim().split(':');
        let (row_src, column_src) = (|| {
            let start = parts.next()?;
            let end = parts.next()?;
            Some((start, end))
        })()
        .ok_or_else(|| format!("invalid position format: {src}"))?;
        let row = row_src
            .parse::<Column>()
            .map_err(|_| format!("invalid row: {row_src}"))?;
        let column = column_src
            .parse::<Row>()
            .map_err(|_| format!("invalid column: {column_src}"))?;
        Ok(Self::new(column, row))
    }

    pub fn offset(&self, offset: impl Into<Offset>) -> Self {
        let offset = offset.into();
        let column = self.column.saturating_add(offset.column); // idc just saturate
        let row = self.row + offset.row;
        Self { column, row }
    }

    pub fn with_row(&self, row: Row) -> Self {
        Self {
            column: self.column,
            row,
        }
    }

    pub fn with_column(&self, column: Column) -> Self {
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

    // TODO Remove this eventually, and find a better name
    pub fn local_to(&self, other: Self) -> (Option<i32>, Option<i32>) {
        let column = self.column - other.column;
        let column = (column >= 0).then_some(column);
        let row = self.row - other.row;
        let row = (row >= 0).then_some(row);
        (column, row)
    }

    pub fn local_to_pos(&self, other: Self) -> Position {
        let column = self.column - other.column;
        let row = self.row - other.row;
        Position::new(column, row)
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

impl std::ops::Add for Position {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            column: self.column + rhs.column,
            row: self.row + rhs.row,
        }
    }
}

impl std::ops::Sub for Position {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            column: self.column - rhs.column,
            row: self.row - rhs.row,
        }
    }
}

impl From<(i32, i32)> for Position {
    fn from(value: (i32, i32)) -> Self {
        Self {
            column: value.0,
            row: value.1,
        }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}:{}", self.row, self.column))
    }
}

// FIXME this is superfluous now that positions are using i32. Remove this
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

impl From<Position> for Offset {
    fn from(value: Position) -> Self {
        Self {
            column: value.column,
            row: value.row,
        }
    }
}
