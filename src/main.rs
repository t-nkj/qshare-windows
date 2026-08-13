#![cfg_attr(windows, windows_subsystem = "windows")]

mod api;
mod cli;
mod config;
mod files;
mod logging;
mod notification;
mod transfer;

use crate::{cli::Command, config::Config, notification::Notifier};
use anyhow::Result;

fn main() {
    let notifier = Notifier::new();
    if let Err(error) = run(&notifier) {
        logging::error(&error.to_string());
        notifier.error(&error.to_string());
    }
}

fn run(notifier: &Notifier) -> Result<()> {
    if let Some(env_path) = Config::create_env_if_missing()? {
        notifier.setup_required(&env_path);
        return Ok(());
    }
    let config = Config::load()?;
    logging::init(config.log_path.as_deref(), config.log_level)?;
    logging::info("QShare を起動しました");
    if std::env::args_os().len() == 1 {
        notifier.error("--sender または --receiver を指定してください");
        return Ok(());
    }
    let command = Command::try_parse()?;
    transfer::run(command, config, notifier)
}
