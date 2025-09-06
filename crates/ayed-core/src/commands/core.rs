use crate::{
    command::{CommandRegistry, helpers::alias},
    panels::FocusedPanel,
    position::Position,
    selection::Selections,
    state::{TextBuffer, View},
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
    cr.register("ss", alias("state-set"));

    cr.register("panel-focus", |opt, ctx| {
        let panel_name = opt
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("missing panel name"))?;

        // Cleanup if needed
        match ctx.state.focused_panel {
            FocusedPanel::Warpdrive => {
                ctx.panels.warpdrive.clear_state();
            }
            FocusedPanel::Modeline(view_handle) => {
                let buffer_handle = ctx.resources.views.get(view_handle).buffer;
                ctx.resources.views.remove(view_handle);
                ctx.resources.buffers.remove(buffer_handle);
            }
            _ => (),
        }

        match panel_name {
            "editor" => {
                ctx.state.focused_panel = FocusedPanel::Editor;
            }
            "modeline" => {
                let buffer = ctx.resources.buffers.insert(TextBuffer::new_empty());
                let view = ctx.resources.views.insert(View {
                    top_left: Position::ZERO,
                    buffer,
                    virtual_buffer: None,
                });
                ctx.resources
                    .buffers
                    .get_mut(buffer)
                    .add_view_selections(view, Selections::new());

                // TODO the modeline view and buffer handles could just be stored in the panel maybe?
                // It would avoid having to cleanup and recreate them (but would still need to clear the buffer).
                ctx.state.focused_panel = FocusedPanel::Modeline(view);
            }
            "warpdrive" => {
                ctx.state.focused_panel = FocusedPanel::Warpdrive;
            }
            _ => return Err(format!("unknown panel '{opt}'")),
        }

        ctx.queue.set_state("panel", panel_name);

        Ok(())
    });

    cr.register("modeline-exec", |_opt, ctx| {
        let FocusedPanel::Modeline(view_handle) = ctx.state.focused_panel else {
            return Err("modeline not focused".into());
        };

        let buffer_handle = ctx.resources.views.get(view_handle).buffer;
        let line = ctx.resources.buffers.get(buffer_handle).first_line();

        ctx.queue.push("panel-focus editor");
        if !line.is_empty() {
            ctx.queue.push(line);
        }

        Ok(())
    });
}
