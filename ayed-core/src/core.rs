use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::buffer::TextBuffer;
use crate::command::Command;
use crate::input::{Input, Key};
use crate::mode_line::{ModeLine, ModeLineInfo};
use crate::panel::Panel;
use crate::panels::warpdrive_panel::WarpDrivePanel;
use crate::state::State;
use crate::text_editor::TextEditor;
use crate::ui_state::{UiPanel, UiState};
use crate::utils::Rect;

pub struct Core {
    state: State,
    editor: TextEditor,
    mode_line: ModeLine,
    warpdrive_panel: Option<WarpDrivePanel>,
    quit: bool,
}

impl Core {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let active_buffer_handle = buffers.allocate(TextBuffer::new_empty());
        let editor = TextEditor::new();
        let viewport_size = (80, 25);
        let state = State {
            buffers,
            active_buffer_handle,
            viewport_size,
            mode_line_infos: Default::default(),
        };

        let mode_line = ModeLine::new();

        Self {
            state,
            editor,
            mode_line,
            warpdrive_panel: None,
            quit: false,
        }
    }

    pub fn is_quit(&self) -> bool {
        self.quit
    }

    pub fn request_quit(&mut self) {
        self.quit = true;
    }

    pub fn create_buffer_from_filepath(&mut self, path: impl AsRef<Path>) -> Handle<TextBuffer> {
        self.state
            .buffers
            .allocate(TextBuffer::from_filepath(path.as_ref()))
    }

    pub fn create_scratch_buffer(&mut self) -> Handle<TextBuffer> {
        self.state.buffers.allocate(TextBuffer::new_empty())
    }

    pub fn edit_buffer(&mut self, buffer: Handle<TextBuffer>) {
        self.editor = TextEditor::new();
        self.state.active_buffer_handle = buffer;
    }

    pub fn save_buffer(&mut self, buffer: Handle<TextBuffer>) {
        self.state.buffers.get(buffer).save().unwrap();
    }

    pub fn input(&mut self, input: Input) {
        // TODO convert input mapping so it is done outside of panels, more globally. and configurable!

        if self.mode_line.has_focus() {
            self.input_mode_line(input);
        } else if input.key == Key::Char(':') && self.editor.is_command_mode() {
            self.mode_line.set_has_focus(true);
        } else if input == Input::parse("w").unwrap() && self.editor.is_command_mode() {
            self.warpdrive_panel = self.make_warp_drive_panel();
            return;
        } else if self.warpdrive_panel.is_some() {
            if let Some(command) = self.input_warpdrive(input) {
                self.execute_command_active_editor(command);
            }
        } else {
            self.input_editor(input);
        }
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        self.state.viewport_size
    }

    pub fn set_viewport_size(&mut self, viewport_size: (u32, u32)) {
        self.state.viewport_size = viewport_size;
    }

    fn make_warp_drive_panel(&mut self) -> Option<WarpDrivePanel> {
        let ui_panel = self.render_editor();
        let text_content = ui_panel.content;
        let position_offset = self.editor.view_top_left_position().to_offset();
        WarpDrivePanel::new(text_content, position_offset)
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
                self.save_buffer(self.state.active_buffer_handle);
            }
            "wq" | "write-quit" => {
                self.save_buffer(self.state.active_buffer_handle);
                self.request_quit();
            }
            _ => panic!("unknown command: {}", command_str),
        }
    }

    fn input_mode_line(&mut self, input: Input) {
        let maybe_line = self.mode_line.input(input, &mut self.state);

        if let Some(line) = maybe_line {
            self.mode_line.set_has_focus(false);
            self.interpret_command(&line);
        }
    }

    fn input_editor(&mut self, input: Input) {
        for command in self.editor.convert_input_to_command(input, &self.state) {
            self.execute_command_active_editor(command);
        }
    }

    fn execute_command_active_editor(&mut self, command: Command) {
        self.editor.execute_command(command, &mut self.state);
    }

    fn input_warpdrive(&mut self, input: Input) -> Option<Command> {
        let wdp = if let Some(wdp) = &mut self.warpdrive_panel {
            wdp
        } else {
            return None;
        };

        let mut maybe_cmd = None;
        for command in wdp.convert_input_to_command(input, &mut self.state) {
            if let Some(cmd) = wdp.execute_command(command, &mut self.state) {
                maybe_cmd = Some(cmd);
            }
        }
        if maybe_cmd.is_some() {
            self.warpdrive_panel = None
        }
        maybe_cmd
    }

    pub fn render(&mut self) -> UiState {
        let editor_panel = self.render_editor();

        let infos = self.mode_line_infos();
        self.state.mode_line_infos.infos = infos;

        let mode_line_panel = self.render_mode_line();
        let mut panels = vec![editor_panel, mode_line_panel];

        if self.warpdrive_panel.is_some() {
            let wdp_panel = self.render_warpdrive_panel();
            panels.push(wdp_panel);
        }

        UiState { panels }
    }

    fn render_editor(&mut self) -> UiPanel {
        let rect = self.compute_editor_rect();
        // TODO rect the editor plz
        self.editor.render(&self.state)
    }

    fn render_warpdrive_panel(&mut self) -> UiPanel {
        let viewport_size = self.compute_editor_rect();
        // TODO rect the warpdrive plz
        self.warpdrive_panel.as_mut().unwrap().render(&self.state)
    }

    fn render_mode_line(&mut self) -> UiPanel {
        self.mode_line.set_rect(self.compute_mode_line_rect());
        let mut panel = self.mode_line.render(&self.state);
        panel.position.1 = self.state.viewport_size.1 - 1;
        panel
    }

    fn compute_editor_rect(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.state.viewport_size.0,
            self.state.viewport_size.1 - 1,
        )
    }

    fn compute_mode_line_rect(&self) -> Rect {
        Rect::new(
            0,
            self.state.viewport_size.1.saturating_sub(1),
            self.state.viewport_size.0,
            1,
        )
    }

    fn mode_line_infos(&mut self) -> Vec<ModeLineInfo> {
        self.editor.mode_line_infos(&self.state)
    }
}

pub struct EditorContextMut<'a> {
    pub buffer: &'a mut TextBuffer,
    pub viewport_size: (u32, u32),
}
