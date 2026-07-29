use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use ayed_lsp_client::LspClient;

use crate::{
    config::Config,
    input::Input,
    panels::{Panels, list_picker::ListPickerState},
    position::Column,
    slotmap::Handle,
    ui::{Color, Rect, Size, Style},
};

mod text_buffer;
pub use text_buffer::{TextBuffer, TextEdit};

mod text_buffer_history;
pub use text_buffer_history::TextBufferHistory;

mod view;
pub use view::View;

mod highlight;
pub use highlight::{Highlight, regex_syntax_highlight};

mod register;
pub use register::Register;

mod resources;
pub use resources::Resources;

mod diagnostics;
pub use diagnostics::{Diagnostic, DiagnosticKind, DiagnosticSource, Diagnostics};

mod completions;
pub use completions::{
    CompletionItem, CompletionItemKind, CompletionSource, CompletionSourceData, Completions,
};

mod modeline;
pub use modeline::{Align, ModelineInfo, ModelineState};

#[derive(Default)]
pub struct State {
    pub is_async_task_ready: Arc<AtomicBool>,
    pub active_editor_view: Option<Handle<View>>,
    pub per_buffer: HashMap<Handle<TextBuffer>, PerBufferState>,
    pub diagnostics: Diagnostics,
    pub completions: Completions,
    pub register: Register,
    pub config: Config,
    pub modeline: ModelineState,
    pub hover_info: Option<String>,
    pub list_picker: ListPickerState,
    pub focused_panel: String,
    pub focused_panel_view_rect: Option<Rect>,
    pub quit_requested: bool,
    pub viewport_size: Size,
    pub editor_rect: Rect,
    pub editor_line_numbers_width: Column,
    pub modeline_rect: Rect,
    pub last_input: Option<Input>,
    pub delta_time: f32,
    pub working_directory: PathBuf,
    pub lsp_client: Option<LspClient>, // TODO Should be one per server type / configured file extension, i guess?
}

#[derive(Default)]
pub struct PerBufferState {
    pub highlights: Vec<Highlight>,
}

impl State {
    pub fn focused_view(&self, panels: &Panels) -> Option<Handle<View>> {
        panels
            .panel_with_name(&self.focused_panel)
            .and_then(|p| p.view())
            .or(self.active_editor_view)
    }

    pub fn focused_view_content_rect(
        &self,
        panels: &Panels,
        resources: &Resources,
    ) -> Option<Rect> {
        let focused_panel = panels.panel_with_name(&self.focused_panel)?;
        let panel_rect = focused_panel.content_rect();
        let view_handle = focused_panel.view().or(self.active_editor_view);
        let top_left = view_handle
            .map(|handle| resources.views.get(handle).top_left)
            .unwrap_or_default();
        Some(Rect::with_position_and_size(top_left, panel_rect.size()))
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

        let diagnostics_stats = self.diagnostics.stats();
        let diagnostics_info = [
            (diagnostics_stats.error_count, "⊙", Color::ERROR),
            (diagnostics_stats.warning_count, "⊙", Color::WARNING),
            (diagnostics_stats.other_count, "⊙", Color::INFO),
        ];
        for (count, icon, color) in diagnostics_info {
            if count < 1 {
                continue;
            }
            let info = ModelineInfo {
                text: format!("{icon}{count}",),
                style: Style {
                    foreground_color: Some(color),
                    ..Default::default()
                },
                align: Align::Left,
            };
            infos.push(info);
        }

        if let Some(active_editor_buffer_handle) = self.active_editor_buffer(resources) {
            let buffer = resources.buffers.get(active_editor_buffer_handle);
            // Path info
            let display_path =
                self.denormalize_path(buffer.path().unwrap_or(Path::new("<scratch>")));
            let mut path_text = display_path.to_string_lossy().to_string();
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

        if let Some(lsp_client) = &self.lsp_client
            && !lsp_client.is_online()
        {
            infos.push(ModelineInfo {
                text: "⧖".to_string(),
                align: Align::Left,
                style: Style::default(),
            });
        }

        self.modeline.infos = infos;
    }

    /// Convert path to an absolute path.
    /// If path was already absolute, the returned value is unchanged.
    /// If path was relative, the returned value is made absolute, using
    /// `state.working_directory`` as base.
    pub fn normalize_path(&self, path: &Path) -> PathBuf {
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
    pub fn denormalize_path(&self, path: &Path) -> PathBuf {
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
