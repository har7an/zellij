// The Plan:
// - Replace async-std with tokio
// - Replace isahc with reqwest (which interacts with tokio)
use async_std::sync::Mutex;
use async_std::{
    fs,
    io::WriteExt,
    stream::StreamExt,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use url::Url;

/// Maximum allowed number of HTTP redirects during downloads. Don't set this arbitrarily high to
/// prevent infinite redirection (from e.g. redirection loops).
const MAX_HTTP_REDIRECTS: usize = 64;

#[derive(Error, Debug)]
pub enum DownloaderError {
    #[error("failed to initialize downloader")]
    CantInitialize(#[from] reqwest::Error),
    #[error("failed to send for '{url}' to server")]
    Client {
        url: String,
        from: reqwest::Error,
    },
    #[error("failed to obtain valid reply from server")]
    Server {
        url: String,
        from: reqwest::Error,
    },
    #[error("IoError: {0}")]
    Io(#[source] std::io::Error),
    #[error("StdIoError: {0}")]
    StdIoError(#[from] std::io::Error),
    #[error("File name cannot be found in URL: {0}")]
    NotFoundFileName(String),
    #[error("Failed to parse URL body: {0}")]
    InvalidUrlBody(String),
}

#[derive(Debug, Clone)]
pub struct Downloader {
    client: reqwest::Client,
    location: PathBuf,
    // the whole thing is an Arc/Mutex so that Downloader is thread safe, and the individual values of
    // the HashMap are Arc/Mutexes (Mutexi?) to represent that individual downloads should not
    // happen concurrently
    download_locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

fn http_client() -> Result<reqwest::Client, DownloaderError> {
    // TODO: timeout?
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(MAX_HTTP_REDIRECTS))
        .build()
        .map_err(|e| DownloaderError::CantInitialize(e))
}

impl Downloader {
    pub fn new(location: PathBuf) -> Result<Self, DownloaderError> {
        Ok(Self {
            client: http_client()?,
            location,
            download_locks: Default::default(),
        })
    }

    pub async fn download(
        &self,
        url: &str,
        file_name: Option<&str>,
    ) -> Result<(), DownloaderError> {
        let file_name = match file_name {
            Some(name) => name.to_string(),
            None => self.parse_name(url)?,
        };

        // we do this to make sure only one download of a specific url is happening at a time
        // otherwise the downloads corrupt each other (and we waste lots of system resources)
        let download_lock = self.acquire_download_lock(&file_name).await;
        // it's important that _lock remains in scope, otherwise it gets dropped and the lock is
        // released before the download is complete
        let _lock = download_lock.lock().await;

        let file_path = self.location.join(file_name.as_str());
        if file_path.exists() {
            log::debug!("File already exists: {:?}", file_path);
            return Ok(());
        }
        let file_part_path = self.location.join(format!("{}.part", file_name));
        let (mut target, file_part_size) = {
            if file_part_path.exists() {
                let file_part = fs::OpenOptions::new()
                    .append(true)
                    .write(true)
                    .open(&file_part_path)
                    .await
                    .map_err(|e| DownloaderError::Io(e))?;

                let file_part_size = file_part
                    .metadata()
                    .await
                    .map_err(|e| DownloaderError::Io(e))?
                    .len();

                log::debug!("Resuming download from {} bytes", file_part_size);

                (file_part, file_part_size)
            } else {
                let file_part = fs::File::create(&file_part_path)
                    .await
                    .map_err(|e| DownloaderError::Io(e))?;

                (file_part, 0)
            }
        };
        let res = self.client.get(url)
            .header("Content-Type", "application/octet-stream")
            .header("Range", format!("bytes={}-", file_part_size))
            .send()
            .await?
            .error_for_status()?;
        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            target.write(&chunk).await.unwrap();
        }

        log::debug!("Download complete: {:?}", file_part_path);

        fs::rename(file_part_path, file_path)
            .await
            .map_err(|e| DownloaderError::Io(e))?;

        Ok(())
    }
    pub async fn download_without_cache(url: &str) -> Result<String, DownloaderError> {
        let client = http_client()?;
        let res = client.get(url)
            .header("Content-Type", "application/octet-stream")
            .send()
            .await
            .map_err(|from| DownloaderError::Client{url: url.to_string(), from})?
            .error_for_status()
            .map_err(|from| DownloaderError::Server{url: url.to_string(), from})?;
        let mut stream = res.bytes_stream();

        let mut downloaded_bytes: Vec<u8> = vec![];
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            downloaded_bytes.extend_from_slice(&chunk);
        }

        log::debug!("Download complete");
        let stringified = String::from_utf8(downloaded_bytes)
            .map_err(|e| DownloaderError::InvalidUrlBody(format!("{}", e)))?;

        Ok(stringified)
    }

    fn parse_name(&self, url: &str) -> Result<String, DownloaderError> {
        Url::parse(url)
            .map_err(|_| DownloaderError::NotFoundFileName(url.to_string()))?
            .path_segments()
            .ok_or_else(|| DownloaderError::NotFoundFileName(url.to_string()))?
            .last()
            .ok_or_else(|| DownloaderError::NotFoundFileName(url.to_string()))
            .map(|s| s.to_string())
    }
    async fn acquire_download_lock(&self, file_name: &String) -> Arc<Mutex<()>> {
        let mut lock_dict = self.download_locks.lock().await;
        let download_lock = lock_dict
            .entry(file_name.clone())
            .or_insert_with(|| Default::default());
        download_lock.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[ignore]
    #[async_std::test]
    async fn test_download_ok() {
        let location = tempdir().expect("Failed to create temp directory");
        let location_path = location.path();

        let downloader = Downloader::new(location_path.to_path_buf());
        let result = downloader
            .download(
                "https://github.com/imsnif/monocle/releases/download/0.39.0/monocle.wasm",
                Some("monocle.wasm"),
            )
            .await
            .is_ok();

        assert!(result);
        assert!(location_path.join("monocle.wasm").exists());

        location.close().expect("Failed to close temp directory");
    }

    #[ignore]
    #[async_std::test]
    async fn test_download_without_file_name() {
        let location = tempdir().expect("Failed to create temp directory");
        let location_path = location.path();

        let downloader = Downloader::new(location_path.to_path_buf());
        let result = downloader
            .download(
                "https://github.com/imsnif/multitask/releases/download/0.38.2v2/multitask.wasm",
                None,
            )
            .await
            .is_ok();

        assert!(result);
        assert!(location_path.join("multitask.wasm").exists());

        location.close().expect("Failed to close temp directory");
    }
}
