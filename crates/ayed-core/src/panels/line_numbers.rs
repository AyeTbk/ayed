use crate::{
    ui_state::{Color, Span, Style, UiPanel},
    utils::{Position, Rect},
};

pub struct LineNumbers {
    rect: Rect,
    total_line_count: u32,
    start_line: u32,
}

impl LineNumbers {
    pub fn new() -> Self {
        Self {
            rect: Rect::new(0, 0, 2, 2),
            start_line: 0,
            total_line_count: 1,
        }
    }

    pub fn set_line_data(&mut self, total_line_count: u32, start_line: u32) {
        self.total_line_count = total_line_count;
        self.start_line = start_line;
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn needed_width(&self) -> u32 {
        self.numbers_width() + self.right_padding_width()
    }

    pub fn numbers_width(&self) -> u32 {
        let line_count_log = (self.total_line_count as f32).log10() as u32;
        line_count_log + 1
    }

    pub fn right_padding_width(&self) -> u32 {
        2
    }

    pub fn render(&self) -> UiPanel {
        let mut content = Vec::new();
        let mut spans = Vec::new();

        for i in 0..self.rect.height {
            let line_no = i + self.start_line + 1;
            let content_line = if line_no <= self.total_line_count {
                let line_no_str = line_no.to_string();
                let padding_len = (self.numbers_width() as usize).saturating_sub(line_no_str.len());
                let padding_str = " ".repeat(padding_len);
                let right_padding = " ".repeat(self.right_padding_width() as usize);
                format!("{padding_str}{line_no_str}{right_padding}")
            } else {
                " ".repeat(self.needed_width() as usize)
            };
            spans.push(Span {
                from: Position::new(0, i),
                to: Position::new(content_line.len() as u32, i),
                style: Style {
                    foreground_color: Some(Color::rgb(127, 127, 127)),
                    background_color: None,
                    ..Default::default()
                },
                ..Default::default()
            });
            content.push(content_line);
        }

        UiPanel {
            position: self.rect.top_left(),
            size: self.rect.size(),
            content,
            spans,
        }
    }
}
