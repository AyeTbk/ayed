mod tui;

fn main() {
    let mut core = ayed_core::core::Core::new();

    for arg in std::env::args().skip(1) {
        let buffer = core.create_buffer_from_filepath(arg);
        core.edit_buffer(buffer);
    }

    let mut tui = tui::Tui::new(core);

    tui.run();
}
