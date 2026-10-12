pub trait Plugin {
    fn name(&mut self) -> String;
    fn load(&mut self, ctx: &mut PluginLoadCtx);
    fn unload(self);
}

pub struct PluginLoadCtx {
    //.
}
