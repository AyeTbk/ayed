use std::rc::Rc;

pub struct CommandRegistry {}

type CommandResult = Result<(), String>;
