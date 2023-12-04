use ayed_core::command::CoreCommand;

mod tui;

fn main() {
    let mut core = ayed_core::core::Core::new();

    for arg in std::env::args().skip(1) {
        core.execute_command(CoreCommand::EditFile(arg).into())
    }

    let mut tui = tui::Tui::new(core);

    tui.run();
}
