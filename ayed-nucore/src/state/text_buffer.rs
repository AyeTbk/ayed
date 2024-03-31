use crate::{
    position::Position,
    selection::{Selection, Selections},
    Ref, WeakRef,
};

// #1. There should always be at least one line. A line is a String in the lines vector.
// #2. The line terminators are not part of the content, they are implied for the
//     current line when there is a following line.
// #3. Positions refer to lines in row and to codepoints (Rust chars) of said line in column.
// #4. A position of column == line.chars().count() is allowed and represents the
//     position of the line terminator, iif a line terminator is implied for that line.
// #5. When inserting text, the line terminator character is '\n'.
pub struct TextBuffer {
    lines: Vec<String>,
    selections: Vec<WeakRef<Selections>>,
    path: Option<String>,
}

impl TextBuffer {
    pub fn new_empty() -> Self {
        Self {
            lines: vec![String::new()], // Uphold #1.
            selections: vec![],
            path: None,
        }
    }

    pub fn new_from_path(path: &str) -> Result<Self, String> {
        let content =
            std::fs::read_to_string(path).map_err(|err| format!("can't read '{path}': {err}"))?;
        let lines = content.split('\n').map(str::to_string).collect();
        Ok(Self {
            lines,
            selections: Vec::new(),
            path: Some(path.to_string()),
        })
    }

    pub fn add_selections(&mut self, selections: &Ref<Selections>) {
        self.selections.push(Ref::downgrade(selections));
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_ref().map(String::as_str)
    }

    pub fn line_count(&self) -> u32 {
        self.lines.len().try_into().unwrap()
    }

    pub fn line(&self, row_index: u32) -> Option<&str> {
        self.lines.get(row_index as usize).map(String::as_str)
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

    pub fn insert_char_at(&mut self, at: Position, ch: char) -> Result<(), String> {
        if ch == '\n' {
            self.split_line(at)?;
        } else {
            let line = self
                .lines
                .get_mut(at.row as usize)
                .ok_or_else(|| format!("position out of bounds: {at:?}"))?;
            let at_idx = char_index_to_byte_index(&line, at.column)
                .ok_or_else(|| format!("position out of bounds: {at:?}"))?;

            line.insert(at_idx, ch);

            self.adjust_selections_after_insert_char(at);
        }

        Ok(())
    }

    pub fn split_line(&mut self, at: Position) -> Result<(), String> {
        let line = self
            .lines
            .get_mut(at.row as usize)
            .ok_or_else(|| format!("position out of bounds: {at:?}"))?;
        let at_idx = char_index_to_byte_index(&line, at.column)
            .ok_or_else(|| format!("position out of bounds: {at:?}"))?;

        let rest = line.split_off(at_idx);
        self.lines.insert(at.row.saturating_add(1) as _, rest);

        self.adjust_selections_after_split_line(at);

        Ok(())
    }

    fn adjust_selections_after_insert_char(&mut self, inserted_at: Position) {
        for selections in self.selections() {
            for selection in selections.borrow_mut().iter_mut() {
                let cursor =
                    Self::adjust_position_after_insert_char(selection.cursor(), inserted_at);
                let anchor =
                    Self::adjust_position_after_insert_char(selection.anchor(), inserted_at);

                *selection = selection
                    .with_provisional_anchor(anchor)
                    .with_provisional_cursor(cursor);
            }
        }
    }

    fn adjust_selections_after_split_line(&mut self, split_at: Position) {
        for selections in self.selections() {
            for selection in selections.borrow_mut().iter_mut() {
                let cursor = Self::adjust_position_after_split_line(selection.cursor(), split_at);
                let anchor = Self::adjust_position_after_split_line(selection.anchor(), split_at);

                *selection = selection
                    .with_provisional_anchor(anchor)
                    .with_provisional_cursor(cursor);
            }
        }
    }

    fn selections(&mut self) -> Vec<Ref<Selections>> {
        let mut sels = Vec::new();
        let mut i = 0;
        while i < self.selections.len() {
            let weak = &self.selections[i];
            if let Some(strong) = WeakRef::upgrade(weak) {
                sels.push(strong);
                i += 1;
            } else {
                self.selections.remove(i);
            }
        }
        sels
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

    fn line_char_count(&self, row: u32) -> Option<u32> {
        self.line(row).map(|line| char_count(line))
    }

    fn last_line_index(&self) -> u32 {
        self.line_count().saturating_sub(1)
    }
}

fn char_index_to_byte_index(s: &str, ch_idx: u32) -> Option<usize> {
    if ch_idx == 0 {
        Some(0)
    } else {
        s.char_indices()
            .skip(ch_idx as _)
            .map(|(idx, _)| idx)
            .next()
    }
}

fn char_count(s: &str) -> u32 {
    s.chars().count().try_into().unwrap()
}
