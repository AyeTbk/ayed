use crate::{command::CommandRegistry, event::EventRegistry, input::Input};

pub fn register_builtin_commands(cr: &mut CommandRegistry, ev: &mut EventRegistry) {
    cr.register("map-input", |opt, ctx| {
        let input = Input::parse(&opt).map_err(|_| format!("invalid input: {opt}"))?;

        if let Some(cmd) = ctx.state.config.get_keybind(input) {
            let (command, options) = cmd.split_once(' ').unwrap_or((&cmd, ""));
            ctx.queue.push(command, options);
        } else if ctx.state.config.get_keybind_else_insert_char() {
            if let Some(ch) = input.char() {
                ctx.queue.push("insert-char", format!("{ch}"));
            }
        }

        Ok(())
    });
    ev.on("input", "map-input");
}
