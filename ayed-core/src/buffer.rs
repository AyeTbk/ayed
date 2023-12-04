use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
};

use crate::{
    selection::{DeletedEditInfo, EditInfo, Selection},
    utils::Position,
};

pub mod char_string;
use self::char_string::CharString;

// FIXME This currently assumes a Rust char is a Unicode 'character', which isn't true.
// To have the correct behavior in the general case, you need to work on 'extended
// grapheme clusters' which may be made up of multiple Rust chars and also you need
// a database to know how to interpret them as far as I understand.

/// Notes:
/// The lines of text must uphold some invariants:
/// 1. There is always at least one line.
/// 2. Every line must have one and only one '\n' and it must be at the end.
#[derive(Debug, Default)]
pub struct TextBuffer {
    lines: Vec<CharString>,
    filepath: Option<PathBuf>,
    pub(crate) modified: bool,
}

impl TextBuffer {
    pub fn new_empty() -> Self {
        Self {
            lines: vec![CharString::from("\n")],
            filepath: None,
            modified: true,
        }
    }

    pub fn from_filepath(filepath: &Path) -> Self {
        if let Ok(file) = std::fs::File::open(filepath) {
            let mut lines: Vec<CharString> = std::io::BufReader::new(file)
                .lines()
                .map(|res| res.expect("TODO error handling").into())
                .collect();
            if lines.is_empty() {
                lines.push(CharString::new());
            }
            for line in lines.iter_mut() {
                line.push('\n');
            }

            Self {
                lines,
                filepath: Some(filepath.to_owned()),
                modified: true,
            }
        } else {
            let mut this = Self::new_empty();
            this.filepath = if filepath.as_os_str().is_empty() {
                None
            } else {
                Some(filepath.to_owned())
            };
            this
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

    pub fn insert_char_at(&mut self, ch: char, position: Position) -> Result<EditInfo, ()> {
        if let Some(mut line) = self.take_line(position.row) {
            if (position.column as usize) > line.len() {
                return Err(());
            }

            let mut edit = EditInfo::AddedOne(position);

            if ch == '\n' {
                edit = EditInfo::LineSplit(position);

                let new_line_content = line.drain(position.column as usize..).collect();
                self.insert_line(position.row + 1, new_line_content);
            }
            line.insert(position.column as usize, ch);

            self.set_line(position.row, line);

            self.modified = true;
            Ok(edit)
        } else {
            Err(())
        }
    }

    pub fn delete_selection(&mut self, selection: Selection) -> Result<DeletedEditInfo, ()> {
        let selection_length = self.selection_length(selection).unwrap();
        let mut pos2: Option<Position> = None;
        for _ in 0..selection_length {
            let edit = self.delete_position(selection.start())?;

            if let Some(pos2) = pos2.as_mut() {
                if edit.pos1_line_index < edit.pos2.row {
                    *pos2 = pos2.with_moved_indices(0, 1).with_column(0);
                } else {
                    *pos2 = pos2.with_moved_indices(1, 0);
                }
            } else {
                pos2 = Some(edit.pos2);
            }
        }

        let edit = DeletedEditInfo {
            pos1_line_index: selection.start().row,
            pos1_before_delete_start_column_index: (selection.start().column) as i64 - 1,
            pos2: pos2.expect(
                "selection length shouldn't be zero which is the only way this would still be None",
            ),
        };

        self.modified = true;
        Ok(edit)
    }

    fn delete_position(&mut self, position: Position) -> Result<DeletedEditInfo, ()> {
        let mut line = self.take_line(position.row).ok_or(())?;
        let mut edit_pos2 = position.with_moved_indices(1, 0);

        match line.get(position.column as usize) {
            Some('\n') => {
                if let Some(next_line) = self.remove_line(position.row + 1) {
                    assert_eq!(position.column as usize, line.len() - 1);
                    line.pop();
                    line.extend(next_line);

                    edit_pos2 = Position::new(0, position.row + 1);
                }
            }
            Some(_) => {
                line.remove(position.column as usize);
            }
            None => {
                return Err(());
            }
        }

        self.set_line(position.row, line);

        let edit = DeletedEditInfo {
            pos1_line_index: position.row,
            pos1_before_delete_start_column_index: position.column as i64 - 1,
            pos2: edit_pos2,
        };

        self.modified = true;
        Ok(edit)
    }

    pub fn moved_position_horizontally(
        &self,
        position: Position,
        column_offset: i32,
    ) -> Option<Position> {
        if column_offset.abs() > 1 {
            todo!("implement this for offsets greater than 1 if needed");
        }

        if self.line(position.row).is_none() {
            panic!("not on a line");
        }

        // FIXME? check that the position is valid?

        let mut new_line_index = position.row;
        let mut new_column_index = position.column;

        if column_offset == -1 && position.column == 0 {
            if position.row != 0 {
                new_line_index -= 1;
                let line = self
                    .line(new_line_index)
                    .expect("bound check is performed just above");
                new_column_index = (line.len() - 1) as _;
            } else {
                // Cant move back, we're literally at the very beginning of the buffer
            }
        } else if column_offset == 1
            && position.column >= (self.line(position.row).unwrap().len() - 1) as _
        // FIXME this crashes
        {
            if position.row < self.last_line_index() {
                new_line_index += 1;
                new_column_index = 0;
            } else {
                // Cant move forward, we're literally at the very end of the buffer
            }
        } else {
            new_column_index = (new_column_index as i64 + column_offset as i64) as u32;
        }

        Some(Position::new(new_column_index, new_line_index))
    }

    pub fn moved_position_vertically(
        &self,
        position: Position,
        line_offset: i32,
    ) -> Result<Position, Position> {
        //  returns a result of either:
        //     Ok(the new position)
        //     Err(the best position nearest to what the position would have been)

        let destination_line_index = (position.row as i64)
            .saturating_add(line_offset as i64)
            .max(0) as u32;

        if let Some(destination_line_len) = self.line_len(destination_line_index) {
            let destination_line_len = destination_line_len as u32;
            if position.column < destination_line_len {
                Ok(Position::new(position.column, destination_line_index))
            } else {
                Err(Position::new(destination_line_len, destination_line_index))
            }
        } else {
            let last_line_index = self.last_line_index();
            let last_line_len = self.line_len(last_line_index).expect("invariant 1") as u32;
            if position.column < last_line_len {
                Ok(Position::new(position.column, last_line_index))
            } else {
                Err(Position::new(last_line_len, last_line_index))
            }
        }
    }

    pub fn limit_selection_to_content(&self, selection: &Selection) -> Selection {
        let cursor = self.limit_position_to_content(selection.cursor());
        let anchor = self.limit_position_to_content(selection.anchor());
        selection
            .with_provisional_cursor(cursor)
            .with_provisional_anchor(anchor)
    }

    pub fn limit_position_to_content(&self, position: Position) -> Position {
        let line_index = position.row.min(self.last_line_index());
        let line_len = self
            .line_len(line_index)
            .expect("line index should be correct because of above");
        let column_index = position.column.min(line_len as u32);

        Position::new(column_index, line_index)
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
            .with_moved_indices(last_column_index_of_last_line as _, self.lines.len() as _)
    }

    pub fn selection_length(&self, selection: Selection) -> Option<u32> {
        let mut len: u32 = 0;

        let start_line_index = selection.start().row;
        let end_line_index = selection.end().row;
        let start_column_index = selection.start().column as usize;
        let end_column_index = selection.end().column as usize;

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

    pub fn copy_line(&self, line_index: u32, buf: &mut String) -> Result<(), ()> {
        let line = self.line(line_index).ok_or(())?;

        buf.clear();
        buf.extend(line.chars());

        // Remove '\n', it only exists in the buffer as a way to simplify things internally
        buf.pop();

        Ok(())
    }

    pub fn line(&self, line_index: u32) -> Option<&CharString> {
        self.lines.get(line_index as usize)
    }

    pub fn line_len(&self, line_index: u32) -> Option<usize> {
        self.lines
            .get(line_index as usize)
            .map(|chrstr| chrstr.len() - 1) // Minus one, because of invariant 2.
    }

    pub fn is_empty(&self) -> bool {
        // Invariant 1 and 2
        self.lines.len() == 1
            && self.lines.first().unwrap().len() == 1
            && self.lines.first().unwrap()[0] == '\n'
    }

    fn set_line(&mut self, line_index: u32, line: CharString) {
        self.modified = true;
        self.lines[line_index as usize] = line;
    }

    fn insert_line(&mut self, line_index: u32, line: CharString) {
        self.modified = true;
        self.lines.insert(line_index as usize, line);
    }

    /// Replace line with with default value and return it.
    fn take_line(&mut self, line_index: u32) -> Option<CharString> {
        let idx = line_index as usize;
        let line = self.lines.get_mut(idx)?;

        self.modified = true;
        Some(std::mem::take(line))
    }

    fn remove_line(&mut self, line_index: u32) -> Option<CharString> {
        let idx = line_index as usize;
        if idx < self.lines.len() {
            self.modified = true;
            Some(self.lines.remove(idx))
        } else {
            None
        }
    }

    fn last_line_index(&self) -> u32 {
        self.line_count() - 1 // note: self.line_count() is > 0 because of invariant 1
    }
}
