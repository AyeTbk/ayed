use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use crate::arena::{Arena, Handle};
use crate::buffer::Buffer;
use crate::command::Command;
use crate::input::Input;
use crate::input_mapper::{InputContext, InputMapper};
use crate::mode::{TextCommandMode, TextEditMode};
use crate::selection::SelectionBounds;
use crate::text_editor::TextEditor;
use crate::ui_state::{Panel, UiState};

pub struct Core {
    buffers: Arena<Buffer>,
    active_editor: TextEditor,
    modes: HashMap<&'static str, Rc<dyn InputMapper>>,
    active_mode: Rc<dyn InputMapper>,
    viewport_size: (u32, u32),
}

impl Core {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let start_buffer = buffers.allocate(Buffer::new_scratch());
        let active_editor = TextEditor::new(start_buffer);
        let modes = Self::make_default_modes();
        let active_mode = modes.get("text-command").unwrap().clone();

        Self {
            buffers,
            active_editor,
            modes,
            active_mode,
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

    pub fn set_mode(&mut self, mode_name: &str) {
        let mode_rc = self.modes.get(mode_name).unwrap().clone();
        self.active_mode = mode_rc;
    }

    pub fn input(&mut self, input: Input) {
        if let Some(command) = self
            .active_mode
            .convert_input_to_command(input, &InputContext::default())
        {
            match command {
                Command::ChangeMode(mode_name) => self.set_mode(mode_name),
                cmd => self.execute_command_in_active_editor(cmd),
            }
        }
    }

    pub fn execute_command_in_active_editor(&mut self, command: Command) {
        let mut ctx = EditorContextMut {
            buffers: &mut self.buffers,
            viewport_size: self.viewport_size,
        };
        self.active_editor.execute_command(command, &mut ctx);
    }

    pub fn viewport_size(&self) -> (u32, u32) {
        self.viewport_size
    }

    pub fn set_viewport_size(&mut self, viewport_size: (u32, u32)) {
        self.viewport_size = viewport_size;
    }

    pub fn ui_state(&self) -> UiState {
        let active_editor_panel = self.active_editor_viewport_panel();
        let panels = vec![active_editor_panel];
        UiState { panels }
    }

    fn active_editor_viewport_panel(&self) -> Panel {
        let ctx = EditorContext {
            buffers: &self.buffers,
            viewport_size: self.viewport_size,
        };
        self.active_editor.viewport_content_panel(&ctx)
    }

    pub fn active_editor_selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
        self.active_editor.selections()
    }

    fn make_default_modes() -> HashMap<&'static str, Rc<dyn InputMapper>> {
        let mut modes: HashMap<&'static str, Rc<dyn InputMapper>> = HashMap::new();
        modes.insert("text-command", Rc::new(TextCommandMode));
        modes.insert("text-edit", Rc::new(TextEditMode));
        modes
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
