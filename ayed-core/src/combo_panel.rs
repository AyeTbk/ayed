use crate::{input::Input, line_builder::LineBuilder, state::State, ui_state::UiPanel};

pub struct ComboPanel {
    infos: ComboInfos,
}

impl ComboPanel {
    pub fn new(infos: ComboInfos) -> Self {
        ComboPanel { infos }
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let mut content = Vec::new();
        let (width, height) = state.viewport_size;
        for _ in 0..height {
            content.push("~".repeat(width as usize));
        }

        let mut buf = String::new();
        for (i, info) in self.infos.infos.iter().enumerate() {
            buf.clear();
            info.input.serialize(&mut buf);

            let builder = LineBuilder::new_with_length(width as usize);
            let builder = builder.add_left_aligned(&buf, ());
            let builder = builder.add_left_aligned(": ", ());
            let builder = builder.add_right_aligned(&info.description, ());

            let line = content.get_mut(i).unwrap();
            *line = builder.build().0;
        }

        UiPanel {
            position: (0, 0),
            size: state.viewport_size,
            content,
            spans: Default::default(),
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
