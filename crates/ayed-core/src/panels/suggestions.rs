use crate::{
    position::Position,
    ui::{
        Rect, Size, Style,
        theme::colors::{ACCENT, ACCENT_BRIGHT},
        ui_state::{StyledRegion, UiPanel},
    },
};

use super::RenderPanelContext;

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

    pub fn render(&self, ctx: &RenderPanelContext) -> Option<UiPanel> {
        if ctx.state.suggestions.items.is_empty() {
            return None;
        }

        let placement = ctx.state.config.get_entry_value("suggestions", "placement");
        let placement = placement.ok()?;
        match placement {
            "cursor" => self.render_at_cursor(ctx),
            "modeline" => self.render_at_modeline(ctx),
            _ => self.render_at_cursor(ctx),
        }
    }

    fn render_at_cursor(&self, ctx: &RenderPanelContext) -> Option<UiPanel> {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return None;
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer_handle = view.buffer;
        let buffer = ctx.resources.buffers.get(buffer_handle);
        let selections = buffer.view_selections(view_handle).unwrap();

        let width = ctx.state.suggestions.items.iter().map(String::len).max();
        let width = i32::max(10, width.unwrap_or(0) as _);
        let height = ctx.state.suggestions.items.len() as i32;
        let size = Size::new(width as u32, height as u32);

        let position_in_buffer = selections.primary().cursor();
        let view_top_left = ctx.state.focused_view_rect(&ctx.resources).top_left();
        let cursor_position =
            position_in_buffer.local_to_pos(view_top_left) + ctx.state.editor_rect.top_left();
        let mut position = cursor_position;

        // Place on the line below the cursor
        position = position.offset((-1, 1));

        // Don't let the panel go past the end of the viewport, rightward
        if position.column + width >= ctx.state.viewport_size.column as i32 {
            let corrected_column = ctx.state.viewport_size.column as i32 - width;
            position = position.with_column(corrected_column);
        }

        // Don't let the panel go past the end of the viewport, downward
        if position.row + height >= ctx.state.viewport_size.row as i32 {
            let corrected_row = cursor_position.row - height;
            position = position.with_row(corrected_row);
        }

        let mut content = Vec::new();
        let mut spans = Vec::new();
        for (i, item) in ctx.state.suggestions.items.iter().enumerate() {
            let mut s = item.clone();
            let pad = " ".repeat(width as usize - s.len());
            s.push_str(&pad);
            content.push(s);

            let color = if ctx.state.suggestions.selected_item == (i as i32 + 1) {
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

    fn render_at_modeline(&self, ctx: &RenderPanelContext) -> Option<UiPanel> {
        let width = ctx.state.modeline_rect.width;
        let height = ctx.state.suggestions.items.len() as i32;
        let size = Size::new(width, height as u32);

        let position = ctx.state.modeline_rect.top_left().offset((0, -height));

        let mut content = Vec::new();
        let mut spans = Vec::new();
        for (i, item) in ctx.state.suggestions.items.iter().enumerate() {
            let mut s = item.clone();
            let pad = " ".repeat(width as usize - s.len());
            s.push_str(&pad);
            content.push(s);

            let color = if ctx.state.suggestions.selected_item == (i as i32 + 1) {
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
