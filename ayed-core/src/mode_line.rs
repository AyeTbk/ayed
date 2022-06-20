use crate::{
    core::{EditorContext, EditorContextMut},
    input::Input,
    panel::Panel,
    selection::Position,
    ui_state::{Color, Panel as UiPanel, Span, Style},
};

pub struct ModeLine {
    //
}

impl ModeLine {
    pub fn new() -> Self {
        Self {}
    }
}

impl Panel for ModeLine {
    fn input(&mut self, _input: Input, _ctx: &mut EditorContextMut) {}

    fn panel(&self, ctx: &EditorContext) -> UiPanel {
        let content = String::from("this is a mode line");

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
