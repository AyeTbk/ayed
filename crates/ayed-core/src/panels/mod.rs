use crate::slotmap::Handle;
use crate::state::{Resources, State, View};

mod editor;
pub use self::editor::Editor;

mod line_numbers;
pub use self::line_numbers::LineNumbers;

pub mod modeline;
pub use self::modeline::Modeline;

pub mod file_picker;
pub use self::file_picker::FilePicker;

pub mod warpdrive;
pub use self::warpdrive::Warpdrive;

mod combo;
pub use self::combo::Combo;

mod suggestions;
pub use self::suggestions::Suggestions;

#[derive(Default)]
pub struct Panels {
    pub editor: Editor,
    pub line_numbers: LineNumbers,
    pub modeline: Modeline,
    pub file_picker: FilePicker,
    pub warpdrive: Warpdrive,
    pub combo: Combo,
    pub suggestion: Suggestions,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    #[default]
    Editor,
    Modeline(Handle<View>),
    FilePicker(Handle<View>),
    Warpdrive,
}

pub struct RenderPanelContext<'a> {
    pub state: &'a State,
    pub resources: &'a Resources,
}
