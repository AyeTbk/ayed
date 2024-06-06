use std::collections::HashMap;

use crate::config::Config;

// FIXME I feel like events might be "superfluous" in a sense.
//          There could be a way to register commands to be executed
//          before/after any other command. With something like that,
//          events would just be dummy commands that the editor
//          queues up. (be wary of infinite loop tho)
#[derive(Debug, Default)]
pub struct EventRegistry {
    event_commands: HashMap<String, Vec<String>>,
    queued_events: Vec<QueuedEvent>,
}

impl EventRegistry {
    pub fn on(&mut self, event: impl Into<String>, command: impl Into<String>) {
        self.event_commands
            .entry(event.into())
            .or_default()
            .push(command.into());
    }

    pub fn emit(&mut self, event: impl Into<String>, options: impl Into<String>) {
        self.queued_events.push(QueuedEvent {
            event: event.into(),
            options: options.into(),
        });
    }

    pub(super) fn emitted_commands(&mut self, config: &Config) -> Vec<String> {
        let mut emitted = Vec::new();
        for ev in std::mem::take(&mut self.queued_events) {
            if let Some(commands) = self.event_commands.get(&ev.event) {
                for command in commands {
                    emitted.push(format!("{} {}", command, ev.options));
                }
            }

            if let Some(hooks) = config.get("hooks").and_then(|h| h.get(&ev.event)) {
                for command in hooks {
                    emitted.push(format!("{} {}", command, ev.options));
                }
            }
        }
        emitted
    }
}

#[derive(Debug)]
pub struct QueuedEvent {
    event: String,
    options: String,
}
