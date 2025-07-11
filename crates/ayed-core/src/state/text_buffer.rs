use std::{cell::Cell, collections::HashMap};

use crate::{
    position::{Column, Offset, Position, Row},
    selection::{Selection, Selections},
    slotmap::Handle,
    utils::string_utils::{char_count, char_index_to_byte_index},
};

use super::View;

// #1. There should always be at least one line. A line is a String in the lines vector.
// #2. The line terminators are not part of the content, they are implied for the
//     current line when there is a following line.
// #3. Positions refer to lines in row and to codepoints (Rust chars) of said line in column.
// #4. A position with column == line.chars().count() is allowed. It can be thought of as the
//     position of the line terminator (also allowed for the last line even though there is
//     no implied line terminator).
// #5. When inserting text, the character '\n' represents a line terminator.
pub struct TextBuffer {
    pub lines: Vec<String>,
    pub selections: HashMap<Handle<View>, Selections>,
    pub path: Option<String>,
    pub dirty: Cell<bool>, // Using Cell just to allow write_atomic and write_to_atomic to be non mut.
    pub history_dirty: Cell<bool>, // Used to prevent saving changes to the undo/redo stack when there is none.
}

impl TextBuffer {
    pub fn new_empty() -> Self {
        Self {
            lines: vec![String::new()], // Uphold #1.
            selections: Default::default(),
            path: None,
            dirty: Default::default(),
            history_dirty: Default::default(),
        }
    }

    pub fn new_from_path(path: &str) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|err| format!("can't read '{path}': {err}"))?;
        let lines = content.split('\n').map(str::to_string).collect();
        Ok(Self {
            lines,
            selections: Default::default(),
            path: Some(path.to_string()),
            dirty: Default::default(),
            history_dirty: Default::default(),
        })
    }

    /// Write the content of this buffer to its path.
    /// Returns an error if no path is set, or if an error happens while
    /// writing.
    /// The write operation is performed atomically.
    pub fn write(&self) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| "missing path".to_string())?;
        self.write_to(path)
    }

    /// Write the content of this buffer to the given path.
    /// The write operation is performed atomically.
    pub fn write_to(&self, path: &str) -> Result<(), String> {
        // Find unique name for tmp file.
        // Write to new tmp file with unique name.
        // Rename tmp file to intended name.

        let tmp_path = format!("{path}.ayed-tmp");
        let tmp_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .map_err(map_io_err)?;

        let mut buf_tmp_file = std::io::BufWriter::new(tmp_file);
        self.write_content(&mut buf_tmp_file).map_err(map_io_err)?;

        std::fs::rename(tmp_path, path).map_err(map_io_err)?;

        self.dirty.set(false);

        Ok(())
    }

    fn write_content<W: std::io::Write>(&self, w: &mut W) -> Result<(), std::io::Error> {
        for (i, line) in self.lines.iter().enumerate() {
            if i != 0 {
                w.write_all(&[b'\n'])?;
            }
            w.write_all(line.as_bytes())?;
        }
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    fn mark_dirty(&self) {
        self.dirty.set(true);
        self.history_dirty.set(true);
    }

    pub fn take_history_dirty(&self) -> bool {
        let d = self.history_dirty.get();
        self.history_dirty.set(false);
        d
    }

    pub fn add_view_selections(&mut self, view: Handle<View>, selections: Selections) {
        self.selections.insert(view, selections);
    }

    pub fn view_selections(&self, view: Handle<View>) -> Option<&Selections> {
        self.selections.get(&view)
    }

    pub fn view_selections_mut(&mut self, view: Handle<View>) -> Option<&mut Selections> {
        self.selections.get_mut(&view)
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_ref().map(String::as_str)
    }

    pub fn set_path<P: Into<String>>(&mut self, path: impl Into<Option<P>>) {
        self.path = path.into().map(Into::into);
    }

    pub fn line(&self, row_index: Row) -> Option<&str> {
        self.lines.get(row_index as usize).map(String::as_str)
    }

    pub fn line_mut(&mut self, row_index: Row) -> Option<&mut String> {
        self.lines.get_mut(row_index as usize)
    }

    pub fn first_line(&self) -> &str {
        self.lines.get(0).expect("TextBuffer invariant #1")
    }

    pub fn last_row(&self) -> Row {
        self.line_count().saturating_sub(1)
    }

    pub fn line_count(&self) -> Row {
        self.lines.len().try_into().unwrap()
    }

    pub fn line_char_count(&self, row: Row) -> Option<Row> {
        self.line(row)
            .map(|line| char_count(line).try_into().unwrap())
    }

    pub fn selection_text(&self, selection: &Selection) -> String {
        let mut text = String::new();
        for (i, line_sel) in selection.split_lines().enumerate() {
            if i != 0 {
                text.push('\n');
            }
            let sel = self.limit_selection_to_content(&line_sel);
            let line = self.line(sel.cursor().row).unwrap();
            let line_char_count = self.line_char_count(sel.cursor().row).unwrap();

            let start: usize = sel.start().column as _;
            let end: usize = sel.end().column as _;
            let ends_on_last_line = sel.end().row == self.last_row();

            if end == line_char_count as _ {
                text.push_str(&line[start..end]);
                if !ends_on_last_line {
                    text.push('\n');
                }
            } else {
                text.push_str(&line[start..=end]);
            }
        }
        text
    }

    pub fn selection_char_count(&self, selection: &Selection) -> usize {
        let start_row = selection.start().row;
        let start_column = selection.start().column;
        let end_row = selection.end().row;
        let end_column = selection.end().column;

        let mut char_count = 0;
        for row in start_row..=end_row {
            let begin_column = if row == start_row { start_column } else { 0 };
            let stop_column = if row == end_row {
                end_column
            } else {
                self.line_char_count(row).unwrap_or(begin_column)
            }
            .checked_add(1)
            .unwrap();
            let row_char_count: usize = (stop_column - begin_column).try_into().unwrap();
            char_count += row_char_count;
        }
        char_count
    }

    pub fn limit_selection_to_content(&self, selection: &Selection) -> Selection {
        let cursor = self.limit_position_to_content(selection.cursor());
        let anchor = self.limit_position_to_content(selection.anchor());
        selection
            .with_provisional_cursor(cursor)
            .with_provisional_anchor(anchor)
    }

    pub fn limit_position_to_content(&self, position: Position) -> Position {
        let row = position.row.clamp(0, self.last_row());
        let column = position
            .column
            .clamp(0, self.line_char_count(row).unwrap_or(0));
        Position::new(column, row)
    }

    pub fn move_position_horizontally(
        &self,
        position: Position,
        direction: i32,
    ) -> Option<Position> {
        let offset = Offset::new(direction.signum(), 0);
        let target_column = position.column as i64 + offset.column as i64;
        let position = if target_column < 0 {
            // Go to end of previous line.
            if position.row == 0 {
                return None;
            }
            let prev_line_row = position.row.saturating_sub(1);
            let column = self.line_char_count(prev_line_row).unwrap_or(0);
            Position::new(column, prev_line_row)
        } else if self
            .line_char_count(position.row)
            .is_some_and(|end_column| target_column > end_column as i64)
        {
            // Go to start of next line.
            if position.row == self.last_row() {
                return None;
            }
            let next_line_row = position.row.saturating_add(1);
            Position::new(0, next_line_row)
        } else {
            position.offset(offset)
        };
        Some(self.limit_position_to_content(position))
    }

    pub fn insert_str_at(&mut self, at: Position, s: &str) -> Result<Selection, String> {
        // TODO PERF maybe make this not one by one, if you ever feel like it.
        let mut cursor = at;
        let mut prior_cursor = at;
        for ch in s.chars() {
            prior_cursor = cursor;
            self.insert_char_at(cursor, ch)?;
            if ch == '\n' {
                cursor = Self::adjust_position_after_split_line(cursor, cursor);
            } else {
                cursor = Self::adjust_position_after_insert_char(cursor, cursor);
            }
        }
        Ok(Selection::new().with_anchor(at).with_cursor(prior_cursor))
    }

    pub fn insert_char_at(&mut self, at: Position, ch: char) -> Result<(), String> {
        if ch == '\n' {
            self.split_line(at)?;
        } else {
            let line = self
                .lines
                .get_mut(at.row as usize)
                .ok_or_else(|| format!("position out of bounds (bad row): {at:?}"))?;
            let at_idx = char_index_to_byte_index(&line, at.column.try_into().unwrap())
                .ok_or_else(|| format!("position out of bounds (bad column): {at:?}"))?;

            line.insert(at_idx, ch);

            self.adjust_selections_after_insert_char(at);
        }

        self.mark_dirty();

        Ok(())
    }

    pub fn split_line(&mut self, at: Position) -> Result<(), String> {
        let line = self
            .lines
            .get_mut(at.row as usize)
            .ok_or_else(|| format!("position out of bounds (bad row): {at:?}"))?;
        let at_idx = char_index_to_byte_index(&line, at.column.try_into().unwrap())
            .ok_or_else(|| format!("position out of bounds (bad column): {at:?}"))?;

        let rest = line.split_off(at_idx);
        self.lines.insert(at.row.saturating_add(1) as _, rest);

        self.adjust_selections_after_split_line(at);

        self.mark_dirty();

        Ok(())
    }

    pub fn delete_at(&mut self, at: Position) -> Result<(), String> {
        let line = self
            .line_mut(at.row)
            .ok_or_else(|| String::from("bad row"))?;
        let (idx, ch) = line
            .char_indices()
            .chain(Some((line.len(), '\n')))
            .nth(at.column as usize)
            .ok_or_else(|| String::from("bad column"))?;
        if ch == '\n' {
            let _ = self.join_line_with_next(at.row);
        } else {
            line.remove(idx);
            self.adjust_selections_after_delete_at(at);
        }

        self.mark_dirty();

        Ok(())
    }

    pub fn delete_selection(&mut self, selection: &Selection) -> Result<(), String> {
        // TODO PERF maybe make this not one by one, if you ever feel like it.
        for _ in 0..self.selection_char_count(selection) {
            self.delete_at(selection.start())?;
        }

        self.mark_dirty();

        Ok(())
    }

    pub fn join_line_with_next(&mut self, row: Row) -> Result<(), String> {
        if row > self.last_row() {
            return Err(String::from("bad row"));
        }

        let next_row = row.checked_add(1).unwrap();
        if next_row > self.last_row() {
            return Err(String::from("no next line to join"));
        }

        let next_line = self.lines.remove(next_row as usize);
        let line = self.line_mut(row).expect("verified above");
        let original_line_char_count = char_count(line);
        line.push_str(&next_line);

        self.adjust_selections_after_join_line_with_next(row, original_line_char_count);

        self.mark_dirty();

        Ok(())
    }

    fn adjust_selections_after_insert_char(&mut self, inserted_at: Position) {
        for selections in self.selections() {
            for selection in selections.iter_mut() {
                let cursor =
                    Self::adjust_position_after_insert_char(selection.cursor(), inserted_at);
                let anchor =
                    Self::adjust_position_after_insert_char(selection.anchor(), inserted_at);
                *selection = selection.with_anchor(anchor).with_cursor(cursor);
            }
        }
    }

    fn adjust_selections_after_split_line(&mut self, split_at: Position) {
        for selections in self.selections() {
            for selection in selections.iter_mut() {
                let cursor = Self::adjust_position_after_split_line(selection.cursor(), split_at);
                let anchor = Self::adjust_position_after_split_line(selection.anchor(), split_at);
                *selection = selection.with_anchor(anchor).with_cursor(cursor);
            }
        }
    }

    fn adjust_selections_after_delete_at(&mut self, deleted_at: Position) {
        for selections in self.selections() {
            for selection in selections.iter_mut() {
                let cursor = Self::adjust_position_after_delete_at(selection.cursor(), deleted_at);
                let anchor = Self::adjust_position_after_delete_at(selection.anchor(), deleted_at);
                *selection = selection.with_anchor(anchor).with_cursor(cursor);
            }
        }
    }

    fn adjust_selections_after_join_line_with_next(
        &mut self,
        row: Row,
        original_line_char_count: usize,
    ) {
        for selections in self.selections() {
            for selection in selections.iter_mut() {
                let cursor = Self::adjust_position_after_join_line_with_next(
                    selection.cursor(),
                    row,
                    original_line_char_count,
                );
                let anchor = Self::adjust_position_after_join_line_with_next(
                    selection.anchor(),
                    row,
                    original_line_char_count,
                );
                *selection = selection.with_anchor(anchor).with_cursor(cursor);
            }
        }
    }

    fn selections(&mut self) -> impl Iterator<Item = &mut Selections> {
        self.selections.values_mut()
    }

    fn adjust_position_after_insert_char(pos: Position, inserted_at: Position) -> Position {
        if pos < inserted_at {
            return pos;
        }

        if pos.row == inserted_at.row {
            return pos.offset((1, 0));
        }

        pos
    }

    fn adjust_position_after_split_line(pos: Position, split_at: Position) -> Position {
        if pos < split_at {
            return pos;
        }

        let row = pos.row.saturating_add(1);
        let column = if pos.row == split_at.row {
            pos.column.saturating_sub(split_at.column)
        } else {
            pos.column
        };

        Position::new(column, row)
    }

    fn adjust_position_after_delete_at(pos: Position, deleted_at: Position) -> Position {
        if pos <= deleted_at {
            return pos;
        }

        if pos.row == deleted_at.row {
            return pos.offset((-1, 0));
        }

        pos
    }

    fn adjust_position_after_join_line_with_next(
        pos: Position,
        row: Row,
        original_line_char_count: usize,
    ) -> Position {
        if pos.row <= row {
            pos
        } else if pos.row == row + 1 {
            let char_count: Column = original_line_char_count.try_into().unwrap();
            Position::new(pos.column + char_count, row)
        } else {
            pos.offset((0, -1))
        }
    }
}

fn map_io_err(err: std::io::Error) -> String {
    err.to_string()
}
