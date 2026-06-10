use std::{
    cell::Cell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    config::Config,
    position::{Column, Offset, Position, Row},
    range::Range,
    selection::{Selection, Selections},
    slotmap::Handle,
    state::TextBufferHistory,
    utils::string_utils::{
        byte_index_to_char_index, char_count, char_index_to_byte_index,
        char_index_to_byte_index_end,
    },
};

use super::View;

// #1. There should always be at least one line. A line is a String in the lines vector.
// #2. The line terminators are not part of the content, they are implied for the
//     current line when there is a following line.
// #3. Positions refer to lines in row and to codepoints (Rust chars) of said line in column.
// #4. A position with  column == line's char count  is allowed. It can be thought of as the
//     position of the line terminator (also allowed for the last line even though there is
//     no implied line terminator).
// #5. For general processing, line terminators are represented by a linefeed '\n'.
pub struct TextBuffer {
    pub lines: Vec<String>,
    pub selections: HashMap<Handle<View>, Selections>,
    pub path: Option<PathBuf>,
    pub dirty: Cell<bool>, // Using Cell just to allow write_atomic and write_to_atomic to be non mut.

    pub history: TextBufferHistory,

    /// Version for the buffer's content. Must increment for every change, including undos. For LSP.
    pub content_version: Cell<i32>,
    /// File format the user set.
    pub forced_format: Option<String>,
}

impl TextBuffer {
    pub fn new_empty() -> Self {
        Self {
            lines: vec![String::new()], // Uphold #1.
            selections: Default::default(),
            path: None,
            dirty: Default::default(),
            history: Default::default(),
            content_version: Default::default(),
            forced_format: None,
        }
    }

    pub fn new_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let content =
            std::fs::read_to_string(path).map_err(|err| format!("can't read '{path:?}': {err}"))?;
        let lines = content.split('\n').map(str::to_string).collect();
        Ok(Self {
            lines,
            selections: Default::default(),
            path: Some(path.to_path_buf()),
            dirty: Default::default(),
            history: Default::default(),
            content_version: Default::default(),
            forced_format: None,
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
    fn write_to(&self, path: &Path) -> Result<(), String> {
        // Find unique name for tmp file.  (// TODO !)
        // Write to new tmp file with unique name.
        // Rename tmp file to intended name.

        let tmp_path = path.with_added_extension(".ayed-tmp");
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

    pub fn content_to_string(&self) -> String {
        let mut buf = Vec::new();
        self.write_content(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
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

    pub fn content_version(&self) -> i32 {
        self.content_version.get()
    }

    fn mark_dirty(&self) {
        self.dirty.set(true);
        self.content_version.update(|n| n + 1);
    }

    #[deprecated = "use set_view_selections"]
    pub fn add_view_selections(&mut self, view: Handle<View>, selections: Selections) {
        self.selections.insert(view, selections);
    }

    pub fn view_selections(&self, view: Handle<View>) -> Option<&Selections> {
        self.selections.get(&view)
    }

    #[deprecated = "use set_view_selections"]
    pub fn view_selections_mut(&mut self, view: Handle<View>) -> Option<&mut Selections> {
        self.selections.get_mut(&view)
    }

    pub fn set_view_selections(
        &mut self,
        view: Handle<View>,
        selections: Selections,
    ) -> Option<Selections> {
        let val = self.selections.insert(view, selections);
        val
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref().map(PathBuf::as_path)
    }

    pub fn set_path(&mut self, path: impl Into<Option<PathBuf>>) {
        self.path = path.into();
    }

    pub fn path_str(&self) -> &str {
        self.path().and_then(|p| p.to_str()).unwrap_or("")
    }

    // I'd document this properly if I knew I to put words together to describe it
    // but basically this is to handle how to display tabs.
    // All code that wants to display a line should use this.
    pub fn logical_line(&self, row_index: Row, config: &Config) -> Option<String> {
        self.lines
            .get(row_index as usize)
            .map(|s| s.replace('\t', &" ".repeat(config.get_editor().indent_size as usize)))
    }

    pub fn logical_line_char_count(&self, row: Row, config: &Config) -> Option<i32> {
        let line = self.line(row)?;
        let mut logical_char_count = 0;
        for ch in line.chars() {
            let count = logical_char_char_count(ch, config);
            logical_char_count += count;
        }
        Some(logical_char_count)
    }

    pub fn map_true_position_to_logical_position(
        &self,
        position: Position,
        config: &Config,
    ) -> Position {
        let Some(line) = self.line(position.row) else {
            return position;
        };
        let mut logical_column = 0;
        let chars = line.chars().chain(Some('\n'));
        for ch in chars.take(position.column as usize) {
            let count = logical_char_char_count(ch, config);
            logical_column += count;
        }
        position.with_column(logical_column)
    }

    pub fn map_logical_position_to_true_position(
        &self,
        logpos: Position,
        config: &Config,
    ) -> Position {
        let Some(line) = self.line(logpos.row) else {
            return logpos;
        };
        let mut logical_char_count = 0;
        let mut char_count: i32 = 0;
        for ch in line.chars() {
            let count = logical_char_char_count(ch, config);
            logical_char_count += count;
            if logical_char_count > logpos.column {
                break;
            }
            char_count += 1;
        }
        logpos.with_column(char_count)
    }

    /// Maps byte index into the buffer to a position.
    pub fn map_byte_index_to_position(&self, idx: usize, byte_index_end: bool) -> Option<Position> {
        let mut line_start_bytes = 0;
        let mut row = 0;
        while row < self.line_count() {
            let line_terminator_size = 1;
            let line = self.line(row).unwrap();
            let line_end_bytes = line_start_bytes + line.len() + line_terminator_size;
            if line_end_bytes <= idx {
                line_start_bytes = line_end_bytes;
                row += 1;
                continue;
            }
            let mut byte_idx = idx - line_start_bytes;
            if byte_index_end {
                byte_idx = byte_idx.saturating_sub(1);
            }
            let column = byte_index_to_char_index(line, byte_idx).unwrap();
            return Some(Position::new(column as Column, row));
        }
        None
    }

    pub fn map_position_to_byte_index(&self, pos: Position) -> Option<usize> {
        if !(pos.row < self.line_count()) {
            return None;
        }

        let line_terminator_size = 1;
        let mut bytes = self
            .lines
            .iter()
            .map(|s| s.len() + line_terminator_size)
            .take(pos.row as usize)
            .sum();

        let line = self.line(pos.row)?;
        let more_bytes = char_index_to_byte_index(line, pos.column as usize)?;
        bytes += more_bytes;

        Some(bytes)
    }

    pub fn line(&self, row: Row) -> Option<&str> {
        self.lines.get(row as usize).map(String::as_str)
    }

    pub fn set_line(&mut self, row_index: Row, new_content: String) -> Result<String, ()> {
        // FIXME check that the line upholds the invariants
        if let Some(line) = self.lines.get_mut(row_index as usize) {
            let old_content = std::mem::take(line);
            *line = new_content;
            self.mark_dirty();
            Ok(old_content)
        } else {
            Err(())
        }
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

    pub fn line_char_count(&self, row: Row) -> Option<Column> {
        self.line(row)
            .map(|line| char_count(line).try_into().unwrap())
    }

    // TODO implement a selection_char_iter() and then reimplement
    //          this and selection_char_count with it.
    pub fn selection_text(&self, selection: &Selection) -> Option<String> {
        let mut text = String::new();

        for line_sel in selection.split_lines() {
            let sel = self.limit_selection_to_content(&line_sel);
            let line = self.line(sel.cursor.row).unwrap();
            let line_char_count = self.line_char_count(sel.cursor.row).unwrap();

            let start_byte: usize = char_index_to_byte_index(line, sel.start().column as _)?;
            let end = sel.end().column;
            let end_byte: usize = char_index_to_byte_index_end(line, end as _)?;
            let end_start_byte: usize = char_index_to_byte_index(line, end as _)?;
            let ends_on_last_line = sel.end().row == self.last_row();

            if end >= line_char_count {
                text.push_str(&line[start_byte..end_start_byte]);
                if !ends_on_last_line {
                    text.push('\n');
                }
            } else {
                text.push_str(&line[start_byte..end_byte]);
            }
        }
        Some(text)
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
        let cursor = self.limit_position_to_content(selection.cursor);
        let anchor = self.limit_position_to_content(selection.anchor);
        selection.with_cursor(cursor).with_anchor(anchor)
    }

    pub fn limit_position_to_content(&self, position: Position) -> Position {
        let row = position.row.clamp(0, self.last_row());
        let column = position
            .column
            .clamp(0, self.line_char_count(row).unwrap_or(0));
        Position::new(column, row)
    }

    pub fn limit_range_to_content(&self, range: Range) -> Range {
        fn limit_range_edge_to_content(this: &TextBuffer, position: Position) -> Position {
            let row = position.row.clamp(0, this.last_row());
            let column = position
                .column
                .clamp(0, this.line_char_count(row).unwrap_or(0) + 1);
            Position::new(column, row)
        }

        let start = limit_range_edge_to_content(self, range.start);
        let end = limit_range_edge_to_content(self, range.end);
        (start, end).into()
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

    pub fn move_logical_position_vertically(
        &self,
        logpos: Position,
        direction: i32,
        config: &Config,
    ) -> Option<Position> {
        let offset = Offset::new(0, direction.signum());
        let moved_logpos = logpos.offset(offset);
        if moved_logpos.row < 0 || moved_logpos.row > self.last_row() {
            return None;
        }
        let correct_row = moved_logpos.row;
        let correct_column = moved_logpos.column.clamp(
            0,
            self.logical_line_char_count(correct_row, config)
                .unwrap_or(0),
        );

        let correct_logpos = Position::new(correct_column, correct_row);

        Some(correct_logpos)
    }

    /// Applies the edit and returns the inverse edit that would undo the edit.
    pub fn apply_edit(&mut self, edit: &TextEdit) -> Result<TextEdit, String> {
        let selections_backup = self.selections.clone();

        let original_text = self.range_text(edit.range)?;
        let mut inverse_edit = TextEdit {
            range: (edit.range.start, edit.range.start).into(),
            text: original_text,
        };

        let should_delete = edit.range.end != edit.range.start;
        if should_delete {
            self.delete_range(edit.range)?;
        }

        let should_insert = !edit.text.is_empty();
        if should_insert {
            let insert_range = self.insert_str(edit.range.end, &edit.text)?;
            inverse_edit.range = insert_range;
        }

        self.history
            .record_edit(inverse_edit.clone(), &selections_backup, &self.selections);

        Ok(inverse_edit)
    }

    /// Dont use directly. Use TextEdit and `.apply_edit(...)` .
    fn insert_str(&mut self, at: Position, s: &str) -> Result<Range, String> {
        let at = self.limit_position_to_content(at);

        let row_idx = at.row as usize;
        let dst_line = std::mem::take(&mut self.lines[row_idx]);
        let dst_line_split_idx = char_index_to_byte_index(&dst_line, at.column as _).unwrap();
        let dst_line_half1 = &dst_line[..dst_line_split_idx];
        let dst_line_half2 = &dst_line[dst_line_split_idx..];

        let lines_count = s.split('\n').count();
        let lines = s.split('\n');

        self.lines
            .splice(row_idx..=row_idx, lines.map(str::to_string));

        let splice_first_line_idx = row_idx;
        let splice_last_line_idx = row_idx + (lines_count - 1);

        self.lines[splice_first_line_idx].insert_str(0, dst_line_half1);
        let range_end_column = char_count(&self.lines[splice_last_line_idx]);
        self.lines[splice_last_line_idx].push_str(dst_line_half2);

        self.mark_dirty();

        let range_row_diff = (lines_count as i32) - 1;
        let range_end = Position::new(range_end_column as i32, at.row + range_row_diff);
        let mut range = (at, range_end).into();
        self.adjust_selections_after_insert_str(range);

        // A range that ends at column position 0 is the same as one that
        // ends past the line separator of the previous line, but that
        // second case is the correct way to form a selection from the range.
        if range.end.column == 0 && range.end.row != 0 {
            let prev_row = range.end.row - 1;
            let prev_line_last_column = self.line_char_count(prev_row).unwrap_or_default();
            range.end = Position::new(prev_line_last_column + 1, prev_row);
        }

        Ok(range)
    }

    fn adjust_selections_after_insert_str(&mut self, insert_range: Range) {
        for selections in self.selections_mut() {
            for selection in selections.iter_mut() {
                let cursor = Self::adjust_position_after_insert_str(selection.cursor, insert_range);
                let anchor = Self::adjust_position_after_insert_str(selection.anchor, insert_range);
                *selection = selection.with_anchor(anchor).with_cursor(cursor);
            }
        }
    }

    fn adjust_position_after_insert_str(pos: Position, insert_range: Range) -> Position {
        if pos < insert_range.start {
            return pos;
        }
        let row_diff = insert_range.end.row - insert_range.start.row;
        let row = pos.row + row_diff;
        let column;
        if pos.row == insert_range.start.row {
            let column_diff = pos.column - insert_range.start.column;
            column = insert_range.end.column + column_diff;
        } else {
            column = pos.column;
        }
        Position { column, row }
    }

    /// Dont use directly. Use TextEdit and `.apply_edit(...)` .
    fn delete_range(&mut self, range: Range) -> Result<(), String> {
        let mut range = self.limit_range_to_content(range).normalized();

        // Check if there is actual work to be done.
        if range.is_empty() {
            return Ok(());
        }

        if self.line_char_count(range.end.row).unwrap() < range.end.column {
            if range.end.row != self.last_row() {
                // Fixup the range to properly handle a line terminator at the end of it
                range.end = Position::new(0, range.end.row + 1);
            } else {
                // Prevent trying to deleting past the end of the buffer by collapsing the range.
                range.end.column -= 1;
            }
        }

        let mut line_left = String::new();
        {
            let row_range = range.start.row as usize..=range.end.row as usize;
            let mut range_lines = self.lines.splice(row_range, [String::new()]);
            let first_range_line = range_lines.next().unwrap();
            let last_range_line_owned = range_lines.next_back();
            let last_range_line = last_range_line_owned.as_ref().unwrap_or(&first_range_line);

            let first_range_line_end_idx =
                char_index_to_byte_index(&first_range_line, range.start.column as usize).unwrap();
            let last_range_line_start_idx =
                char_index_to_byte_index(&last_range_line, range.end.column as usize).unwrap();

            line_left.push_str(&first_range_line[..first_range_line_end_idx]);
            line_left.push_str(&last_range_line[last_range_line_start_idx..]);
        }

        std::mem::swap(&mut line_left, &mut self.lines[range.start.row as usize]);

        self.mark_dirty();

        self.adjust_selections_after_delete_range(range);

        Ok(())
    }

    fn adjust_selections_after_delete_range(&mut self, delete_range: Range) {
        for selections in self.selections_mut() {
            for selection in selections.iter_mut() {
                let cursor =
                    Self::adjust_position_after_delete_range(selection.cursor, delete_range);
                let anchor =
                    Self::adjust_position_after_delete_range(selection.anchor, delete_range);
                *selection = selection.with_anchor(anchor).with_cursor(cursor);
            }
        }
    }

    fn adjust_position_after_delete_range(pos: Position, delete_range: Range) -> Position {
        if pos < delete_range.start {
            return pos;
        }
        if delete_range.contains(pos) {
            return delete_range.start;
        }
        let row_diff = delete_range.end.row - delete_range.start.row;
        let row = pos.row - row_diff;
        let column;
        if pos.row == delete_range.end.row {
            let column_diff = pos.column - delete_range.end.column;
            column = delete_range.start.column + column_diff;
        } else {
            column = pos.column;
        }
        Position { column, row }
    }

    pub fn range_text(&mut self, range: Range) -> Result<String, String> {
        let range = range.normalized();
        if range.is_empty() {
            return Ok(String::new());
        }
        let sel = Selection::new().with_start_and_end(range.start, range.end.offset((-1, 0)));
        if let Some(s) = self.selection_text(&sel) {
            Ok(s)
        } else {
            Ok(String::new())
        }
    }

    pub fn insert_str_at<S>(&mut self, s: S, at: Position) -> Result<Selection, String>
    where
        S: Into<String>,
    {
        let revedit = self.apply_edit(&TextEdit::insert_str_at(s, at))?;
        Ok(Selection::from_range(revedit.range))
    }

    pub fn insert_char_at(&mut self, ch: char, at: Position) -> Result<(), String> {
        self.apply_edit(&TextEdit::insert_char_at(ch, at))?;
        Ok(())
    }

    pub fn delete_selection(&mut self, selection: &Selection) -> Result<(), String> {
        self.apply_edit(&TextEdit::delete_selection(selection))?;
        Ok(())
    }

    pub fn delete_at(&mut self, at: Position) -> Result<(), String> {
        self.apply_edit(&TextEdit::delete_at(at))?;
        Ok(())
    }

    pub fn end_position(&self) -> Position {
        let row = self.last_row();
        let column = self
            .line_char_count(row)
            .expect("last_row should gives correct row");
        Position::new(column, row)
    }

    pub fn undo(&mut self) -> Result<bool, String> {
        if !self.history.can_undo() {
            return Ok(false);
        }

        let mut history = std::mem::take(&mut self.history);
        history.record_checkpoint();

        let edit_group = history.current_edit_group_mut().expect("can undo");
        for edit in edit_group.edits.iter_mut().rev() {
            let revedit = self.apply_edit(edit)?; // FIXME if this ? happends, history wont be reassigned to the buffer.
            *edit = revedit;
        }
        self.selections = edit_group.selections_before.clone();
        history.current_group_edge_idx = history.current_group_edge_idx.saturating_sub(1);

        self.history = history;

        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, String> {
        if !self.history.can_redo() {
            return Ok(false);
        }

        let mut history = std::mem::take(&mut self.history);

        history.current_group_edge_idx = history.current_group_edge_idx + 1;

        let edit_group = history.current_edit_group_mut().expect("can redo");
        for revedit in edit_group.edits.iter_mut() {
            let edit = self.apply_edit(revedit)?; // FIXME if this ? happends, history wont be reassigned to the buffer.
            *revedit = edit;
        }
        self.selections = edit_group.selections_after.clone();

        self.history = history;

        Ok(true)
    }

    pub fn checkpoint(&mut self) {
        self.history.record_checkpoint();
    }

    fn selections_mut(&mut self) -> impl Iterator<Item = &mut Selections> {
        self.selections.values_mut()
    }
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub text: String,
}

impl TextEdit {
    pub fn insert_str_at(s: impl Into<String>, at: Position) -> Self {
        Self {
            range: Range::from(at),
            text: s.into(),
        }
    }

    pub fn insert_char_at(ch: char, at: Position) -> Self {
        let mut buf = [0u8; char::MAX_LEN_UTF8];
        let s = ch.encode_utf8(&mut buf);
        Self::insert_str_at(s, at)
    }

    pub fn delete_range(range: Range) -> Self {
        Self {
            range,
            text: String::new(),
        }
    }

    pub fn delete_selection(selection: &Selection) -> Self {
        Self::delete_range(selection.to_range())
    }

    pub fn delete_at(at: Position) -> Self {
        Self::delete_range((at, at.offset((1, 0))).into())
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty() && self.text.is_empty()
    }
}

fn logical_char_char_count(ch: char, config: &Config) -> i32 {
    if ch == '\t' {
        config.get_editor().indent_size
    } else {
        1
    }
}

fn map_io_err(err: std::io::Error) -> String {
    err.to_string()
}
