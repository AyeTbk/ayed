use std::{collections::HashMap, path::Path};

use crate::{
    arena::{Arena, Handle},
    buffer::TextBuffer,
    config::{Config, ConfigState},
    highlight::Highlight,
    mode_line::ModeLineInfos,
    text_editor::TextEditor,
    utils::Size,
};

pub struct State {
    pub buffers: Buffers,
    pub editors: Editors,
    pub viewport_size: Size,
    pub mode_line_infos: ModeLineInfos,
    //
    pub active_combo_mode_name: Option<String>,
    pub active_editor_name: String,
    pub active_mode_name: String,
    //
    pub config: Config,
    //
    pub quit: bool,
}

impl State {
    // FIXME The active buffer should be determined by the active editor. The
    // active_buffer_handle field should be removed.
    pub fn active_buffer_handle(&self) -> Handle<TextBuffer> {
        self.buffers.active_buffer_handle
    }

    pub fn create_scratch_buffer(&mut self) -> Handle<TextBuffer> {
        self.buffers.buffers_arena.allocate(TextBuffer::new_empty())
    }

    pub fn get_buffer_from_filepath(&mut self, path: impl AsRef<Path>) -> Handle<TextBuffer> {
        let path = path.as_ref();

        let alreay_opened_buffer = self
            .buffers
            .buffers_arena
            .elements()
            .find_map(|(hnd, buf)| {
                if let Some(f) = buf.filepath() {
                    if f == path {
                        Some(hnd)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        if let Some(buffer) = alreay_opened_buffer {
            buffer
        } else {
            self.buffers
                .buffers_arena
                .allocate(TextBuffer::from_filepath(path.as_ref()))
        }
    }

    pub fn edit_buffer(&mut self, buffer: Handle<TextBuffer>) {
        let maybe_preexisting_editor =
            self.editors.editors_arena.elements().find_map(|(hnd, ed)| {
                if ed.buffer_handle() == buffer {
                    Some(hnd)
                } else {
                    None
                }
            });

        let editor = if let Some(preexisting_editor) = maybe_preexisting_editor {
            preexisting_editor
        } else {
            self.editors.editors_arena.allocate(TextEditor::new(buffer))
        };

        self.set_active_editor(editor);
    }

    pub fn save_buffer(&mut self, buffer: Handle<TextBuffer>) {
        self.buffers.buffers_arena.get(buffer).save().unwrap();
    }

    pub fn set_active_editor(&mut self, editor: Handle<TextEditor>) {
        self.editors.active_editor_handle = editor;

        let active_buffer = self.editors.active_editor().buffer_handle();
        let active_editor_mode = self.editors.active_editor().mode();

        self.buffers.active_buffer_handle = active_buffer;
        self.active_mode_name = active_editor_mode.to_owned();
    }

    pub fn set_active_editor_mode(&mut self, mode: String) {
        self.active_mode_name = mode.clone();
        self.editors.active_editor_mut().set_mode(mode);
    }

    pub fn extract_config_state(&self) -> ConfigState {
        let mut cs = ConfigState::new();
        let file = self
            .buffers
            .active_buffer()
            .filepath()
            .and_then(|p| p.to_str().map(str::to_string))
            .unwrap_or_default();
        cs.set("file", file);
        cs
    }

    pub fn request_quit(&mut self) {
        self.quit = true;
    }
}

pub struct Buffers {
    pub buffers_arena: Arena<TextBuffer>,
    pub active_buffer_handle: Handle<TextBuffer>,
    pub highlights: HashMap<Handle<TextBuffer>, Vec<Highlight>>,
}

impl Buffers {
    pub fn active_buffer(&self) -> &TextBuffer {
        self.buffers_arena.get(self.active_buffer_handle)
    }

    pub fn active_buffer_mut(&mut self) -> &mut TextBuffer {
        self.buffers_arena.get_mut(self.active_buffer_handle)
    }
}

pub struct Editors {
    pub editors_arena: Arena<TextEditor>,
    pub active_editor_handle: Handle<TextEditor>,
}

impl Editors {
    pub fn active_editor(&self) -> &TextEditor {
        self.editors_arena.get(self.active_editor_handle)
    }

    pub fn active_editor_mut(&mut self) -> &mut TextEditor {
        self.editors_arena.get_mut(self.active_editor_handle)
    }
}
