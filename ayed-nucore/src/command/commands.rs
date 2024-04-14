use std::cell::RefCell;

use crate::{
    config::ConfigState,
    event::EventRegistry,
    panels::FocusedPanel,
    position::{Offset, Position},
    selection::Selections,
    state::{TextBuffer, View},
    Ref,
};

use super::CommandRegistry;

pub fn register_builtin_commands(cr: &mut CommandRegistry, _ev: &mut EventRegistry) {
    cr.register("quit!", |_opt, ctx| {
        ctx.state.quit_requested = true;
        Ok(())
    });
    cr.register("q!", |_opt, ctx| {
        ctx.queue.push("quit!");
        Ok(())
    });

    cr.register("error", |opt, _ctx| Err(opt.to_string()));

    cr.register("focus-panel", |opt, ctx| {
        match opt {
            "editor" => {
                ctx.state.focused_panel = FocusedPanel::Editor;
            }
            "modeline" => {
                // FIXME cleanup old view and buffer
                let selections = Ref::new(RefCell::new(Selections::new()));

                let buffer = ctx.state.buffers.insert(TextBuffer::new_empty());
                ctx.state
                    .buffers
                    .get_mut(buffer)
                    .add_selections(&selections);

                let view = ctx.state.views.insert(View {
                    top_left: Position::ZERO,
                    buffer,
                    selections,
                });

                ctx.state.focused_panel = FocusedPanel::Modeline(view);
            }
            _ => return Err(format!("unknown panel '{opt}'")),
        }

        ctx.state.config.set_state("panel", opt);

        Ok(())
    });

    cr.register("modeline-exec", |_opt, ctx| {
        let FocusedPanel::Modeline(view_handle) = ctx.state.focused_panel else {
            return Err("modeline not focused".into());
        };

        let buffer_handle = ctx.state.views.get(view_handle).buffer;
        let line = ctx.state.buffers.get(buffer_handle).first_line();

        ctx.queue.push("focus-panel editor");
        if !line.is_empty() {
            ctx.queue.push(line);
        }

        Ok(())
    });

    cr.register("edit", |opt, ctx| {
        let path = opt;
        let buffer_handle = match ctx.state.buffer_with_path(path) {
            Some(handle) => handle,
            None => ctx.state.open_file(path)?,
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
                    top_left: Position::ZERO,
                    buffer: buffer_handle,
                    selections,
                })
            }
        };

        ctx.state.active_editor_view = Some(view_handle);

        ctx.state.config.set_state(ConfigState::FILE, path);

        Ok(())
    });

    cr.register("look", |opt, ctx| {
        let mut offset = Offset::new(0, 0);
        for ch in opt.chars() {
            match ch {
                'u' => offset.row -= 1,
                'd' => offset.row += 1,
                'l' => offset.column -= 1,
                'r' => offset.column += 1,
                _ => return Err(format!("invalid option: {ch}")),
            }
        }

        if let Some(view_handle) = ctx.state.focused_view() {
            let view = ctx.state.views.get_mut(view_handle);
            view.top_left = view.top_left.offset(offset);
        }

        Ok(())
    });

    cr.register("insert-char", |opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let the_char = opt
            .chars()
            .next()
            .ok_or_else(|| format!("not a char: {opt}"))?;

        let view = ctx.state.views.get(view_handle);
        let buffer = ctx.state.buffers.get_mut(view.buffer);

        let sel_count = {
            // Using a block to limit the borrow to this line.
            view.selections.borrow().count()
        };
        for sel_idx in (0..sel_count).rev() {
            let Some(sel) = view.selections.borrow().get(sel_idx) else {
                continue;
            };
            buffer.insert_char_at(sel.cursor(), the_char)?;
        }

        Ok(())
    });
}
