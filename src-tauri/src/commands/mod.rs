use std::{path::PathBuf, process::Stdio, sync::Arc};

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::{
    archive,
    db::models::{ImportSummary, JobStatus, ProfileState, Tweet, TweetPage, TweetQuery},
    Backend,
};

const MAX_VIDEO_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EMBEDDED_VIDEO_BITRATE: u64 = 2_500_000;

fn embedded_video_url(media_id: &str, fallback: &str, raw_json: Option<&str>) -> String {
    let Some(raw_json) = raw_json else {
        return fallback.into();
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(raw_json) else {
        return fallback.into();
    };
    let Some(media) = root
        .get("tweet")
        .and_then(|tweet| tweet.get("media"))
        .and_then(|media| media.get("all"))
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items.iter().find(|item| {
                item.get("id")
                    .and_then(|id| {
                        id.as_str()
                            .map(str::to_owned)
                            .or_else(|| id.as_u64().map(|id| id.to_string()))
                    })
                    .as_deref()
                    == Some(media_id)
            })
        })
    else {
        return fallback.into();
    };
    let variants = media
        .get("formats")
        .or_else(|| media.get("variants"))
        .and_then(serde_json::Value::as_array);
    variants
        .into_iter()
        .flatten()
        .filter_map(|variant| {
            let url = variant.get("url")?.as_str()?;
            let bitrate = variant.get("bitrate")?.as_u64()?;
            let is_mp4 = variant.get("container").and_then(serde_json::Value::as_str)
                == Some("mp4")
                || variant
                    .get("content_type")
                    .and_then(serde_json::Value::as_str)
                    == Some("video/mp4");
            (is_mp4 && bitrate <= MAX_EMBEDDED_VIDEO_BITRATE).then_some((bitrate, url))
        })
        .max_by_key(|(bitrate, _)| *bitrate)
        .map(|(_, url)| url.to_owned())
        .unwrap_or_else(|| fallback.into())
}

fn validated_video_url(raw: &str) -> Result<url::Url, String> {
    let url =
        url::Url::parse(raw).map_err(|error| format!("Stored video URL is invalid: {error}"))?;
    if url.scheme() != "https" || url.host_str() != Some("video.twimg.com") {
        return Err("Stored video URL is not an allowed X media URL".into());
    }
    Ok(url)
}

#[tauri::command]
pub async fn import_archive(
    path: String,
    state: State<'_, Arc<Backend>>,
) -> Result<ImportSummary, String> {
    state.queue.begin_import(&path);
    let archive_path = PathBuf::from(&path);
    let db = state.db.clone();
    let parsed_path = archive_path.clone();
    let result = match tokio::task::spawn_blocking(move || {
        let parsed = archive::parse_archive(&parsed_path)?;
        db.import_archive(&parsed, &parsed_path)
    })
    .await
    {
        Ok(result) => result,
        Err(error) => {
            let message = format!("Archive import task failed: {error}");
            state.queue.import_failed(message.clone());
            return Err(message);
        }
    };
    match result {
        Ok((summary, ids)) => {
            if ids.is_empty() {
                state
                    .queue
                    .import_complete(summary.imported, summary.existing, summary.failed);
            } else {
                state.queue.enqueue(ids);
            }
            Ok(summary)
        }
        Err(error) => {
            state.queue.import_failed(error.clone());
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn get_tweets(
    request: TweetQuery,
    state: State<'_, Arc<Backend>>,
) -> Result<TweetPage, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_tweets(request))
        .await
        .map_err(|error| format!("Tweet query task failed: {error}"))?
}

#[tauri::command]
pub async fn get_tweet(id: String, state: State<'_, Arc<Backend>>) -> Result<Tweet, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_tweet(&id))
        .await
        .map_err(|error| format!("Tweet query task failed: {error}"))?
}

#[tauri::command]
pub async fn get_profiles(state: State<'_, Arc<Backend>>) -> Result<ProfileState, String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.get_profiles())
        .await
        .map_err(|error| format!("Profile query task failed: {error}"))?
}

#[tauri::command]
pub async fn set_active_profile(
    profile_id: String,
    state: State<'_, Arc<Backend>>,
) -> Result<(), String> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.set_active_profile(&profile_id))
        .await
        .map_err(|error| format!("Profile switch task failed: {error}"))?
}

#[tauri::command]
pub async fn retry_enrichment(id: String, state: State<'_, Arc<Backend>>) -> Result<(), String> {
    let db = state.db.clone();
    let queued_id = id.clone();
    tokio::task::spawn_blocking(move || db.retry(&id))
        .await
        .map_err(|error| format!("Retry task failed: {error}"))??;
    state.queue.enqueue([queued_id]);
    Ok(())
}

#[tauri::command]
pub fn get_import_status(state: State<'_, Arc<Backend>>) -> JobStatus {
    state.queue.status()
}

#[tauri::command]
pub async fn load_video(
    media_id: String,
    state: State<'_, Arc<Backend>>,
) -> Result<tauri::ipc::Response, String> {
    let db = state.db.clone();
    let query_media_id = media_id.clone();
    let (fallback_url, raw_json) =
        tokio::task::spawn_blocking(move || db.video_source(&query_media_id))
            .await
            .map_err(|error| format!("Video URL query task failed: {error}"))??;
    let url = embedded_video_url(&media_id, &fallback_url, raw_json.as_deref());
    let url = validated_video_url(&url)?;
    let response = state
        .client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Could not load video: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Could not load video: {error}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_VIDEO_BYTES)
    {
        return Err("Video is larger than the 128 MB in-app playback limit".into());
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !content_type.starts_with("video/") {
        return Err(format!(
            "Video server returned unsupported content type: {content_type}"
        ));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Could not read video: {error}"))?;
    if bytes.len() as u64 > MAX_VIDEO_BYTES {
        return Err("Video is larger than the 128 MB in-app playback limit".into());
    }
    Ok(tauri::ipc::Response::new(bytes.to_vec()))
}

#[tauri::command]
pub async fn play_video_mpv(
    media_id: String,
    state: State<'_, Arc<Backend>>,
) -> Result<(), String> {
    let db = state.db.clone();
    let (url, _) = tokio::task::spawn_blocking(move || db.video_source(&media_id))
        .await
        .map_err(|error| format!("Video URL query task failed: {error}"))??;
    let url = validated_video_url(&url)?;
    let mut child = tokio::process::Command::new("mpv")
        .args([
            "--force-window=yes",
            "--no-terminal",
            "--title=X Knowledge Base",
            "--",
        ])
        .arg(url.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start mpv: {error}"))?;
    tauri::async_runtime::spawn(async move {
        let _ = child.wait().await;
    });
    Ok(())
}

#[tauri::command]
pub async fn open_original(
    id: String,
    app: AppHandle,
    state: State<'_, Arc<Backend>>,
) -> Result<(), String> {
    let db = state.db.clone();
    let url = tokio::task::spawn_blocking(move || db.canonical_url(&id))
        .await
        .map_err(|error| format!("URL query task failed: {error}"))??;
    let parsed =
        url::Url::parse(&url).map_err(|error| format!("Stored post URL is invalid: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Stored post URL has an unsupported scheme".into());
    }
    app.opener()
        .open_url(parsed.as_str(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{embedded_video_url, validated_video_url};

    #[test]
    fn video_loader_only_accepts_https_x_media_urls() {
        assert!(validated_video_url("https://video.twimg.com/ext_tw_video/example.mp4").is_ok());
        assert!(validated_video_url("http://video.twimg.com/example.mp4").is_err());
        assert!(validated_video_url("https://example.com/video.mp4").is_err());
    }

    #[test]
    fn embedded_video_uses_the_best_webkit_safe_mp4_variant() {
        let raw = r#"{"tweet":{"media":{"all":[{"id":"video-1","formats":[
            {"container":"mp4","bitrate":832000,"url":"https://video.twimg.com/360.mp4"},
            {"container":"mp4","bitrate":2176000,"url":"https://video.twimg.com/720.mp4"},
            {"container":"mp4","bitrate":10368000,"url":"https://video.twimg.com/1080.mp4"}
        ]}]}}}"#;
        assert_eq!(
            embedded_video_url("video-1", "https://video.twimg.com/fallback.mp4", Some(raw)),
            "https://video.twimg.com/720.mp4"
        );
    }
}
