use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::buffer::Buffer;
use crate::input::Input;
use crate::mode_line::{ModeLine, ModeLineInfo};
use crate::panel::Panel;
use crate::selection::SelectionBounds;
use crate::text_editor::TextEditor;
use crate::ui_state::{UiPanel, UiState};

pub struct Core {
    buffers: Arena<Buffer>,
    active_buffer: Handle<Buffer>,
    active_editor: TextEditor,
    mode_line: ModeLine,
    viewport_size: (u32, u32),
    quit: bool,
}

impl Core {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let active_buffer = buffers.allocate(Buffer::new_empty());
        let active_editor = TextEditor::new();
        let mode_line = ModeLine::new();

        Self {
            buffers,
            active_buffer,
            active_editor,
            mode_line,
            viewport_size: (80, 25),
            quit: false,
        }
    }

    pub fn is_quit(&self) -> bool {
        self.quit
    }

    pub fn create_buffer_from_filepath(&mut self, path: impl AsRef<Path>) -> Handle<Buffer> {
        self.buffers.allocate(Buffer::from_filepath(path.as_ref()))
    }

    pub fn create_scratch_buffer(&mut self) -> Handle<Buffer> {
        self.buffers.allocate(Buffer::new_empty())
    }

    pub fn edit_buffer(&mut self, buffer: Handle<Buffer>) {
        self.active_editor = TextEditor::new();
        self.active_buffer = buffer;
    }

    pub fn input(&mut self, input: Input) {
        if self.mode_line.has_focus() {
            self.input_mode_line(input);
        } else if input == Input::Char(':') {
            self.mode_line.set_has_focus(true);
        } else {
            self.input_active_editor(input);
        }
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        self.viewport_size
    }

    pub fn set_viewport_size(&mut self, viewport_size: (u32, u32)) {
        self.viewport_size = viewport_size;
    }

    pub fn ui_state(&mut self) -> UiState {
        let active_editor_panel = self.active_editor_panel();

        let infos = self.mode_line_infos();
        self.mode_line.set_infos(infos);

        let mode_line_panel = self.mode_line_panel();
        let panels = vec![active_editor_panel, mode_line_panel];
        UiState { panels }
    }

    pub fn active_editor_selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
        self.active_editor.selections()
    }

    fn interpret_command(&mut self, command_str: &str) {
        match command_str {
            "q" | "quit" => self.quit = true,
            _ => (),
        }
    }

    fn input_mode_line(&mut self, input: Input) {
        let viewport_size = self.mode_line_viewport_size();
        let buffer = self.buffers.get_mut(self.active_buffer);
        let mut ctx = EditorContextMut {
            buffer,
            viewport_size,
        };
        if let Some(command) = self.mode_line.convert_input_to_command(input, &mut ctx) {
            if let Some(line) = self.mode_line.send_command(command, &mut ctx) {
                self.mode_line.set_has_focus(false);
                self.interpret_command(&line);
            }
        }
    }

    fn input_active_editor(&mut self, input: Input) {
        let viewport_size = self.active_editor_viewport_size();
        let buffer = self.buffers.get_mut(self.active_buffer);
        let mut ctx = EditorContextMut {
            buffer,
            viewport_size,
        };
        if let Some(command) = self.active_editor.convert_input_to_command(input, &mut ctx) {
            self.active_editor.execute_command(command, &mut ctx);
        }
    }

    fn active_editor_panel(&mut self) -> UiPanel {
        let viewport_size = self.active_editor_viewport_size();
        let buffer = self.buffers.get_mut(self.active_buffer);
        let ctx = EditorContextMut {
            buffer,
            viewport_size,
        };
        self.active_editor.panel(&ctx)
    }

    fn active_editor_viewport_size(&self) -> (u32, u32) {
        (self.viewport_size.0, self.viewport_size.1 - 1)
    }

    fn mode_line_panel(&mut self) -> UiPanel {
        let viewport_size = self.mode_line_viewport_size();
        let buffer = self.buffers.get_mut(self.active_buffer);
        let ctx = EditorContextMut {
            buffer,
            viewport_size,
        };

        let mut panel = self.mode_line.panel(&ctx);
        panel.position.1 = self.viewport_size.1 - 1;
        panel
    }

    fn mode_line_infos(&mut self) -> Vec<ModeLineInfo> {
        let viewport_size = self.active_editor_viewport_size();
        let buffer = self.buffers.get_mut(self.active_buffer);
        let ctx = EditorContextMut {
            buffer,
            viewport_size,
        };
        self.active_editor.mode_line_infos(&ctx)
    }

    fn mode_line_viewport_size(&self) -> (u32, u32) {
        (self.viewport_size.0, 1)
    }
}

pub struct EditorContextMut<'a> {
    pub buffer: &'a mut Buffer,
    pub viewport_size: (u32, u32),
}
