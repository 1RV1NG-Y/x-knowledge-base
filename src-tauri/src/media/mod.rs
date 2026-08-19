use std::path::{Path, PathBuf};

use crate::enrichment::NormalizedTweet;

#[derive(Clone)]
pub struct MediaCache {
    root: PathBuf,
    client: reqwest::Client,
}

impl MediaCache {
    pub fn new(root: PathBuf, client: reqwest::Client) -> Result<Self, String> {
        for directory in ["images", "thumbnails", "avatars", "videos"] {
            std::fs::create_dir_all(root.join(directory)).map_err(|e| e.to_string())?;
        }
        Ok(Self { root, client })
    }

    pub async fn cache(&self, tweet: &mut NormalizedTweet) {
        if let Some(author) = tweet.author.as_mut() {
            if let Some(url) = author.avatar_url.as_deref() {
                let path = self.root.join("avatars").join(format!(
                    "{}.{}",
                    safe_component(&author.id),
                    extension(url, "jpg")
                ));
                if self.download(url, &path).await.is_ok() {
                    author.avatar_local_path = Some(path.to_string_lossy().into_owned());
                }
            }
        }
        for (index, media) in tweet.media.iter_mut().enumerate() {
            if media.kind == "image" {
                if let Some(url) = media.remote_url.as_deref() {
                    let path = self
                        .root
                        .join("images")
                        .join(safe_component(&tweet.id))
                        .join(format!("{index}.{}", extension(url, "jpg")));
                    if self.download(url, &path).await.is_ok() {
                        media.local_path = Some(path.to_string_lossy().into_owned());
                    }
                }
            } else if matches!(media.kind.as_str(), "video" | "gif") {
                if let Some(url) = media.preview_url.as_deref() {
                    let path = self
                        .root
                        .join("thumbnails")
                        .join(safe_component(&tweet.id))
                        .join(format!("{index}.{}", extension(url, "jpg")));
                    if self.download(url, &path).await.is_ok() {
                        media.thumbnail_path = Some(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    async fn download(&self, url: &str, path: &Path) -> Result<(), String> {
        if path.is_file() {
            return Ok(());
        }
        let parsed = url::Url::parse(url).map_err(|e| e.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("Unsupported media URL scheme".into());
        }
        let response = self
            .client
            .get(parsed)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        if bytes.is_empty() {
            return Err("Media response was empty".into());
        }
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        if !path.exists() {
            tokio::fs::write(path, bytes)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn safe_component(value: &str) -> String {
    let value = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if value.is_empty() {
        "unknown".into()
    } else {
        value
    }
}

fn extension(url: &str, fallback: &str) -> String {
    let extension = url::Url::parse(url).ok().and_then(|url| {
        Path::new(url.path())
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
    });
    match extension.as_deref() {
        Some("jpg" | "jpeg" | "png" | "webp" | "gif") => extension.unwrap(),
        _ => fallback.into(),
    }
}
