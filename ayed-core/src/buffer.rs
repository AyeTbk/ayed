use crate::command::Command;

pub struct Buffer {
    content: BufferContent,
    selections: Selections,
    viewport_top_left_position: Position,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            content: BufferContent::default(),
            selections: Selections::new(),
            viewport_top_left_position: Position::ZERO,
        }
    }

    pub fn viewport_content_string(&self, output: &mut String, viewport_size: (u32, u32)) {
        *output = self.content.inner.clone();
        todo!() // Also maybe change viewport_size parameter for a context
    }

    pub fn execute_command(&mut self, command: Command, viewport_size: (u32, u32)) {
        match command {
            Command::Insert(c) => self.insert_char(c),
            Command::MoveSelectionUp => self.move_selection(-1, 0),
            Command::MoveSelectionDown => self.move_selection(1, 0),
            Command::MoveSelectionLeft => self.move_selection(0, -1),
            Command::MoveSelectionRight => self.move_selection(0, 1),
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        for selection in self.selections.iter_mut() {
            if let Ok(new_position) = self.content.insert_char_at(ch, selection.position) {
                selection.position = new_position;
            }
        }
    }

    pub fn move_selection(&mut self, line_offset: i32, column_offset: i32) {
        for selection in self.selections.iter_mut() {
            let moved_position = selection.position.moved(line_offset, column_offset);
            let clamped_moved_position = self.content.clamp_position(moved_position);
            selection.position = clamped_moved_position;
        }
    }
}

#[derive(Debug, Default)]
struct BufferContent {
    inner: String,
}

impl BufferContent {
    pub fn insert_char_at(&mut self, ch: char, position: Position) -> Result<Position, ()> {
        if let Some(idx) = self.position_to_content_index(position) {
            self.inner.insert(idx, ch);

            let mut position = position;
            if ch == '\n' {
                position.line_index += 1;
                position.column_index = 0;
            } else {
                position.column_index += 1;
            }
            Ok(position)
        } else {
            Err(())
        }
    }

    pub fn clamp_position(&self, position: Position) -> Position {
        if self.position_to_content_index(position).is_some() {
            position
        } else {
            self.end_of_content_position()
        }
    }

    fn end_of_content_position(&self) -> Position {
        match self.inner.chars().count() {
            0 => Position::ZERO,
            len => self
                .content_index_to_position(len - 1)
                .expect("index is in bounds, so there should be a position"),
        }
    }

    fn content_index_to_position(&self, index: usize) -> Option<Position> {
        let mut current_index: usize = 0;
        let mut position: Position = Position::ZERO;
        for ch in self.inner.chars() {
            if current_index == index {
                break;
            }
            if ch == '\n' {
                position.line_index += 1;
                position.column_index = 0;
            } else {
                position.column_index += 1;
            }
            current_index += 1;
        }
        if current_index == index {
            Some(position)
        } else {
            None
        }
    }

    /// Returns the index corresponding to position, if it's not beyond the content.
    fn position_to_content_index(&self, position: Position) -> Option<usize> {
        let mut index: usize = 0;
        let mut current_position: Position = Position::ZERO;
        for ch in self.inner.chars() {
            if current_position == position {
                break;
            }
            if ch == '\n' {
                current_position.line_index += 1;
                current_position.column_index = 0;
            } else {
                current_position.column_index += 1;
            }
            index += 1;
        }
        if current_position == position {
            Some(index)
        } else {
            None
        }
    }
}

struct Selections {
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

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Selection> {
        Some(&mut self.primary_selection)
            .into_iter()
            .chain(self.extra_selections.iter_mut())
    }
}

#[derive(Debug, Clone, Copy)]
struct Selection {
    position: Position,
    extra_length: u32,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            position: Position::ZERO,
            extra_length: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Position {
    line_index: u32,
    column_index: u32,
}

impl Position {
    pub const ZERO: Self = Self {
        line_index: 0,
        column_index: 0,
    };

    pub fn moved(&self, line_offset: i32, column_offset: i32) -> Self {
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
