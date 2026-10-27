use crate::{Range, position::PPosition};

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub text: String,
}

impl TextEdit {
    pub fn insert_str_at(s: impl Into<String>, at: PPosition) -> Self {
        Self {
            range: Range::new(at, at),
            text: s.into(),
        }
    }

    pub fn insert_char_at(ch: char, at: PPosition) -> Self {
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

    pub fn delete_at(at: PPosition) -> Self {
        Self::delete_range(Range::new(at, at.offset((1, 0))))
    }

    pub fn is_empty(&self) -> bool {
        self.range.is_empty() && self.text.is_empty()
    }
}
