use crate::{
    buffer::Buffer,
    command::Command,
    core::EditorContextMut,
    input_mapper::InputMapper,
    panel::Panel,
    selection::Position,
    text_editor::TextEditor,
    text_mode::TextEditMode,
    ui_state::{Color, Span, Style, UiPanel},
};

pub struct LineEdit {
    editor: TextEditor,
    buffer: Buffer,
}

impl LineEdit {
    pub fn new() -> Self {
        Self {
            editor: TextEditor::new(),
            buffer: Buffer::new_empty(),
        }
    }
}

impl Panel for LineEdit {
    fn execute_command(&mut self, command: Command, ctx: &mut EditorContextMut) {
        let mut line_edit_ctx = EditorContextMut {
            viewport_size: ctx.viewport_size,
            buffer: &mut self.buffer,
        };
        match command {
            Command::Insert('\n') => panic!("wow"),
            _ => self.editor.execute_command(command, &mut line_edit_ctx),
        }
    }

    fn convert_input_to_command(
        &self,
        input: crate::input::Input,
        ctx: &mut EditorContextMut,
    ) -> Option<Command> {
        TextEditMode.convert_input_to_command(input, ctx)
    }

    fn panel(&mut self, ctx: &EditorContextMut) -> UiPanel {
        let line_edit_viewport_size = (
            ctx.viewport_size.0.checked_sub(1).unwrap(),
            ctx.viewport_size.1,
        );
        let line_edit_ctx = EditorContextMut {
            viewport_size: line_edit_viewport_size,
            buffer: &mut self.buffer,
        };
        let mut panel = self.editor.panel(&line_edit_ctx);
        for span in &mut panel.spans {
            span.from.column_index += 1;
            span.to.column_index += 1;
        }

        if let Some(first_line) = panel.content.first_mut() {
            first_line.insert(0, '›');
        }

        panel.spans.push(Span {
            from: Position::ZERO,
            to: Position::ZERO.with_moved_indices(0, 1),
            style: Style {
                foreground_color: None,
                background_color: Some(Color::rgb(96, 32, 200)),
                invert: false,
            },
            importance: 0,
        });
        panel
    }
}
