use std::collections::HashMap;

use crate::{
    config::Config,
    input::Input,
    panels::{
        FocusedPanel,
        modeline::{Align, ModelineInfo, ModelineState},
    },
    slotmap::Handle,
    ui::{Rect, Size, Style},
};

mod text_buffer_history;
pub use text_buffer_history::TextBufferHistory;

mod text_buffer;
pub use text_buffer::TextBuffer;

mod view;
pub use view::View;

mod highlight;
pub use highlight::{Highlight, regex_syntax_highlight};

mod register;
pub use register::Register;

mod resources;
pub use resources::Resources;

mod suggestions;
pub use suggestions::Suggestions;

#[derive(Default)]
pub struct State {
    pub active_editor_view: Option<Handle<View>>,
    pub highlights: HashMap<Handle<TextBuffer>, Vec<Highlight>>,
    pub edit_histories: HashMap<Handle<TextBuffer>, TextBufferHistory>,
    pub suggestions: Suggestions,
    pub register: Register,
    pub config: Config,
    pub modeline: ModelineState,
    pub focused_panel: FocusedPanel,
    pub quit_requested: bool,
    pub viewport_size: Size,
    pub editor_rect: Rect,
    pub modeline_rect: Rect,
    pub last_input: Option<Input>,
}

impl State {
    pub fn focused_view(&self) -> Option<Handle<View>> {
        match self.focused_panel {
            FocusedPanel::Editor | FocusedPanel::Warpdrive => self.active_editor_view,
            FocusedPanel::Modeline(view) => Some(view),
        }
    }

    pub fn focused_view_rect(&self, resources: &Resources) -> Rect {
        let (view_handle, panel_rect) = match self.focused_panel {
            FocusedPanel::Modeline(handle) => (Some(handle), self.modeline_rect),
            FocusedPanel::Editor | FocusedPanel::Warpdrive => {
                (self.active_editor_view, self.editor_rect)
            }
        };
        let top_left = view_handle
            .map(|handle| resources.views.get(handle).top_left)
            .unwrap_or_default();
        Rect::with_position_and_size(top_left, panel_rect.size())
    }

    pub fn active_editor_view_rect(&self, resources: &Resources) -> Rect {
        let (view_handle, panel_rect) = (self.active_editor_view, self.editor_rect);
        let top_left = view_handle
            .map(|handle| resources.views.get(handle).top_left)
            .unwrap_or_default();
        Rect::with_position_and_size(top_left, panel_rect.size())
    }

    pub fn active_editor_buffer(&self, resources: &Resources) -> Option<Handle<TextBuffer>> {
        Some(resources.views.get(self.active_editor_view?).buffer)
    }

    pub fn fill_modeline_infos(&mut self, resources: &Resources) {
        let mode_info = ModelineInfo {
            text: self
                .config
                .state_value("mode")
                .unwrap_or("<no mode>")
                .to_string(),
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

        let mut infos = vec![mode_info, input_info];

        if let Some(active_editor_buffer_handle) = self.active_editor_buffer(resources) {
            let buffer = resources.buffers.get(active_editor_buffer_handle);
            let mut path_text = buffer.path().unwrap_or("<scratch>").to_string();
            if buffer.is_dirty() {
                path_text.push_str("*");
            }
            let path_info = ModelineInfo {
                text: path_text,
                style: Style::default(),
                align: Align::Right,
            };
            infos.push(path_info);
        }

        self.modeline.infos = infos;
    }
}
