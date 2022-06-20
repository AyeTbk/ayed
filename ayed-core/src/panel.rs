use crate::{
    core::{EditorContext, EditorContextMut},
    input::Input,
    ui_state::Panel as UiPanel,
};

pub trait Panel {
    fn input(&mut self, input: Input, ctx: &mut EditorContextMut);
    fn panel(&self, ctx: &EditorContext) -> UiPanel;
}
