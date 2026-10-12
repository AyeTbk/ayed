#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: i32,
    pub row: i32,
}

/// Physical position. Position in a buffer's raw content.
pub type PPosition = Position;
/// Logical position. Position in the virtual representation of the buffer's content.
pub type LPosition = Position;
/// View position. Position relative to the view into the buffer.
pub type VPosition = Position;

impl Position {
    pub const ZERO: Self = Self { column: 0, row: 0 };
    pub const MAX: Self = Self {
        column: i32::MAX,
        row: i32::MAX,
    };

    pub fn new(column: i32, row: i32) -> Self {
        Self { column, row }
    }

    /// Parse a position from a pattern like `row:column`, one-based.
    pub fn parse(src: &str) -> Result<Self, String> {
        let mut parts = src.trim().split(':');
        let (row_src, column_src) = (|| {
            let start = parts.next()?;
            let end = parts.next()?;
            Some((start, end))
        })()
        .ok_or_else(|| format!("invalid position format: {src}"))?;
        let row_plus_one = row_src
            .parse::<i32>()
            .map_err(|_| format!("invalid row: {row_src}"))?;
        let column_plus_one = column_src
            .parse::<i32>()
            .map_err(|_| format!("invalid column: {column_src}"))?;

        let row = i32::max(0, row_plus_one.saturating_sub(1));
        let column = i32::max(0, column_plus_one.saturating_sub(1));
        Ok(Self::new(column, row))
    }

    pub fn offset(&self, offset: impl Into<Offset>) -> Self {
        let offset = offset.into();
        let column = self.column.saturating_add(offset.column);
        let row = self.row.saturating_add(offset.row);
        Self { column, row }
    }

    pub fn offset_between(&self, other: &Self) -> Offset {
        (*self - *other).to_offset()
    }

    pub fn to_offset(&self) -> Offset {
        Offset {
            column: self.column as i32,
            row: self.row as i32,
        }
    }

    pub fn with_row(&self, row: i32) -> Self {
        Self {
            column: self.column,
            row,
        }
    }

    pub fn with_column(&self, column: i32) -> Self {
        Self {
            column,
            row: self.row,
        }
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

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}:{}",
            self.row.saturating_add(1),
            self.column.saturating_add(1)
        ))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
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

impl From<(i32, i32)> for Offset {
    fn from(value: (i32, i32)) -> Self {
        Self {
            column: value.0,
            row: value.1,
        }
    }
}
