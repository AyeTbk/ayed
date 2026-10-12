use std::any::Any;

use crate::Storage;

pub trait Plugin {
    fn name(&mut self) -> String;
    fn load(&mut self, ctx: &mut PluginLoadCtx);
}

pub struct PluginLoadCtx<'a> {
    pub(crate) data: &'a mut Storage,
}

impl<'a> PluginLoadCtx<'a> {
    pub fn register_data<T: Any>(&mut self, data: T) {
        self.data.insert(data);
    }
}
