use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::buffer::Buffer;
use crate::input::Input;
use crate::mode_line::{ModeLine, ModeLineInfo};
use crate::panel::Panel;
use crate::selection::SelectionBounds;
use crate::text_editor::TextEditor;
use crate::ui_state::{Panel as UiPanel, UiState};

pub struct Core {
    buffers: Arena<Buffer>,
    active_editor: TextEditor,
    mode_line: ModeLine,
    viewport_size: (u32, u32),
}

impl Core {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let start_buffer = buffers.allocate(Buffer::new_scratch());
        let active_editor = TextEditor::new(start_buffer);
        let mode_line = ModeLine::new();

        Self {
            buffers,
            active_editor,
            mode_line,
            viewport_size: (80, 25),
        }
    }

    pub fn create_buffer_from_filepath(&mut self, path: impl AsRef<Path>) -> Handle<Buffer> {
        self.buffers.allocate(Buffer::from_filepath(path.as_ref()))
    }

    pub fn create_scratch_buffer(&mut self) -> Handle<Buffer> {
        self.buffers.allocate(Buffer::new_scratch())
    }

    pub fn edit_buffer(&mut self, buffer: Handle<Buffer>) {
        self.active_editor = TextEditor::new(buffer);
    }

    pub fn input(&mut self, input: Input) {
        if self.mode_line.wants_focus() {
            let mut ctx = EditorContextMut {
                viewport_size: self.mode_line_viewport_size(),
                buffers: &mut self.buffers,
            };
            self.mode_line.input(input, &mut ctx);
        } else {
            let mut ctx = EditorContextMut {
                viewport_size: self.active_editor_viewport_size(),
                buffers: &mut self.buffers,
            };
            self.active_editor.input(input, &mut ctx);
        }
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        self.viewport_size
    }

    pub fn set_viewport_size(&mut self, viewport_size: (u32, u32)) {
        self.viewport_size = viewport_size;
    }

    pub fn ui_state(&mut self) -> UiState {
        let active_editor_panel = self.active_editor_panel();

        self.mode_line.set_infos(self.mode_line_infos());

        let mode_line_panel = self.mode_line_panel();
        let panels = vec![active_editor_panel, mode_line_panel];
        UiState { panels }
    }

    pub fn active_editor_selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
        self.active_editor.selections()
    }

    fn active_editor_panel(&self) -> UiPanel {
        let ctx = EditorContext {
            buffers: &self.buffers,
            viewport_size: self.active_editor_viewport_size(),
        };
        self.active_editor.panel(&ctx)
    }

    fn active_editor_viewport_size(&self) -> (u32, u32) {
        (self.viewport_size.0, self.viewport_size.1 - 1)
    }

    fn mode_line_panel(&self) -> UiPanel {
        let ctx = EditorContext {
            buffers: &self.buffers,
            viewport_size: self.mode_line_viewport_size(),
        };

        let mut panel = self.mode_line.panel(&ctx);
        panel.position.1 = self.viewport_size.1 - 1;
        panel
    }

    fn mode_line_infos(&self) -> Vec<ModeLineInfo> {
        let ctx = EditorContext {
            buffers: &self.buffers,
            viewport_size: self.active_editor_viewport_size(),
        };
        self.active_editor.mode_line_infos(&ctx)
    }

    fn mode_line_viewport_size(&self) -> (u32, u32) {
        (self.viewport_size.0, 1)
    }
}

pub struct EditorContextMut<'a> {
    pub buffers: &'a mut Arena<Buffer>,
    pub viewport_size: (u32, u32),
}

pub struct EditorContext<'a> {
    pub buffers: &'a Arena<Buffer>,
    pub viewport_size: (u32, u32),
}
