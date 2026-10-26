//! A logger for the `log` crate. On Cloud Run each message is one JSON line, otherwise plain text.

use std::error::Error;
use std::process::ExitCode;

use log::{Level, LevelFilter, Metadata, Record};

struct Logger {
    on_cloud_run: bool,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let severity = match record.level() {
            Level::Error => "ERROR",
            Level::Warn => "WARNING",
            Level::Info => "INFO",
            Level::Debug | Level::Trace => "DEBUG",
        };
        let message = record.args();
        if self.on_cloud_run {
            eprintln!(
                "{}",
                serde_json::json!({ "severity": severity, "message": message.to_string() })
            );
        } else {
            eprintln!("{:<9} {message}", format!("[{severity}]"));
        }
    }

    fn flush(&self) {}
}

/// Sends the `log` macros' messages to stderr.
pub fn init() {
    let on_cloud_run =
        std::env::var_os("K_SERVICE").is_some() || std::env::var_os("CLOUD_RUN_JOB").is_some();
    if log::set_boxed_logger(Box::new(Logger { on_cloud_run })).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}

pub fn exit_code(result: Result<(), Box<dyn Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log::error!("{}", crate::report(&*err));
            ExitCode::FAILURE
        }
    }
}
