use crate::slotmap::Handle;
use crate::state::View;

mod editor;
pub use self::editor::Editor;

mod line_numbers;
pub use self::line_numbers::LineNumbers;

pub mod modeline;
pub use self::modeline::Modeline;

#[derive(Default)]
pub struct Panels {
    pub editor: Editor,
    pub line_numbers: LineNumbers,
    pub modeline: Modeline,
    pub warpdrive: (),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum FocusedPanel {
    #[default]
    Editor,
    Modeline(Handle<View>),
}

fn line_clamped_filled(line: &str, start: u32, char_count: u32, fill: char) -> String {
    let mut s = String::new();
    let mut char_taken_count = 0;
    for ch in line.chars().skip(start as _).take(char_count as _) {
        s.push(ch);
        char_taken_count += 1;
    }
    let missing_char_count = char_count.saturating_sub(char_taken_count);
    for _ in 0..missing_char_count {
        s.push(fill);
    }
    s
}
