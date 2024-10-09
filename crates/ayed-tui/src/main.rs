use ayed_nucore::core::Core;

mod tui;

fn main() {
    let mut core = Core::with_builtins();

    let mut any_path_specified = false;
    for arg in std::env::args().skip(1) {
        core.queue_command(format!("edit {arg}"));
        any_path_specified = true;
    }
    if !any_path_specified {
        core.queue_command("edit --scratch".to_string());
    }
    core.tick();

    let mut tui = tui::Tui::new(core);

    tui.run();
}
