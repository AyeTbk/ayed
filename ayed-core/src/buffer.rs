use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

use crate::selection::{Position, Selection};

pub mod char_string;
use self::char_string::CharString;

// Notes:
// The lines of text must uphold some invariants:
// 1. There is always at least one line.
// 2. Every line must have one and only one '\n' and it must be at the end.
#[derive(Debug, Default)]
pub struct Buffer {
    lines: Vec<CharString>,
    filepath: Option<PathBuf>,
}

impl Buffer {
    pub fn new_empty() -> Self {
        Self {
            lines: vec![CharString::from("\n")],
            filepath: None,
        }
    }

    pub fn from_filepath(filepath: &Path) -> Self {
        let file = std::fs::File::open(filepath).expect("TODO error handling");
        let mut inner: Vec<CharString> = std::io::BufReader::new(file)
            .lines()
            .map(|res| res.expect("TODO error handling").into())
            .collect();
        for line in inner.iter_mut() {
            line.push('\n');
        }

        Self {
            lines: inner,
            filepath: Some(filepath.to_owned()),
        }
    }

    pub fn filepath(&self) -> Option<&Path> {
        self.filepath.as_ref().map(|p| p.as_path())
    }

    pub fn save(&self) -> Result<(), ()> {
        if let Some(filepath) = &self.filepath {
            let file = std::fs::File::create(filepath).expect("TODO error handling");
            let mut writer = std::io::BufWriter::new(file);
            for line in &self.lines {
                line.write_all(&mut writer).expect("TODO error handling");
            }
            writer.flush().expect("TODO error handling");
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn insert_char_at(&mut self, ch: char, position: Position) -> Result<Position, ()> {
        if let Some(mut line) = self.take_line(position.line_index) {
            if (position.column_index as usize) > line.len() {
                return Err(());
            }

            // FIXME Im not convinced the new position of the cursor is something the buffer should
            // even care about. It probably should just be handled in the TextEditor.
            let mut new_position = position;
            new_position.column_index += 1;

            if ch == '\n' {
                new_position.line_index += 1;
                new_position.column_index = 0;
                let new_line_content = line.drain(position.column_index as usize..).collect();
                self.insert_line(new_position.line_index, new_line_content);
            }
            line.insert(position.column_index as usize, ch);

            self.set_line(position.line_index, line);

            Ok(new_position)
        } else {
            Err(())
        }
    }

    pub fn delete_selection(&mut self, selection: Selection) {
        let selection_length = self.selection_length(selection).unwrap();
        for _ in 0..selection_length {
            self.delete_position(selection.start()).unwrap();
        }
    }

    fn delete_position(&mut self, position: Position) -> Result<(), ()> {
        let mut line = self.take_line(position.line_index).ok_or(())?;

        match line.get(position.column_index as usize) {
            Some('\n') => {
                if let Some(next_line) = self.remove_line(position.line_index + 1) {
                    assert_eq!(position.column_index as usize, line.len() - 1);
                    line.pop();
                    line.extend(next_line);
                }
            }
            Some(_) => {
                line.remove(position.column_index as usize);
            }
            None => {
                return Err(());
            }
        }

        self.set_line(position.line_index, line);

        Ok(())
    }

    pub fn moved_position_horizontally(
        &self,
        position: Position,
        column_offset: i32,
    ) -> Option<Position> {
        if column_offset.abs() > 1 {
            todo!("implement this for offsets greater than 1 if needed");
        }

        if self.line(position.line_index).is_none() {
            panic!("not on a line");
        }

        // FIXME? check that the position is valid?

        let mut new_line_index = position.line_index;
        let mut new_column_index = position.column_index;

        if column_offset == -1 && position.column_index == 0 {
            if position.line_index != 0 {
                new_line_index -= 1;
                let line = self
                    .line(new_line_index)
                    .expect("bound check is performed just above");
                new_column_index = (line.len() - 1) as _;
            } else {
                // Cant move back, we're literally at the very beginning of the buffer
            }
        } else if column_offset == 1
            && position.column_index >= (self.line(position.line_index).unwrap().len() - 1) as _
        // FIXME this crashes
        {
            if position.line_index < self.line_count() - 1 {
                new_line_index += 1;
                new_column_index = 0;
            } else {
                // Cant move forward, we're literally at the very end of the buffer
            }
        } else {
            new_column_index = (new_column_index as i64 + column_offset as i64) as u32;
        }

        Some(Position::new(new_line_index, new_column_index))
    }

    pub fn moved_position_vertically(
        &self,
        position: Position,
        line_offset: i32,
    ) -> Option<Position> {
        let destination_line_index = position.line_index as i64 + line_offset as i64;

        if let Some(destination_line) = self.line(destination_line_index as _) {
            let destination_column_index =
                position.column_index.min((destination_line.len() - 1) as _); // FIXME? what if dest_line.len() is 0?
            Some(Position::new(
                destination_line_index as _,
                destination_column_index,
            ))
        } else {
            None
        }
    }

    pub fn start_of_content_position(&self) -> Position {
        Position::ZERO
    }

    pub fn end_of_content_position(&self) -> Position {
        let last_column_index_of_last_line = self
            .lines
            .last()
            .expect("there must always be at least one line")
            .len()
            - 1;
        Position::ZERO
            .with_moved_indices(self.lines.len() as _, last_column_index_of_last_line as _)
    }

    pub fn selection_length(&self, selection: Selection) -> Option<u32> {
        let mut len: u32 = 0;

        let start_line_index = selection.start().line_index;
        let end_line_index = selection.end().line_index;
        let start_column_index = selection.start().column_index as usize;
        let end_column_index = selection.end().column_index as usize;

        for line_index in start_line_index..=end_line_index {
            let line = self.line(line_index)?;

            let is_start_line = line_index == start_line_index;
            let is_end_line = line_index == end_line_index;

            if is_start_line && is_end_line {
                len += line[start_column_index..=end_column_index].len() as u32;
            } else if is_start_line {
                len += line[start_column_index..].len() as u32;
            } else if is_end_line {
                len += line[..=end_column_index].len() as u32;
            } else {
                len += line.len() as u32;
            }
        }
        Some(len)
    }

    pub fn line_count(&self) -> u32 {
        self.lines.len() as _
    }

    pub fn copy_line(&self, line_index: u32, buf: &mut String) -> Option<()> {
        let line = self.line(line_index)?;

        buf.clear();
        buf.extend(line.chars());

        // Remove '\n', it only exists in the buffer as a way to simplify things internally
        buf.pop();

        Some(())
    }

    pub fn line(&self, line_index: u32) -> Option<&CharString> {
        self.lines.get(line_index as usize)
    }

    pub fn line_len(&self, line_index: u32) -> Option<usize> {
        self.lines
            .get(line_index as usize)
            .map(|chrstr| chrstr.len() - 1) // Minus one, because of invariant 2.
    }

    fn set_line(&mut self, line_index: u32, line: CharString) {
        self.lines[line_index as usize] = line;
    }

    fn insert_line(&mut self, line_index: u32, line: CharString) {
        self.lines.insert(line_index as usize, line);
    }

    /// Replace line with with default value and return it.
    fn take_line(&mut self, line_index: u32) -> Option<CharString> {
        let idx = line_index as usize;
        let line = self.lines.get_mut(idx)?;
        Some(std::mem::take(line))
    }

    fn remove_line(&mut self, line_index: u32) -> Option<CharString> {
        let idx = line_index as usize;
        if idx < self.lines.len() {
            Some(self.lines.remove(idx))
        } else {
            None
        }
    }
}
