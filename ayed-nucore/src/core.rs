use crate::{
    command::{CommandQueue, CommandRegistry, ExecuteCommandContext},
    event::EventRegistry,
    panels::Panels,
    state::State,
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
    pub fn tick(&mut self) {
        loop {
            self.queue.extend_front(self.events.emitted_commands());

            let Some((command, options)) = self.queue.pop() else {
                break;
            };
            self.commands
                .execute_command(
                    &command,
                    &options,
                    ExecuteCommandContext {
                        events: &mut self.events,
                        queue: &mut self.queue,
                        state: &mut self.state,
                    },
                )
                .unwrap();
        }
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
