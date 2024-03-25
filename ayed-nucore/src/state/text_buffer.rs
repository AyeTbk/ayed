use crate::{selection::Selections, Ref, WeakRef};

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
        let content = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
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
}
