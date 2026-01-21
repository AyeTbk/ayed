use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::{
    config::Config,
    input::Input,
    panels::{
        FocusedPanel,
        file_picker::FilePickerState,
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
    pub file_picker: FilePickerState,
    pub focused_panel: FocusedPanel,
    pub quit_requested: bool,
    pub viewport_size: Size,
    pub editor_rect: Rect,
    pub modeline_rect: Rect,
    pub file_picker_rect: Rect,
    pub last_input: Option<Input>,
    pub working_directory: PathBuf,
}

impl State {
    pub fn focused_view(&self) -> Option<Handle<View>> {
        match self.focused_panel {
            FocusedPanel::Editor | FocusedPanel::Warpdrive => self.active_editor_view,
            FocusedPanel::Modeline(view) => Some(view),
            FocusedPanel::FilePicker(view) => Some(view),
        }
    }

    pub fn focused_view_rect(&self, resources: &Resources) -> Rect {
        let (view_handle, panel_rect) = match self.focused_panel {
            FocusedPanel::Modeline(handle) => (Some(handle), self.modeline_rect),
            FocusedPanel::FilePicker(handle) => (Some(handle), self.file_picker_rect),
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
            // Path info
            let mut path_text = buffer.path().unwrap_or(Path::new("<scratch>")).to_string_lossy().to_string();
            if buffer.is_dirty() {
                path_text.push_str("*");
            }
            let path_info = ModelineInfo {
                text: path_text,
                style: Style::default(),
                align: Align::Right,
            };
            infos.push(path_info);

            // Cursor info
            let sels = buffer
                .view_selections(
                    resources
                        .view_with_buffer(active_editor_buffer_handle)
                        .unwrap(),
                )
                .unwrap();
            let cursor = sels.primary().cursor;
            let logicursor = buffer.map_true_position_to_logical_position(cursor, &self.config);
            let cursor_info = ModelineInfo {
                text: format!("{cursor} / {logicursor}"),
                style: Style::default(),
                align: Align::Right,
            };
            infos.push(cursor_info);
        }

        self.modeline.infos = infos;
    }

    /// Convert path to an absolute path.
    /// If path was already absolute, the returned value is unchanged.
    /// If path was relative, the returned value is made absolute, using
    /// `state.working_directory`` as base.
    pub fn normalize_path(&self, path: &str) -> PathBuf {
        let ppath = Path::new(path);
        if ppath.is_absolute() {
            ppath.to_path_buf()
        } else {
            let absolute_path = self.working_directory.join(path);
            absolute_path
        }
    }

    /// Converts path to a relative path, if it is a descendant of
    /// `state.working_directory``, else returns the path unchanged.
    pub fn denormalize_path(&self, path: &str) -> PathBuf {
        let mut is_descendant_of_working_directory = false;
        let mut new_path = PathBuf::new();
        for part in Path::new(path).iter() {
            if !is_descendant_of_working_directory && new_path == self.working_directory {
                is_descendant_of_working_directory = true;
                new_path.clear();
            }

            new_path.push(part);
        }

        new_path
    }
}
