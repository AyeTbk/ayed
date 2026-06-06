use crate::{
    command::{
        CommandRegistry,
        options::{Options, ParsedOptions},
    },
    selection::{Selection, Selections},
    slotmap::Handle,
    state::{State, TextBuffer, View},
};

use super::{CommandQueue, ExecuteCommandContext};

pub fn alias(
    original_command: impl Into<String>,
) -> impl Fn(&ParsedOptions, ExecuteCommandContext) -> Result<(), String> {
    let cmd = original_command.into();
    move |opt, ctx| {
        ctx.queue.push(format!("{cmd} {}", opt.raw()));
        Ok(())
    }
}

#[expect(dead_code)]
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
    f: impl Fn(&ParsedOptions, FocusedBufferCommandContext) -> Result<(), String>,
) -> impl Fn(&ParsedOptions, ExecuteCommandContext) -> Result<(), String> {
    move |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view(&ctx.panels) else {
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

#[expect(dead_code)]
pub struct SelectionMovementCommandContext<'a> {
    pub view_handle: Handle<View>,
    pub view: &'a View,
    pub buffer_handle: Handle<TextBuffer>,
    pub buffer: &'a TextBuffer,
    pub selection: Selection,
    pub queue: &'a mut CommandQueue,
    pub state: &'a mut State,
}

pub fn register_selection_movement(
    cr: &mut CommandRegistry,
    name: impl Into<String>,
    opts: impl Into<Options>,
    f: impl Fn(&ParsedOptions, SelectionMovementCommandContext) -> Result<Option<Selection>, String>
    + 'static,
) {
    let opts = opts
        .into()
        .flag("anchored")
        .flag("reanchored")
        .flag("primary");

    cr.register(name, opts, move |opt, ctx| {
        let anchored = opt.contains("anchored");
        let reanchored = opt.contains("reanchored");
        let primary = opt.contains("primary");

        let Some(view_handle) = ctx.state.focused_view(&ctx.panels) else {
            return Ok(());
        };
        let view = ctx.resources.views.get_mut(view_handle);
        let buffer_handle = view.buffer;
        let buffer = ctx.resources.buffers.get_mut(buffer_handle);
        let mut selections = buffer.view_selections(view_handle).unwrap().clone();
        let mut updated_selections = Vec::new();
        for original_sel in selections.iter_mut() {
            let args = SelectionMovementCommandContext {
                view_handle,
                view,
                buffer_handle,
                buffer,
                selection: *original_sel,
                queue: ctx.queue,
                state: ctx.state,
            };
            let updated_sel = f(opt, args)?;
            if let Some(mut sel) = updated_sel {
                if reanchored {
                    sel = sel.with_anchor(original_sel.cursor);
                } else if anchored {
                    sel = sel.with_anchor(original_sel.anchor);
                }
                updated_selections.push(sel);
                // TODO check if any selection actually changed before emitting selections-modified event?
                // if *sel == *selection {
                //     continue;
                // }
            }
        }

        if primary {
            updated_selections.drain(1..);
        }

        ctx.queue.emit("selections-modified", "");

        if let Ok(updated_sels) = Selections::from_vec_2(updated_selections) {
            *buffer.view_selections_mut(view_handle).unwrap() = updated_sels;
            Ok(())
        } else {
            Err(format!("no selection left!"))
        }
    })
}

pub trait ErrorExt {
    type Inner;
    fn or_strerr(self) -> Result<Self::Inner, String>;
}

impl<T, E> ErrorExt for Result<T, E>
where
    E: ToString,
{
    type Inner = T;
    fn or_strerr(self) -> Result<Self::Inner, String> {
        self.map_err(|e| e.to_string())
    }
}
