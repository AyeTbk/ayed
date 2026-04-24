use crate::{
    command::{CommandRegistry, helpers::alias, options::Options},
    panels::PanelContext,
};

pub fn register_core_commands(cr: &mut CommandRegistry) {
    cr.register("quit", |_opt, ctx| {
        for (_, view) in ctx.resources.views.iter() {
            let buffer = ctx.resources.buffers.get(view.buffer);
            if buffer.is_dirty() {
                return Err(format!("there are unsaved changes"));
            }
        }
        ctx.state.quit_requested = true;
        Ok(())
    });
    cr.register("quit!", |_opt, ctx| {
        ctx.state.quit_requested = true;
        Ok(())
    });
    cr.register("q", alias("quit"));
    cr.register("q!", alias("quit!"));

    cr.register("error", |opt, _ctx| Err(opt.to_string()));

    cr.register("message", |opt, ctx| {
        ctx.state.modeline.set_message(opt.to_string());
        Ok(())
    });

    cr.register("state-set", |opt, ctx| {
        let (state, rest) = opt
            .split_once(|ch: char| ch.is_ascii_whitespace())
            .ok_or_else(|| format!("bad options `{}`", opt))?;

        let state = state.trim();
        let value = rest.trim();

        ctx.queue
            .emit(format!("state-before-modified:{state}"), value);

        ctx.queue.push(format!("state-set__part2 {opt}"));

        Ok(())
    });
    cr.register("state-set__part2", |opt, ctx| {
        let (state, rest) = opt
            .split_once(|ch: char| ch.is_ascii_whitespace())
            .ok_or_else(|| format!("bad options `{}`", opt))?;

        let state = state.trim();
        let value = rest.trim();

        ctx.state.config.set_state(state, value);
        ctx.queue.emit(format!("state-modified:{state}"), value);

        Ok(())
    });
    cr.register("set", alias("state-set"));

    cr.register("panel-focus", |opt, ctx| {
        let panel_name = opt
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("missing panel name"))?;

        // Unfocus previously focused panel
        if let Some(panel) = ctx.panels.panel_with_name_mut(&ctx.state.focused_panel) {
            panel.on_unfocus(&mut PanelContext {
                resources: ctx.resources,
                state: ctx.state,
            });
        }

        ctx.state.focused_panel = panel_name.to_string();

        // Focus newly focused panel
        if let Some(panel) = ctx.panels.panel_with_name_mut(&ctx.state.focused_panel) {
            panel.on_focus(&mut PanelContext {
                resources: ctx.resources,
                state: ctx.state,
            });
        }

        ctx.queue.set_state("panel", &ctx.state.focused_panel);

        Ok(())
    });

    cr.register("prompt-exec", |opt, ctx| {
        let command_to_execute_override = opt.trim();

        let view_handle = ctx
            .panels
            .panel_with_name(&ctx.state.focused_panel)
            .and_then(|p| p.view())
            .ok_or_else(|| "prompt not focused".to_string())?;

        let buffer_handle = ctx.resources.views.get(view_handle).buffer;
        let line = ctx.resources.buffers.get(buffer_handle).first_line();

        ctx.queue.push("panel-focus editor");

        let maybe_history = if let Some(prompt_mode) = ctx.state.config.state_value("prompt-mode") {
            let key = prompt_mode.to_string();
            let history = ctx.state.modeline.histories.entry(key).or_default();
            Some(history)
        } else {
            None
        };

        let mut unprocessed_command = line.to_string();

        if let Some(history) = maybe_history {
            if line.is_empty() {
                if let Some(entry) = history.entries.last() {
                    unprocessed_command = entry.to_string();
                }
            }

            if !line.is_empty() {
                history.entries.push(unprocessed_command.clone());
                history.selected_item = history.entries.len();
            }
        }

        let command;
        if command_to_execute_override.is_empty() {
            command = unprocessed_command;
        } else {
            command = command_to_execute_override.replace("<PROMPT>", &unprocessed_command);
        }

        if !command.trim().is_empty() {
            ctx.queue.push(command);
        }

        Ok(())
    });

    cr.register("prompt-history", |opt, ctx| {
        let opts = Options::new().flag("next").flag("previous").parse(opt)?;
        let next = opts.contains("next");
        let previous = opts.contains("previous");

        let Some(prompt_mode) = ctx.state.config.state_value("prompt-mode") else {
            return Ok(());
        };
        let Some(history) = ctx.state.modeline.histories.get_mut(prompt_mode) else {
            return Ok(());
        };

        let view_handle = ctx
            .panels
            .panel_with_name(&ctx.state.focused_panel)
            .and_then(|p| p.view())
            .ok_or_else(|| "prompt not focused".to_string())?;

        let buffer_handle = ctx.resources.views.get(view_handle).buffer;
        let buffer = ctx.resources.buffers.get_mut(buffer_handle);
        if buffer.line(0).is_some() {
            let max = history.entries.len();
            let item_idx = &mut history.selected_item;
            if next {
                *item_idx = usize::min(item_idx.saturating_add(1), max);
            }
            if previous {
                *item_idx = item_idx.saturating_sub(1);
            }

            if *item_idx == max {
                buffer.set_line(0, String::new()).unwrap();
            } else {
                let item = &history.entries[*item_idx];
                buffer.set_line(0, item.clone()).unwrap();

                ctx.queue.push("move-to-edge line-past-end");
            }
        }

        Ok(())
    });
}
