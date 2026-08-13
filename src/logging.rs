use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};

use crate::config::LogLevel;

struct Logger {
    file: Mutex<std::fs::File>,
    level: LogLevel,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub fn init(path: Option<&Path>, level: Option<LogLevel>) -> Result<()> {
    let (Some(path), Some(level)) = (path, level) else {
        return Ok(());
    };
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("ログファイルを開けません: {}", path.display()))?;
    let _ = LOGGER.set(Logger {
        file: Mutex::new(file),
        level,
    });
    info("ログを有効化しました");
    Ok(())
}

pub fn error(message: &str) {
    write(LogLevel::Error, "ERROR", message);
}

pub fn info(message: &str) {
    write(LogLevel::Info, "INFO", message);
}

pub fn debug(message: &str) {
    write(LogLevel::Debug, "DEBUG", message);
}

fn write(level: LogLevel, label: &str, message: &str) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    if level > logger.level {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    if let Ok(mut file) = logger.file.lock() {
        let _ = writeln!(file, "{timestamp} [{label}] {message}");
    }
}
