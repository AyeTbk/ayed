use crate::command::CommandRegistry;

mod config;
mod core;
mod editor;
mod list_picker;
mod lsp;
mod misc;

pub fn register_builtin_commands(cr: &mut CommandRegistry) {
    core::register_core_commands(cr);
    config::register_config_commands(cr);
    editor::register_editor_commands(cr);
    misc::register_misc_commands(cr);
    lsp::register_lsp_commands(cr);
    list_picker::register_list_picker_commands(cr);
}
