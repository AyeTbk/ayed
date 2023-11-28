use crate::state::State;

pub struct ScriptedCommand {
    f: Box<dyn FnMut(&mut State, &str) -> Result<(), String>>,
}

impl ScriptedCommand {
    pub fn new(f: impl FnMut(&mut State, &str) -> Result<(), String> + 'static) -> Self {
        Self { f: Box::new(f) }
    }

    pub fn call(&mut self, state: &mut State, args: &str) -> Result<(), String> {
        (self.f)(state, args)
    }
}
