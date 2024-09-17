use crate::{
    position::Position,
    slotmap::Handle,
    state::{State, View},
    ui::{
        ui_state::{StyledRegion, UiPanel},
        Color, Rect, Style,
    },
    utils::string_utils::line_clamped_filled,
};

const PRIMARY_CURSOR_COLOR: Color = Color::WHITE;
const PRIMARY_SELECTION_COLOR: Color = Color::rgb(18, 72, 150);
const SECONDARY_CURSOR_COLOR: Color = Color::rgb(180, 180, 180);
const SECONDARY_SELECTION_COLOR: Color = Color::rgb(12, 52, 100);

const PRIMARY_CURSOR_ALT_COLOR: Color = Color::RED;
const PRIMARY_SELECTION_ALT_COLOR: Color = Color::rgb(100, 32, 96);
const SECONDARY_CURSOR_ALT_COLOR: Color = Color::rgb(170, 10, 10);
const SECONDARY_SELECTION_ALT_COLOR: Color = Color::rgb(80, 26, 76);

const SELECTION_TEXT_COLOR: Color = Color::rgb(200, 200, 200);

const NIL_LINE_COLOR: Color = Color::rgb(155, 100, 200);

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

    pub fn render(&self, state: &State) -> UiPanel {
        let size = self.rect.size();

        let Some(view_handle) = self.view.or(state.active_editor_view) else {
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

        let view = state.views.get(view_handle);

        let mut content: Vec<String> = Vec::new();
        let mut spans: Vec<StyledRegion> = Vec::new();

        let mut buf = String::new();
        for i in 0..size.row {
            if view
                .render_view_line(i as u32, &mut buf, &state.buffers)
                .is_some()
            {
                content.push(line_clamped_filled(
                    &buf,
                    view.top_left.column,
                    size.column,
                    ' ',
                ));
            } else {
                let nil_line = String::from("~") + &" ".repeat(size.column.saturating_sub(1) as _);
                spans.push(StyledRegion {
                    from: Position::new(0, i as u32),
                    to: Position::new(0, i as u32),
                    style: Style {
                        foreground_color: Some(NIL_LINE_COLOR),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                content.push(nil_line);
            }
        }

        for (i, selection) in view.selections.borrow().iter().enumerate() {
            let is_primary = i == 0;
            // FIXME dont hardcode text/insert here, make this configurable in the config somehow
            let use_alt_style = state.config.state_value("mode") == Some("text/insert");

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

            // Cursor style
            if let Some(cursor) = view.map_true_position_to_view_position(selection.cursor()) {
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
            }

            // FIXME This doesn't handle selection that span multiple virtual fragments.
            // This is visible with line-wrap enabled.
            // Selection style
            for split_selection in selection.split_lines() {
                let buffer = state.buffers.get(view.buffer);
                let sel = buffer.limit_selection_to_content(&split_selection);
                let from = view.map_true_position_to_view_position(sel.start());
                let to = view.map_true_position_to_view_position(sel.end());
                if let (Some(from), Some(to)) = (from, to) {
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
        }

        // FIXME Same as above, doesnt support highlights that span multiple fragments
        // Syntax highlight
        if let Some(highlights) = state.highlights.get(&view.buffer) {
            spans.extend(highlights.iter().filter_map(|hl| {
                let from = view.map_true_position_to_view_position(hl.styled_region.from);
                let to = view.map_true_position_to_view_position(hl.styled_region.to);
                if let (Some(from), Some(to)) = (from, to) {
                    Some(StyledRegion {
                        from,
                        to,
                        ..hl.styled_region
                    })
                } else {
                    None
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
