use crate::{
    command::{CommandRegistry, helpers::focused_buffer_command, options::Options},
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

        register.content = buffer.selection_text(&selections.primary_selection);
        register.extra_content.clear();
        for selection in selections.extra_selections.iter() {
            register
                .extra_content
                .push(buffer.selection_text(selection));
        }

        Ok(())
    });

    cr.register(
        "paste",
        focused_buffer_command(|opt, mut ctx| {
            let opts = Options::new().flag("before").parse(opt)?;
            let before = opts.contains("before");

            let enumerated_sels = ctx.selections.iter_mut().enumerate().collect::<Vec<_>>();
            for (i, sel) in enumerated_sels.into_iter().rev() {
                let text = ctx
                    .state
                    .register
                    .iter()
                    .cycle()
                    .nth(i)
                    .expect("register.iter is never empty");
                let insert_at = if before {
                    sel.start()
                } else {
                    ctx.buffer
                        .move_position_horizontally(sel.end(), 1)
                        .unwrap_or(sel.end())
                };

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

    cr.register("suggestions-select", |opt, ctx| {
        if ctx.state.suggestions.items.is_empty() {
            return Ok(());
        }

        let opts = Options::new().flag("next").flag("previous").parse(opt)?;
        let next = opts.contains("next");
        let previous = opts.contains("previous");

        let cycling_from_original = ctx.state.suggestions.selected_item == 0;
        ctx.state.suggestions.selected_item += next as i32 - (previous as i32);
        let modulo = ctx.state.suggestions.items.len() as i32 + 1;
        ctx.state.suggestions.selected_item =
            ctx.state.suggestions.selected_item.rem_euclid(modulo);
        let selected_item_idx = (ctx.state.suggestions.selected_item - 1).max(0) as usize;
        let cycling_to_original = ctx.state.suggestions.selected_item == 0;

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get_mut(view.buffer);
        let sel_count = buffer.view_selections(view_handle).unwrap().count();

        if cycling_from_original {
            ctx.state.suggestions.original_symbols.clear();
            for sel_idx in 0..sel_count {
                let selections = buffer.view_selections(view_handle).unwrap();
                let sel = selections.get(sel_idx).unwrap();
                let original = buffer.selection_text(&sel);
                ctx.state.suggestions.original_symbols.push(original);
            }
        }

        for sel_idx in 0..sel_count {
            let selections = buffer.view_selections(view_handle).unwrap();
            let sel = selections.get(sel_idx).unwrap();

            buffer.delete_selection(&sel)?;

            let text_to_insert;
            if cycling_to_original {
                text_to_insert = ctx.state.suggestions.original_symbols[sel_idx].as_str();
            } else {
                text_to_insert = ctx.state.suggestions.items[selected_item_idx].as_str();
            }
            let new_sel = buffer.insert_str_at(sel.start(), text_to_insert)?;

            let selections = buffer.view_selections_mut(view_handle).unwrap();
            *selections.get_mut(sel_idx).unwrap() = new_sel;
        }

        ctx.queue.emit("buffer-modified", "");
        ctx.queue.emit("selections-modified", "");

        Ok(())
    });

    cr.register("suggestions-gather", |_opt, ctx| {
        // Look at symbol under primary cursor (or right before)
        // If it's the same as old one saved in state, bail.
        // Else, clear suggs and gather new from configured source

        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };
        let view = ctx.resources.views.get(view_handle);
        let buffer = ctx.resources.buffers.get(view.buffer);
        let selections = buffer.view_selections(view_handle).unwrap();

        selections.primary().cursor();
        // todo!("get symbol primary cursor is within or right after");

        ctx.state.suggestions.items.clear();
        ctx.state.suggestions.selected_item = 0;

        let source = ctx.state.config.get_entry_value("suggestions", "source")?;
        if source != "active-buffer" {
            return Err(format!(
                "only 'active-buffer' is supported as suggestion source"
            ));
        }

        Ok(())
    });

    cr.register("vbuf-clear", |_opt, ctx| {
        // FIXME I dont believe the vbuffer should be handled directly
        // by commands like this. Its settings should be set (with
        // states) and the changes should take effect automatically.
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.resources.views.get_mut(view_handle);
        view.virtual_buffer = None;

        Ok(())
    });
    cr.register("vbuf-line-wrap-rebuild", |_opt, ctx| {
        let Some(view_handle) = ctx.state.focused_view() else {
            return Ok(());
        };

        let view = ctx.resources.views.get_mut(view_handle);
        view.rebuild_line_wrap(
            &ctx.resources.buffers,
            ctx.state.editor_rect.size().column.try_into().unwrap(),
        );

        Ok(())
    });
}
