use ayed_nucore::core::Core;

mod tui;

fn main() {
    let mut core = Core::with_builtins();

    for arg in std::env::args().skip(1) {
        core.queue_command(format!("edit {arg}"));
    }
    core.tick();

    let mut tui = tui::Tui::new(core);

    tui.run();
}
