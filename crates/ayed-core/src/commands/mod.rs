use crate::command::CommandRegistry;

pub mod completions;
mod config;
mod core;
mod editor;
mod lsp;
mod misc;

pub fn register_builtin_commands(cr: &mut CommandRegistry) {
    core::register_core_commands(cr);
    config::register_config_commands(cr);
    editor::register_editor_commands(cr);
    misc::register_misc_commands(cr);
    lsp::register_lsp_commands(cr);
    completions::register_completions_commands(cr);
}
