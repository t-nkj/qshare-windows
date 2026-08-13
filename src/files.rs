use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

pub fn validate_upload_paths(paths: &[PathBuf]) -> Result<u64> {
    if paths.is_empty() {
        return Ok(0);
    }
    let mut total = 0_u64;
    for path in paths {
        let metadata = fs::metadata(path)
            .with_context(|| format!("ファイルを確認できません: {}", path.display()))?;
        if !metadata.is_file() {
            bail!("通常ファイルだけ送信できます: {}", path.display());
        }
        if metadata.len() > 100 * 1024 * 1024 {
            bail!("ファイルは 100 MiB 以下にしてください: {}", path.display());
        }
        total = total
            .checked_add(metadata.len())
            .context("ファイル合計サイズが大きすぎます")?;
    }
    if total > 1024 * 1024 * 1024 {
        bail!("ファイル合計は 1 GiB 以下にしてください");
    }
    Ok(total)
}

pub fn unique_destination(directory: &Path, name: &str) -> Result<PathBuf> {
    fs::create_dir_all(directory)
        .with_context(|| format!("保存先を作成できません: {}", directory.display()))?;
    let safe_name = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .context("受信ファイル名が不正です")?;
    let candidate = directory.join(safe_name);
    if !candidate.exists() {
        return Ok(candidate);
    }
    let source = Path::new(safe_name);
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(safe_name);
    let extension = source.extension().and_then(|value| value.to_str());
    for index in 1..=u32::MAX {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = directory.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("同名ファイルが多すぎるため保存先を決定できません")
}

pub fn write_response<R: Read, F: FnMut(u64)>(
    mut input: R,
    destination: &Path,
    mut on_bytes: F,
) -> Result<()> {
    let temporary = destination.with_extension(format!(
        "{}.qshare-download",
        destination
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
    ));
    let result = (|| -> Result<()> {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .with_context(|| format!("受信ファイルを作成できません: {}", temporary.display()))?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = input
                .read(&mut buffer)
                .context("ファイルをダウンロードできません")?;
            if read == 0 {
                break;
            }
            output
                .write_all(&buffer[..read])
                .context("受信ファイルを書き込めません")?;
            on_bytes(read as u64);
        }
        output.flush().context("受信ファイルを保存できません")?;
        fs::rename(&temporary, destination)
            .with_context(|| format!("受信ファイルを確定できません: {}", destination.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub struct ProgressReader<R, F> {
    inner: R,
    on_bytes: F,
}

impl<R, F> ProgressReader<R, F> {
    pub fn new(inner: R, on_bytes: F) -> Self {
        Self { inner, on_bytes }
    }
}

impl<R: Read, F: FnMut(u64)> Read for ProgressReader<R, F> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        if read > 0 {
            (self.on_bytes)(read as u64);
        }
        Ok(read)
    }
}

pub fn open_file(path: &Path) -> Result<File> {
    File::open(path).with_context(|| format!("ファイルを開けません: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::unique_destination;

    #[test]
    fn adds_a_number_to_duplicate_name() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("report.pdf"), "old").unwrap();
        assert_eq!(
            unique_destination(directory.path(), "report.pdf")
                .unwrap()
                .file_name()
                .unwrap(),
            "report (1).pdf"
        );
    }
}
