use crate::{
    command::CommandRegistry, event::EventRegistry, input::Input, state::regex_syntax_highlight,
};

pub fn register_builtin_commands(cr: &mut CommandRegistry, ev: &mut EventRegistry) {
    cr.register("map-input", |opt, ctx| {
        let input = Input::parse(&opt).map_err(|_| format!("invalid input: {opt}"))?;

        if let Some(command) = ctx.state.config.get_keybind(input) {
            ctx.queue.push(command);
        } else if ctx.state.config.get_keybind_else_insert_char() {
            if let Some(ch) = input.char() {
                ctx.queue.push(format!("insert-char {ch}"));
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
    // FIXME this should be hooked in the config file, not hardcoded here
    ev.on("buffer-opened", "generate-highlights");
    ev.on("buffer-modified", "generate-highlights");
}
