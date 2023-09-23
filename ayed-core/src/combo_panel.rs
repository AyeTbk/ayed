use crate::{
    grid_string_builder::{Cell, GridStringBuilder},
    input::Input,
    selection::Position,
    state::State,
    ui_state::{Color, Span, Style, UiPanel},
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

        grid.set_cell_span((1, 0), (2, 0));
        grid.set_cell((1, 0), Cell::new("Combo"));

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
        grid.set_cell((0, 0), Cell::new(" "));
        grid.set_cell((3, 0), Cell::new(" "));

        let (size, content) = grid.build();
        let column = state.viewport_size.0.saturating_sub(size.0);
        let line = state.viewport_size.1.saturating_sub(size.1 + 1);
        let position = (column, line);

        UiPanel {
            position,
            size,
            content,
            // FIXME This should work but it doesnt. Also, mixing x,y points and sizes with Positions (line,column) is confusing...
            spans: vec![Span {
                from: Position::new(0, 0),
                to: Position::new(1000, 1000),
                importance: 20,
                style: Style {
                    background_color: Some(Color::RED),
                    foreground_color: Some(Color::BLUE),
                    ..Default::default()
                },
            }],
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
