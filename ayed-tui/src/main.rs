use ayed_nucore::core::Core;

mod tui;

fn main() {
    let mut core = Core::with_builtins();

    for arg in std::env::args().skip(1) {
        core.queue_command("edit".to_string(), arg);
    }
    core.tick();

    core.commands.register("input-look", |opt, ctx| {
        let command = match opt {
            "<up>" => Some(("look", "u")),
            "<down>" => Some(("look", "d")),
            "<left>" => Some(("look", "l")),
            "<right>" => Some(("look", "r")),
            _ => None,
        };
        if let Some((command, options)) = command {
            ctx.queue.push(command, options)
        }
        Ok(())
    });
    core.events.on("input", "input-look");

    let mut tui = tui::Tui::new(core);

    tui.run();
}
