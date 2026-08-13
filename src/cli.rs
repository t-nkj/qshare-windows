use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(disable_help_flag = true, disable_version_flag = true)]
struct Arguments {
    #[arg(long)]
    sender: bool,
    #[arg(long)]
    receiver: bool,
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Sender(Vec<PathBuf>),
    Receiver,
}

impl Command {
    pub fn try_parse() -> Result<Self> {
        let arguments = Arguments::try_parse();
        Self::from_arguments(arguments?)
    }

    fn from_arguments(arguments: Arguments) -> Result<Self> {
        match (arguments.sender, arguments.receiver) {
            (true, false) => Ok(Self::Sender(arguments.files)),
            (false, true) if arguments.files.is_empty() => Ok(Self::Receiver),
            (false, true) => bail!("--receiver にファイル引数は指定できません"),
            (true, true) => bail!("--sender と --receiver は同時に指定できません"),
            (false, false) => bail!("--sender または --receiver を指定してください"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Arguments, Command};

    #[test]
    fn accepts_sender_with_files() {
        let command = Command::from_arguments(Arguments {
            sender: true,
            receiver: false,
            files: vec!["example.txt".into()],
        })
        .unwrap();
        assert_eq!(command, Command::Sender(vec!["example.txt".into()]));
    }

    #[test]
    fn rejects_receiver_files() {
        let result = Command::from_arguments(Arguments {
            sender: false,
            receiver: true,
            files: vec!["example.txt".into()],
        });
        assert!(result.is_err());
    }
}
