use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::buffer::TextBuffer;
use crate::combo_panel::{ComboInfo, ComboInfos, ComboPanel};
use crate::command::{Command, CoreCommand, EditorCommand};
use crate::input::{Input, Key, Modifiers};
use crate::input_manager::{initialize_input_manager, InputManager};
use crate::mode_line::{self, Align, ModeLine, ModeLineInfo};
use crate::state::State;
use crate::text_editor::TextEditor;
use crate::ui_state::{Color, Style, UiPanel, UiState};
use crate::utils::{Rect, Size};
use crate::warpdrive::WarpDrive;

pub struct Core {
    state: State,
    input_manager: InputManager,
    editors: Editors,
    mode_line: ModeLine,
    warpdrive: Option<WarpDrive>,
    combo_panel: Option<ComboPanel>,
    quit: bool,
    last_input: Input,
    deferred_commands: Vec<Command>,
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
            viewport_size: (80, 25).into(),
            mode_line_infos: Default::default(),
            //
            active_combo_mode_name: None,
            active_editor_name: "text".to_string(),
            active_mode_name: "command".to_string(),
        };

        let mode_line = ModeLine::new();

        let mut this = Self {
            state,
            input_manager: initialize_input_manager(),
            editors: Editors {
                editors: editors_arena,
                active_editor,
            },
            mode_line,
            warpdrive: None,
            combo_panel: None,
            quit: false,
            last_input: Input {
                key: Key::Char('\0'),
                modifiers: Modifiers::default(),
            },
            deferred_commands: Default::default(),
        };
        this.set_active_editor(active_editor);
        this
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

    pub fn set_active_editor(&mut self, editor: Handle<TextEditor>) {
        self.editors.active_editor = editor;

        let active_buffer = self.editors.active_editor().buffer();
        let active_editor_mode = self.editors.active_editor().mode();

        self.state.active_buffer_handle = active_buffer;
        self.state.active_mode_name = active_editor_mode.to_owned();
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

        self.set_active_editor(editor);
    }

    pub fn save_buffer(&mut self, buffer: Handle<TextBuffer>) {
        self.state.buffers.get(buffer).save().unwrap();
    }

    pub fn input(&mut self, input: Input) {
        self.clear_mode_line();

        self.last_input = input.normalized();

        let commands = if self.mode_line.has_focus() || self.warpdrive.is_some() {
            self.input_manager
                .convert_input_with_editor_mode(input, "control", "line", &self.state)
        } else {
            self.input_manager.convert_input(input, &self.state)
        };

        if commands.is_empty() && self.combo_panel.is_some() {
            self.set_combo_mode(None);
        }

        for command in commands {
            self.set_combo_mode(None);
            self.execute_command(command);
        }

        self.execute_deferred_commands();
    }

    pub fn execute_command(&mut self, command: Command) {
        use CoreCommand::*;
        match command {
            Command::Core(core_command) => match core_command {
                ShowModeLinePrompt => self.mode_line.set_has_focus(true),
                ShowWarpdrive => {
                    self.warpdrive = self.make_warp_drive_panel();
                }
                SetEditorMode(mode) => {
                    self.state.active_mode_name = mode.clone();
                    self.editors.active_editor_mut().set_mode(mode);
                    self.execute_command_in_editor(EditorCommand::Noop);
                }
                SetComboMode(mode) => {
                    if !self.input_manager.combo_mapping(&mode).is_empty() {
                        self.set_combo_mode(Some(mode));
                    } else {
                        self.set_mode_line_error(format!("combo mode '{mode}' is empty"));
                    }
                }
                EditFile(filepath) => {
                    let buffer = self.get_buffer_from_filepath(filepath);
                    self.edit_buffer(buffer);
                }
                WriteBuffer => {
                    self.save_buffer(self.state.active_buffer_handle);
                    let path = self
                        .state
                        .buffers
                        .get(self.state.active_buffer_handle)
                        .filepath()
                        .map(Path::to_string_lossy)
                        .unwrap_or_default();
                    self.set_mode_line_message(format!("saved as {path}"));
                }
                WriteBufferQuit => {
                    self.save_buffer(self.state.active_buffer_handle);
                    self.request_quit();
                }
                Quit => {
                    self.request_quit();
                }
            },
            Command::Editor(editor_command) => {
                if self.mode_line.has_focus() {
                    self.input_mode_line(editor_command);
                } else if self.warpdrive.is_some() {
                    if let Some(wcmd) = self.input_warpdrive(editor_command) {
                        self.execute_command_in_editor(wcmd);
                    }
                } else {
                    self.execute_command_in_editor(editor_command)
                }
            }
        }
    }

    fn convert_prompt_command_to_command(
        &self,
        command_str: &str,
    ) -> Option<Result<Command, String>> {
        let mut parts = command_str.split(' ');
        let command = parts.next().expect("command expected");
        Some(Ok(match command {
            "" => return None,
            "q" | "quit" => Command::Core(CoreCommand::Quit),
            "e" | "edit" => {
                let filename = match parts.next() {
                    None | Some("") => return Some(Err(format!("filename expected"))),
                    Some(s) => s.to_string(),
                };
                Command::Core(CoreCommand::EditFile(filename))
            }
            "w" | "write" => Command::Core(CoreCommand::WriteBuffer),
            "wq" | "write-quit" => Command::Core(CoreCommand::WriteBufferQuit),
            _ => return Some(Err(format!("unknown command: {}", command_str))),
        }))
    }

    fn set_mode_line_error(&mut self, error_message: impl Into<String>) {
        self.mode_line
            .set_content_override(Some(mode_line::ContentOverride {
                text: error_message.into(),
                style: Style {
                    foreground_color: None,
                    background_color: Some(crate::theme::colors::ERROR_DARK),
                    invert: false,
                },
            }));
    }

    fn set_mode_line_message(&mut self, message: impl Into<String>) {
        self.mode_line
            .set_content_override(Some(mode_line::ContentOverride {
                text: message.into(),
                style: Style {
                    foreground_color: Some(mode_line::FG_COLOR),
                    background_color: Some(mode_line::BG_COLOR),
                    invert: false,
                },
            }));
    }

    fn clear_mode_line(&mut self) {
        self.mode_line.set_content_override(None);
    }

    fn set_combo_mode(&mut self, mode: Option<String>) {
        if let Some(mode) = mode {
            let give_this_an_actual_name = self.input_manager.combo_mapping(&mode);
            self.combo_panel = Some(ComboPanel::new(ComboInfos {
                infos: give_this_an_actual_name
                    .into_iter()
                    .map(|(input, description)| ComboInfo { input, description })
                    .collect(),
            }));
            self.state.active_combo_mode_name = Some(mode);
        } else {
            self.combo_panel = None;
            self.state.active_combo_mode_name = None;
        }
    }

    fn input_mode_line(&mut self, command: EditorCommand) {
        let maybe_line = self.mode_line.execute_command(command, &mut self.state);

        if let Some(line) = maybe_line {
            self.mode_line.set_has_focus(false);
            if let Some(convert_result) = self.convert_prompt_command_to_command(&line) {
                match convert_result {
                    Ok(command) => self.defer_command(command),
                    Err(err_msg) => self.set_mode_line_error(err_msg),
                }
            }
        }
    }

    fn execute_command_in_editor(&mut self, command: EditorCommand) {
        self.editors
            .active_editor_mut()
            .execute_command(command, &mut self.state);
    }

    pub fn viewport_size(&self) -> Size {
        self.state.viewport_size
    }

    pub fn set_viewport_size(&mut self, viewport_size: Size) {
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

    fn input_warpdrive(&mut self, command: EditorCommand) -> Option<EditorCommand> {
        let wdp = if let Some(wdp) = &mut self.warpdrive {
            wdp
        } else {
            return None;
        };

        let mut maybe_cmd = None;
        if let Some(cmd) = wdp.execute_command(command, &mut self.state) {
            maybe_cmd = Some(cmd);
        }
        if maybe_cmd.is_some() {
            self.warpdrive = None
        }
        maybe_cmd
    }

    pub fn render(&mut self) -> UiState {
        let mut panels = Vec::new();

        let editor_panel = self.render_editor();
        panels.push(editor_panel);

        if self.warpdrive.is_some() {
            let wdp_panel = self.render_warpdrive_panel();
            panels.push(wdp_panel);
        }

        if let Some(combo_panel) = self.combo_panel.as_mut() {
            panels.push(combo_panel.render(&self.state));
        }

        let infos = self.mode_line_infos();
        self.state.mode_line_infos.infos = infos;
        let mode_line_panel = self.render_mode_line();
        panels.push(mode_line_panel);

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
            self.state.viewport_size.column,
            self.state.viewport_size.row - 1,
        )
    }

    fn compute_mode_line_rect(&self) -> Rect {
        Rect::new(
            0,
            self.state.viewport_size.row.saturating_sub(1),
            self.state.viewport_size.column,
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

    fn defer_command(&mut self, command: Command) {
        self.deferred_commands.push(command);
    }

    fn execute_deferred_commands(&mut self) {
        for command in std::mem::take(&mut self.deferred_commands) {
            self.execute_command(command);
        }
    }
}

struct Editors {
    editors: Arena<TextEditor>,
    active_editor: Handle<TextEditor>, // Maybe this should be optional
}

impl Editors {
    pub fn active_editor(&self) -> &TextEditor {
        self.editors.get(self.active_editor)
    }

    pub fn active_editor_mut(&mut self) -> &mut TextEditor {
        self.editors.get_mut(self.active_editor)
    }
}
