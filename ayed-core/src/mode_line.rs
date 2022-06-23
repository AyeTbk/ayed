use crate::{
    core::{EditorContext, EditorContextMut},
    input::Input,
    line_builder::LineBuilder,
    panel::Panel,
    selection::Position,
    ui_state::{Color, Panel as UiPanel, Span, Style},
};

pub struct ModeLine {
    infos: Vec<ModeLineInfo>,
}

impl ModeLine {
    pub fn new() -> Self {
        Self {
            infos: Default::default(),
        }
    }

    pub fn set_infos(&mut self, infos: Vec<ModeLineInfo>) {
        self.infos = infos;
    }
}

impl Panel for ModeLine {
    fn input(&mut self, _input: Input, _ctx: &mut EditorContextMut) {}

    fn panel(&self, ctx: &EditorContext) -> UiPanel {
        let mut line_builder = LineBuilder::new_with_length(ctx.viewport_size.0 as _);

        for info in &self.infos {
            line_builder = line_builder.add_right_aligned(&info.text, ());
        }

        let (content, _) = line_builder.build();

        UiPanel {
            position: (0, 0),
            size: ctx.viewport_size,
            content: vec![content],
            spans: vec![Span {
                from: Position::ZERO,
                to: Position::ZERO.with_moved_indices(0, ctx.viewport_size.0 as _),
                style: Style {
                    foreground_color: Some(Color::rgb(200, 200, 0)),
                    background_color: None,
                    invert: false,
                },
                importance: 1,
            }],
        }
    }
}

pub struct ModeLineInfo {
    pub text: String,
    pub style: Style,
}
