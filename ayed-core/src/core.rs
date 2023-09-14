use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::buffer::TextBuffer;
use crate::command::EditorCommand;
use crate::input::{Input, Key, Modifiers};
use crate::input_manager::{initialize_input_manager, InputManager};
use crate::mode_line::{self, Align, ModeLine, ModeLineInfo};
use crate::state::State;
use crate::text_editor::TextEditor;
use crate::ui_state::{Color, Style, UiPanel, UiState};
use crate::utils::Rect;
use crate::warpdrive::WarpDrive;

pub struct Core {
    state: State,
    input_manager: InputManager,
    editors: Editors,
    mode_line: ModeLine,
    warpdrive: Option<WarpDrive>,
    quit: bool,
    last_input: Input,
}

impl Core {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let buffer = buffers.allocate(TextBuffer::new_empty());

        let mut editors_arena = Arena::new();
        let active_editor = editors_arena.allocate(TextEditor::new(buffer));

        let state = State {
            buffers,
            active_buffer_handle: buffer,
            viewport_size: (80, 25),
            mode_line_infos: Default::default(),
            //
            active_combo_mode_name: None,
            active_editor_name: "text",
            active_mode_name: "command",
        };

        let mode_line = ModeLine::new();

        Self {
            state,
            input_manager: initialize_input_manager(),
            editors: Editors {
                editors: editors_arena,
                active_editor,
            },
            mode_line,
            warpdrive: None,
            quit: false,
            last_input: Input {
                key: Key::Char('\0'),
                modifiers: Modifiers::default(),
            },
        }
    }

    pub fn is_quit(&self) -> bool {
        self.quit
    }

    pub fn request_quit(&mut self) {
        self.quit = true;
    }

    pub fn get_buffer_from_filepath(&mut self, path: impl AsRef<Path>) -> Handle<TextBuffer> {
        let path = path.as_ref();

        let alreay_opened_buffer = self.state.buffers.elements().find_map(|(hnd, buf)| {
            if let Some(f) = buf.filepath() {
                if f == path {
                    Some(hnd)
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(buffer) = alreay_opened_buffer {
            buffer
        } else {
            self.state
                .buffers
                .allocate(TextBuffer::from_filepath(path.as_ref()))
        }
    }

    pub fn create_scratch_buffer(&mut self) -> Handle<TextBuffer> {
        self.state.buffers.allocate(TextBuffer::new_empty())
    }

    pub fn edit_buffer(&mut self, buffer: Handle<TextBuffer>) {
        let maybe_preexisting_editor = self.editors.editors.elements().find_map(|(hnd, ed)| {
            if ed.buffer() == buffer {
                Some(hnd)
            } else {
                None
            }
        });

        let editor = if let Some(preexisting_editor) = maybe_preexisting_editor {
            preexisting_editor
        } else {
            self.editors.editors.allocate(TextEditor::new(buffer))
        };

        self.editors.active_editor = editor;
        self.state.active_buffer_handle = buffer;
    }

    pub fn save_buffer(&mut self, buffer: Handle<TextBuffer>) {
        self.state.buffers.get(buffer).save().unwrap();
    }

    pub fn input(&mut self, input: Input) {
        self.clear_mode_line_error();

        self.last_input = input.normalized();

        if self.mode_line.has_focus() {
            self.input_mode_line(input);
        } else if input.key == Key::Char(':') && self.editors.active_editor().is_command_mode() {
            self.mode_line.set_has_focus(true);
        } else if input == Input::parse("w").unwrap()
            && self.editors.active_editor().is_command_mode()
        {
            self.warpdrive = self.make_warp_drive_panel();
            return;
        } else if self.warpdrive.is_some() {
            if let Some(command) = self.input_warpdrive(input) {
                self.execute_command_in_editor(command);
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

    fn make_warp_drive_panel(&mut self) -> Option<WarpDrive> {
        let ui_panel = self.render_editor();
        let text_content = ui_panel.content;
        let position_offset = self
            .editors
            .active_editor()
            .view_top_left_position()
            .to_offset();
        WarpDrive::new(text_content, position_offset)
    }

    fn interpret_prompt_command(&mut self, command_str: &str) {
        let mut parts = command_str.split(' ');
        let command = parts.next().expect("command expected");
        match command {
            "" => (),
            "q" | "quit" => self.request_quit(),
            "e" | "edit" => {
                let arg = parts.next().expect("name expected");
                let buffer = self.get_buffer_from_filepath(arg);
                self.edit_buffer(buffer);
            }
            "w" | "write" => {
                self.save_buffer(self.state.active_buffer_handle);
            }
            "wq" | "write-quit" => {
                self.save_buffer(self.state.active_buffer_handle);
                self.request_quit();
            }
            _ => self.set_mode_line_error(format!("unknown command: {}", command_str)),
        }
    }

    fn set_mode_line_error(&mut self, error_message: String) {
        self.mode_line
            .set_content_override(Some(mode_line::ContentOverride {
                text: format!("error: {}", error_message),
                style: Style {
                    foreground_color: None,
                    background_color: Some(Color::rgb(48, 16, 16)),
                    invert: false,
                },
            }));
    }

    fn clear_mode_line_error(&mut self) {
        self.mode_line.set_content_override(None);
    }

    fn input_mode_line(&mut self, input: Input) {
        let commands = self.input_manager.convert_input_with_editor_mode(
            input,
            "control",
            "line",
            &self.state,
        );
        for command in commands {
            let maybe_line = self.mode_line.execute_command(command, &mut self.state);

            if let Some(line) = maybe_line {
                self.mode_line.set_has_focus(false);
                self.interpret_prompt_command(&line);
            }
        }
    }

    fn input_editor(&mut self, input: Input) {
        for command in self.input_manager.convert_input(input, &self.state) {
            match command {
                EditorCommand::ChangeMode(mode) => {
                    self.state.active_mode_name = mode;
                }
                _ => (),
            }
            self.execute_command_in_editor(command);
        }
    }

    fn execute_command_in_editor(&mut self, command: EditorCommand) {
        self.editors
            .active_editor_mut()
            .execute_command(command, &mut self.state);
    }

    fn input_warpdrive(&mut self, input: Input) -> Option<EditorCommand> {
        let wdp = if let Some(wdp) = &mut self.warpdrive {
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
            self.warpdrive = None
        }
        maybe_cmd
    }

    pub fn render(&mut self) -> UiState {
        let editor_panel = self.render_editor();

        let infos = self.mode_line_infos();
        self.state.mode_line_infos.infos = infos;

        let mode_line_panel = self.render_mode_line();
        let mut panels = vec![editor_panel, mode_line_panel];

        if self.warpdrive.is_some() {
            let wdp_panel = self.render_warpdrive_panel();
            panels.push(wdp_panel);
        }

        UiState { panels }
    }

    fn render_editor(&mut self) -> UiPanel {
        let rect = self.compute_editor_rect();
        self.editors.active_editor_mut().set_rect(rect);

        self.editors.active_editor_mut().render(&self.state)
    }

    fn render_warpdrive_panel(&mut self) -> UiPanel {
        self.warpdrive.as_mut().unwrap().render(&self.state)
    }

    fn render_mode_line(&mut self) -> UiPanel {
        self.mode_line.set_rect(self.compute_mode_line_rect());

        self.mode_line.render(&self.state)
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
        let filepath_text = if let Some(path) = self.state.active_buffer().filepath() {
            path.to_string_lossy().into_owned()
        } else {
            "*scratch*".to_string()
        };
        let file_info = ModeLineInfo {
            text: filepath_text,
            style: Style::default().with_foreground_color(Color::BLUE),
            align: Align::Right,
        };

        let mut input_text = String::new();
        self.last_input.serialize(&mut input_text);
        let input_info = ModeLineInfo {
            text: input_text,
            style: Style::default(),
            align: Align::Right,
        };

        let editor_mode_info = ModeLineInfo {
            text: format!(
                "{}/{}",
                self.state.active_editor_name, self.state.active_mode_name,
            ),
            style: Style::default(),
            align: Align::Left,
        };

        vec![editor_mode_info, input_info, file_info]
    }
}

struct Editors {
    editors: Arena<TextEditor>,
    active_editor: Handle<TextEditor>,
}

impl Editors {
    pub fn active_editor(&self) -> &TextEditor {
        self.editors.get(self.active_editor)
    }

    pub fn active_editor_mut(&mut self) -> &mut TextEditor {
        self.editors.get_mut(self.active_editor)
    }
}
