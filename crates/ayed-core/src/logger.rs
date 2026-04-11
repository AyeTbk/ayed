use std::{fs::File, io::Write, path::PathBuf};

use log::{Level, Log, Metadata, Record, SetLoggerError};

#[allow(dead_code)]
pub struct Logger {
    out_filepath: PathBuf,
    out_file: File,
    err_filepath: PathBuf,
    err_file: File,
}

pub static LOG_LEVEL: Level = if cfg!(debug_assertions) {
    Level::Debug
} else {
    Level::Info
};

impl Logger {
    pub fn init() -> Result<(), SetLoggerError> {
        let mut out_filepath = std::env::temp_dir();
        out_filepath.push("ayed.log");
        let out_file = std::fs::File::options()
            .append(true)
            .create(true)
            .open(&out_filepath)
            .unwrap();

        let mut err_filepath = std::env::temp_dir();
        err_filepath.push("ayed.log");
        let err_file = std::fs::File::options()
            .append(true)
            .create(true)
            .open(&err_filepath)
            .unwrap();

        let logger = Logger {
            out_filepath,
            out_file,
            err_filepath,
            err_file,
        };
        log::set_boxed_logger(Box::new(logger))
            .map(|()| log::set_max_level(LOG_LEVEL.to_level_filter()))
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LOG_LEVEL
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if matches!(record.level(), Level::Error | Level::Debug | Level::Trace) {
            writeln!(
                &self.err_file,
                "[{}][{}:{}] {}",
                record.level(),
                record.file().unwrap_or_default(),
                record.line().unwrap_or_default(),
                record.args()
            )
            .unwrap();
        } else {
            writeln!(
                &self.out_file,
                "[{}][{}] {}",
                record.level(),
                record.module_path().unwrap_or_default(),
                record.args()
            )
            .unwrap();
        }
    }

    fn flush(&self) {}
}
