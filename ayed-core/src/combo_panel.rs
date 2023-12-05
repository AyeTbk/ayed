use crate::{
    grid_string_builder::{Cell, GridStringBuilder},
    input::Input,
    state::State,
    ui_state::{Span, Style, UiPanel},
    utils::{Position, Size},
};

pub struct ComboPanel {
    infos: ComboInfos,
}

impl ComboPanel {
    pub fn new(infos: ComboInfos) -> Self {
        ComboPanel { infos }
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let mut grid = GridStringBuilder::new();

        let header_title = state
            .active_combo_mode_name
            .as_ref()
            .map(String::as_str)
            .unwrap_or("");
        let header_text = format!("╌ {header_title}");
        grid.set_cell_span((1, 0), (2, 0));
        grid.set_cell((1, 0), Cell::new(header_text));

        for (i, info) in self.infos.infos.iter().enumerate() {
            let mut serialized_input = String::new();
            serialized_input.clear();
            info.input.serialize(&mut serialized_input);
            serialized_input.push_str(": ");

            let y = (i + 1) as _;
            grid.set_cell((1, y), Cell::new(serialized_input));
            grid.set_cell((2, y), Cell::new(info.description.clone()));
        }

        // left and right padding
        grid.set_cell((0, 0), Cell::new("╭─"));
        grid.set_cell((3, 0), Cell::new("─╮"));

        let (size, content) = grid.build();
        let size: Size = size.into();
        let column = state.viewport_size.column.saturating_sub(size.column);
        let line = state.viewport_size.row.saturating_sub(size.row + 1);
        let position = (column, line).into();

        let border_style = Style {
            foreground_color: Some(crate::theme::colors::MODELINE_TEXT),
            background_color: Some(crate::theme::colors::ACCENT),
            ..Default::default()
        };

        let mut spans = vec![Span {
            from: Position::ZERO,
            to: Position::new(size.column, 0),
            importance: 1,
            style: border_style,
        }];

        for row in 1..=size.row {
            let right_column = size.column.saturating_sub(1);
            spans.extend([
                Span {
                    from: Position::new(0, row),
                    to: Position::new(0, row),
                    importance: 1,
                    style: border_style,
                },
                Span {
                    from: Position::new(right_column, row),
                    to: Position::new(right_column, row),
                    importance: 1,
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

pub struct ComboInfos {
    pub infos: Vec<ComboInfo>,
}

pub struct ComboInfo {
    pub input: Input,
    pub description: String,
}
