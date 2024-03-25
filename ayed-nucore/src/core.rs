use crate::{
    command::{
        builtins::register_builtin_commands, CommandQueue, CommandRegistry, ExecuteCommandContext,
    },
    event::EventRegistry,
    input::Input,
    panels::Panels,
    state::State,
    ui::{ui_state::UiState, Size},
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
        register_builtin_commands(&mut this.commands);
        this
    }

    pub fn queue_command(&mut self, command: String, options: String) {
        self.queue.push(command, options)
    }

    pub fn emit_input_event(&mut self, input: Input) {
        let mut buf = String::new();
        input.serialize(&mut buf);
        self.events.emit("input", buf);
    }

    pub fn quit_requested(&self) -> bool {
        self.state.quit_requested
    }

    pub fn viewport_size(&self) -> Size {
        self.state.viewport_size
    }

    pub fn set_viewport_size(&mut self, size: Size) {
        self.state.viewport_size = size;
    }

    pub fn tick(&mut self) {
        loop {
            self.queue.extend_front(self.events.emitted_commands());

            let Some((command, options)) = self.queue.pop() else {
                break;
            };

            self.queue.start_scope();

            let res = self.commands.execute_command(
                &command,
                &options,
                ExecuteCommandContext {
                    events: &mut self.events,
                    queue: &mut self.queue,
                    state: &mut self.state,
                },
            );

            match res {
                Ok(()) => (),
                Err(err) => {
                    self.queue.clear();
                    self.state.modeline_err = Some(err);
                    dbg!(&self.state.modeline_err);
                    return;
                }
            }
        }
    }

    pub fn render(&mut self) -> UiState {
        let panels = vec![self.panels.editor.render(&self.state)];
        UiState { panels }
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
