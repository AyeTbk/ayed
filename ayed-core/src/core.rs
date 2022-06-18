use std::path::Path;

use crate::arena::{Arena, Handle};
use crate::buffer::Buffer;
use crate::command::Command;
use crate::input::Input;
use crate::input_mapper::{InputContext, InputMapper, InputMapperImpl};
use crate::text_editor::TextEditor;

pub struct Core {
    buffers: Arena<Buffer>,
    active_editor: TextEditor,
    input_mapper: InputMapperImpl,
    viewport_size: (u32, u32),
}

impl Core {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let start_buffer = buffers.allocate(Buffer::new_scratch());
        let active_editor = TextEditor::new(start_buffer);

        Self {
            buffers,
            active_editor,
            input_mapper: Default::default(),
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
        let command = self
            .input_mapper
            .convert_input_to_command(input, &InputContext::default());
        self.execute_command_in_active_editor(command);
    }

    pub fn execute_command_in_active_editor(&mut self, command: Command) {
        let mut ctx = EditorContext {
            buffers: &mut self.buffers,
            viewport_size: self.viewport_size,
        };
        self.active_editor.execute_command(command, &mut ctx);
    }

    pub fn viewport_size(&mut self) -> (u32, u32) {
        self.viewport_size
    }

    pub fn set_viewport_size(&mut self, viewport_size: (u32, u32)) {
        self.viewport_size = viewport_size;
    }

    pub fn active_editor_viewport_content(&mut self, content: &mut Vec<String>) {
        let ctx = EditorContext {
            buffers: &mut self.buffers,
            viewport_size: self.viewport_size,
        };
        self.active_editor.viewport_content_string(content, &ctx);
    }

    // pub fn active_editor(&self) -> &Buffer {
    //     self.buffers.get(self.active_editor)
    // }

    // pub fn active_editor_selections(&self) -> impl Iterator<Item = SelectionBounds> + '_ {
    //     self.active_editor().selections()
    // }
}

pub struct EditorContext<'a> {
    pub buffers: &'a mut Arena<Buffer>,
    pub viewport_size: (u32, u32),
}
