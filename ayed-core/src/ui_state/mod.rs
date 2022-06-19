use crate::selection::Position;

pub struct UiState {
    pub panels: Vec<Panel>,
}

pub struct Panel {
    pub position: (u32, u32),
    pub size: (u32, u32),
    pub content: Vec<String>,
    pub spans: Vec<Span>,
}

impl Panel {
    /// Modify span list so that none are overlapping.
    pub fn normalize_spans(&mut self) {
        todo!()
    }

    pub fn spans_on_line(&self, line_index: u32) -> impl Iterator<Item = &Span> {
        self.spans
            .iter()
            .filter(move |span| span.from.line_index == line_index)
    }
}

pub struct Span {
    pub from: Position,
    pub to: Position,
    pub style: Style,
    pub importance: u8,
}

pub struct Style {
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub invert: bool,
}

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };
}
