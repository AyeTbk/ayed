use crate::{command::Command, core::EditorContextMut, input::Input, ui_state::Panel as UiPanel};

pub trait Panel {
    fn execute_command(&mut self, command: Command, ctx: &mut EditorContextMut);
    fn convert_input_to_command(&self, input: Input, ctx: &mut EditorContextMut)
        -> Option<Command>;
    fn panel(&self, ctx: &EditorContextMut) -> UiPanel;
}
