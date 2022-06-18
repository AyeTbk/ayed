use std::path::{Path, PathBuf};

use crate::selection::{Position, Selection};

#[derive(Debug, Default)]
pub struct Buffer {
    inner: String,
    filepath: Option<PathBuf>,
}

impl Buffer {
    pub fn new_scratch() -> Self {
        Self {
            inner: String::new(),
            filepath: None,
        }
    }

    pub fn from_filepath(filepath: &Path) -> Self {
        let inner = std::fs::read_to_string(filepath).expect("TODO error handling");
        Self {
            inner,
            filepath: Some(filepath.to_owned()),
        }
    }

    pub fn filepath(&self) -> Option<&Path> {
        self.filepath.as_ref().map(|p| p.as_path())
    }

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
