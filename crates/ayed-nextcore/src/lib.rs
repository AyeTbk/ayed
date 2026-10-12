mod buffer;
mod command;
mod config;
mod core;
mod data;
mod iofacade;
mod plugin;
mod position;
mod range;
mod util;

pub use core::Core;
pub use data::Storage;
pub use plugin::{Plugin, PluginLoadCtx};

use buffer::TextBuffer;
use command::CommandRegistry;
use position::Position;
use range::Range;
