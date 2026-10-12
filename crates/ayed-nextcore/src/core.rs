use crate::{Plugin, PluginLoadCtx, Storage, command::CommandRegistry};

pub struct Core {
    commands: CommandRegistry,
    data: Storage,
}

impl Core {
    pub fn register_plugin<T: Plugin>(&mut self, mut plugin: T) {
        let mut ctx = PluginLoadCtx {
            data: &mut self.data,
        };
        plugin.load(&mut ctx);
    }
}
