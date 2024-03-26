use crate::{
    position::Position,
    state::State,
    ui::{ui_state::UiPanel, Rect},
};

use super::line_clamped_filled;

#[derive(Default)]
pub struct Editor {
    // Just use state.active_view for now
    // view: Option<Handle<View>>,
    rect: Rect,
}

impl Editor {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, state: &State) -> UiPanel {
        let size = self.rect.size();

        let Some(view_handle) = state.active_view else {
            let mut content = vec![" ".repeat(size.column as _); size.row as _];
            content[0] =
                String::new() + "no view" + &(" ".repeat((size.column.saturating_sub(7)) as _));
            return UiPanel {
                position: Position::ZERO,
                size,
                content,
                spans: Vec::new(),
            };
        };

        let buffer_handle = state.views.get(view_handle).buffer;

        let mut content = Vec::new();
        let buffer = state.buffers.get(buffer_handle);
        for row_index in 0..size.row {
            if let Some(line) = buffer.line(row_index) {
                content.push(line_clamped_filled(line, size.column, ' '));
            } else {
                content.push(" ".repeat(size.column as _));
            }
        }

        UiPanel {
            position: self.rect.top_left(),
            size,
            content,
            spans: Vec::new(),
        }
    }
}
