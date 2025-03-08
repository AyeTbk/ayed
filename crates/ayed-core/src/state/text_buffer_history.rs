use std::collections::HashMap;

use crate::{selection::Selections, slotmap::Handle};

use super::{TextBuffer, View};

pub struct TextBufferHistory {
    initial_state: State,
    state_stack: Vec<State>, // Top of stack is the current state
}

impl TextBufferHistory {
    pub fn new(buffer: &TextBuffer) -> Self {
        let initial_state = Self::extract_state(buffer);
        Self {
            state_stack: vec![initial_state.clone()],
            initial_state,
        }
    }

    pub fn save_state(&mut self, buffer: &TextBuffer) {
        if buffer.take_history_dirty() {
            dbg!(&buffer.lines);
            self.state_stack.push(Self::extract_state(buffer));
        }
    }

    pub fn undo(&mut self, buffer: &mut TextBuffer) -> bool {
        // Dismiss the current state
        self.state_stack.pop();

        if self.state_stack.is_empty() {
            // Oops, the initial state copy was popped.
            // That means no changes..?
            self.state_stack.push(self.initial_state.clone());
            return false;
        }

        // Restore the previous state
        if let Some(state) = self.state_stack.last().cloned() {
            dbg!(&state.whole_content);
            buffer.lines = state.whole_content;
            buffer.selections = state.all_selections;
            true
        } else {
            unreachable!()
        }
    }

    pub fn redo(&mut self, _buffer: &mut TextBuffer) -> bool {
        todo!()
    }

    fn extract_state(buffer: &TextBuffer) -> State {
        State {
            whole_content: buffer.lines.clone(),
            all_selections: buffer.selections.clone(),
        }
    }
}

#[derive(Clone)]
struct State {
    whole_content: Vec<String>,
    all_selections: HashMap<Handle<View>, Selections>,
}
