use ayed_nucore::core::Core;

pub fn main() {
    let mut core = Core::default();

    core.commands.register(
        "say-hi",
        Box::new(|_, _| {
            println!("hi");
            Ok(())
        }),
    );
    core.events.on("app-start", "say-hi");

    core.commands.register(
        "insert",
        Box::new(|opt, ctx| {
            let ch = opt.chars().next().unwrap();
            println!("{ch}");
            ctx.queue.push("say-hi", "");
            Ok(())
        }),
    );
    core.events.on("input", "insert");

    core.events.emit("app-start", "");
    core.events.emit("input", "a");

    core.tick();
}
