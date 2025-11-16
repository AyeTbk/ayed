use crate::{
    position::{Column, Position, Row},
    slotmap::Handle,
    state::View,
    ui::{
        Color, Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::{
        line_clamped_filled,
        ops::{is_whitespace, take_while},
    },
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

    pub fn render(&self, ctx: &RenderPanelContext) -> Vec<UiPanel> {
        let size = self.rect.size();

        let Some(view_handle) = self.view.or(ctx.state.active_editor_view) else {
            let mut content = vec![" ".repeat(size.column as _); size.row as _];
            if size.row > 0 {
                content[0] = String::new()
                    + "[no view]"
                    + &(" ".repeat((size.column.saturating_sub(7)) as _));
            }
            return vec![UiPanel {
                position: Position::ZERO,
                size,
                content,
                spans: Vec::new(),
            }];
        };

        let view = ctx.resources.views.get(view_handle);

        let mut content: Vec<String> = Vec::new();
        let mut spans: Vec<StyledRegion> = Vec::new();

        let line_count = size.row.try_into().unwrap();

        let foreground_color = ctx.state.config.get_theme_color("editor-fg");
        let background_color = ctx.state.config.get_theme_color("editor-bg");
        let nil_line_fg = ctx.state.config.get_theme_color("editor-nil-line");
        for i in 0..line_count {
            spans.push(StyledRegion {
                from: Position::new(0, i),
                to: Position::new(Column::MAX, i),
                style: Style {
                    foreground_color,
                    background_color,
                    ..Default::default()
                },
                ..Default::default()
            });
        }

        let view_line_start = view.top_left.column as usize;
        let mut buf = String::new();
        for i in 0..line_count {
            if view
                .render_view_line(i, &mut buf, &ctx.resources.buffers, &ctx.state.config)
                .is_some()
            {
                content.push(line_clamped_filled(
                    &buf,
                    view_line_start,
                    size.column as usize,
                    ' ',
                ));
            } else {
                let nil_line = String::from("~") + &" ".repeat(size.column.saturating_sub(1) as _);
                spans.push(StyledRegion {
                    from: Position::new(0, i),
                    to: Position::new(0, i),
                    style: Style {
                        foreground_color: nil_line_fg,
                        bold: true,
                        ..Default::default()
                    },
                    priority: 1,
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
                    .line_char_count(selection.cursor.row)
                    .is_some_and(|count| count == selection.cursor.column)
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
            let cursor = view.map_logical_position_to_view_position(
                buffer.map_true_position_to_logical_position(selection.cursor, &ctx.state.config),
            );
            let cursor_end = view
                .map_logical_position_to_view_position(
                    buffer.map_true_position_to_logical_position(
                        selection.cursor.offset((1, 0)),
                        &ctx.state.config,
                    ),
                )
                .offset((-1, 0));
            spans.push(StyledRegion {
                from: cursor,
                to: cursor_end,
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
                let from = view.map_logical_position_to_view_position(
                    buffer.map_true_position_to_logical_position(sel.start(), &ctx.state.config),
                );
                let to = view.map_logical_position_to_view_position(
                    buffer.map_true_position_to_logical_position(sel.end(), &ctx.state.config),
                );
                spans.push(StyledRegion {
                    from,
                    to,
                    style: Style {
                        foreground_color: None,
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
                let from = view.map_logical_position_to_view_position(hl.styled_region.from);
                let to = view.map_logical_position_to_view_position(hl.styled_region.to);
                StyledRegion {
                    from,
                    to,
                    ..hl.styled_region
                }
            }));
        }

        let mut editor_panel = UiPanel {
            position: self.rect.top_left(),
            size,
            content,
            spans,
        };
        self.render_idents(&mut editor_panel, view_line_start, ctx);
        vec![editor_panel]
    }

    fn render_idents(
        &self,
        editor_panel: &mut UiPanel,
        view_line_start: usize,
        ctx: &RenderPanelContext,
    ) {
        let foreground_color = ctx.state.config.get_theme_color("editor-indent");
        let background_color = ctx.state.config.get_theme_color("editor-bg");
        let indent_size = ctx.state.config.get_editor().indent_size;

        // NOTE: editor's view lines are logical lines, so all indent should be normalized to spaces already, and so we can assume 1 byte == 1 char.

        let mut indents: Vec<(Row, usize)> = Vec::new();
        let mut prev_max = 0;
        for (y, line) in editor_panel.content.iter().enumerate() {
            let row = y as i32;
            let (indent_str, _) = take_while(line, is_whitespace);
            let mut max = indent_str.len();
            if indent_str.len() == line.len() {
                // Skip empty lines
                max = prev_max;
            }
            prev_max = max;
            for idx in view_line_start..max {
                if idx % indent_size as usize == 0 {
                    indents.push((row, idx));
                }
            }
        }

        for (row, idx) in indents.into_iter().rev() {
            let pos = Position::new(idx as i32, row);
            editor_panel.spans.push(StyledRegion {
                from: pos,
                to: pos,
                style: Style {
                    foreground_color,
                    background_color,
                    ..Default::default()
                },
                priority: 1,
                ..Default::default()
            });
            editor_panel.content[row as usize].replace_range(idx..idx + 1, "▏");
        }
    }
}
