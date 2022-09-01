use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::buffer::Buffer;
use crate::input::{Input, Key};
use crate::mode_line::{ModeLine, ModeLineInfo};
use crate::panel::Panel;
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

    pub fn request_quit(&mut self) {
        self.quit = true;
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

    pub fn save_buffer(&mut self, buffer: Handle<Buffer>) {
        self.buffers.get(buffer).save().unwrap();
    }

    pub fn input(&mut self, input: Input) {
        if self.mode_line.has_focus() {
            self.input_mode_line(input);
        } else if input.key == Key::Char(':') {
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

    fn interpret_command(&mut self, command_str: &str) {
        let mut parts = command_str.split(' ');
        let command = parts.next().expect("command expected");
        match command {
            "" => (),
            "q" | "quit" => self.request_quit(),
            "e" | "edit" => {
                let arg = parts.next().expect("name expected");
                let buffer = self.create_buffer_from_filepath(arg);
                self.edit_buffer(buffer);
            }
            "w" | "write" | "s" | "save" => {
                self.save_buffer(self.active_buffer);
            }
            "wq" | "write-quit" => {
                self.save_buffer(self.active_buffer);
                self.request_quit();
            }
            _ => panic!("unknown command: {}", command_str),
        }
    }

    fn input_mode_line(&mut self, input: Input) {
        let viewport_size = self.mode_line_viewport_size();
        // FIXME this code stinks, gotta recreate the same ctx at multiple different place? ew
        let commands = {
            let buffer = self.buffers.get_mut(self.active_buffer);
            let mut ctx = EditorContextMut {
                buffer,
                viewport_size,
            };
            self.mode_line.convert_input_to_command(input, &mut ctx)
        };

        for command in commands {
            let buffer = self.buffers.get_mut(self.active_buffer);
            let mut ctx = EditorContextMut {
                buffer,
                viewport_size,
            };
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
        for command in self.active_editor.convert_input_to_command(input, &mut ctx) {
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
