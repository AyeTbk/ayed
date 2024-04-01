use crate::{
    position::Position,
    slotmap::Handle,
    state::{State, View},
    ui::{
        ui_state::{StyledRegion, UiPanel},
        Color, Rect, Style,
    },
};

use super::line_clamped_filled;

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
            content[0] =
                String::new() + "[no view]" + &(" ".repeat((size.column.saturating_sub(7)) as _));
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

        let buffer = state.buffers.get(view.buffer);
        let row_from = view.top_left.row;
        let row_to = view.top_left.row.saturating_add(size.row);
        for (i, row_index) in (row_from..row_to).enumerate() {
            if let Some(line) = buffer.line(row_index) {
                content.push(line_clamped_filled(
                    line,
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
            let use_alt_style = false;

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
            if let (Some(column), Some(row)) = selection.cursor().local_to(view.top_left) {
                let cursor = Position::new(column, row);
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

            // Selection style
            for split_selection in selection.split_lines() {
                // FIXME dont produce styled regions outside the viewport plz.
                let (from_column, from_row) = buffer
                    .limit_position_to_content(split_selection.start())
                    .local_to(view.top_left);
                let (to_column, to_row) = buffer
                    .limit_position_to_content(split_selection.end())
                    .local_to(view.top_left);

                let maybe_from =
                    (|column, row| Some(Position::new(column?, row?)))(from_column, from_row);
                let maybe_to =
                    (|column, row| Some(Position::new(column?, row?)))(to_column, to_row);

                if maybe_from.is_none() && maybe_to.is_none() {
                    continue;
                }

                spans.push(StyledRegion {
                    from: maybe_from
                        .unwrap_or(Position::ZERO.with_row(from_row.unwrap_or_default())),
                    to: maybe_to.unwrap_or(Position::ZERO.with_row(to_row.unwrap_or_default())),
                    style: Style {
                        foreground_color: Some(SELECTION_TEXT_COLOR),
                        background_color: Some(selection_color),
                        ..Default::default()
                    },
                    priority: 254,
                });
            }
        }

        UiPanel {
            position: self.rect.top_left(),
            size,
            content,
            spans,
        }
    }
}
