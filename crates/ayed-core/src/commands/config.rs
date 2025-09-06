use crate::{command::CommandRegistry, input::Input, state::regex_syntax_highlight};

pub fn register_config_commands(cr: &mut CommandRegistry) {
    cr.register("map-input", |opt, ctx| {
        // hackish support for combo modes
        let is_combo = ctx
            .state
            .config
            .state_value("mode")
            .is_some_and(|m| m.starts_with("combo-"));
        if is_combo {
            ctx.queue.set_state("mode", "normal");
        }

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

    cr.register("generate-highlights", |_opt, ctx| {
        let Some(buffer_handle) = ctx.state.active_editor_buffer(&ctx.resources) else {
            return Ok(());
        };

        let buffer = ctx.resources.buffers.get(buffer_handle);
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
