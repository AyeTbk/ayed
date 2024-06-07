use crate::{
    command::{self, parse_command, CommandQueue, CommandRegistry, ExecuteCommandContext},
    config,
    event::EventRegistry,
    input::Input,
    panels::Panels,
    state::State,
    ui::{ui_state::UiState, Rect, Size},
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
            // FIXME When a command both emits and event and also pushes commands,
            // its commands should all happen first (and then their commands and
            // events and so on), and then its events hooks should happen.
            // Right now, events always come first and it's already causing
            // problem when state-set'ing.
            // Currently visible when trying to get syntax highlight to show on
            // buffer-opened, doesn't happen 'cause the 'file' state isn't set
            // before said event hooks are executed.
            // I recommend giving more structure to the command queue, make it
            // a real 'stacked queue' with proper and clear structure rather than
            // whatever you have going right now. It'll make fixing this issue a
            // lot easier.
            self.queue
                .extend_front(self.events.emitted_commands(&self.state.config));

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

        self.state.fill_modeline_infos();
    }

    pub fn render(&mut self) -> UiState {
        let panels = vec![
            self.panels.editor.render(&self.state),
            self.panels.modeline.render(&self.state),
        ];

        UiState { panels }
    }

    fn update_viewport_size(&mut self, viewport_size: Size) {
        self.state.viewport_size = viewport_size;

        self.panels.editor.set_rect(Rect::new(
            0,
            0,
            self.state.viewport_size.column,
            self.state.viewport_size.row.saturating_sub(1),
        ));
        self.state.editor_size = self.panels.editor.rect().size();

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
