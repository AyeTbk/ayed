use crate::{command::Command, input::Input, state::State, ui_state::UiPanel};

pub trait Panel {
    fn execute_command(&mut self, command: Command, state: &mut State) -> Option<Command>;
    fn convert_input_to_command(&self, input: Input, state: &State) -> Vec<Command>;
    fn render(&mut self, state: &State) -> UiPanel;
}
