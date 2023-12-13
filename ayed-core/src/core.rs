use std::collections::HashMap;
use std::path::Path;

use crate::arena::Arena;
use crate::buffer::TextBuffer;
use crate::command::{Command, CoreCommand, EditorCommand};
use crate::config::make_config;
use crate::highlight::regex_syntax_highlight;
use crate::input::{Input, Key, Modifiers};
use crate::input_manager::{initialize_input_manager, InputManager};
use crate::panels::{
    combo_panel::{ComboInfo, ComboInfos, ComboPanel},
    mode_line::{self, Align, ModeLine, ModeLineInfo},
    text_editor::TextEditor,
    warpdrive::WarpDrive,
};
use crate::scripted_command::ScriptedCommand;
use crate::state::{Buffers, Editors, State};
use crate::ui_state::{Color, Style, UiPanel, UiState};
use crate::utils::{Rect, Size};

pub struct Core {
    state: State,
    input_manager: InputManager,
    mode_line: ModeLine,
    warpdrive: Option<WarpDrive>,
    combo_panel: Option<ComboPanel>,
    last_input: Input,
    deferred_commands: Vec<Command>,
    scripted_commands: HashMap<String, ScriptedCommand>,
}

impl Core {
    pub fn new() -> Self {
        let mut buffers_arena = Arena::new();
        let active_buffer_handle = buffers_arena.allocate(TextBuffer::new_empty());

        let mut editors_arena = Arena::new();
        let active_editor_handle = editors_arena.allocate(TextEditor::new(active_buffer_handle));

        let state = State {
            buffers: Buffers {
                buffers_arena,
                active_buffer_handle,
                highlights: Default::default(),
            },
            editors: Editors {
                editors_arena,
                active_editor_handle,
            },
            viewport_size: (80, 25).into(),
            mode_line_infos: Default::default(),
            //
            active_combo_mode_name: None,
            active_editor_name: "text".to_string(),
            active_mode_name: "command".to_string(),
            //
            config: make_config(),
            //
            quit: false,
        };

        let mode_line = ModeLine::new();

        let mut this = Self {
            state,
            input_manager: initialize_input_manager(),
            mode_line,
            warpdrive: None,
            combo_panel: None,
            last_input: Input {
                key: Key::Char('\0'),
                modifiers: Modifiers::default(),
            },
            deferred_commands: Default::default(),
            scripted_commands: Default::default(),
        };

        this.scripted_commands.insert(
            "conf".into(),
            ScriptedCommand::new(|state, _args| {
                dbg!(state.extract_applied_config());
                Ok(())
            }),
        );
        this.scripted_commands.insert(
            "builtin-syntax-highlight".into(),
            ScriptedCommand::new(|state, _args| {
                let config = state.extract_applied_config();
                let Some(syntax) = config.get("syntax") else {
                    return Ok(());
                };
                let Some(syntax_style) = config.get("syntax-style") else {
                    return Ok(());
                };
                let spans =
                    regex_syntax_highlight(state.buffers.active_buffer(), &syntax, &syntax_style);
                state
                    .buffers
                    .highlights
                    .insert(state.buffers.active_buffer_handle, spans);
                Ok(())
            }),
        );
        this.scripted_commands.insert(
            "builtin-auto-indent".into(),
            ScriptedCommand::new(|_state, args| {
                let Some(chr) = args.chars().next() else {
                    return Err("expected char argument".to_string());
                };
                if chr != '\n' {
                    return Ok(());
                }

                dbg!("TODO auto indent plz");

                Ok(())
            }),
        );

        this.state.set_active_editor(active_editor_handle);

        this
    }

    pub fn is_quit(&self) -> bool {
        self.state.quit
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
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

        self.run_hooks_modify_buffer();
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
                    self.state.set_active_editor_mode(mode);
                    self.execute_command_in_editor(EditorCommand::Noop); // NOTE I think this is to force an immediate update?
                }
                SetComboMode(mode) => {
                    if !self.input_manager.combo_mapping(&mode).is_empty() {
                        self.set_combo_mode(Some(mode));
                    } else {
                        self.set_mode_line_error(format!("combo mode '{mode}' is empty"));
                    }
                }
                EditFile(filepath) => {
                    let buffer = self.state.get_buffer_from_filepath(filepath);
                    self.state.edit_buffer(buffer);
                    self.run_hooks_modify_buffer(); // TODO maybe change this to an open-buffer hook?
                }
                WriteBuffer => {
                    self.state.save_buffer(self.state.active_buffer_handle());
                    let path = self
                        .state
                        .buffers
                        .active_buffer()
                        .filepath()
                        .map(Path::to_string_lossy)
                        .unwrap_or_default();
                    self.set_mode_line_message(format!("saved as {path}"));
                }
                WriteBufferQuit => {
                    self.state.save_buffer(self.state.active_buffer_handle());
                    self.state.request_quit();
                }
                Quit => {
                    self.state.request_quit();
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
            Command::ScriptedCommand(scripted_command) => {
                let (name, args) = scripted_command
                    .split_once(' ')
                    .unwrap_or((scripted_command.as_str(), ""));
                if let Some(sc) = self.scripted_commands.get_mut(name) {
                    match sc.call(&mut self.state, args) {
                        Ok(()) => (),
                        Err(err_msg) => self.set_mode_line_error(format!("{name}: {err_msg}")),
                    }
                } else {
                    self.set_mode_line_error(format!("unknown scripted command: {name}"));
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
            "rsc" => {
                // DEBUG Run Scripted Command
                // this is temporary. If you see this and it looks useless, delete it
                // Could all prompt commands just be scripted commands?
                let mut cmd_string = String::new();
                for (i, part) in parts.enumerate() {
                    if i != 0 {
                        cmd_string.push(' ');
                    }
                    cmd_string.push_str(part);
                }
                Command::ScriptedCommand(cmd_string)
            }
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
                    ..Default::default()
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
                    ..Default::default()
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
        let maybe_line = self.mode_line.execute_command(command);

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
        let editor = self.state.editors.active_editor_mut();
        let buffer = editor.get_buffer_mut(&mut self.state.buffers.buffers_arena);
        editor.execute_command(command, buffer);

        if let EditorCommand::Insert(chr) = command {
            self.run_hooks_after_insert(chr);
        }
    }

    pub fn viewport_size(&self) -> Size {
        self.state.viewport_size
    }

    pub fn set_viewport_size(&mut self, viewport_size: Size) {
        self.state.viewport_size = viewport_size;
    }

    fn make_warp_drive_panel(&mut self) -> Option<WarpDrive> {
        let ui_panel = self
            .render_editor()
            .into_iter()
            .next()
            .expect("the editor panel should be there, first in the list");
        let text_content = ui_panel.content;
        let position_offset = self
            .state
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
        if let Some(cmd) = wdp.execute_command(command) {
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
        panels.extend(editor_panel);

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

    fn render_editor(&mut self) -> Vec<UiPanel> {
        let rect = self.compute_editor_rect();
        let active_editor = self.state.editors.active_editor_mut();
        let highlights = self
            .state
            .buffers
            .highlights
            .get(&active_editor.buffer_handle())
            .map(|v| &v[..])
            .unwrap_or(&[]);
        let buffer = active_editor.get_buffer(&self.state.buffers.buffers_arena);
        active_editor.set_rect(rect);
        active_editor.render(buffer, highlights)
    }

    fn render_warpdrive_panel(&mut self) -> UiPanel {
        let active_editor = self.state.editors.active_editor_mut();
        let editor_rect = active_editor.inner_rect();
        self.warpdrive.as_mut().unwrap().render(editor_rect)
    }

    fn render_mode_line(&mut self) -> UiPanel {
        self.mode_line.set_rect(self.compute_mode_line_rect());

        self.mode_line.render(&self.state)
    }

    fn compute_editor_rect(&self) -> Rect {
        Rect::new(
            0,
            0,
            self.viewport_size().column,
            self.viewport_size().row.saturating_sub(1),
        )
    }

    fn compute_mode_line_rect(&self) -> Rect {
        Rect::new(
            0,
            self.viewport_size().row.saturating_sub(1),
            self.viewport_size().column,
            1,
        )
    }

    fn mode_line_infos(&mut self) -> Vec<ModeLineInfo> {
        let filepath_text = if let Some(path) = self.state.buffers.active_buffer().filepath() {
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

    fn run_hooks_modify_buffer(&mut self) {
        let active_buffer = self.state.buffers.active_buffer_mut();
        if !active_buffer.modified {
            return;
        }
        active_buffer.modified = false;

        let config = self.state.extract_applied_config();
        let Some(cmds) = config
            .get("hooks")
            .and_then(|hooks| hooks.get("modify-buffer"))
        else {
            return;
        };

        for command in cmds.iter().cloned() {
            self.execute_command(Command::ScriptedCommand(command));
        }
    }

    fn run_hooks_after_insert(&mut self, chr: char) {
        let config = self.state.extract_applied_config();
        let Some(cmds) = config
            .get("hooks")
            .and_then(|hooks| hooks.get("after-insert"))
        else {
            return;
        };

        for mut command in cmds.iter().cloned() {
            command.push(' ');
            command.push(chr);
            self.execute_command(Command::ScriptedCommand(command));
        }
    }
}
