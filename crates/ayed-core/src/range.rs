use crate::position::Position;

/// A range that is [inclusive, exclusive[
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn normalized(self) -> Self {
        let (start, end) = if self.end < self.start {
            (self.end, self.start)
        } else {
            (self.start, self.end)
        };
        Self { start, end }
    }

    pub fn contains(&self, pos: Position) -> bool {
        self.start <= pos && pos < self.end
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl From<(Position, Position)> for Range {
    fn from(value: (Position, Position)) -> Self {
        Self {
            start: value.0,
            end: value.1,
        }
    }
}

impl From<Position> for Range {
    fn from(value: Position) -> Self {
        Self {
            start: value,
            end: value,
        }
    }
}
