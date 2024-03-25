use std::cell::RefCell;

use crate::{selection::Selections, state::View, Ref};

use super::CommandRegistry;

pub fn register_builtin_commands(cr: &mut CommandRegistry) {
    cr.register(
        "show-err",
        Box::new(|opt, _ctx| Err(format!("error: {}", opt))),
    );

    cr.register(
        "edit",
        Box::new(|opt, ctx| {
            let buffer_handle = match ctx.state.buffer_with_path(opt) {
                Some(handle) => handle,
                None => ctx.state.open_file(opt)?,
            };

            let view_handle = match ctx.state.view_with_buffer(buffer_handle) {
                Some(handle) => handle,
                None => {
                    let selections = Ref::new(RefCell::new(Selections::new()));

                    ctx.state
                        .buffers
                        .get_mut(buffer_handle)
                        .add_selections(&selections);

                    ctx.state.views.insert(View {
                        buffer: buffer_handle,
                        selections,
                    })
                }
            };

            ctx.state.active_view = Some(view_handle);

            Ok(())
        }),
    );
}
