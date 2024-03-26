mod editor;
pub use self::editor::Editor;

pub mod modeline;
pub use self::modeline::Modeline;

#[derive(Default)]
pub struct Panels {
    pub editor: Editor,
    pub modeline: Modeline,
    pub warpdrive: (),
}

fn line_clamped_filled(line: &str, char_count: u32, fill: char) -> String {
    let mut s = String::new();
    let mut char_taken_count = 0;
    for ch in line.chars().take(char_count as _) {
        s.push(ch);
        char_taken_count += 1;
    }
    let missing_char_count = char_count.saturating_sub(char_taken_count);
    for _ in 0..missing_char_count {
        s.push(fill);
    }
    s
}
