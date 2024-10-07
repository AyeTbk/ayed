use crate::{
    command::CommandRegistry, event::EventRegistry, input::Input, state::regex_syntax_highlight,
};

pub fn register_builtin_commands(cr: &mut CommandRegistry, ev: &mut EventRegistry) {
    cr.register("map-input", |opt, ctx| {
        let input = Input::parse(&opt).map_err(|_| format!("invalid input: {opt}"))?;

        if let Some(cmds) = ctx.state.config.get_keybind(input) {
            for cmd in cmds {
                ctx.queue.push(cmd);
            }
        } else if let Some(cmds) = ctx.state.config.get_keybind_else() {
            if cmds.len() == 1 {
                if let Some(ch) = input.char() {
                    let cmd = cmds.first().expect("len is 1");
                    ctx.queue.push(format!("{cmd} {ch}"));
                }
            } else {
                for cmd in cmds {
                    ctx.queue.push(cmd);
                }
            }
        }

        Ok(())
    });
    ev.on("input", "map-input");

    cr.register("generate-highlights", |_opt, ctx| {
        let Some(buffer_handle) = ctx.state.active_editor_buffer() else {
            return Ok(());
        };

        let buffer = ctx.state.buffers.get(buffer_handle);
        let syntax = ctx.state.config.get_syntax();
        let syntax_style = ctx
            .state
            .config
            .get("syntax-style")
            .expect("syntax-style should exist");
        let highlights = regex_syntax_highlight(buffer, syntax, syntax_style);

        ctx.state.highlights.insert(buffer_handle, highlights);
        Ok(())
    });
}
