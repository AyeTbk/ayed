use crate::{
    config::Config,
    input::Input,
    panels::{
        modeline::{Align, ModelineInfo, ModelineState},
        FocusedPanel,
    },
    slotmap::{Handle, SlotMap},
    ui::{Rect, Size, Style},
};

mod text_buffer;
pub use text_buffer::TextBuffer;

mod view;
pub use view::View;

#[derive(Default)]
pub struct State {
    pub views: SlotMap<View>,
    pub active_editor_view: Option<Handle<View>>,
    pub buffers: SlotMap<TextBuffer>,
    pub config: Config,
    pub modeline: ModelineState,
    pub focused_panel: FocusedPanel,
    pub quit_requested: bool,
    pub viewport_size: Size,
    pub editor_size: Size,
    pub last_input: Option<Input>,
}

impl State {
    pub fn open_file(&mut self, path: &str) -> Result<Handle<TextBuffer>, String> {
        Ok(self.buffers.insert(TextBuffer::new_from_path(path)?))
    }

    pub fn buffer_with_path(&self, path: &str) -> Option<Handle<TextBuffer>> {
        self.buffers
            .iter()
            .find(|(_, buf)| buf.path() == Some(path))
            .map(|(handle, _)| handle)
    }

    pub fn view_with_buffer(&self, buffer: Handle<TextBuffer>) -> Option<Handle<View>> {
        self.views
            .iter()
            .find(|(_, view)| view.buffer == buffer)
            .map(|(handle, _)| handle)
    }

    pub fn focused_view(&self) -> Option<Handle<View>> {
        match self.focused_panel {
            FocusedPanel::Editor => self.active_editor_view,
            FocusedPanel::Modeline(view) => Some(view),
        }
    }

    pub fn focused_view_rect(&self) -> Rect {
        let top_left = self
            .active_editor_view
            .map(|handle| self.views.get(handle).top_left)
            .unwrap_or_default();
        Rect::with_position_and_size(top_left, self.editor_size)
    }

    pub fn active_editor_buffer(&self) -> Option<Handle<TextBuffer>> {
        Some(self.views.get(self.active_editor_view?).buffer)
    }

    pub fn active_editor_buffer_path(&self) -> Option<&str> {
        self.buffers.get(self.active_editor_buffer()?).path()
    }

    pub fn fill_modeline_infos(&mut self) {
        let hello_info = ModelineInfo {
            text: "hello".to_string(),
            style: Style::default(),
            align: Align::Left,
        };

        let mut input_text = String::new();
        self.last_input
            .map(|input| input.serialize(&mut input_text));
        let input_info = ModelineInfo {
            text: input_text,
            style: Style::default(),
            align: Align::Right,
        };

        let path_info = ModelineInfo {
            text: self
                .active_editor_buffer_path()
                .unwrap_or("<no path>")
                .to_string(),
            style: Style::default(),
            align: Align::Right,
        };

        self.modeline.infos = vec![hello_info, input_info, path_info]
    }
}
