use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
    position::Position,
    state::TextBufferHistory,
};

pub fn register_misc_commands(cr: &mut CommandRegistry) {
    cr.register("history-save", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);

        let history_entry = ctx.state.edit_histories.entry(view.buffer);
        use std::collections::hash_map::Entry;
        match history_entry {
            Entry::Occupied(mut history) => {
                history.get_mut().save_state(buffer);
            }
            Entry::Vacant(history) => {
                history.insert(TextBufferHistory::new(buffer));
            }
        }

        Ok(())
    });

    cr.register("history-undo", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);

        let undid = ctx
            .state
            .edit_histories
            .get_mut(&view.buffer)
            .is_some_and(|history| history.undo(buffer));

        if undid {
            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");
        } else {
            ctx.queue.push("message no remaining history");
        }

        Ok(())
    });

    cr.register("yank", |_opt, ctx| {
        let Some(view_handle) = ctx.state.active_editor_view else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get(view.buffer);

        let selections = buffer.view_selections(view_handle).unwrap();
        let register = &mut ctx.state.register;

        register.content = buffer
            .selection_text(&selections.primary_selection)
            .unwrap();
        register.extra_content.clear();
        for selection in selections.extra_selections.iter() {
            register
                .extra_content
                .push(buffer.selection_text(selection).unwrap());
        }

        let sel_count = register.extra_content.len() + 1;
        ctx.queue.push(format!(
            "message yanked {} selection{}",
            sel_count,
            if sel_count != 1 { "s" } else { "" }
        ));

        Ok(())
    });

    cr.register(
        "paste",
        focused_buffer_command(|opt, mut ctx| {
            let opts = Options::new().flag("before").parse(opt)?;
            let before = opts.contains("before");

            let enumerated_sels = ctx.selections.iter_mut().enumerate().collect::<Vec<_>>();
            for (i, sel) in enumerated_sels.into_iter().rev() {
                let mut text = ctx
                    .state
                    .register
                    .iter()
                    .cycle()
                    .nth(i)
                    .expect("register.iter is never empty");

                let line_pasting_mode = text.ends_with('\n');

                let insert_at = if line_pasting_mode {
                    if before {
                        // Line start of selection start
                        Position::new(0, sel.start().row)
                    } else {
                        // Line start of row after selection end row
                        Position::new(0, sel.end().row + 1)
                    }
                } else {
                    if before {
                        sel.start()
                    } else {
                        if sel.end() == ctx.buffer.end_position() {
                            Position::new(0, sel.end().row + 1)
                        } else {
                            ctx.buffer
                                .move_position_horizontally(sel.end(), 1)
                                .unwrap_or(sel.end())
                        }
                    }
                };

                if insert_at > ctx.buffer.end_position() {
                    ctx.buffer.insert_char_at(ctx.buffer.end_position(), '\n')?;
                    text = text.strip_suffix('\n').unwrap_or(text);
                }

                let inserted_sel = ctx.buffer.insert_str_at(insert_at, text)?;
                *sel = inserted_sel;
            }

            let sels = ctx.buffer.view_selections_mut(ctx.view_handle).unwrap();
            *sels = ctx.selections;

            ctx.queue.emit("buffer-modified", "");
            ctx.queue.emit("selections-modified", "");

            Ok(())
        }),
    );
}
