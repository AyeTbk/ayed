use crate::{command::Command, core::EditorContextMut, input::Input, ui_state::UiPanel};

pub trait Panel {
    fn execute_command(&mut self, command: Command, ctx: &mut EditorContextMut) -> Option<Command>;
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut) -> Vec<Command>;
    fn panel(&mut self, ctx: &EditorContextMut) -> UiPanel;
}
