use crate::{
    position::Position,
    state::State,
    ui::{
        ui_state::{StyledRegion, UiPanel},
        Color, Rect, Style,
    },
};

#[derive(Default)]
pub struct LineNumbers {
    rect: Rect,
}

impl LineNumbers {
    const RIGHT_PAD_LEN: u32 = 2;

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn required_width(&self, state: &State) -> u32 {
        let Some(buffer_handle) = state.active_editor_buffer() else {
            return 2;
        };
        let max_line = state.buffers.get(buffer_handle).line_count();
        const LEFT_PAD_LEN: u32 = 1;
        let width = (max_line.ilog10() + 1) + LEFT_PAD_LEN + Self::RIGHT_PAD_LEN;
        width
    }

    pub fn render(&self, state: &State) -> UiPanel {
        let mut content = Vec::new();
        let mut spans = Vec::new();

        let Some(view) = state.active_editor_view.map(|h| state.views.get(h)) else {
            return UiPanel {
                position: Position::ZERO,
                size: self.rect.size(),
                content: Vec::new(),
                spans,
            };
        };

        let buffer = state.buffers.get(view.buffer);

        let width = self.rect.width;
        let mut previous_number = 0;
        for i in 0..state.focused_view_rect().height {
            let Some(line_number) = view.map_view_line_idx_to_line_number(i) else {
                content.push(" ".repeat(width as usize));
                continue;
            };

            let should_be_blank =
                (line_number == previous_number) || (line_number > buffer.line_count());
            previous_number = line_number;
            if should_be_blank {
                content.push(" ".repeat(width as usize));
                continue;
            }

            let mut s = line_number.to_string();
            let left_pad_len = (width as usize)
                .saturating_sub(s.len())
                .saturating_sub(Self::RIGHT_PAD_LEN as _);
            s.insert_str(0, &" ".repeat(left_pad_len));
            s.push_str(&" ".repeat(Self::RIGHT_PAD_LEN as _));
            content.push(s);

            let current_row = { view.selections.borrow().primary().cursor().row };
            let color = if current_row + 1 == line_number {
                Color::rgb(230, 230, 230)
            } else if line_number == buffer.line_count() {
                Color::rgb(81, 81, 81)
            } else {
                Color::rgb(140, 140, 140)
            };
            spans.push(StyledRegion {
                from: Position::new(0, i),
                to: Position::new(width, i),
                style: Style {
                    foreground_color: Some(color),
                    ..Default::default()
                },
                ..Default::default()
            })
        }

        UiPanel {
            position: Position::ZERO,
            size: self.rect.size(),
            content,
            spans,
        }
    }
}
