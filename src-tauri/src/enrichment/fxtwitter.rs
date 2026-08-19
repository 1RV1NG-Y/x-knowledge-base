use std::time::Duration;

use async_trait::async_trait;
use chrono::DateTime;
use reqwest::{header::RETRY_AFTER, StatusCode};
use serde_json::Value;

use super::NormalizedTweet;
use crate::db::models::{Author, Media, Quote};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    Deleted,
    Protected,
    Suspended,
    Unavailable,
    RateLimited,
    Failed,
}

impl FailureKind {
    pub fn status(self) -> &'static str {
        match self {
            Self::Deleted => "deleted",
            Self::Protected => "protected",
            Self::Suspended => "suspended",
            Self::Unavailable => "unavailable",
            Self::RateLimited => "rate_limited",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug)]
pub struct EnrichError {
    pub kind: FailureKind,
    pub message: String,
    pub transient: bool,
    pub retry_after: Option<Duration>,
}

#[async_trait]
pub trait TweetEnricher: Send + Sync {
    async fn enrich(&self, tweet_id: &str) -> Result<NormalizedTweet, EnrichError>;
}

#[derive(Clone)]
pub struct FxTwitter {
    client: reqwest::Client,
}

impl FxTwitter {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl TweetEnricher for FxTwitter {
    async fn enrich(&self, tweet_id: &str) -> Result<NormalizedTweet, EnrichError> {
        let url = format!("https://api.fxtwitter.com/status/{tweet_id}");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|error| EnrichError {
                kind: FailureKind::Failed,
                message: format!("FxTwitter request failed: {error}"),
                transient: error.is_timeout() || error.is_connect() || error.is_request(),
                retry_after: None,
            })?;
        let status = response.status();
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| Duration::from_secs(seconds.min(300)));
        let body = response.text().await.map_err(|error| EnrichError {
            kind: FailureKind::Failed,
            message: format!("Could not read FxTwitter response: {error}"),
            transient: true,
            retry_after: None,
        })?;
        if !status.is_success() {
            return Err(classify_http(status, &body, retry_after));
        }
        let value: Value = serde_json::from_str(&body).map_err(|error| EnrichError {
            kind: FailureKind::Failed,
            message: format!("FxTwitter returned invalid JSON: {error}"),
            transient: false,
            retry_after: None,
        })?;
        if let Some(code) = provider_code(&value).filter(|code| *code >= 400) {
            let message = provider_message(&value);
            return Err(classify_code(code, &message, retry_after));
        }
        let has_tweet = value.get("tweet").is_some()
            || value.get("data").is_some()
            || value.get("id").is_some()
            || value.get("id_str").is_some()
            || value.get("text").is_some();
        if !has_tweet {
            let message = provider_message(&value);
            if !message.eq_ignore_ascii_case("ok") {
                let code = if message.to_ascii_lowercase().contains("rate limit") {
                    429
                } else {
                    400
                };
                return Err(classify_code(code, &message, retry_after));
            }
        }
        normalize(tweet_id, value, body)
    }
}

fn classify_http(status: StatusCode, body: &str, retry_after: Option<Duration>) -> EnrichError {
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .as_ref()
        .map(provider_message)
        .filter(|message| !message.is_empty())
        .unwrap_or_else(|| format!("FxTwitter returned HTTP {status}"));
    classify_code(status.as_u16() as i64, &message, retry_after)
}

fn classify_code(code: i64, message: &str, retry_after: Option<Duration>) -> EnrichError {
    let lower = message.to_ascii_lowercase();
    let kind = if code == 429 || lower.contains("rate limit") || lower.contains("too many request")
    {
        FailureKind::RateLimited
    } else if lower.contains("protected") || lower.contains("private") || code == 401 || code == 403
    {
        FailureKind::Protected
    } else if lower.contains("suspend") {
        FailureKind::Suspended
    } else if lower.contains("delet") || lower.contains("not found") || code == 404 || code == 410 {
        FailureKind::Deleted
    } else if code >= 500 {
        FailureKind::Failed
    } else {
        FailureKind::Unavailable
    };
    EnrichError {
        kind,
        message: message.to_string(),
        transient: matches!(kind, FailureKind::RateLimited | FailureKind::Failed)
            && (code == 429 || code >= 500),
        retry_after,
    }
}

fn provider_code(value: &Value) -> Option<i64> {
    value.get("code").and_then(|code| {
        code.as_i64()
            .or_else(|| code.as_str().and_then(|code| code.parse().ok()))
    })
}

fn provider_message(value: &Value) -> String {
    ["message", "error", "detail"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
        .unwrap_or("FxTwitter could not provide this post")
        .to_string()
}

pub fn normalize(
    expected_id: &str,
    root: Value,
    raw_json: String,
) -> Result<NormalizedTweet, EnrichError> {
    let tweet = at(&root, &["tweet"])
        .or_else(|| at(&root, &["data", "tweet"]))
        .or_else(|| {
            at(&root, &["data"]).filter(|value| {
                value.get("id").is_some()
                    || value.get("id_str").is_some()
                    || value.get("text").is_some()
            })
        })
        .or_else(|| {
            (root.get("id").is_some() || root.get("id_str").is_some() || root.get("text").is_some())
                .then_some(&root)
        })
        .filter(|value| value.is_object())
        .ok_or_else(|| schema_error("FxTwitter response did not contain a tweet object"))?;
    let response_id = string_at_any(tweet, &[&["id"], &["id_str"], &["tweet_id"]]);
    if response_id.as_deref().is_some_and(|id| id != expected_id) {
        return Err(schema_error("FxTwitter returned a different tweet ID"));
    }
    let id = expected_id.to_string();
    let text =
        string_at_any(tweet, &[&["text"], &["full_text"], &["fullText"]]).unwrap_or_default();
    let created_at = string_at_any(tweet, &[&["created_at"], &["createdAt"]]).map(normalize_date);
    let canonical_url = string_at_any(tweet, &[&["url"], &["tweet_url"], &["canonical_url"]])
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
        .unwrap_or_else(|| format!("https://x.com/i/web/status/{id}"));
    let author = normalize_author(tweet);
    let media = normalize_media(tweet, &id);
    let quote = normalize_quote(tweet);
    Ok(NormalizedTweet {
        id,
        text,
        created_at,
        canonical_url,
        author,
        media,
        quote,
        raw_json,
    })
}

fn normalize_author(tweet: &Value) -> Option<Author> {
    let value = at(tweet, &["author"]).or_else(|| at(tweet, &["user"]))?;
    let username = string_at_any(value, &[&["screen_name"], &["username"], &["user_name"]])
        .unwrap_or_default();
    let display_name =
        string_at_any(value, &[&["name"], &["display_name"], &["displayName"]]).unwrap_or_default();
    let id = string_at_any(value, &[&["id"], &["id_str"]]).or_else(|| {
        (!username.is_empty()).then(|| format!("username:{}", username.to_ascii_lowercase()))
    })?;
    let avatar_url = string_at_any(
        value,
        &[&["avatar_url"], &["profile_image_url_https"], &["avatar"]],
    );
    Some(Author {
        id,
        username,
        display_name,
        avatar_url,
        avatar_local_path: None,
    })
}

fn normalize_media(tweet: &Value, tweet_id: &str) -> Vec<Media> {
    let media_root = at(tweet, &["media"]);
    let mut candidates: Vec<&Value> = media_root
        .and_then(|media| at(media, &["all"]))
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default();
    if candidates.is_empty() {
        for key in ["photos", "videos", "media"] {
            if let Some(items) = media_root
                .and_then(|media| at(media, &[key]))
                .and_then(Value::as_array)
            {
                candidates.extend(items);
            }
        }
        if candidates.is_empty() {
            if let Some(items) =
                at(tweet, &["extended_entities", "media"]).and_then(Value::as_array)
            {
                candidates.extend(items);
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut output = Vec::new();
    for (index, value) in candidates.into_iter().enumerate() {
        let raw_kind = string_at_any(value, &[&["type"], &["kind"]])
            .unwrap_or_else(|| "unknown".into())
            .to_ascii_lowercase();
        let kind = match raw_kind.as_str() {
            "photo" | "image" => "image",
            "video" => "video",
            "animated_gif" | "gif" => "gif",
            _ => "unknown",
        }
        .to_string();
        let remote_url = string_at_any(
            value,
            &[&["url"], &["media_url_https"], &["media_url"], &["src"]],
        )
        .or_else(|| best_video_variant(value));
        let preview_url = string_at_any(
            value,
            &[&["thumbnail_url"], &["preview_url"], &["poster_url"]],
        );
        let identity = remote_url
            .clone()
            .or_else(|| preview_url.clone())
            .unwrap_or_else(|| format!("{tweet_id}:{index}"));
        if !seen.insert(identity) {
            continue;
        }
        let media_id = string_at_any(value, &[&["id"], &["id_str"]])
            .unwrap_or_else(|| format!("{tweet_id}:{index}"));
        output.push(Media {
            id: media_id,
            kind,
            remote_url,
            preview_url,
            local_path: None,
            thumbnail_path: None,
            width: integer_at_any(value, &[&["width"], &["original_info", "width"]]),
            height: integer_at_any(value, &[&["height"], &["original_info", "height"]]),
            duration_ms: duration_ms(value),
        });
    }
    output
}

fn best_video_variant(value: &Value) -> Option<String> {
    let variants = at(value, &["variants"])
        .or_else(|| at(value, &["video_info", "variants"]))?
        .as_array()?;
    variants
        .iter()
        .filter(|variant| {
            string_at_any(variant, &[&["content_type"], &["contentType"]])
                .map_or(true, |kind| kind.contains("mp4"))
        })
        .filter_map(|variant| {
            string_at_any(variant, &[&["url"]])
                .map(|url| (integer_at_any(variant, &[&["bitrate"]]).unwrap_or(0), url))
        })
        .max_by_key(|(bitrate, _)| *bitrate)
        .map(|(_, url)| url)
}

fn duration_ms(value: &Value) -> Option<i64> {
    if let Some(value) = integer_at_any(value, &[&["duration_ms"], &["duration_millis"]]) {
        return Some(value);
    }
    let duration = at(value, &["duration"]).and_then(Value::as_f64)?;
    Some(if duration > 10_000.0 {
        duration.round() as i64
    } else {
        (duration * 1000.0).round() as i64
    })
}

fn normalize_quote(tweet: &Value) -> Option<Quote> {
    let value = at(tweet, &["quote"]).or_else(|| at(tweet, &["quoted_tweet"]))?;
    let id = string_at_any(value, &[&["id"], &["id_str"], &["tweet_id"]]).unwrap_or_default();
    let author = at(value, &["author"]).or_else(|| at(value, &["user"]));
    Some(Quote {
        id,
        text: string_at_any(value, &[&["text"], &["full_text"]]).unwrap_or_default(),
        canonical_url: string_at_any(value, &[&["url"], &["tweet_url"]]),
        author_name: author.and_then(|value| string_at_any(value, &[&["name"], &["display_name"]])),
        username: author.and_then(|value| string_at_any(value, &[&["screen_name"], &["username"]])),
    })
}

fn normalize_date(value: String) -> String {
    DateTime::parse_from_rfc3339(&value)
        .or_else(|_| DateTime::parse_from_rfc2822(&value))
        .or_else(|_| DateTime::parse_from_str(&value, "%a %b %d %H:%M:%S %z %Y"))
        .map(|date| date.to_rfc3339())
        .unwrap_or(value)
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

fn string_at_any(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| at(value, path))
        .and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
}

fn integer_at_any(value: &Value, paths: &[&[&str]]) -> Option<i64> {
    paths
        .iter()
        .find_map(|path| at(value, path))
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        })
}

fn schema_error(message: &str) -> EnrichError {
    EnrichError {
        kind: FailureKind::Failed,
        message: message.into(),
        transient: false,
        retry_after: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FX_FIXTURE: &str = r#"{
      "code": 200,
      "message": "OK",
      "tweet": {
        "id": "1840123456789012345",
        "url": "https://x.com/example/status/1840123456789012345",
        "text": "A representative enriched post",
        "created_at": "Sat Sep 28 12:34:56 +0000 2024",
        "author": {"id":"42","name":"Example Person","screen_name":"example","avatar_url":"https://pbs.twimg.com/profile_images/avatar.jpg"},
        "media": {"all":[
          {"id":"photo-1","type":"photo","url":"https://pbs.twimg.com/media/photo.jpg","width":1200,"height":800},
          {"id":"video-1","type":"video","thumbnail_url":"https://pbs.twimg.com/ext_tw_video_thumb/thumb.jpg","duration":12.5,
           "variants":[{"content_type":"video/mp4","bitrate":256000,"url":"https://video.twimg.com/low.mp4"},{"content_type":"video/mp4","bitrate":2176000,"url":"https://video.twimg.com/high.mp4"}]}
        ]},
        "quote": {"id":"99","text":"Quoted context","url":"https://x.com/other/status/99","author":{"name":"Other","screen_name":"other"}}
      }
    }"#;

    #[test]
    fn normalizes_representative_fxtwitter_response() {
        let value: Value = serde_json::from_str(FX_FIXTURE).unwrap();
        let tweet = normalize("1840123456789012345", value, FX_FIXTURE.into()).unwrap();
        assert_eq!(tweet.author.as_ref().unwrap().username, "example");
        assert_eq!(tweet.media.len(), 2);
        assert_eq!(
            tweet.media[1].remote_url.as_deref(),
            Some("https://video.twimg.com/high.mp4")
        );
        assert_eq!(tweet.media[1].duration_ms, Some(12_500));
        assert_eq!(
            tweet.quote.as_ref().unwrap().username.as_deref(),
            Some("other")
        );
        assert!(tweet
            .created_at
            .as_deref()
            .unwrap()
            .starts_with("2024-09-28T12:34:56"));
        assert_eq!(tweet.raw_json, FX_FIXTURE);
    }
}
