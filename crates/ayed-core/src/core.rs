use log::debug;

use crate::{
    command::{CommandQueue, CommandRegistry, ExecuteCommandContext, parse_command},
    commands, config,
    input::Input,
    logger::Logger,
    panels::{self, Editor, FilePicker, PanelContext, Panels, Prompt},
    state::{Resources, State},
    ui::{Size, ui_state::UiState},
};

#[derive(Default)]
pub struct Core {
    pub commands: CommandRegistry,
    pub queue: CommandQueue,
    pub state: State,
    pub resources: Resources,
    pub panels: Panels,
    pub delta_time_acc: f32,
}

impl Core {
    pub fn with_builtins() -> Self {
        Logger::init().unwrap();

        let mut this = Self::default();

        this.register_builtin_events();

        commands::register_builtin_commands(&mut this.commands);

        this.state.config = config::make_builtin_config();

        panels::warpdrive::commands::register_warpdrive_commands(&mut this.commands);
        panels::file_picker::commands::register_file_picker_commands(&mut this.commands);

        this.state.working_directory = std::env::current_dir().unwrap();

        // // DEBUG DEBUG DEBUG
        // let make_item = |s: &str| CompletionItem {
        //     label: s.to_string(),
        //     edit: CompletionEdit {
        //         range: (Position::ZERO, Position::ZERO),
        //         text: s.to_string(),
        //     },
        //     extra_edits: vec![],
        // };
        // this.state.completions.items = vec![
        //     make_item("ahow"),
        //     make_item("beeboo"),
        //     make_item("cachow"),
        //     make_item("dabidibum"),
        //     make_item("fahow"),
        //     make_item("gbeeboo"),
        //     make_item("hcachow"),
        //     make_item("idabidibum"),
        //     make_item("jahow"),
        //     make_item("kbeeboo"),
        //     make_item("lcachow"),
        //     make_item("mdabidibum"),
        //     make_item("ahow"),
        //     make_item("beeboo"),
        //     make_item("cachow"),
        //     make_item("dabidibum"),
        //     make_item("fahow"),
        //     make_item("gbeeboo"),
        //     make_item("hcachow"),
        //     make_item("idabidibum"),
        //     make_item("jahow"),
        //     make_item("kbeeboo"),
        //     make_item("lcachow"),
        //     make_item("mdabidibum"),
        //     make_item("ahow"),
        //     make_item("beeboo"),
        //     make_item("cachow"),
        //     make_item("dabidibum"),
        //     make_item("fahow"),
        //     make_item("gbeeboo"),
        //     make_item("hcachow"),
        //     make_item("idabidibum"),
        //     make_item("jahow"),
        //     make_item("kbeeboo"),
        //     make_item("lcachow"),
        //     make_item("mdabidibum"),
        // ];

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

    pub fn take_is_async_task_ready(&self) -> bool {
        self.state
            .is_async_task_ready
            .swap(false, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn viewport_size(&self) -> Size {
        self.state.viewport_size
    }

    pub fn set_viewport_size(&mut self, size: Size) {
        self.update_viewport_size(size);
        self.queue_command(format!("resized {} {}", size.column, size.row));

        self.tick();
    }

    pub fn time_tick(&mut self, delta_time: f32) {
        const TICK_EVENT_DELAY: f32 = 0.06;
        self.delta_time_acc += delta_time;
        if self.delta_time_acc >= TICK_EVENT_DELAY {
            self.state.delta_time = self.delta_time_acc;
            self.delta_time_acc = 0.0;
            self.queue_command(format!("time-tick {delta_time}"));
        }
    }

    pub fn tick(&mut self) {
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
            if !res.unknown {
                self.queue.extend(hooks);
            }

            let (command_name, _) = parse_command(&command);

            if command_name == "input" {
                self.state.modeline.clear_content_override();
                self.state.hover_info = None;
            }

            if let Err(err_str) = res.output {
                let err_msg;
                if res.unknown {
                    err_msg = err_str;
                } else {
                    err_msg = format!("{command_name}: {err_str}");
                }
                self.queue.clear();
                self.state.modeline.set_error(err_msg, &self.state.config);
                return;
            }
        }

        if self.state.config.state_value("cmdlog") == Some("true") {
            if let Some(debug_log) = self.queue.take_debug_log() {
                debug!("{}", debug_log);
            }
        }

        self.state.fill_modeline_infos(&self.resources);

        self.queue.clear();

        // Updating the viewport is needed here since the size of some panels
        // (ex: line numbers) depends on the contents, which might have been
        // modified.
        self.update_viewport_size(self.state.viewport_size);
    }

    pub fn render(&mut self) -> UiState {
        let mut ctx = PanelContext {
            state: &mut self.state,
            resources: &mut self.resources,
        };

        let ui_panels = self.panels.render(&mut ctx);
        UiState { panels: ui_panels }
    }

    fn register_builtin_events(&mut self) {
        self.commands.register_event("started");
        self.commands.register_event("resized");
        self.commands.register_event("input");
        self.commands.register_event("time-tick");
        self.commands.register_event("buffer-opened");
        self.commands.register_event("buffer-modified");
        self.commands.register_event("buffer-closed");
        self.commands.register_event("selections-modified");
        self.commands.register_event("completion-sources-modified");
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

        self.panels.compute_layout(viewport_size);

        let editor_panel = self.panels.panel_of_type::<Editor>().unwrap();
        self.state.editor_rect = editor_panel.rect();

        let ctx = PanelContext {
            state: &mut self.state,
            resources: &mut self.resources,
        };
        self.state.editor_line_numbers_width = editor_panel.line_numbers_width(&ctx);

        self.state.file_picker_rect = self.panels.panel_of_type::<FilePicker>().unwrap().rect();
        self.state.modeline_rect = self.panels.panel_of_type::<Prompt>().unwrap().rect();

        // Needed for positioning Completion panel.
        self.state.focused_panel_view_rect =
            self.state.focused_view_rect(&self.panels, &self.resources);
    }
}
