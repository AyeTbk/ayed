use crate::{
    arena::Handle,
    buffer::TextBuffer,
    command::Command,
    controls::TextBufferEdit,
    input::Input,
    input_mapper::InputMap,
    selection::Position,
    state::State,
    text_mode::{TextCommandMode, TextEditMode},
    ui_state::UiPanel,
    utils::Rect,
};

pub struct TextEditor {
    buffer_handle: Handle<TextBuffer>,
    inner: TextBufferEdit,
    is_command_mode: bool,
}

impl TextEditor {
    pub fn new(buffer_handle: Handle<TextBuffer>) -> Self {
        Self {
            buffer_handle,
            inner: TextBufferEdit::new(),
            is_command_mode: true,
        }
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.inner.set_rect(rect);
    }

    pub fn is_command_mode(&self) -> bool {
        self.is_command_mode
    }

    pub fn view_top_left_position(&self) -> Position {
        self.inner.view_top_left_position()
    }

    pub fn convert_input_to_command(&self, input: Input, state: &State) -> Vec<Command> {
        if self.is_command_mode {
            TextCommandMode.convert_input_to_command(input, state)
        } else {
            TextEditMode.convert_input_to_command(input, state)
        }
    }

    pub fn execute_command(&mut self, command: Command, state: &mut State) {
        let mut fake_state = state.dummy_clone();
        let buffer = state.buffers.get_mut(self.buffer_handle);
        self.inner.execute_command(command, buffer, &mut fake_state);
    }

    pub fn render(&mut self, state: &State) -> UiPanel {
        let buffer = state.buffers.get(self.buffer_handle);
        self.inner.render(buffer, state)
    }
}
