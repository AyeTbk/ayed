use std::collections::HashMap;

#[derive(Default)]
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

    pub(super) fn emitted_commands(&mut self) -> Vec<(String, String)> {
        let mut emitted = Vec::new();
        for ev in std::mem::take(&mut self.queued_events) {
            let Some(commands) = self.event_commands.get(&ev.event) else {
                continue;
            };
            for command in commands {
                emitted.push((command.clone(), ev.options.clone()));
            }
        }
        emitted
    }
}

pub struct QueuedEvent {
    event: String,
    options: String,
}
