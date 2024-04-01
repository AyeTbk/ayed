use crate::{command::CommandRegistry, event::EventRegistry, input::Input};

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
}
