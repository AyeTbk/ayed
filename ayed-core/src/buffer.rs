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

    pub fn viewport_content_string<'a>(
        &'a self,
        output: &mut Vec<&'a str>,
        viewport_size: (u32, u32),
    ) {
        let start_line_index = self.viewport_top_left_position.line_index;
        let end_line_index = start_line_index + viewport_size.1;
        let start_column_index = self.viewport_top_left_position.column_index;
        let line_slice_max_len = viewport_size.0;

        output.clear();

        for line_index in start_line_index..=end_line_index {
            let full_line = if let Some(line) = self.content.line(line_index) {
                line
            } else {
                break;
            };
            let (start_column, end_column) = if start_column_index as usize >= full_line.len() {
                (0, 0)
            } else {
                let expected_end = start_column_index as usize + line_slice_max_len as usize;
                let end = expected_end.min(full_line.len());
                (start_column_index as usize, end)
            };
            let sliced_line = &full_line[start_column..end_column];
            output.push(sliced_line);
        }
    }

    pub fn execute_command(&mut self, command: Command, _viewport_size: (u32, u32)) {
        match command {
            Command::Insert(c) => self.insert_char(c),
            Command::DeleteBeforeSelection => self.delete_before_selection(),
            Command::DeleteSelection => self.delete_selection(),
            Command::MoveSelectionUp => self.move_selection_vertically(-1),
            Command::MoveSelectionDown => self.move_selection_vertically(1),
            Command::MoveSelectionLeft => self.move_selection_horizontally(-1),
            Command::MoveSelectionRight => self.move_selection_horizontally(1),
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        for selection in self.selections.iter_mut() {
            if let Ok(new_position) = self.content.insert_char_at(ch, selection.position) {
                selection.position = new_position;
            }
        }
    }

    pub fn delete_before_selection(&mut self) {
        for selection in self.selections.iter_mut() {
            if selection.position == self.content.start_of_content_position() {
                // Can't delete before the beginning!
                continue;
            }
            let before_selection = self
                .content
                .moved_position_horizontally(selection.position, -1)
                .expect("wow?");
            self.content
                .delete_selection(Selection::new().with_position(before_selection));

            let new_selection = selection.with_position(before_selection);
            *selection = new_selection;
        }
    }

    pub fn delete_selection(&mut self) {
        for selection in self.selections.iter_mut() {
            self.content.delete_selection(*selection);
            *selection = selection.shrunk();
        }
    }

    pub fn move_selection_horizontally(&mut self, column_offset: i32) {
        for selection in self.selections.iter_mut() {
            let new_position = if let Some(moved_position) = self
                .content
                .moved_position_horizontally(selection.position, column_offset)
            {
                moved_position
            } else {
                if column_offset < 0 {
                    self.content.start_of_content_position()
                } else {
                    self.content.end_of_content_position()
                }
            };
            selection.position = new_position;
        }
    }

    pub fn move_selection_vertically(&mut self, line_offset: i32) {
        for selection in self.selections.iter_mut() {
            if let Some(moved_position) = self
                .content
                .moved_position_vertically(selection.position, line_offset)
            {
                selection.position = moved_position;
            }
        }
    }

    pub fn selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
        // FIXME this only shows selections as hacing a length
        self.selections.iter().map(|selection| SelectionBounds {
            from: selection.position.with_moved_indices(0, 0),
            to: selection.position.with_moved_indices(0, 1),
        })
    }
}

#[derive(Debug, Default)]
struct BufferContent {
    inner: String,
}

impl BufferContent {
    pub fn line(&self, line_index: u32) -> Option<&str> {
        let mut current_line_index = 0;
        let mut line_start_idx = None;
        let mut line_end_idx = None;
        for (idx, ch) in self.inner.char_indices() {
            if line_start_idx.is_none() && current_line_index == line_index {
                line_start_idx = Some(idx);
            }
            if ch == '\n' {
                current_line_index += 1;

                if current_line_index == line_index + 1 {
                    line_end_idx = Some(idx);
                    break;
                }
            }
        }

        match (line_start_idx, line_end_idx) {
            (None, None) => None,
            (Some(start_idx), None) => Some(&self.inner[start_idx..]),
            (Some(start_idx), Some(end_idx)) => Some(&self.inner[start_idx..end_idx]),
            (None, Some(_)) => unreachable!(),
        }
    }

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

    pub fn delete_selection(&mut self, selection: Selection) {
        if let Some(start_idx) = self.position_to_content_index(selection.position) {
            let end_idx = start_idx + selection.length() as usize;
            let range = if end_idx < self.inner.len() {
                start_idx..end_idx
            } else {
                start_idx..self.inner.len()
            };
            self.inner.drain(range);
        }
    }

    pub fn moved_position_horizontally(
        &self,
        position: Position,
        column_offset: i32,
    ) -> Option<Position> {
        if let Some(position_idx) = self.position_to_content_index(position) {
            let moved_idx = position_idx as isize + column_offset as isize;
            if moved_idx < 0 {
                None
            } else {
                self.content_index_to_position(moved_idx as usize)
            }
        } else {
            None
        }
    }

    pub fn moved_position_vertically(
        &self,
        position: Position,
        line_offset: i32,
    ) -> Option<Position> {
        // TODO maybe check if position is within content as some kind of sanity check? idk
        let new_line_index = position.line_index as i64 + line_offset as i64;
        if new_line_index < 0 {
            return None;
        }
        let new_line_index = new_line_index as u32;
        if let Some(line) = self.line(new_line_index) {
            let new_column_index = position.column_index.min(line.len() as u32);
            Some(Position {
                column_index: new_column_index,
                line_index: new_line_index,
            })
        } else {
            None
        }
    }

    pub fn start_of_content_position(&self) -> Position {
        Position::ZERO
    }

    pub fn end_of_content_position(&self) -> Position {
        match self.inner.chars().count() {
            0 => Position::ZERO,
            len => self
                .content_index_to_position(len)
                .expect("index is in bounds, so there should be a position"),
        }
    }

    fn _clamped_position(&self, position: Position) -> Position {
        // TODO remove this method if it's unused
        if self.position_to_content_index(position).is_some() {
            position
        } else {
            self.end_of_content_position()
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

pub struct SelectionBounds {
    pub from: Position,
    pub to: Position,
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
