mod editor;
pub use self::editor::Editor;

#[derive(Default)]
pub struct Panels {
    pub editor: Editor,
    pub modeline: (),
    pub warpdrive: (),
}
