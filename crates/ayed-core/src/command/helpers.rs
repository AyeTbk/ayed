use crate::{
    selection::Selections,
    slotmap::Handle,
    state::{State, TextBuffer, View},
};

use super::{CommandQueue, ExecuteCommandContext};

pub fn alias(
    original_command: impl Into<String>,
) -> impl Fn(&str, ExecuteCommandContext) -> Result<(), String> {
    let cmd = original_command.into();
    move |opt, ctx| {
        ctx.queue.push(format!("{cmd} {opt}"));
        Ok(())
    }
}

pub struct FocusedBufferCommandContext<'a> {
    pub view_handle: Handle<View>,
    pub view: &'a mut View,
    pub buffer_handle: Handle<TextBuffer>,
    pub buffer: &'a mut TextBuffer,
    pub selections: Selections,
    pub queue: &'a mut CommandQueue,
    pub state: &'a mut State,
}

pub fn focused_buffer_command(
    f: impl Fn(&str, FocusedBufferCommandContext) -> Result<(), String>,
) -> impl Fn(&str, ExecuteCommandContext) -> Result<(), String> {
    move |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get_mut(view_handle);
        let buffer_handle = view.buffer;
        let buffer = ctx.resources.buffers.get_mut(buffer_handle);
        let selections = buffer.view_selections(view_handle).unwrap().clone();
        let args = FocusedBufferCommandContext {
            view_handle,
            view,
            buffer_handle,
            buffer,
            selections,
            queue: ctx.queue,
            state: ctx.state,
        };
        f(opt, args)
    }
}
