use crate::{
    command::{self, CommandQueue, CommandRegistry, ExecuteCommandContext, parse_command},
    config,
    event::EventRegistry,
    input::Input,
    panels::{self, Panels},
    state::State,
    ui::{Rect, Size, ui_state::UiState},
};

#[derive(Default)]
pub struct Core {
    pub events: EventRegistry,
    pub commands: CommandRegistry,
    pub queue: CommandQueue,
    pub state: State,
    pub panels: Panels,
}

impl Core {
    pub fn with_builtins() -> Self {
        let mut this = Self::default();
        command::commands::register_builtin_commands(&mut this.commands, &mut this.events);

        config::commands::register_builtin_commands(&mut this.commands, &mut this.events);
        this.state.config = config::make_builtin_config();

        panels::warpdrive::commands::register_warpdrive_commands(
            &mut this.commands,
            &mut this.events,
        );

        this.events.emit("started", "");
        this.tick();

        this
    }

    pub fn queue_command(&mut self, command: String) {
        self.queue.push(command)
    }

    pub fn emit_input_event(&mut self, input: Input) {
        self.state.last_input = Some(input);
        self.events.emit("input", input.to_string());
    }

    pub fn quit_requested(&self) -> bool {
        self.state.quit_requested
    }

    pub fn viewport_size(&self) -> Size {
        self.state.viewport_size
    }

    pub fn set_viewport_size(&mut self, size: Size) {
        self.update_viewport_size(size);
        self.events
            .emit("resized", format!("{} {}", size.column, size.row));

        self.tick();
    }

    pub fn tick(&mut self) {
        self.state.modeline.clear_content_override();

        loop {
            self.queue
                .extend(self.events.emitted_commands(&self.state.config));

            let Some(command) = self.queue.pop() else {
                break;
            };

            self.queue.start_scope();

            let res = self.commands.execute_command(
                &command,
                ExecuteCommandContext {
                    events: &mut self.events,
                    queue: &mut self.queue,
                    state: &mut self.state,
                    panels: &mut self.panels,
                },
            );

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

        // eprintln!("{}", self.queue.take_debug_log()); // DEBUG

        self.state.fill_modeline_infos();

        // Updating the viewport is needed here since the size of some panels
        // (ex: line numbers) depends on the contents, which might have been
        // modified.
        self.update_viewport_size(self.state.viewport_size);
    }

    pub fn render(&mut self) -> UiState {
        let mut panels = vec![
            self.panels.editor.render(&self.state),
            self.panels.line_numbers.render(&self.state),
            self.panels.modeline.render(&self.state),
        ];

        if let Some(ui_panel) = self.panels.warpdrive.render(&self.state) {
            panels.push(ui_panel);
        }

        let mode = self.state.config.state_value("mode");
        let show_combo = mode.is_some_and(|m| m.starts_with("combo-"));
        if show_combo {
            panels.push(self.panels.combo.render(&self.state));
        }

        UiState { panels }
    }

    fn update_viewport_size(&mut self, viewport_size: Size) {
        self.state.viewport_size = viewport_size;

        let line_numbers_width = self.panels.line_numbers.required_width(&self.state);
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
        self.state.editor_size = self.panels.editor.rect().size();

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
    }
}

// todo undo/redo
// rearch???

// event driven, command queue
// pure data (buffers, selections, undoredo states, ...)?
// command queue that can be pushed to by commands,
//   prepopulated every tick by registered tick commands?

// input event issued
// -> input mapping, according to active buffer config state
//    command event issued (scoped to active buffer view?)
//    ->
// .

// UI: still a collection of panels.
//     One panel has focus at a time.
//     Having focus determines active view.
//     Keymap per panel kind, per control kind, per mode
//     Keymap maps an Input to a command.
// Modeline:
//     On insert \n, queue buffer content as command.
//     ...
// How panels are rendered:
//     Just have hardcoded panel types aggregate named State::panels
//     and do as old core does?

// [global]
//     [panel (or control?) focus]

// [event] input a
// -> [hook] insert a  # implicitly acts on active view
//     -> [queue] insert-at-sel -sel=0 a  # idem
//         -> [queue] fix-sels -span=(0:0,1:0) insert
//     -> [queue] insert-at-sel -sel=1 a
//         -> [queue] fix-sels -span=(0:0,1:0) insert
//     -> [queue] insert-at-sel -sel=2 a
//         -> [queue] fix-sels -span=(0:0,1:0) insert
