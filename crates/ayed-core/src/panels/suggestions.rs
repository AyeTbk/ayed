use crate::{
    position::Position,
    state::State,
    ui::{
        Rect, Size, Style,
        theme::colors::{ACCENT, ACCENT_BRIGHT},
        ui_state::{StyledRegion, UiPanel},
    },
};

#[derive(Default)]
pub struct Suggestions {
    rect: Rect,
}

impl Suggestions {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, state: &State) -> Option<UiPanel> {
        if state.suggestions.items.is_empty() {
            return None;
        }

        let placement = state.config.get_entry_value("suggestions", "placement");
        let placement = placement.ok()?;
        match placement {
            "cursor" => self.render_at_cursor(state),
            "modeline" => self.render_at_modeline(state),
            _ => self.render_at_cursor(state),
        }
    }

    fn render_at_cursor(&self, state: &State) -> Option<UiPanel> {
        let Some(view_handle) = state.active_editor_view else {
            return None;
        };
        let view = state.views.get(view_handle);
        let buffer_handle = view.buffer;
        let buffer = state.buffers.get(buffer_handle);
        let selections = buffer.view_selections(view_handle).unwrap();

        let position_in_buffer = selections.primary().cursor();
        let view_top_left = state.focused_view_rect().top_left();
        let position =
            position_in_buffer.local_to_pos(view_top_left) + state.editor_rect.top_left();
        let position = position.offset((0, 1));

        let width = state.suggestions.items.iter().map(String::len).max();
        let width = width.unwrap_or(0).min(10) as i32;
        let height = state.suggestions.items.len() as i32;
        let size = Size::new(width as u32, height as u32);

        let mut content = Vec::new();
        let mut spans = Vec::new();
        for (i, item) in state.suggestions.items.iter().enumerate() {
            let mut s = item.clone();
            let pad = " ".repeat(width as usize - s.len());
            s.push_str(&pad);
            content.push(s);

            let color = if state.suggestions.selected_item == (i as i32 + 1) {
                ACCENT_BRIGHT
            } else {
                ACCENT
            };
            spans.push(StyledRegion {
                from: Position::new(0, i as i32),
                to: Position::new(width as i32, i as i32),
                style: Style {
                    foreground_color: None,
                    background_color: Some(color),
                    ..Default::default()
                },
                priority: 0,
            });
        }

        Some(UiPanel {
            position,
            size,
            content,
            spans,
        })
    }

    fn render_at_modeline(&self, state: &State) -> Option<UiPanel> {
        let width = state.modeline_rect.width;
        let height = state.suggestions.items.len() as i32;
        let size = Size::new(width, height as u32);

        let position = state.modeline_rect.top_left().offset((0, -height));

        let mut content = Vec::new();
        let mut spans = Vec::new();
        for (i, item) in state.suggestions.items.iter().enumerate() {
            let mut s = item.clone();
            let pad = " ".repeat(width as usize - s.len());
            s.push_str(&pad);
            content.push(s);

            let color = if state.suggestions.selected_item == (i as i32 + 1) {
                ACCENT_BRIGHT
            } else {
                ACCENT
            };
            spans.push(StyledRegion {
                from: Position::new(0, i as i32),
                to: Position::new(width as i32, i as i32),
                style: Style {
                    foreground_color: None,
                    background_color: Some(color),
                    ..Default::default()
                },
                priority: 0,
            });
        }

        Some(UiPanel {
            position,
            size,
            content,
            spans,
        })
    }
}
