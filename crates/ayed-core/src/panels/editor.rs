use crate::{
    position::{Column, Position},
    slotmap::Handle,
    state::View,
    ui::{
        Color, Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::line_clamped_filled,
};

use super::RenderPanelContext;

const PRIMARY_CURSOR_COLOR: Color = Color::WHITE;
const PRIMARY_SELECTION_COLOR: Color = Color::rgb(18, 72, 150);
const SECONDARY_CURSOR_COLOR: Color = Color::rgb(180, 180, 180);
const SECONDARY_SELECTION_COLOR: Color = Color::rgb(12, 52, 100);

const PRIMARY_CURSOR_ALT_COLOR: Color = Color::rgb(230, 30, 30);
const PRIMARY_SELECTION_ALT_COLOR: Color = Color::rgb(100, 32, 96);
const SECONDARY_CURSOR_ALT_COLOR: Color = Color::rgb(160, 15, 15);
const SECONDARY_SELECTION_ALT_COLOR: Color = Color::rgb(80, 26, 76);

const SELECTION_TEXT_COLOR: Color = Color::rgb(200, 200, 200);

const NIL_LINE_COLOR: Color = Color::rgb(155, 100, 200);

const PRIMARY_CURSOR_END_OF_LINE_COLOR: Color = Color::rgb(155, 100, 200);
const SECONDARY_CURSOR_END_OF_LINE_COLOR: Color = Color::rgb(110, 70, 150);
const PRIMARY_CURSOR_END_OF_LINE_ALT_COLOR: Color = Color::rgb(220, 90, 120);
const SECONDARY_CURSOR_END_OF_LINE_ALT_COLOR: Color = Color::rgb(160, 60, 90);

#[derive(Default)]
pub struct Editor {
    view: Option<Handle<View>>,
    rect: Rect,
}

impl Editor {
    pub fn with_view(view: Handle<View>) -> Self {
        Self {
            view: Some(view),
            rect: Rect::default(),
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, ctx: &RenderPanelContext) -> UiPanel {
        let size = self.rect.size();

        let Some(view_handle) = self.view.or(ctx.state.active_editor_view) else {
            let mut content = vec![" ".repeat(size.column as _); size.row as _];
            if size.row > 0 {
                content[0] = String::new()
                    + "[no view]"
                    + &(" ".repeat((size.column.saturating_sub(7)) as _));
            }
            return UiPanel {
                position: Position::ZERO,
                size,
                content,
                spans: Vec::new(),
            };
        };

        let view = ctx.resources.views.get(view_handle);

        let mut content: Vec<String> = Vec::new();
        let mut spans: Vec<StyledRegion> = Vec::new();

        let mut buf = String::new();
        let line_count = size.row.try_into().unwrap();
        for i in 0..line_count {
            if view
                .render_view_line(i, &mut buf, &ctx.resources.buffers)
                .is_some()
            {
                content.push(line_clamped_filled(
                    &buf,
                    view.top_left.column as usize,
                    size.column as usize,
                    ' ',
                ));
            } else {
                let nil_line = String::from("~") + &" ".repeat(size.column.saturating_sub(1) as _);
                spans.push(StyledRegion {
                    from: Position::new(0, i),
                    to: Position::new(0, i),
                    style: Style {
                        foreground_color: Some(NIL_LINE_COLOR),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                content.push(nil_line);
            }
        }

        let buffer = ctx.resources.buffers.get(view.buffer);
        let selections = buffer.view_selections(view_handle).unwrap();
        for (i, selection) in selections.iter().enumerate() {
            let is_primary = i == 0;
            // FIXME dont hardcode insert/append here, make this configurable in the config somehow
            let use_alt_style = matches!(
                ctx.state.config.state_value("mode"),
                Some("insert" | "insert-append")
            );

            let cursor_color = match (is_primary, use_alt_style) {
                (true, false) => PRIMARY_CURSOR_COLOR,
                (true, true) => PRIMARY_CURSOR_ALT_COLOR,
                (false, false) => SECONDARY_CURSOR_COLOR,
                (false, true) => SECONDARY_CURSOR_ALT_COLOR,
            };
            let selection_color = match (is_primary, use_alt_style) {
                (true, false) => PRIMARY_SELECTION_COLOR,
                (true, true) => PRIMARY_SELECTION_ALT_COLOR,
                (false, false) => SECONDARY_SELECTION_COLOR,
                (false, true) => SECONDARY_SELECTION_ALT_COLOR,
            };

            let is_end_of_line = {
                let buffer = ctx.resources.buffers.get(view.buffer);
                buffer
                    .line(selection.cursor().row)
                    .is_some_and(|line| line.len() as Column == selection.cursor().column)
            };
            let cursor_color = if is_end_of_line {
                match (is_primary, use_alt_style) {
                    (true, false) => PRIMARY_CURSOR_END_OF_LINE_COLOR,
                    (true, true) => PRIMARY_CURSOR_END_OF_LINE_ALT_COLOR,
                    (false, false) => SECONDARY_CURSOR_END_OF_LINE_COLOR,
                    (false, true) => SECONDARY_CURSOR_END_OF_LINE_ALT_COLOR,
                }
            } else {
                cursor_color
            };

            // Cursor style
            let cursor = view.map_true_position_to_view_position(selection.cursor());
            spans.push(StyledRegion {
                from: cursor,
                to: cursor,
                style: Style {
                    foreground_color: Some(cursor_color),
                    invert: true,
                    ..Default::default()
                },
                priority: 255,
            });

            // Selection style
            for split_selection in selection.split_lines() {
                let buffer = ctx.resources.buffers.get(view.buffer);
                let sel = buffer.limit_selection_to_content(&split_selection);
                let from = view.map_true_position_to_view_position(sel.start());
                let to = view.map_true_position_to_view_position(sel.end());
                spans.push(StyledRegion {
                    from,
                    to,
                    style: Style {
                        foreground_color: Some(SELECTION_TEXT_COLOR),
                        background_color: Some(selection_color),
                        ..Default::default()
                    },
                    priority: 254,
                });
            }
        }

        // FIXME Same as above, doesnt support highlights that span multiple fragments
        // Syntax highlight
        if let Some(highlights) = ctx.state.highlights.get(&view.buffer) {
            spans.extend(highlights.iter().map(|hl| {
                let from = view.map_true_position_to_view_position(hl.styled_region.from);
                let to = view.map_true_position_to_view_position(hl.styled_region.to);
                StyledRegion {
                    from,
                    to,
                    ..hl.styled_region
                }
            }));
        }

        UiPanel {
            position: self.rect.top_left(),
            size,
            content,
            spans,
        }
    }
}
