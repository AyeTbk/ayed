use crate::slotmap::Handle;
use crate::state::View;

mod editor;
pub use self::editor::Editor;

mod line_numbers;
pub use self::line_numbers::LineNumbers;

pub mod modeline;
pub use self::modeline::Modeline;

pub mod warpdrive;
pub use self::warpdrive::Warpdrive;

#[derive(Default)]
pub struct Panels {
    pub editor: Editor,
    pub line_numbers: LineNumbers,
    pub modeline: Modeline,
    pub warpdrive: Warpdrive,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum FocusedPanel {
    #[default]
    Editor,
    Modeline(Handle<View>),
    Warpdrive,
}
