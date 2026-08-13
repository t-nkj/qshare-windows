use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result, bail};
use arboard::Clipboard;

use crate::{
    api::{ApiClient, Latest, SharedFile},
    cli::Command,
    config::Config,
    files,
    notification::Notifier,
};

pub fn run(command: Command, config: Config, notifier: &Notifier) -> Result<()> {
    let api = ApiClient::new(&config)?;
    match command {
        Command::Sender(paths) if paths.is_empty() => send_clipboard(&api, notifier),
        Command::Sender(paths) => send_files(&api, paths, notifier),
        Command::Receiver => receive(&api, &config.download_dir, notifier),
    }
}

fn send_clipboard(api: &ApiClient, notifier: &Notifier) -> Result<()> {
    let mut clipboard = Clipboard::new().context("クリップボードを開けません")?;
    let text = clipboard
        .get_text()
        .context("クリップボードにテキストがありません")?;
    if text.trim().is_empty() {
        bail!("クリップボードのテキストが空です");
    }
    api.send_memo(&text)?;
    notifier.success("クリップボードの内容をQShareへ送信しました");
    Ok(())
}

fn send_files(api: &ApiClient, paths: Vec<PathBuf>, notifier: &Notifier) -> Result<()> {
    let total = files::validate_upload_paths(&paths)?;
    let progress = TransferProgress::new("QShare ファイル送信中", total, notifier.clone());
    api.upload_files(&paths, |path| {
        let progress = progress.clone();
        let file = files::open_file(path)?;
        Ok(Box::new(files::ProgressReader::new(file, move |bytes| {
            progress.add(bytes)
        })))
    })?;
    progress.complete();
    notifier.success(&format!(
        "{} 件のファイルをQShareへ送信しました",
        paths.len()
    ));
    Ok(())
}

fn receive(api: &ApiClient, download_dir: &std::path::Path, notifier: &Notifier) -> Result<()> {
    match api.latest()? {
        Latest::Memo { memo } => {
            let mut clipboard = Clipboard::new().context("クリップボードを開けません")?;
            clipboard
                .set_text(memo.content)
                .context("メモをクリップボードへコピーできません")?;
            notifier.success("最新のメモをクリップボードへコピーしました");
        }
        Latest::Url { url } => {
            open_browser(&url.url)?;
            notifier.success("最新のURLを標準ブラウザーで開きました");
        }
        Latest::File {
            files: remote_files,
        } => receive_files(api, download_dir, remote_files, notifier)?,
    }
    Ok(())
}

fn receive_files(
    api: &ApiClient,
    download_dir: &std::path::Path,
    remote_files: Vec<SharedFile>,
    notifier: &Notifier,
) -> Result<()> {
    if remote_files.is_empty() {
        bail!("受信するファイルがありません");
    }
    let total = remote_files.iter().map(|file| file.size).sum();
    let progress = TransferProgress::new("QShare ファイル受信中", total, notifier.clone());
    for file in &remote_files {
        let destination = files::unique_destination(download_dir, &file.name)?;
        let response = api.download_file(&file.id)?;
        files::write_response(response, &destination, |bytes| progress.add(bytes))?;
    }
    progress.complete();
    notifier.success(&format!(
        "{} 件のファイルを {} に保存しました",
        remote_files.len(),
        download_dir.display()
    ));
    Ok(())
}

#[cfg(windows)]
fn open_browser(url: &str) -> Result<()> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .context("標準ブラウザーを起動できません")?;
    Ok(())
}

#[cfg(not(windows))]
fn open_browser(_url: &str) -> Result<()> {
    bail!("URLを開く機能は Windows でのみ利用できます")
}

#[derive(Clone)]
struct TransferProgress {
    title: String,
    total: u64,
    completed: Arc<AtomicU64>,
    last_percent: Arc<AtomicU8>,
    notifier: Notifier,
}

impl TransferProgress {
    fn new(title: &str, total: u64, notifier: Notifier) -> Self {
        let progress = Self {
            title: title.to_owned(),
            total,
            completed: Arc::new(AtomicU64::new(0)),
            last_percent: Arc::new(AtomicU8::new(u8::MAX)),
            notifier,
        };
        progress.report(0);
        progress
    }

    fn add(&self, bytes: u64) {
        let completed = self
            .completed
            .fetch_add(bytes, Ordering::Relaxed)
            .saturating_add(bytes);
        self.report(completed);
    }

    fn complete(&self) {
        self.report(self.total);
    }

    fn report(&self, completed: u64) {
        let percent = crate::notification::percentage(completed, self.total);
        if self.last_percent.swap(percent, Ordering::Relaxed) != percent {
            self.notifier.progress(&self.title, completed, self.total);
        }
    }
}
