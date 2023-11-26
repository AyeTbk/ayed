use crate::{
    grid_string_builder::{Cell, GridStringBuilder},
    input::Input,
    state::State,
    ui_state::{Color, Span, Style, UiPanel},
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
        let size: Size = size.into();
        let column = state.viewport_size.column.saturating_sub(size.column);
        let line = state.viewport_size.row.saturating_sub(size.row + 1);
        let position = (column, line).into();

        // let position = Position::ZERO.offset((0, 0));

        UiPanel {
            position,
            size,
            content,
            // FIXME This should work but it doesnt. Also, mixing x,y points and sizes with Positions (line,column) is confusing...
            spans: vec![Span {
                from: Position::ZERO,
                to: Position::new(size.column, size.row),
                importance: 1,
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
