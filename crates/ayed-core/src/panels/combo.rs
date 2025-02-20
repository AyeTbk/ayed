use crate::{
    position::Position,
    state::State,
    ui::{
        ui_state::{StyledRegion, UiPanel},
        Rect, Size, Style,
    },
    utils::string_utils::grid_string_builder::{Cell, GridStringBuilder},
};

#[derive(Default)]
pub struct Combo {
    rect: Rect,
}

impl Combo {
    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn render(&self, state: &State) -> UiPanel {
        let mut grid = GridStringBuilder::new();

        // let header_title = state.config.state_value("combo").unwrap_or_default();
        let header_title = "";
        let header_text = format!("╌ {header_title}");
        grid.set_cell_span((1, 0), (2, 0));
        grid.set_cell((1, 0), Cell::new(header_text));

        if let Some(keybinds_doc) = state.config.get("keybinds-doc") {
            for (i, (keybind, doc)) in keybinds_doc.iter().enumerate() {
                // let mut serialized_input = String::new();
                // serialized_input.clear();
                // info.input.serialize(&mut serialized_input);
                // serialized_input.push_str(": ");
                let y = (i + 1) as _;
                grid.set_cell((1, y), Cell::new(format!("{keybind}: ")));
                // FIXME unecessary clone
                grid.set_cell((2, y), Cell::new(doc.first().cloned().unwrap_or_default()));
            }
        }

        // left and right padding
        grid.set_cell((0, 0), Cell::new("╭─"));
        grid.set_cell((3, 0), Cell::new("─╮"));

        let (size, content) = grid.build();
        let size: Size = size.into();
        let column = self.rect.size().column.saturating_sub(size.column);
        let line = self.rect.size().row.saturating_sub(size.row);
        let position = (column, line).into();

        let border_style = Style {
            foreground_color: Some(crate::ui::theme::colors::MODELINE_TEXT),
            background_color: Some(crate::ui::theme::colors::ACCENT),
            ..Default::default()
        };

        let mut spans = vec![StyledRegion {
            from: Position::ZERO,
            to: Position::new(size.column, 0),
            priority: 1,
            style: border_style,
        }];

        for row in 1..=size.row {
            let right_column = size.column.saturating_sub(1);
            spans.extend([
                StyledRegion {
                    from: Position::new(0, row),
                    to: Position::new(0, row),
                    priority: 1,
                    style: border_style,
                },
                StyledRegion {
                    from: Position::new(right_column, row),
                    to: Position::new(right_column, row),
                    priority: 1,
                    style: border_style,
                },
            ]);
        }

        UiPanel {
            position,
            size,
            content,
            spans,
        }
    }
}
