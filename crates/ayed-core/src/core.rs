use crate::{
    command::{self, CommandQueue, CommandRegistry, ExecuteCommandContext, parse_command},
    commands, config,
    input::Input,
    panels::{self, Panels, RenderPanelContext},
    state::{Resources, State},
    ui::{Rect, Size, ui_state::UiState},
};

#[derive(Default)]
pub struct Core {
    pub commands: CommandRegistry,
    pub queue: CommandQueue,
    pub state: State,
    pub resources: Resources,
    pub panels: Panels,
}

impl Core {
    pub fn with_builtins() -> Self {
        let mut this = Self::default();

        this.register_builtin_events();

        commands::register_builtin_commands(&mut this.commands);

        this.state.config = config::make_builtin_config();

        panels::warpdrive::commands::register_warpdrive_commands(&mut this.commands);

        this.queue_command("started".to_string());
        this.tick();

        this
    }

    pub fn queue_command(&mut self, command: String) {
        self.queue.push(command)
    }

    pub fn emit_input_event(&mut self, input: Input) {
        self.state.last_input = Some(input);
        self.queue_command(format!("input {input}"));
    }

    pub fn quit_requested(&self) -> bool {
        self.state.quit_requested
    }

    pub fn viewport_size(&self) -> Size {
        self.state.viewport_size
    }

    pub fn set_viewport_size(&mut self, size: Size) {
        self.update_viewport_size(size);
        self.queue_command(format!("resized {} {}", size.column, size.row));

        self.tick();
    }

    pub fn tick(&mut self) {
        self.state.modeline.clear_content_override();

        loop {
            let Some(command) = self.queue.pop() else {
                break;
            };

            self.queue.start_scope();

            let res = self.commands.execute_command(
                &command,
                ExecuteCommandContext {
                    queue: &mut self.queue,
                    state: &mut self.state,
                    resources: &mut self.resources,
                    panels: &mut self.panels,
                },
            );

            let hooks = self.hooks_of_command(&command);
            // If the command isn't registered, but it has hooks, it is likely
            // an event and not and error.
            if !res.is_err() {
                self.queue.extend(hooks);
            }

            match res {
                Ok(Err(cmd_err)) => {
                    self.queue.clear();
                    let (command_name, _) = parse_command(&command);
                    let err_msg = format!("{command_name}: {cmd_err}");
                    self.state.modeline.set_error(err_msg);
                    return;
                }
                Err(exec_err) => {
                    self.queue.clear();
                    self.state.modeline.set_error(exec_err);
                    return;
                }
                _ => (),
            }
        }

        if self.state.config.state_value("cmdlog") == Some("true") {
            eprintln!("{}", self.queue.take_debug_log());
        }

        self.state.fill_modeline_infos(&self.resources);

        self.queue.clear();

        // Updating the viewport is needed here since the size of some panels
        // (ex: line numbers) depends on the contents, which might have been
        // modified.
        self.update_viewport_size(self.state.viewport_size);
    }

    pub fn render(&mut self) -> UiState {
        let render_ctx = RenderPanelContext {
            state: &self.state,
            resources: &self.resources,
        };

        let mut panels = vec![
            self.panels.editor.render(&render_ctx),
            self.panels.line_numbers.render(&render_ctx),
            self.panels.modeline.render(&render_ctx),
        ];

        if let Some(ui_panel) = self.panels.warpdrive.render(&render_ctx) {
            panels.push(ui_panel);
        }

        if let Some(suggestion_panel) = self.panels.suggestion.render(&render_ctx) {
            panels.push(suggestion_panel);
        }

        let mode = self.state.config.state_value("mode");
        let show_combo = mode.is_some_and(|m| m.starts_with("combo-"));
        if show_combo {
            panels.push(self.panels.combo.render(&self.state));
        }

        UiState { panels }
    }

    fn register_builtin_events(&mut self) {
        self.commands.register_event("started");
        self.commands.register_event("resized");
        self.commands.register_event("input");
        self.commands.register_event("buffer-opened");
        self.commands.register_event("buffer-modified");
        self.commands.register_event("selections-modified");
    }

    fn hooks_of_command(&mut self, command: &str) -> Vec<String> {
        let mut acc = Vec::new();
        let (command_name, command_options) = parse_command(&command);
        let hooks_map = self.state.config.get("hooks");
        let hooks = hooks_map.and_then(|h| h.get(command_name));
        if let Some(hooks) = hooks {
            for command in hooks {
                if command.contains(' ') {
                    acc.push(format!("{}", command));
                } else {
                    acc.push(format!("{} {}", command, command_options));
                }
            }
        }
        acc
    }

    fn update_viewport_size(&mut self, viewport_size: Size) {
        self.state.viewport_size = viewport_size;

        let render_ctx = RenderPanelContext {
            state: &self.state,
            resources: &self.resources,
        };

        let line_numbers_width = self.panels.line_numbers.required_width(&render_ctx);
        let editor_width = self
            .state
            .viewport_size
            .column
            .saturating_sub(line_numbers_width);
        let editor_height = self.state.viewport_size.row.saturating_sub(1);

        self.panels.editor.set_rect(Rect::new(
            line_numbers_width,
            0,
            editor_width,
            editor_height,
        ));
        self.state.editor_rect = self.panels.editor.rect();

        self.panels.warpdrive.set_rect(self.panels.editor.rect());
        self.panels.combo.set_rect(self.panels.editor.rect());

        self.panels
            .line_numbers
            .set_rect(Rect::new(0, 0, line_numbers_width, editor_height));

        self.panels.modeline.set_rect(Rect::new(
            0,
            self.state.viewport_size.row.saturating_sub(1),
            self.state.viewport_size.column,
            1,
        ));
        self.state.modeline_rect = self.panels.modeline.rect();
    }
}
