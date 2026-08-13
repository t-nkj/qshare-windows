use std::{io::Read, path::Path};

use anyhow::{Context, Result};
use reqwest::{
    StatusCode,
    blocking::{
        Client, Response,
        multipart::{Form, Part},
    },
};
use serde::Deserialize;

use crate::config::Config;

#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Latest {
    Url { url: SharedUrl },
    Memo { memo: SharedMemo },
    File { files: Vec<SharedFile> },
}

#[derive(Debug, Deserialize)]
pub struct SharedUrl {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct SharedMemo {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct SharedFile {
    pub id: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct UploadResult {
    pub created: Vec<SharedFile>,
    pub failed: Vec<UploadFailure>,
}

#[derive(Debug, Deserialize)]
pub struct UploadFailure {
    pub name: Option<String>,
    pub error: UploadFailureError,
}

#[derive(Debug, Deserialize)]
pub struct UploadFailureError {
    pub message: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .build()
            .context("HTTP クライアントを初期化できません")?;
        Ok(Self {
            client,
            base_url: config.api_base_url.clone(),
            token: config.token.clone(),
        })
    }

    pub fn latest(&self) -> Result<Latest> {
        let response = self
            .request(self.client.get(self.endpoint("v1/latest/muf")))
            .send()?;
        if !response.status().is_success() {
            return Err(response_error(response));
        }
        response
            .json()
            .context("最新のQShareデータを読み取れません")
    }

    pub fn send_memo(&self, content: &str) -> Result<()> {
        let response = self
            .request(
                self.client
                    .post(self.endpoint("v1/memos"))
                    .json(&serde_json::json!({
                        "content": content,
                        "autoDetectUrls": true,
                    })),
            )
            .send()?;
        ensure_success(response)
    }

    pub fn upload_files<R>(&self, files: &[std::path::PathBuf], reader: R) -> Result<UploadResult>
    where
        R: Fn(&Path) -> Result<Box<dyn Read + Send>>,
    {
        let mut form = Form::new();
        for path in files {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .context("ファイル名を取得できません")?;
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            let length = std::fs::metadata(path)
                .with_context(|| format!("ファイルを確認できません: {}", path.display()))?
                .len();
            let part = Part::reader_with_length(reader(path)?, length)
                .file_name(file_name.to_owned())
                .mime_str(&mime)
                .context("ファイルの MIME type を設定できません")?;
            form = form.part("files", part);
        }
        let response = self
            .request(self.client.post(self.endpoint("v1/files")).multipart(form))
            .send()?;
        if !response.status().is_success() {
            return Err(response_error(response));
        }
        response.json().context("ファイル送信結果を読み取れません")
    }

    pub fn download_file(&self, id: &str) -> Result<Response> {
        let response = self
            .request(self.client.get(self.endpoint(&format!("v1/files/{id}"))))
            .send()?;
        if response.status().is_success() {
            return Ok(response);
        }
        Err(response_error(response))
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn request(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        request
            .header("Accept", "application/json")
            .bearer_auth(&self.token)
    }
}

fn ensure_success(response: Response) -> Result<()> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(response_error(response))
    }
}

fn response_error(response: Response) -> anyhow::Error {
    let status = response.status();
    let message = response
        .text()
        .ok()
        .as_deref()
        .and_then(api_error_message)
        .unwrap_or_else(|| format!("QShare API は HTTP {status} を返しました"));
    if status == StatusCode::UNAUTHORIZED {
        anyhow::anyhow!("トークンが無効です。.env の QSHARE_TOKEN を確認してください: {message}")
    } else {
        anyhow::anyhow!("QShare API HTTP {status}: {message}")
    }
}

fn api_error_message(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct ErrorResponse {
        error: ErrorBody,
    }

    #[derive(Deserialize)]
    struct ErrorBody {
        message: String,
    }

    serde_json::from_str::<ErrorResponse>(body)
        .ok()
        .map(|response| response.error.message)
}

#[cfg(test)]
mod tests {
    use super::api_error_message;
    use crate::config::normalize_api_base_url;

    #[test]
    fn base_url_accepts_api_prefix() {
        assert_eq!(
            normalize_api_base_url("https://qshare.trap.show/api/").unwrap(),
            "https://qshare.trap.show/api/"
        );
    }

    #[test]
    fn reads_the_api_error_message() {
        assert_eq!(
            api_error_message(r#"{"error":{"message":"Upload limit exceeded"}}"#).as_deref(),
            Some("Upload limit exceeded")
        );
    }
}
