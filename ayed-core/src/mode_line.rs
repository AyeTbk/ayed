use crate::{
    command::Command,
    controls::LineEdit,
    core::EditorContextMut,
    input::Input,
    input_mapper::InputMapper,
    line_builder::LineBuilder,
    panel::Panel,
    selection::Position,
    text_mode::TextEditMode,
    ui_state::{Color, Span, Style, UiPanel},
};

pub struct ModeLine {
    infos: Vec<ModeLineInfo>,
    wants_focus: bool,
    line_edit: LineEdit,
}

impl ModeLine {
    pub fn new() -> Self {
        Self {
            infos: Default::default(),
            wants_focus: Default::default(),
            line_edit: LineEdit::new(),
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

    fn execute_command_inner(&mut self, command: Command, ctx: &mut EditorContextMut) {
        if self.wants_focus {
            self.line_edit.execute_command(command, ctx);
        }
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

    fn panel(&mut self, ctx: &EditorContextMut) -> UiPanel {
        let mut line_builder = LineBuilder::new_with_length(ctx.viewport_size.0 as _);

        for info in &self.infos {
            line_builder = line_builder.add_right_aligned(&info.text, ());
        }

        if self.wants_focus() {
            // TODO unify this with the rest maybe idk figure it out
            return self.line_edit.panel(ctx);
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
