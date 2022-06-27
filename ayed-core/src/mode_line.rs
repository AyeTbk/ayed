use crate::{
    command::Command,
    core::{EditorContext, EditorContextMut},
    input::Input,
    input_mapper::InputMapper,
    line_builder::LineBuilder,
    panel::Panel,
    selection::Position,
    text_mode::TextEditMode,
    ui_state::{Color, Panel as UiPanel, Span, Style},
};

pub struct ModeLine {
    infos: Vec<ModeLineInfo>,
    wants_focus: bool,
}

impl ModeLine {
    pub fn new() -> Self {
        Self {
            infos: Default::default(),
            wants_focus: Default::default(),
        }
    }

    pub fn set_infos(&mut self, infos: Vec<ModeLineInfo>) {
        self.infos = infos;
    }

    pub fn wants_focus(&self) -> bool {
        self.wants_focus
    }

    pub fn set_wants_focus(&mut self, wants_focus: bool) {
        self.wants_focus = wants_focus;
    }

    fn execute_command_inner(&mut self, _command: Command, _ctx: &mut EditorContextMut) {
        //
    }
}

impl Panel for ModeLine {
    fn convert_input_to_command(
        &self,
        input: Input,
        ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        TextEditMode.convert_input_to_command(input, ctx)
    }

    fn execute_command(&mut self, command: Command, ctx: &mut EditorContextMut) {
        self.execute_command_inner(command, ctx);
    }

    fn panel(&self, ctx: &EditorContext) -> UiPanel {
        let mut line_builder = LineBuilder::new_with_length(ctx.viewport_size.0 as _);

        for info in &self.infos {
            line_builder = line_builder.add_right_aligned(&info.text, ());
        }

        if self.wants_focus() {
            line_builder = line_builder.add_left_aligned(":", ());
            line_builder = line_builder.add_left_aligned(
                "edit the file plz tyvm rlly appreciated like i mean it dude",
                (),
            );
            line_builder = line_builder.add_left_aligned(" ", ());
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
