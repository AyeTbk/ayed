mod buffer;
mod command;
mod core;
mod data;
mod plugin;
mod position;
mod util;

pub use core::Core;
pub use data::Storage;
pub use plugin::{Plugin, PluginLoadCtx};

use buffer::text_buffer::TextBuffer;
use command::CommandRegistry;
use position::Position;
