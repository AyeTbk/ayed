mod tui;

fn main() {
    let mut tui = tui::Tui::new(ayed_core::core::Core::new());
    tui.run();
}
