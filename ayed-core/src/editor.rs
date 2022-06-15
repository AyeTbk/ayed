use crate::arena::{Arena, Handle};
use crate::buffer::Buffer;
use crate::command::Command;
use crate::input::Input;
use crate::input_mapper::{InputContext, InputMapper, InputMapperImpl};

pub struct Editor {
    buffers: Arena<Buffer>,
    active_buffer: Handle<Buffer>,
    input_mapper: InputMapperImpl,
    viewport_size: (u32, u32),
}

impl Editor {
    pub fn new() -> Self {
        let mut buffers = Arena::new();
        let active_buffer = buffers.allocate(Buffer::new());

        Self {
            buffers,
            active_buffer,
            input_mapper: Default::default(),
            viewport_size: (80, 25),
        }
    }

    pub fn create_scratch_buffer(&mut self) -> Handle<Buffer> {
        self.buffers.allocate(Buffer::new())
    }

    pub fn input(&mut self, input: Input) {
        let command = self
            .input_mapper
            .convert_input_to_command(input, &InputContext::default());
        self.execute_command_in_active_buffer(command);
    }

    pub fn execute_command_in_active_buffer(&mut self, command: Command) {
        self.execute_command_in_buffer(self.active_buffer, command)
    }

    pub fn execute_command_in_buffer(&mut self, buffer_handle: Handle<Buffer>, command: Command) {
        let buffer = self.buffers.get_mut(buffer_handle);
        buffer.execute_command(command, self.viewport_size);
    }

    pub fn set_viewport_size(&mut self, viewport_size: (u32, u32)) {
        self.viewport_size = viewport_size;
    }

    pub fn active_buffer_viewport_content<'a>(&'a self, content: &mut Vec<&'a str>) {
        self.active_buffer()
            .viewport_content_string(content, self.viewport_size);
    }

    pub fn active_buffer(&self) -> &Buffer {
        self.buffers.get(self.active_buffer)
    }
}
