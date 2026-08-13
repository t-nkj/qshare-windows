use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

const DEFAULT_API_BASE_URL: &str = "https://qshare.trap.show/api/";
const ENV_EXAMPLE: &str = include_str!("../.env.example");

#[derive(Clone, Debug)]
pub struct Config {
    pub token: String,
    pub api_base_url: String,
    pub download_dir: PathBuf,
}

impl Config {
    pub fn create_env_if_missing() -> Result<Option<PathBuf>> {
        let env_path = Self::env_path()?;
        if env_path.exists() {
            return Ok(None);
        }
        fs::write(&env_path, ENV_EXAMPLE)
            .with_context(|| format!("設定ファイルを作成できません: {}", env_path.display()))?;
        Ok(Some(env_path))
    }

    pub fn load() -> Result<Self> {
        let env_path = Self::env_path()?;
        dotenvy::from_path_override(&env_path)
            .with_context(|| format!("設定ファイルを読み込めません: {}", env_path.display()))?;

        let token = env::var("QSHARE_TOKEN").context(".env に QSHARE_TOKEN を設定してください")?;
        if token.trim().is_empty() {
            bail!("QSHARE_TOKEN は空にできません");
        }
        let api_base_url = normalize_api_base_url(
            &env::var("API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_owned()),
        )?;
        let download_dir = match env::var_os("QSHARE_DOWNLOAD_DIR") {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => Self::executable_dir()?.join("files"),
        };
        Ok(Self {
            token,
            api_base_url,
            download_dir,
        })
    }

    fn env_path() -> Result<PathBuf> {
        Ok(Self::executable_dir()?.join(".env"))
    }

    fn executable_dir() -> Result<PathBuf> {
        let executable = env::current_exe().context("実行ファイルの場所を取得できません")?;
        Ok(executable
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf())
    }
}

pub fn normalize_api_base_url(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        bail!("API_BASE_URL は http:// または https:// で指定してください");
    }
    Ok(format!("{}/", trimmed.trim_end_matches('/')))
}

#[cfg(test)]
mod tests {
    use super::normalize_api_base_url;

    #[test]
    fn normalizes_trailing_slash() {
        assert_eq!(
            normalize_api_base_url("https://example.test/api").unwrap(),
            "https://example.test/api/"
        );
    }
}
