use crate::{buffer::TextBuffer, ui_state::Style, utils::Position};

#[derive(Debug, Default, Clone)]
pub struct Highlight {
    pub position: HighlightPosition,
    pub style: Style,
    pub importance: u8,
}

#[derive(Debug, Clone)]
pub enum HighlightPosition {
    Panel { from: Position, to: Position },
    Content { from: Position, to: Position },
}

impl Default for HighlightPosition {
    fn default() -> Self {
        Self::Panel {
            from: Default::default(),
            to: Default::default(),
        }
    }
}

pub fn make_some_kind_of_highlights(_buffer: &TextBuffer) -> Vec<Highlight> {
    vec![Highlight {
        position: HighlightPosition::Content {
            from: Position::ZERO.offset((3, 5)),
            to: Position::ZERO.offset((5, 5)),
        },
        style: Style {
            underlined: true,
            ..Default::default()
        },
        importance: 10,
    }]
}
