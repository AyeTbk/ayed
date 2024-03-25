use crate::{position::Position, state::State, ui::ui_state::UiPanel};

#[derive(Default)]
pub struct Editor {
    // Just use state.active_view for now
    // view: Option<Handle<View>>,
}

impl Editor {
    pub fn render(&self, state: &State) -> UiPanel {
        let size = state.viewport_size;

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
            position: Position::ZERO,
            size,
            content,
            spans: Vec::new(),
        }
    }
}

fn line_clamped_filled(line: &str, char_count: u32, fill: char) -> String {
    let mut s = String::new();
    let mut char_taken_count = 0;
    for ch in line.chars().take(char_count as _) {
        s.push(ch);
        char_taken_count += 1;
    }
    let missing_char_count = char_count.saturating_sub(char_taken_count);
    for _ in 0..missing_char_count {
        s.push(fill);
    }
    s
}
