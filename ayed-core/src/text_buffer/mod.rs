use std::io;

use crate::{
    selection::{Selection, Selections},
    utils::Position,
};

pub mod commands;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectionsId(usize);

// 1. There should always be at least one line. A line is a String in the lines vector.
// 2. The line terminators are not part of the content, they are implied for the
//     current line when there is a following line.
// 3. Positions refer to lines in row and to codepoints (Rust chars) of said line in column.
// 4. A position of column == line.chars().count() is allowed and represents the
//     position of the line terminator, iif a line terminator is implied for that line.
// 5. When inserting text, the line terminator character is '\n'.
pub struct TextBuffer {
    lines: Vec<String>,
    // There should always be at least one
    selections_sets: Vec<Selections>,
    filepath: Option<String>,
    modified: bool,
}

impl TextBuffer {
    pub fn new_empty() -> Self {
        Self {
            lines: vec![String::new()],
            selections_sets: vec![Selections::new()],
            filepath: None,
            modified: false,
        }
    }

    pub fn from_filepath(filepath: impl Into<String>) -> io::Result<Self> {
        let filepath = filepath.into();
        let content = std::fs::read_to_string(&filepath)?;
        let lines = content.lines().map(str::to_string).collect();
        Ok(Self {
            lines,
            selections_sets: vec![Selections::new()],
            filepath: Some(filepath),
            modified: true,
        })
    }

    pub fn save(&self) -> Option<io::Result<()>> {
        if let Some(filepath) = &self.filepath {
            let contents = self.lines.join("\n");
            Some(std::fs::write(filepath, contents))
        } else {
            None
        }
    }

    pub fn filepath(&self) -> Option<&str> {
        self.filepath.as_ref().map(String::as_str)
    }

    pub fn take_is_modified(&mut self) -> bool {
        let modified = self.modified;
        self.modified = false;
        modified
    }

    pub fn add_selections_set(&mut self) -> SelectionsId {
        let id = self.selections_sets.len();
        self.selections_sets.push(Selections::new());
        SelectionsId(id)
    }

    pub fn get_selections(&self, id: SelectionsId) -> &Selections {
        &self.selections_sets[id.0]
    }

    pub fn get_selections_mut(&mut self, id: SelectionsId) -> &mut Selections {
        &mut self.selections_sets[id.0]
    }

    pub fn get_selection(&self, set_id: SelectionsId, index: usize) -> Option<Selection> {
        self.selections_sets[set_id.0].get(index)
    }

    pub fn get_selection_mut(
        &mut self,
        set_id: SelectionsId,
        index: usize,
    ) -> Option<&mut Selection> {
        self.selections_sets[set_id.0].get_mut(index)
    }

    pub fn insert(&mut self, id: SelectionsId, text: &str) {
        for selection in self.selections_sets[id.0].clone().iter() {
            self.insert_at(selection.cursor(), text);
        }
    }

    pub fn insert_char(&mut self, id: SelectionsId, chr: char) {
        let mut buf = [0u8; 4];
        chr.encode_utf8(&mut buf);
        self.insert(id, std::str::from_utf8(&buf[..chr.len_utf8()]).unwrap())
    }

    pub fn insert_char_at(&mut self, pos: Position, chr: char) {
        let mut buf = [0u8; 4];
        chr.encode_utf8(&mut buf);
        self.insert_at(pos, std::str::from_utf8(&buf[..chr.len_utf8()]).unwrap())
    }

    pub fn insert_at(&mut self, pos: Position, text: &str) {
        let (insert_index, _) = self.position_as_indices(pos).unwrap();
        let text_lines: Vec<_> = text.split('\n').collect();

        let mut to_column = pos.column;
        let mut to_row = pos.row;

        match &text_lines[..] {
            [] => { /* Do nothing */ }
            [text] => {
                let line = self.line_mut(pos.row).unwrap();
                line.insert_str(insert_index, text);
                to_column = to_column.saturating_add(char_count(text));
            }
            [first, inner_text_lines @ .., last] => {
                let line = self.line_mut(pos.row).unwrap();
                let end = line.split_off(insert_index);

                line.push_str(first);

                let mut insert_line_index = pos.row + 1;
                to_row += 1;
                for &inner_text_line in inner_text_lines {
                    self.insert_line(insert_line_index, inner_text_line);
                    insert_line_index += 1;
                    to_row += 1;
                }

                let last_line = format!("{last}{end}");
                to_column = 0; //char_count(&last_line); // Idk what im doing just work plz
                self.insert_line(insert_line_index, last_line);
            }
        }

        let (from, to) = (pos, Position::new(to_column, to_row));
        self.displace_selections(from, to, None, EditKind::Insert);
        self.modified = true;

        self.make_selections_desired();
    }

    pub fn delete(&mut self, id: SelectionsId) {
        for selection in self.selections_sets[id.0].clone().iter() {
            self.delete_selection(*selection);
        }
    }

    pub fn delete_selection(&mut self, selection: Selection) {
        let start = selection.start();
        let end = selection.end();

        let (first_line_delete_start, _) = self.position_as_indices(start).unwrap();
        let (last_line_delete_end, _) = self.position_as_indices(end.offset((1, 0))).unwrap();
        let join_next_line = end.column >= self.line_char_count(end.row).unwrap();

        if start.row == end.row {
            let line = self.line_mut(start.row).unwrap();
            line.drain(first_line_delete_start..last_line_delete_end);
        } else {
            let mut last_line = std::mem::take(self.line_mut(end.row).unwrap());
            last_line.drain(..last_line_delete_end);

            let first_line = self.line_mut(start.row).unwrap();
            first_line.drain(first_line_delete_start..);
            first_line.push_str(&last_line);

            self.lines
                .drain((start.row as usize + 1)..=(end.row as usize));
        }

        let mut join_next_line_fix = None;
        if join_next_line {
            let next_line_row = start.row.saturating_add(1);
            if next_line_row <= self.last_line_index() {
                let next_line = self.lines.remove(next_line_row as usize);
                join_next_line_fix = Some(self.line_char_count(start.row).unwrap());
                let first_line = self.line_mut(start.row).unwrap();
                first_line.push_str(&next_line);
            }
        }

        self.displace_selections(start, end, join_next_line_fix, EditKind::Delete);
        self.modified = true;

        self.make_selections_desired();
    }

    pub fn displace_selections(
        &mut self,
        from: Position,
        to: Position,
        join_next_line_fix: Option<u32>,
        edit: EditKind,
    ) {
        for selections in &mut self.selections_sets {
            for selection in selections.iter_mut() {
                *selection =
                    Self::displace_selection(selection, from, to, join_next_line_fix, edit);
            }
        }

        self.merge_overlapping_selections();
    }

    pub fn displace_selection(
        selection: &Selection,
        from: Position,
        to: Position,
        join_next_line_fix: Option<u32>,
        edit: EditKind,
    ) -> Selection {
        let scursor = selection.cursor();
        let sanchor = selection.anchor();
        let cursor = Self::displace_position_from_edit(scursor, from, to, join_next_line_fix, edit);
        let anchor = Self::displace_position_from_edit(sanchor, from, to, join_next_line_fix, edit);
        selection
            .with_provisional_cursor(cursor)
            .with_provisional_anchor(anchor)
    }

    pub fn displace_position_from_edit(
        position: Position,
        from: Position,
        to: Position,
        join_next_line_fix: Option<u32>,
        edit: EditKind,
    ) -> Position {
        if from > to {
            // Nonsensical request, just return the position
            return position;
        }

        let row_diff = to.row.saturating_sub(from.row);
        let column_diff = (to.column as i64) - (from.column as i64);

        match edit {
            EditKind::Insert => {
                if position < from {
                    position
                } else {
                    let row = position.row.saturating_add(row_diff);
                    let column = if position.row == from.row {
                        let i64_column = position.column as i64 + column_diff;
                        i64_column.clamp(u32::MIN as _, u32::MAX as _) as u32
                    } else {
                        position.column
                    };
                    Position::new(column, row)
                }
            }
            EditKind::Delete => {
                if from <= position && position <= to {
                    from
                } else if position > to {
                    if position.row == to.row {
                        let i64_column = position.column as i64 - column_diff - 1;
                        let column = i64_column.clamp(u32::MIN as _, u32::MAX as _) as u32;
                        Position::new(column, from.row)
                    } else {
                        match join_next_line_fix {
                            Some(to_row_original_char_count)
                                if position.row == to.row.saturating_add(1) =>
                            {
                                Position::new(to_row_original_char_count, to.row)
                            }
                            _ => {
                                let row = position.row.saturating_sub(row_diff);
                                Position::new(position.column, row)
                            }
                        }
                    }
                } else {
                    position
                }
            }
        }
    }

    pub fn make_selections_desired(&mut self) {
        for selections in self.selections_sets.iter_mut() {
            for selection in selections.iter_mut() {
                *selection = selection.to_desired();
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
        let row = position.row.clamp(0, self.last_line_index());
        let column = position
            .column
            .clamp(0, self.line_char_count(row).unwrap_or(0));
        Position::new(column, row)
    }

    pub fn move_position_up(&self, position: Position) -> Position {
        let new_position = if position.row == 0 {
            position
        } else {
            position.offset((0, -1))
        };
        return self.limit_position_to_content(new_position);
    }

    pub fn move_position_down(&self, position: Position) -> Position {
        let new_position = if position.row < self.last_line_index() {
            position.offset((0, 1))
        } else {
            position
        };
        return self.limit_position_to_content(new_position);
    }

    pub fn move_position_left(&self, position: Position) -> Position {
        let mut new_column = position.column;
        let mut new_row = position.row;
        if position.column == 0 {
            if new_row == 0 {
                // This means position is at the buffer start, can't move it left.
                return position;
            }
            new_row -= 1;
            let new_row_line_char_count = self.line_char_count(new_row).unwrap_or(0);
            new_column = new_row_line_char_count;
        } else {
            new_column -= 1;
        }
        Position::new(new_column, new_row)
    }

    pub fn move_position_right(&self, position: Position) -> Position {
        let mut new_column = position.column.saturating_add(1);
        let mut new_row = position.row;
        let line_char_count = self.line_char_count(new_row).unwrap_or(0);
        if new_column > line_char_count {
            if new_row == self.last_line_index() {
                new_column = line_char_count;
            } else {
                new_column = 0;
                new_row = u32::min(new_row.saturating_add(1), self.last_line_index());
            }
        }
        Position::new(new_column, new_row)
    }

    pub fn start_of_content_position(&self) -> Position {
        Position::ZERO
    }

    pub fn end_of_content_position(&self) -> Position {
        let row = self.last_line_index();
        let column = self.line_char_count(row).unwrap_or(0);
        Position::new(column, row)
    }

    pub fn is_empty(&self) -> bool {
        self.line_char_count(0)
            .map(|count| count == 0)
            .unwrap_or(true)
    }

    pub fn line_count(&self) -> u32 {
        self.lines.len() as u32
    }

    pub fn line(&self, row: u32) -> Option<&str> {
        self.lines.get(row as usize).map(String::as_str)
    }

    pub fn line_mut(&mut self, row: u32) -> Option<&mut String> {
        self.modified = true;
        self.lines.get_mut(row as usize)
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    pub fn line_char_count(&self, row: u32) -> Option<u32> {
        self.line(row).map(|line| char_count(line))
    }

    pub fn last_line_index(&self) -> u32 {
        self.line_count().saturating_sub(1)
    }

    /// Returns the byte index of the column and the line index of the row of
    /// the given Position, if it is within the content of the buffer.
    pub fn position_as_indices(&self, pos: Position) -> Option<(usize, usize)> {
        let line = self.line(pos.row)?;
        let column_byte_idx = line
            .char_indices()
            .skip(pos.column as usize)
            .map(|(idx, _)| idx)
            .next()
            .unwrap_or_else(|| line.len());
        Some((column_byte_idx, pos.row as usize))
    }

    pub fn merge_overlapping_selections(&mut self) {
        for selections in self.selections_sets.iter_mut() {
            *selections = selections.overlapping_selections_merged();
        }
    }

    fn insert_line(&mut self, line_index: u32, string: impl Into<String>) {
        self.lines.insert(line_index as usize, string.into());
    }
}

pub fn char_count(string: &str) -> u32 {
    string.chars().count().try_into().unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Insert,
    Delete,
}

pub fn first_non_whitespace_column_of_line(buffer: &TextBuffer, row: u32) -> Option<u32> {
    buffer.line(row).and_then(|line| {
        line.char_indices().find_map(|(i, ch)| {
            if !ch.is_ascii_whitespace() {
                Some(i as u32)
            } else {
                None
            }
        })
    })
}
