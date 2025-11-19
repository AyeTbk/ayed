use crate::{
    position::{Position, Row},
    ui::{
        Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
};

use super::RenderPanelContext;

#[derive(Default)]
pub struct LineNumbers {
    rect: Rect,
}

impl LineNumbers {
    const RIGHT_PAD_LEN: i32 = 2;

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn required_width(&self, ctx: &RenderPanelContext) -> i32 {
        let Some(buffer_handle) = ctx.state.active_editor_buffer(&ctx.resources) else {
            return 2;
        };
        let max_line = ctx.resources.buffers.get(buffer_handle).line_count();
        const LEFT_PAD_LEN: i32 = 1;
        let width = ((max_line.ilog10() as i32) + 1) + LEFT_PAD_LEN + Self::RIGHT_PAD_LEN;
        width
    }

    pub fn render(&self, ctx: &RenderPanelContext) -> UiPanel {
        let mut content = Vec::new();
        let mut spans = Vec::new();

        let Some(view_handle) = ctx.state.active_editor_view else {
            return UiPanel {
                position: self.rect.top_left(),
                size: self.rect.size(),
                content: Vec::new(),
                spans,
            };
        };

        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get(view.buffer);

        let foreground_color = ctx.state.config.get_theme_color("linenumbers-fg");
        let background_color = ctx.state.config.get_theme_color("linenumbers-bg");
        let curr_line_color = ctx.state.config.get_theme_color("linenumbers-current");
        let last_line_color = ctx.state.config.get_theme_color("linenumbers-last");

        let width = self.rect.width;
        let height = self.rect.height;

        spans.push(StyledRegion {
            from: Position::ZERO,
            to: Position::new(width, height),
            style: Style {
                background_color,
                ..Default::default()
            },
            ..Default::default()
        });

        let mut previous_number = 0;
        let line_count: Row = ctx.state.active_editor_view_rect(&ctx.resources).height;
        for i in 0..line_count {
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

            let mut right_pad_len = Self::RIGHT_PAD_LEN;
            // Show when view is scolled horizontally to the right
            if view.top_left.column != 0 {
                s.push_str(&"‹");
                right_pad_len = right_pad_len.saturating_sub(1);
            }

            s.push_str(&" ".repeat(right_pad_len as _));
            content.push(s);

            let current_row = {
                let selections = buffer.view_selections(view_handle).unwrap();
                selections.primary().cursor.row
            };
            // TODO theme
            let color = if current_row + 1 == line_number {
                curr_line_color
            } else if line_number == buffer.line_count() {
                last_line_color
            } else {
                foreground_color
            };
            spans.push(StyledRegion {
                from: Position::new(0, i),
                to: Position::new(width, i),
                style: Style {
                    foreground_color: color,
                    background_color,
                    ..Default::default()
                },
                ..Default::default()
            })
        }

        UiPanel {
            position: self.rect.top_left(),
            size: self.rect.size(),
            content,
            spans,
        }
    }
}
