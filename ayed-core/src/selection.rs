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

    pub fn on_line(&self, line_index: u32) -> Vec<Selection> {
        // FIXME handle more than just the start of the selection

        let mut selections = Vec::new();
        let mut push_if_on_line = |sel: Selection| {
            if sel.position.line_index == line_index {
                selections.push(self.primary_selection);
            }
        };

        push_if_on_line(self.primary_selection);
        for selection in self.extra_selections.iter().copied() {
            push_if_on_line(selection);
        }

        selections
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
    pub position: Position,
    pub extra_length: u32,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            position: Position::ZERO,
            extra_length: 0,
        }
    }

    pub fn with_position(&self, position: Position) -> Self {
        Self {
            position,
            extra_length: self.extra_length,
        }
    }

    pub fn shrunk(&self) -> Self {
        let mut this = *self;
        this.extra_length = 0;
        this
    }

    pub fn length(&self) -> u32 {
        self.extra_length + 1
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
        let line_index = saturating_add_signed(self.line_index, line_offset);
        let column_index = saturating_add_signed(self.column_index, column_offset);
        Self {
            line_index,
            column_index,
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
