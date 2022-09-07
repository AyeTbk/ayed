use crate::{
    command::Command,
    core::EditorContextMut,
    input::Input,
    input_mapper::{InputMap, InputMapper},
    panel::Panel,
    selection::{Offset, Position},
    ui_state::{Color, Span, Style, UiPanel},
};

#[derive(Default)]
pub struct WarpDrivePanel {
    position_offset: Offset,
    text_content: Vec<String>,
    jump_points: (),
}

impl WarpDrivePanel {
    fn execute_command_inner(&mut self, command: Command) -> Option<Command> {
        use Command::*;
        match command {
            Insert('\n') => Some(FlipSelection),
            Insert(_) => None,
            _ => None,
        }
    }
}

impl Panel for WarpDrivePanel {
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut) -> Vec<Command> {
        let mut im = InputMapper::default();
        im.register_char_insert();
        im.convert_input_to_command(input, ctx)
    }

    fn execute_command(&mut self, command: Command, ctx: &mut EditorContextMut) -> Option<Command> {
        self.execute_command_inner(command)
    }

    fn panel(&mut self, ctx: &EditorContextMut) -> UiPanel {
        UiPanel {
            position: (0, 0),
            size: ctx.viewport_size,
            content: vec!["  W A R P D R I V E  ".to_string()],
            spans: vec![Span {
                from: Position::ZERO,
                to: Position::ZERO.with_column_index(ctx.viewport_size.0),
                style: Style {
                    foreground_color: Some(Color::BLUE),
                    background_color: Some(Color::RED),
                    invert: false,
                },
                importance: 10,
            }],
        }
    }
}
