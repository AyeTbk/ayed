use crate::{
    arena::Arena,
    buffer::TextBuffer,
    command::Command,
    input::Input,
    input_mapper::InputMap,
    panel::Panel,
    selection::Position,
    state::State,
    text_editor::TextEditor,
    text_mode::TextEditMode,
    ui_state::{Color, Span, Style, UiPanel},
    utils::Rect,
};

pub struct LineEdit {
    editor: TextEditor,
    inner_state: State,
    rect: Rect,
}

impl LineEdit {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let active_buffer_handle = buffers.allocate(TextBuffer::new_empty());
        let inner_state = State {
            buffers,
            active_buffer_handle,
            viewport_size: (0, 0),
            mode_line_infos: Default::default(),
        };

        Self {
            editor: TextEditor::new(),
            inner_state,
            rect: Rect::new(0, 0, 25, 1),
        }
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn input(&mut self, input: Input, state: &mut State) -> Option<String> {
        let commands = TextEditMode.convert_input_to_command(input, state);
        for command in commands {
            match command {
                Command::Insert('\n') => {
                    let mut line = String::new();
                    self.buffer().copy_line(0, &mut line).ok()?;
                    self.reset();
                    return Some(line);
                }
                _ => {
                    self.editor.execute_command(command, &mut self.inner_state);
                }
            }
        }

        None
    }

    pub fn render(&mut self, _state: &State) -> UiPanel {
        let (w, h) = self.rect.size();
        let editor_width = w - 1;
        self.inner_state.viewport_size = (editor_width, h);
        let mut editor_panel = self.editor.render(&self.inner_state);

        editor_panel.position = self.rect.top_left();
        editor_panel.size = self.rect.size();

        for span in &mut editor_panel.spans {
            span.from.column_index += 1;
            span.to.column_index += 1;
        }

        for line in &mut editor_panel.content {
            line.insert(0, '›');
        }

        // Prompt color
        editor_panel.spans.push(Span {
            from: Position::ZERO,
            to: Position::ZERO,
            style: Style {
                foreground_color: None,
                background_color: Some(Color::rgb(96, 32, 200)),
                invert: false,
            },
            importance: 1,
        });

        // Bg color
        editor_panel.spans.push(Span {
            from: Position::ZERO,
            to: Position::ZERO.with_moved_indices(0, w as _),
            style: Style {
                foreground_color: None,
                background_color: Some(Color::rgb(40, 30, 50)),
                invert: false,
            },
            importance: 0,
        });

        editor_panel
    }

    fn buffer(&self) -> &TextBuffer {
        self.inner_state.active_buffer()
    }

    fn reset(&mut self) {
        self.editor = TextEditor::new();
        *self.inner_state.active_buffer_mut() = TextBuffer::new_empty();
    }
}
