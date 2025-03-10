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
        let state = Self::extract_state(buffer);
        let all_selections = state.all_selections.clone();

        // Only add a new state if needed
        if buffer.take_history_dirty() {
            self.state_stack.push(state);
        }

        // Always update current state's selections
        let current_state = self.state_stack.last_mut().expect("should never be empty");
        current_state.all_selections = all_selections;
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
