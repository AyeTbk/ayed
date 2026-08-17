use ayed_glob::Glob;

fn main() {
    let g = Glob::new("/crates/ayed-tui/stderr.txt");
    dbg!(&g);
    // Should not match
    dbg!(g.is_match_path("/crates/ayed-tui", false));
}
