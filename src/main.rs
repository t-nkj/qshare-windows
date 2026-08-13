#![cfg_attr(windows, windows_subsystem = "windows")]

mod api;
mod cli;
mod config;
mod files;
mod notification;
mod transfer;

use crate::{cli::Command, config::Config, notification::Notifier};
use anyhow::Result;

fn main() {
    let notifier = Notifier::new();
    if let Err(error) = run(&notifier) {
        notifier.error(&error.to_string());
    }
}

fn run(notifier: &Notifier) -> Result<()> {
    if let Some(env_path) = Config::create_env_if_missing()? {
        notifier.setup_required(&env_path);
        return Ok(());
    }
    if std::env::args_os().len() == 1 {
        notifier.error("--sender または --receiver を指定してください");
        return Ok(());
    }
    let command = Command::try_parse()?;
    let config = Config::load()?;
    transfer::run(command, config, notifier)
}
