use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::Value;
use walkdir::WalkDir;

const MAX_LIKES_BYTES: u64 = 256 * 1024 * 1024;
const MAX_IDENTITY_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseLike {
    pub id: String,
    pub text: Option<String>,
    pub canonical_url: String,
    pub liked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveProfile {
    pub id: String,
    pub username: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub from_account: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedArchive {
    pub profile: ArchiveProfile,
    pub likes: Vec<SparseLike>,
    pub failed: usize,
}

pub fn parse_archive(path: &Path) -> Result<ParsedArchive, String> {
    let (likes, account, profile) = if path.is_dir() {
        read_archive_folder(path)?
    } else {
        read_archive_zip(path)?
    };
    let identity = match account {
        Some(account) => parse_account_js(&account, profile.as_deref())?,
        None => fallback_profile(path),
    };
    parse_like_js(&likes, identity)
}

fn read_archive_folder(root: &Path) -> Result<(String, Option<String>, Option<String>), String> {
    let likes_path = locate_likes_file(root)?;
    let likes = read_limited_file(&likes_path, MAX_LIKES_BYTES, "Likes")?;
    let data_dir = likes_path
        .parent()
        .ok_or_else(|| "Likes data has no parent folder".to_string())?;
    let account = locate_sibling(data_dir, "account.js")
        .map(|path| read_limited_file(&path, MAX_IDENTITY_BYTES, "Account"))
        .transpose()?;
    let profile = locate_sibling(data_dir, "profile.js")
        .map(|path| read_limited_file(&path, MAX_IDENTITY_BYTES, "Profile"))
        .transpose()?;
    Ok((likes, account, profile))
}

fn read_limited_file(path: &Path, limit: u64, label: &str) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if metadata.len() > limit {
        return Err(format!("Archive {label} file is too large"));
    }
    std::fs::read_to_string(path).map_err(|e| format!("Could not read {label} data: {e}"))
}

fn locate_sibling(root: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(name)
        })
        .map(|entry| entry.path())
}

fn locate_likes_file(root: &Path) -> Result<PathBuf, String> {
    let mut candidates = WalkDir::new(root)
        .follow_links(false)
        .max_depth(8)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("like.js")
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    candidates.sort_by_key(|path| {
        let in_data = path
            .parent()
            .and_then(Path::file_name)
            .map(|name| !name.to_string_lossy().eq_ignore_ascii_case("data"))
            .unwrap_or(true);
        (in_data, path.components().count())
    });
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| "Could not find data/like.js in the selected archive folder".into())
}

fn read_archive_zip(path: &Path) -> Result<(String, Option<String>, Option<String>), String> {
    let file = File::open(path).map_err(|e| format!("Could not open archive ZIP: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Invalid archive ZIP: {e}"))?;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|e| e.to_string())?;
        if entry.is_file() {
            files.push((entry.name().replace('\\', "/"), index));
        }
    }
    let mut candidates = files
        .iter()
        .filter_map(|(name, index)| {
            let lower = name.to_ascii_lowercase();
            (lower == "data/like.js"
                || lower.ends_with("/data/like.js")
                || lower == "like.js"
                || lower.ends_with("/like.js"))
            .then(|| {
                let score =
                    usize::from(!(lower == "data/like.js" || lower.ends_with("/data/like.js")));
                (score, name.matches('/').count(), *index, lower)
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    let (_, _, likes_index, likes_name) = candidates
        .into_iter()
        .next()
        .ok_or_else(|| "Could not find data/like.js in the selected archive ZIP".to_string())?;
    let prefix = likes_name
        .strip_suffix("like.js")
        .expect("selected ZIP entry ends in like.js");
    let sibling = |name: &str| {
        let expected = format!("{prefix}{name}");
        files
            .iter()
            .find(|(entry_name, _)| entry_name.to_ascii_lowercase() == expected)
            .map(|(_, index)| *index)
    };
    let account_index = sibling("account.js");
    let profile_index = sibling("profile.js");
    let likes = read_zip_entry(&mut archive, likes_index, MAX_LIKES_BYTES, "Likes")?;
    let account = account_index
        .map(|index| read_zip_entry(&mut archive, index, MAX_IDENTITY_BYTES, "Account"))
        .transpose()?;
    let profile = profile_index
        .map(|index| read_zip_entry(&mut archive, index, MAX_IDENTITY_BYTES, "Profile"))
        .transpose()?;
    Ok((likes, account, profile))
}

fn read_zip_entry(
    archive: &mut zip::ZipArchive<File>,
    index: usize,
    limit: u64,
    label: &str,
) -> Result<String, String> {
    let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
    if entry.size() > limit {
        return Err(format!("Archive {label} file is too large"));
    }
    let mut content = String::with_capacity(entry.size() as usize);
    entry
        .read_to_string(&mut content)
        .map_err(|e| format!("Could not read {label} data from ZIP: {e}"))?;
    Ok(content)
}

fn parse_account_js(input: &str, profile_input: Option<&str>) -> Result<ArchiveProfile, String> {
    let records: Vec<Value> = serde_json::from_str(extract_json_array(input)?)
        .map_err(|e| format!("Account data is not valid JSON: {e}"))?;
    let account = records
        .first()
        .and_then(|value| value.get("account").or(Some(value)))
        .ok_or_else(|| "Account data is empty".to_string())?;
    let id = ["accountId", "account_id"]
        .iter()
        .find_map(|key| account.get(*key))
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_u64().map(|value| value.to_string()))
        })
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value.bytes().all(|byte| byte.is_ascii_digit())
        })
        .ok_or_else(|| "Account data has no accountId".to_string())?;
    let username = first_string(account, &["username"])
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let display_name = first_string(account, &["accountDisplayName", "displayName"])
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .or_else(|| username.clone())
        .unwrap_or_else(|| "X account".to_string());
    let avatar_url = profile_input
        .map(parse_profile_avatar)
        .transpose()?
        .flatten();
    Ok(ArchiveProfile {
        id,
        username,
        display_name,
        avatar_url,
        from_account: true,
    })
}

fn parse_profile_avatar(input: &str) -> Result<Option<String>, String> {
    let records: Vec<Value> = serde_json::from_str(extract_json_array(input)?)
        .map_err(|e| format!("Profile data is not valid JSON: {e}"))?;
    Ok(records
        .first()
        .and_then(|value| value.get("profile").or(Some(value)))
        .and_then(|profile| first_string(profile, &["avatarMediaUrl", "avatar_media_url"]))
        .filter(|value| !value.is_empty())
        .map(str::to_owned))
}

fn fallback_profile(source: &Path) -> ArchiveProfile {
    let source = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let key = source.to_string_lossy().replace('\\', "/");
    let hash = key.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    let label = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("archive");
    ArchiveProfile {
        id: format!("source-{hash:016x}"),
        username: None,
        display_name: format!("Archive ({label})"),
        avatar_url: None,
        from_account: false,
    }
}

fn parse_like_js(input: &str, profile: ArchiveProfile) -> Result<ParsedArchive, String> {
    let json = extract_json_array(input)?;
    let records: Vec<Value> =
        serde_json::from_str(json).map_err(|e| format!("Likes data is not valid JSON: {e}"))?;
    let mut likes: BTreeMap<String, SparseLike> = BTreeMap::new();
    let mut failed = 0;
    for wrapper in records {
        let value = wrapper.get("like").unwrap_or(&wrapper);
        let Some(id) = recover_id(value) else {
            failed += 1;
            continue;
        };
        let text = first_string(value, &["fullText", "full_text", "tweetText", "text"])
            .map(str::to_owned)
            .filter(|value| !value.is_empty());
        let url = first_string(
            value,
            &[
                "expandedUrl",
                "expanded_url",
                "tweetUrl",
                "tweet_url",
                "url",
            ],
        )
        .filter(|url| recover_id_from_url(url).as_deref() == Some(id.as_str()))
        .map(str::to_owned)
        .or_else(|| find_status_url(value, &id))
        .unwrap_or_else(|| format!("https://x.com/i/web/status/{id}"));
        let liked_at = first_string(value, &["likedAt", "liked_at", "createdAt", "created_at"])
            .map(str::to_owned);
        let candidate = SparseLike {
            id: id.clone(),
            text,
            canonical_url: url,
            liked_at,
        };
        likes
            .entry(id)
            .and_modify(|current| merge_sparse(current, &candidate))
            .or_insert(candidate);
    }
    Ok(ParsedArchive {
        profile,
        likes: likes.into_values().collect(),
        failed,
    })
}

fn extract_json_array(input: &str) -> Result<&str, String> {
    let input = input.trim_start_matches('\u{feff}').trim();
    let start = if input.starts_with('[') {
        0
    } else {
        let equals = input
            .find('=')
            .ok_or_else(|| "Likes data has no JavaScript assignment".to_string())?;
        let lhs = input[..equals].trim();
        if !lhs.starts_with("window.YTD.")
            || !lhs
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '$'))
        {
            return Err("Likes data has an unexpected JavaScript assignment".into());
        }
        equals
            + 1
            + input[equals + 1..]
                .find('[')
                .ok_or_else(|| "Likes assignment has no JSON array".to_string())?
    };
    let bytes = input.as_bytes();
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    for index in start..bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'[' => depth += 1,
            b']' => {
                if depth == 0 {
                    return Err("Likes JSON has an unmatched closing bracket".into());
                }
                depth -= 1;
                if depth == 0 {
                    return Ok(&input[start..=index]);
                }
            }
            _ => {}
        }
    }
    Err("Likes JSON array is incomplete".into())
}

pub fn recover_id(value: &Value) -> Option<String> {
    for key in ["tweetId", "tweet_id", "tweetID", "id_str", "id"] {
        if let Some(candidate) = value.get(key).and_then(value_as_id) {
            return Some(candidate);
        }
    }
    find_id_in_urls(value)
}

fn value_as_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if valid_id(value) => Some(value.clone()),
        Value::Number(value) => value
            .as_u64()
            .map(|value| value.to_string())
            .filter(|value| valid_id(value)),
        _ => None,
    }
}

fn valid_id(value: &str) -> bool {
    value.len() >= 5 && value.len() <= 30 && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn recover_id_from_url(raw: &str) -> Option<String> {
    let parsed = url::Url::parse(raw).ok()?;
    if !matches!(
        parsed.host_str(),
        Some("twitter.com" | "www.twitter.com" | "x.com" | "www.x.com" | "mobile.twitter.com")
    ) {
        return None;
    }
    let parts = parsed.path_segments()?.collect::<Vec<_>>();
    for window in parts.windows(2) {
        if matches!(window[0], "status" | "statuses") {
            let id = window[1]
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .unwrap_or("");
            if valid_id(id) {
                return Some(id.to_string());
            }
        }
    }
    None
}

fn find_id_in_urls(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => recover_id_from_url(value),
        Value::Array(values) => values.iter().find_map(find_id_in_urls),
        Value::Object(values) => values.values().find_map(find_id_in_urls),
        _ => None,
    }
}

fn find_status_url(value: &Value, id: &str) -> Option<String> {
    match value {
        Value::String(value) if recover_id_from_url(value).as_deref() == Some(id) => {
            Some(value.clone())
        }
        Value::Array(values) => values.iter().find_map(|value| find_status_url(value, id)),
        Value::Object(values) => values.values().find_map(|value| find_status_url(value, id)),
        _ => None,
    }
}

fn first_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
}

fn merge_sparse(current: &mut SparseLike, candidate: &SparseLike) {
    if candidate.text.as_ref().map_or(0, String::len) > current.text.as_ref().map_or(0, String::len)
    {
        current.text.clone_from(&candidate.text);
    }
    if current.canonical_url.contains("/i/web/status/")
        && !candidate.canonical_url.contains("/i/web/status/")
    {
        current.canonical_url.clone_from(&candidate.canonical_url);
    }
    if current.liked_at.is_none() {
        current.liked_at.clone_from(&candidate.liked_at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile() -> ArchiveProfile {
        ArchiveProfile {
            id: "test-account".into(),
            username: Some("tester".into()),
            display_name: "Tester".into(),
            avatar_url: None,
            from_account: true,
        }
    }

    #[test]
    fn parses_assignment_and_deduplicates_using_best_text() {
        let input = r#"window.YTD.like.part0 = [{"like":{"tweetId":"12345","fullText":"short","expandedUrl":"https://twitter.com/i/web/status/12345"}},{"like":{"tweetId":"12345","fullText":"a longer [archived] text","expandedUrl":"https://x.com/alice/status/12345"}}];"#;
        let parsed = parse_like_js(input, test_profile()).unwrap();
        assert_eq!(parsed.failed, 0);
        assert_eq!(parsed.likes.len(), 1);
        assert_eq!(
            parsed.likes[0].text.as_deref(),
            Some("a longer [archived] text")
        );
        assert_eq!(
            parsed.likes[0].canonical_url,
            "https://x.com/alice/status/12345"
        );
    }

    #[test]
    fn recovers_ids_from_explicit_fields_and_status_urls() {
        assert_eq!(
            recover_id(&serde_json::json!({"tweetId":"987654321"})).as_deref(),
            Some("987654321")
        );
        assert_eq!(
            recover_id(
                &serde_json::json!({"expandedUrl":"https://twitter.com/user/status/123456789?s=20"})
            )
            .as_deref(),
            Some("123456789")
        );
        assert_eq!(
            recover_id(&serde_json::json!({"url":"https://example.com/status/123456789"})),
            None
        );
    }

    #[test]
    fn rejects_untrusted_javascript_instead_of_slicing_it() {
        assert!(parse_like_js("doSomething() = [];", test_profile()).is_err());
    }

    #[test]
    fn parses_folder_account_and_profile_identity_without_email() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(
            data.join("like.js"),
            r#"window.YTD.like.part0 = [{"like":{"tweetId":"12345"}}];"#,
        )
        .unwrap();
        std::fs::write(
            data.join("account.js"),
            r#"window.YTD.account.part0 = [{"account":{"accountId":"42","username":"AbsoluteMememan","accountDisplayName":"Mr. Mememan","email":"private@example.com"}}];"#,
        )
        .unwrap();
        std::fs::write(
            data.join("profile.js"),
            r#"window.YTD.profile.part0 = [{"profile":{"avatarMediaUrl":"https://pbs.twimg.com/profile_images/avatar.jpg"}}];"#,
        )
        .unwrap();

        let parsed = parse_archive(temp.path()).unwrap();
        assert_eq!(parsed.profile.id, "42");
        assert_eq!(parsed.profile.username.as_deref(), Some("AbsoluteMememan"));
        assert_eq!(parsed.profile.display_name, "Mr. Mememan");
        assert_eq!(
            parsed.profile.avatar_url.as_deref(),
            Some("https://pbs.twimg.com/profile_images/avatar.jpg")
        );
        assert!(parsed.profile.from_account);
    }

    #[test]
    fn parses_zip_identity_next_to_selected_likes_file() {
        use std::io::Write;

        let temp = tempfile::tempdir().unwrap();
        let zip_path = temp.path().join("archive.zip");
        let file = File::create(&zip_path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("bundle/data/like.js", options).unwrap();
        writer
            .write_all(br#"window.YTD.like.part0 = [{"like":{"tweetId":"12345"}}];"#)
            .unwrap();
        writer
            .start_file("bundle/data/account.js", options)
            .unwrap();
        writer
            .write_all(br#"window.YTD.account.part0 = [{"account":{"accountId":"84","username":"zipuser","accountDisplayName":"ZIP User"}}];"#)
            .unwrap();
        writer.finish().unwrap();

        let parsed = parse_archive(&zip_path).unwrap();
        assert_eq!(parsed.profile.id, "84");
        assert_eq!(parsed.profile.username.as_deref(), Some("zipuser"));
        assert_eq!(parsed.profile.display_name, "ZIP User");
    }

    #[test]
    fn missing_account_uses_deterministic_source_identity() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        std::fs::create_dir(&data).unwrap();
        std::fs::write(
            data.join("like.js"),
            r#"window.YTD.like.part0 = [{"like":{"tweetId":"12345"}}];"#,
        )
        .unwrap();

        let first = parse_archive(temp.path()).unwrap();
        let second = parse_archive(temp.path()).unwrap();
        assert_eq!(first.profile, second.profile);
        assert!(first.profile.id.starts_with("source-"));
        assert!(!first.profile.from_account);
    }
}
