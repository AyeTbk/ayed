use crate::{
    position::{Column, Position, Row},
    slotmap::Handle,
    state::View,
    ui::{
        Rect, Style,
        ui_state::{StyledRegion, UiPanel},
    },
    utils::string_utils::{
        line_clamped_filled,
        ops::{is_whitespace, take_while},
    },
};

use super::RenderPanelContext;

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

        let clr = |name| ctx.state.config.get_theme_color(name);
        let foreground_color = clr("editor-fg");
        let background_color = clr("editor-bg");
        let nil_line_fg = clr("editor-nil-line");

        let clr_cursor = clr("cursor");
        let clr_cursor_extra = clr("cursor-extra").or(clr_cursor);
        let clr_cursor_eol = clr("cursor-eol").or(clr_cursor);
        let clr_cursor_extra_eol = clr("cursor-extra-eol").or(clr_cursor_eol);

        let clr_selection = clr("selection");
        let clr_selection_extra = clr("selection-extra").or(clr_selection);

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
            let is_end_of_line = {
                let buffer = ctx.resources.buffers.get(view.buffer);
                buffer
                    .line_char_count(selection.cursor.row)
                    .is_some_and(|count| count == selection.cursor.column)
            };

            let cursor_color = match (is_primary, is_end_of_line) {
                (true, false) => clr_cursor,
                (true, true) => clr_cursor_eol,
                (false, false) => clr_cursor_extra,
                (false, true) => clr_cursor_extra_eol,
            };
            let selection_color = match is_primary {
                true => clr_selection,
                false => clr_selection_extra,
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
                    foreground_color: cursor_color,
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
                        background_color: selection_color,
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
