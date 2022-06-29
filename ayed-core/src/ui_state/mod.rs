use crate::selection::Position;

pub struct UiState {
    pub panels: Vec<UiPanel>,
}

pub struct UiPanel {
    pub position: (u32, u32),
    pub size: (u32, u32),
    pub content: Vec<String>,
    pub spans: Vec<Span>,
}

impl UiPanel {
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

#[derive(Default, Clone, Copy)]
pub struct Style {
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub invert: bool,
}

impl Style {
    pub fn with_foreground_color(&self, color: Color) -> Self {
        let mut this = *self;
        this.foreground_color = Some(color);
        this
    }
}

#[derive(Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Self { r, g, b }
    }

    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
}
