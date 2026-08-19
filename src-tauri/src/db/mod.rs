pub mod models;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::{
    archive::{self, ArchiveProfile, ParsedArchive},
    enrichment::NormalizedTweet,
};
use models::{
    Author, ImportSummary, Media, Profile, ProfileState, Quote, Tweet, TweetPage, TweetQuery,
};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/0001_initial.sql")),
    (2, include_str!("../../migrations/0002_fts_rowid.sql")),
    (
        3,
        include_str!("../../migrations/0003_retry_locked_enrichments.sql"),
    ),
    (4, include_str!("../../migrations/0004_profiles.sql")),
];

#[derive(Debug, Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn initialize(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let db = Self { path };
        let mut connection = db.connect()?;
        connection
            .execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
            .map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);",
            )
            .map_err(|e| e.to_string())?;
        for (version, sql) in MIGRATIONS {
            let applied: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                    [version],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if !applied {
                let tx = connection.transaction().map_err(|e| e.to_string())?;
                tx.execute_batch(sql).map_err(|e| e.to_string())?;
                tx.execute(
                    "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, ?2)",
                    params![version, Utc::now().to_rfc3339()],
                )
                .map_err(|e| e.to_string())?;
                tx.commit().map_err(|e| e.to_string())?;
            }
        }
        drop(connection);
        db.reconcile_legacy_profiles()?;
        Ok(db)
    }

    fn connect(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path).map_err(|e| e.to_string())?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| e.to_string())?;
        Ok(connection)
    }

    pub fn import_archive(
        &self,
        parsed: &ParsedArchive,
        source: &Path,
    ) -> Result<(ImportSummary, Vec<String>), String> {
        let mut connection = self.connect()?;
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        let source = source.to_string_lossy();
        let profile_id = parsed.profile.id.as_str();
        upsert_profile(&tx, &parsed.profile)?;
        adopt_legacy_source(&tx, &parsed.profile, source.as_ref())?;
        let mut imported = 0_u64;
        let mut existing = 0_u64;
        let mut queue = Vec::new();

        for like in &parsed.likes {
            let existed: bool = tx
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM likes WHERE profile_id = ?1 AND tweet_id = ?2
                    )",
                    params![profile_id, like.id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if existed {
                existing += 1;
            } else {
                imported += 1;
            }

            tx.execute(
                "INSERT INTO tweets(id, text, canonical_url, imported_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                    text = CASE
                        WHEN tweets.enrichment_status = 'ok' AND tweets.text <> '' THEN tweets.text
                        WHEN length(excluded.text) > length(tweets.text) THEN excluded.text
                        ELSE tweets.text END,
                    canonical_url = CASE
                        WHEN tweets.canonical_url = '' THEN excluded.canonical_url
                        WHEN tweets.canonical_url LIKE '%/i/web/status/%'
                             AND excluded.canonical_url NOT LIKE '%/i/web/status/%' THEN excluded.canonical_url
                        ELSE tweets.canonical_url END",
                params![like.id, like.text.as_deref().unwrap_or(""), like.canonical_url, now],
            ).map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO likes(profile_id, tweet_id, liked_at, archive_text, archive_source)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(profile_id, tweet_id) DO UPDATE SET
                    liked_at = COALESCE(likes.liked_at, excluded.liked_at),
                    archive_text = CASE
                        WHEN length(COALESCE(excluded.archive_text, '')) > length(COALESCE(likes.archive_text, ''))
                        THEN excluded.archive_text ELSE likes.archive_text END,
                    archive_source = excluded.archive_source",
                params![profile_id, like.id, like.liked_at, like.text, source.as_ref()],
            ).map_err(|e| e.to_string())?;
            let status: String = tx
                .query_row(
                    "SELECT enrichment_status FROM tweets WHERE id = ?1",
                    [&like.id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if matches!(status.as_str(), "pending" | "failed" | "rate_limited") {
                tx.execute(
                    "UPDATE tweets SET enrichment_status = 'pending', enrichment_error = NULL WHERE id = ?1",
                    [&like.id],
                ).map_err(|e| e.to_string())?;
                queue.push(like.id.clone());
            }
        }
        tx.execute(
            "INSERT INTO app_settings(key, value) VALUES ('active_profile_id', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [profile_id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        let pending = queue.len() as u64;
        Ok((
            ImportSummary {
                imported,
                existing,
                pending,
                failed: parsed.failed as u64,
            },
            queue,
        ))
    }

    fn reconcile_legacy_profiles(&self) -> Result<(), String> {
        let mut connection = self.connect()?;
        let sources = {
            let mut statement = connection
                .prepare(
                    "SELECT DISTINCT archive_source
                     FROM likes
                     WHERE profile_id = '__legacy__' AND archive_source IS NOT NULL
                     ORDER BY archive_source",
                )
                .map_err(|e| e.to_string())?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| e.to_string())?;
            rows
        };
        for source in sources {
            let path = Path::new(&source);
            if !path.exists() {
                continue;
            }
            let Ok(parsed) = archive::parse_archive(path) else {
                continue;
            };
            if !parsed.profile.from_account {
                continue;
            }
            let tx = connection.transaction().map_err(|e| e.to_string())?;
            upsert_profile(&tx, &parsed.profile)?;
            adopt_legacy_source(&tx, &parsed.profile, &source)?;
            tx.execute(
                "INSERT INTO app_settings(key, value) VALUES ('active_profile_id', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [&parsed.profile.id],
            )
            .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_profiles(&self) -> Result<ProfileState, String> {
        let connection = self.connect()?;
        let active_profile_id = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'active_profile_id'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT p.id, p.username, p.display_name, p.avatar_url,
                        p.avatar_local_path, COUNT(l.tweet_id)
                 FROM profiles p
                 LEFT JOIN likes l ON l.profile_id = p.id
                 WHERE p.id <> '__legacy__'
                    OR EXISTS(SELECT 1 FROM likes legacy WHERE legacy.profile_id = p.id)
                 GROUP BY p.id
                 ORDER BY CASE WHEN p.id = ?1 THEN 0 ELSE 1 END,
                          LOWER(p.display_name), p.id",
            )
            .map_err(|e| e.to_string())?;
        let profiles = statement
            .query_map([active_profile_id.as_deref()], |row| {
                let count: i64 = row.get(5)?;
                Ok(Profile {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    avatar_url: row.get(3)?,
                    avatar_local_path: row.get(4)?,
                    post_count: count.max(0) as u64,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(ProfileState {
            profiles,
            active_profile_id,
        })
    }

    pub fn set_active_profile(&self, profile_id: &str) -> Result<(), String> {
        let connection = self.connect()?;
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM profiles p
                    WHERE p.id = ?1
                      AND (p.id <> '__legacy__'
                           OR EXISTS(
                               SELECT 1 FROM likes l WHERE l.profile_id = p.id
                           ))
                )",
                [profile_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !exists {
            return Err(format!("Profile {profile_id} was not found"));
        }
        connection
            .execute(
                "INSERT INTO app_settings(key, value) VALUES ('active_profile_id', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [profile_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn pending_ids(&self) -> Result<Vec<String>, String> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare("SELECT id FROM tweets WHERE enrichment_status = 'pending' ORDER BY imported_at, id")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn retry(&self, id: &str) -> Result<(), String> {
        let connection = self.connect()?;
        let changed = connection.execute(
            "UPDATE tweets SET enrichment_status = 'pending', enrichment_error = NULL, retry_count = 0 WHERE id = ?1",
            [id],
        ).map_err(|e| e.to_string())?;
        if changed == 0 {
            Err(format!("Tweet {id} was not found"))
        } else {
            Ok(())
        }
    }

    pub fn mark_failure(
        &self,
        id: &str,
        status: &str,
        error: &str,
        attempts: u32,
    ) -> Result<(), String> {
        let connection = self.connect()?;
        connection.execute(
            "UPDATE tweets SET enrichment_status = ?2, enrichment_error = ?3, retry_count = ?4 WHERE id = ?1",
            params![id, status, error, attempts],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_enriched(&self, enriched: &NormalizedTweet) -> Result<(), String> {
        let mut connection = self.connect()?;
        let tx = connection.transaction().map_err(|e| e.to_string())?;
        if let Some(author) = &enriched.author {
            tx.execute(
                "INSERT INTO users(id, username, display_name, avatar_url, avatar_local_path)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    username = COALESCE(NULLIF(excluded.username, ''), users.username),
                    display_name = COALESCE(NULLIF(excluded.display_name, ''), users.display_name),
                    avatar_url = COALESCE(excluded.avatar_url, users.avatar_url),
                    avatar_local_path = COALESCE(excluded.avatar_local_path, users.avatar_local_path)",
                params![author.id, author.username, author.display_name, author.avatar_url, author.avatar_local_path],
            ).map_err(|e| e.to_string())?;
        }
        tx.execute(
            "UPDATE tweets SET
                author_id = COALESCE(?2, author_id),
                text = CASE WHEN ?3 <> '' THEN ?3 ELSE text END,
                created_at = COALESCE(?4, created_at),
                canonical_url = CASE WHEN ?5 <> '' THEN ?5 ELSE canonical_url END,
                raw_json = ?6,
                enrichment_status = 'ok', enrichment_error = NULL,
                enriched_at = ?7, retry_count = 0
             WHERE id = ?1",
            params![
                enriched.id,
                enriched.author.as_ref().map(|a| a.id.as_str()),
                enriched.text,
                enriched.created_at,
                enriched.canonical_url,
                enriched.raw_json,
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;

        tx.execute("DELETE FROM media WHERE tweet_id = ?1", [&enriched.id])
            .map_err(|e| e.to_string())?;
        for item in &enriched.media {
            tx.execute(
                "INSERT INTO media(id, tweet_id, kind, remote_url, preview_url, local_path, thumbnail_path, width, height, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![item.id, enriched.id, item.kind, item.remote_url, item.preview_url, item.local_path, item.thumbnail_path, item.width, item.height, item.duration_ms],
            ).map_err(|e| e.to_string())?;
        }
        tx.execute("DELETE FROM quotes WHERE tweet_id = ?1", [&enriched.id])
            .map_err(|e| e.to_string())?;
        if let Some(quote) = &enriched.quote {
            tx.execute(
                "INSERT INTO quotes(tweet_id, quoted_id, text, canonical_url, author_name, username)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![enriched.id, quote.id, quote.text, quote.canonical_url, quote.author_name, quote.username],
            ).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn get_tweets(&self, request: TweetQuery) -> Result<TweetPage, String> {
        let connection = self.connect()?;
        let (where_sql, values, _) = build_filters(&request)?;
        let from = " FROM app_settings active
                     JOIN likes l ON l.profile_id = active.value
                     JOIN tweets t ON t.id = l.tweet_id
                     LEFT JOIN users u ON u.id = t.author_id ";
        let count_sql = format!("SELECT COUNT(DISTINCT t.id){from}{where_sql}");
        let total: i64 = connection
            .query_row(&count_sql, params_from_iter(values.iter()), |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;

        let limit = request.limit.clamp(1, 100) as i64;
        let offset = request.offset.min(1_000_000) as i64;
        let chronology = match request.sort_order.as_deref().unwrap_or("newest") {
            "newest" => "DESC",
            "oldest" => "ASC",
            value => return Err(format!("Unsupported sort order: {value}")),
        };
        let select_sql = format!(
            "SELECT DISTINCT t.id, t.text, l.archive_text, t.created_at, l.liked_at,
                    t.canonical_url, t.enrichment_status, t.enrichment_error,
                    u.id, u.username, u.display_name, u.avatar_url, u.avatar_local_path
             {from}{where_sql}
             ORDER BY CAST(t.id AS INTEGER) {chronology}
             LIMIT ? OFFSET ?"
        );
        let mut page_values = values;
        page_values.push(Value::Integer(limit));
        page_values.push(Value::Integer(offset));
        let mut statement = connection.prepare(&select_sql).map_err(|e| e.to_string())?;
        let bases = statement
            .query_map(params_from_iter(page_values.iter()), map_tweet_base)
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let mut items = Vec::with_capacity(bases.len());
        for base in bases {
            items.push(hydrate(&connection, base)?);
        }
        Ok(TweetPage {
            items,
            total: total as u64,
        })
    }

    pub fn get_tweet(&self, id: &str) -> Result<Tweet, String> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(
                "SELECT t.id, t.text, l.archive_text, t.created_at, l.liked_at,
                    t.canonical_url, t.enrichment_status, t.enrichment_error,
                    u.id, u.username, u.display_name, u.avatar_url, u.avatar_local_path
             FROM app_settings active
             JOIN likes l ON l.profile_id = active.value
             JOIN tweets t ON t.id = l.tweet_id
             LEFT JOIN users u ON u.id = t.author_id
             WHERE active.key = 'active_profile_id' AND t.id=?1",
            )
            .map_err(|e| e.to_string())?;
        let base = statement
            .query_row([id], map_tweet_base)
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Tweet {id} was not found"))?;
        hydrate(&connection, base)
    }

    pub fn canonical_url(&self, id: &str) -> Result<String, String> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT canonical_url FROM tweets WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Tweet {id} was not found"))
    }

    pub fn video_source(&self, media_id: &str) -> Result<(String, Option<String>), String> {
        let connection = self.connect()?;
        connection
            .query_row(
                "SELECT m.remote_url, t.raw_json
                 FROM media m
                 JOIN tweets t ON t.id = m.tweet_id
                 WHERE m.id = ?1 AND m.kind = 'video' AND m.remote_url IS NOT NULL",
                [media_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Video media {media_id} was not found"))
    }
}

fn upsert_profile(tx: &rusqlite::Transaction<'_>, profile: &ArchiveProfile) -> Result<(), String> {
    tx.execute(
        "INSERT INTO profiles(id, username, display_name, avatar_url, avatar_local_path)
         VALUES (?1, ?2, ?3, ?4, NULL)
         ON CONFLICT(id) DO UPDATE SET
            username = COALESCE(NULLIF(excluded.username, ''), profiles.username),
            display_name = CASE
                WHEN excluded.display_name <> '' THEN excluded.display_name
                ELSE profiles.display_name END,
            avatar_url = COALESCE(excluded.avatar_url, profiles.avatar_url)",
        params![
            profile.id,
            profile.username,
            profile.display_name,
            profile.avatar_url
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn adopt_legacy_source(
    tx: &rusqlite::Transaction<'_>,
    profile: &ArchiveProfile,
    source: &str,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO likes(profile_id, tweet_id, liked_at, archive_text, archive_source)
         SELECT ?1, legacy.tweet_id, legacy.liked_at, legacy.archive_text, legacy.archive_source
         FROM likes legacy
         WHERE legacy.profile_id = '__legacy__' AND legacy.archive_source = ?2
         ON CONFLICT(profile_id, tweet_id) DO UPDATE SET
            liked_at = COALESCE(likes.liked_at, excluded.liked_at),
            archive_text = CASE
                WHEN length(COALESCE(excluded.archive_text, '')) >
                     length(COALESCE(likes.archive_text, ''))
                THEN excluded.archive_text ELSE likes.archive_text END,
            archive_source = excluded.archive_source",
        params![profile.id, source],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM likes
         WHERE profile_id = '__legacy__' AND archive_source = ?1",
        [source],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn build_filters(request: &TweetQuery) -> Result<(String, Vec<Value>, bool), String> {
    let mut clauses = Vec::new();
    let mut values = Vec::new();
    let mut author = request.author.clone();
    let mut media_type = request.media_type.clone();
    let mut before = request.before.clone();
    let mut after = request.after.clone();
    let mut status = request.status.clone();
    let mut terms = Vec::new();
    if let Some(search) = request.search.as_deref() {
        for token in search.split_whitespace() {
            if let Some(value) = token.strip_prefix("from:") {
                author = Some(value.to_string());
            } else if token.eq_ignore_ascii_case("has:image") {
                media_type = Some("image".into());
            } else if token.eq_ignore_ascii_case("has:video") {
                media_type = Some("video".into());
            } else if let Some(value) = token.strip_prefix("before:") {
                before = Some(value.to_string());
            } else if let Some(value) = token.strip_prefix("after:") {
                after = Some(value.to_string());
            } else if let Some(value) = token.strip_prefix("status:") {
                status = Some(value.to_string());
            } else {
                terms.push(token);
            }
        }
    }
    let use_fts = !terms.is_empty();
    if use_fts {
        clauses.push(
            "(t.rowid IN (SELECT rowid FROM tweet_fts WHERE tweet_fts MATCH ?)
              OR l.rowid IN (SELECT rowid FROM like_fts WHERE like_fts MATCH ?))"
                .to_string(),
        );
        let query = terms
            .into_iter()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");
        values.push(Value::Text(query.clone()));
        values.push(Value::Text(query));
    }
    if let Some(author) = author.filter(|v| !v.trim().is_empty()) {
        clauses.push("LOWER(u.username) = LOWER(?)".into());
        values.push(Value::Text(author.trim_start_matches('@').to_string()));
    }
    if let Some(kind) = media_type.filter(|v| !v.is_empty()) {
        if !matches!(kind.as_str(), "image" | "video") {
            return Err("mediaType must be image or video".into());
        }
        clauses.push("EXISTS(SELECT 1 FROM media m WHERE m.tweet_id=t.id AND m.kind=?)".into());
        values.push(Value::Text(kind));
    }
    if let Some(value) = before.filter(|v| !v.is_empty()) {
        clauses.push("COALESCE(t.created_at, l.liked_at) < ?".into());
        values.push(Value::Text(value));
    }
    if let Some(value) = after.filter(|v| !v.is_empty()) {
        clauses.push("COALESCE(t.created_at, l.liked_at) >= ?".into());
        values.push(Value::Text(value));
    }
    if let Some(value) = status.filter(|v| !v.is_empty()) {
        clauses.push("t.enrichment_status = ?".into());
        values.push(Value::Text(value));
    }
    clauses.insert(0, "active.key = 'active_profile_id'".into());
    let sql = format!(" WHERE {}", clauses.join(" AND "));
    Ok((sql, values, use_fts))
}

fn map_tweet_base(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tweet> {
    let author_id: Option<String> = row.get(8)?;
    Ok(Tweet {
        id: row.get(0)?,
        text: row.get(1)?,
        archive_text: row.get(2)?,
        created_at: row.get(3)?,
        liked_at: row.get(4)?,
        canonical_url: row.get(5)?,
        enrichment_status: row.get(6)?,
        enrichment_error: row.get(7)?,
        author: author_id.map(|id| Author {
            id,
            username: row
                .get::<_, Option<String>>(9)
                .ok()
                .flatten()
                .unwrap_or_default(),
            display_name: row
                .get::<_, Option<String>>(10)
                .ok()
                .flatten()
                .unwrap_or_default(),
            avatar_url: row.get(11).ok().flatten(),
            avatar_local_path: row.get(12).ok().flatten(),
        }),
        media: Vec::new(),
        quote: None,
    })
}

fn hydrate(connection: &Connection, mut tweet: Tweet) -> Result<Tweet, String> {
    let mut media_statement = connection.prepare(
        "SELECT id, kind, remote_url, preview_url, local_path, thumbnail_path, width, height, duration_ms
         FROM media WHERE tweet_id=?1 ORDER BY id"
    ).map_err(|e| e.to_string())?;
    tweet.media = media_statement
        .query_map([&tweet.id], |row| {
            Ok(Media {
                id: row.get(0)?,
                kind: row.get(1)?,
                remote_url: row.get(2)?,
                preview_url: row.get(3)?,
                local_path: row.get(4)?,
                thumbnail_path: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                duration_ms: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    tweet.quote = connection.query_row(
        "SELECT quoted_id, text, canonical_url, author_name, username FROM quotes WHERE tweet_id=?1",
        [&tweet.id],
        |row| Ok(Quote { id: row.get::<_, Option<String>>(0)?.unwrap_or_default(), text: row.get(1)?, canonical_url: row.get(2)?, author_name: row.get(3)?, username: row.get(4)? })
    ).optional().map_err(|e| e.to_string())?;
    Ok(tweet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::{ArchiveProfile, ParsedArchive, SparseLike};

    fn database() -> (tempfile::TempDir, Database) {
        let temp = tempfile::tempdir().unwrap();
        let db = Database::initialize(temp.path().join("test.db")).unwrap();
        (temp, db)
    }

    fn profile(id: &str) -> ArchiveProfile {
        ArchiveProfile {
            id: id.into(),
            username: Some(format!("user{id}")),
            display_name: format!("User {id}"),
            avatar_url: None,
            from_account: true,
        }
    }

    #[test]
    fn import_is_idempotent_and_preserves_better_archive_text() {
        let (_temp, db) = database();
        let one = ParsedArchive {
            profile: profile("1"),
            likes: vec![SparseLike {
                id: "12345".into(),
                text: Some("short".into()),
                canonical_url: "https://x.com/i/web/status/12345".into(),
                liked_at: None,
            }],
            failed: 0,
        };
        let (first, _) = db.import_archive(&one, Path::new("archive")).unwrap();
        let better = ParsedArchive {
            profile: profile("1"),
            likes: vec![SparseLike {
                id: "12345".into(),
                text: Some("a much better archive text".into()),
                canonical_url: "https://x.com/alice/status/12345".into(),
                liked_at: None,
            }],
            failed: 0,
        };
        let (second, _) = db.import_archive(&better, Path::new("archive")).unwrap();
        assert_eq!((first.imported, first.existing), (1, 0));
        assert_eq!((second.imported, second.existing), (0, 1));
        let tweet = db.get_tweet("12345").unwrap();
        assert_eq!(
            tweet.archive_text.as_deref(),
            Some("a much better archive text")
        );
        assert_eq!(tweet.canonical_url, "https://x.com/alice/status/12345");
    }

    #[test]
    fn search_and_sql_filters_compose() {
        let (_temp, db) = database();
        let parsed = ParsedArchive {
            profile: profile("1"),
            likes: vec![
                SparseLike {
                    id: "11111".into(),
                    text: Some("Rust desktop tools".into()),
                    canonical_url: "https://x.com/i/web/status/11111".into(),
                    liked_at: None,
                },
                SparseLike {
                    id: "22222".into(),
                    text: Some("Cooking notes".into()),
                    canonical_url: "https://x.com/i/web/status/22222".into(),
                    liked_at: None,
                },
            ],
            failed: 0,
        };
        db.import_archive(&parsed, Path::new("archive")).unwrap();
        let page = db
            .get_tweets(TweetQuery {
                search: Some("Rust status:pending".into()),
                limit: 20,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].id, "11111");
        let empty = db
            .get_tweets(TweetQuery {
                media_type: Some("image".into()),
                limit: 20,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(empty.total, 0);
    }

    #[test]
    fn chronology_sort_is_numeric_and_reversible() {
        let (_temp, db) = database();
        let parsed = ParsedArchive {
            profile: profile("1"),
            likes: ["99999", "100000"]
                .into_iter()
                .map(|id| SparseLike {
                    id: id.into(),
                    text: Some(format!("Post {id}")),
                    canonical_url: format!("https://x.com/i/web/status/{id}"),
                    liked_at: None,
                })
                .collect(),
            failed: 0,
        };
        db.import_archive(&parsed, Path::new("archive")).unwrap();

        let newest = db
            .get_tweets(TweetQuery {
                limit: 20,
                ..Default::default()
            })
            .unwrap();
        let oldest = db
            .get_tweets(TweetQuery {
                sort_order: Some("oldest".into()),
                limit: 20,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(
            newest
                .items
                .iter()
                .map(|tweet| tweet.id.as_str())
                .collect::<Vec<_>>(),
            ["100000", "99999"]
        );
        assert_eq!(
            oldest
                .items
                .iter()
                .map(|tweet| tweet.id.as_str())
                .collect::<Vec<_>>(),
            ["99999", "100000"]
        );
        assert!(db
            .get_tweets(TweetQuery {
                sort_order: Some("random".into()),
                ..Default::default()
            })
            .is_err());
    }

    #[test]
    fn concurrent_enrichment_writes_do_not_lock_the_database() {
        let (_temp, db) = database();
        let parsed = ParsedArchive {
            profile: profile("1"),
            likes: (0..6)
                .map(|index| SparseLike {
                    id: format!("3000{index}"),
                    text: Some(format!("Post {index}")),
                    canonical_url: format!("https://x.com/i/web/status/3000{index}"),
                    liked_at: None,
                })
                .collect(),
            failed: 0,
        };
        db.import_archive(&parsed, Path::new("archive")).unwrap();

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(6));
        let workers = (0..6)
            .map(|index| {
                let db = db.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    for _ in 0..20 {
                        db.mark_failure(&format!("3000{index}"), "failed", "test failure", 1)
                            .unwrap();
                    }
                })
            })
            .collect::<Vec<_>>();

        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn lock_recovery_migration_only_requeues_lock_failures() {
        let (_temp, db) = database();
        let parsed = ParsedArchive {
            profile: profile("1"),
            likes: ["40000", "40001"]
                .into_iter()
                .map(|id| SparseLike {
                    id: id.into(),
                    text: None,
                    canonical_url: format!("https://x.com/i/web/status/{id}"),
                    liked_at: None,
                })
                .collect(),
            failed: 0,
        };
        db.import_archive(&parsed, Path::new("archive")).unwrap();
        db.mark_failure(
            "40000",
            "failed",
            "Could not store enrichment result: database is locked",
            1,
        )
        .unwrap();
        db.mark_failure("40001", "failed", "Provider schema changed", 1)
            .unwrap();

        db.connect()
            .unwrap()
            .execute_batch(include_str!(
                "../../migrations/0003_retry_locked_enrichments.sql"
            ))
            .unwrap();

        assert_eq!(db.get_tweet("40000").unwrap().enrichment_status, "pending");
        assert_eq!(db.get_tweet("40001").unwrap().enrichment_status, "failed");
    }

    #[test]
    fn profiles_isolate_overlapping_likes_and_reimport_per_profile() {
        let (_temp, db) = database();
        let first = ParsedArchive {
            profile: profile("10"),
            likes: ["11111", "22222"]
                .into_iter()
                .map(|id| SparseLike {
                    id: id.into(),
                    text: Some(format!("first {id}")),
                    canonical_url: format!("https://x.com/i/web/status/{id}"),
                    liked_at: None,
                })
                .collect(),
            failed: 0,
        };
        let second = ParsedArchive {
            profile: profile("20"),
            likes: ["11111", "33333"]
                .into_iter()
                .map(|id| SparseLike {
                    id: id.into(),
                    text: Some(format!("second {id}")),
                    canonical_url: format!("https://x.com/i/web/status/{id}"),
                    liked_at: None,
                })
                .collect(),
            failed: 0,
        };

        db.import_archive(&first, Path::new("first")).unwrap();
        db.import_archive(&second, Path::new("second")).unwrap();
        let second_page = db
            .get_tweets(TweetQuery {
                limit: 20,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            second_page
                .items
                .iter()
                .map(|tweet| tweet.id.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            ["11111", "33333"].into_iter().collect()
        );
        assert!(db.get_tweet("22222").is_err());

        db.set_active_profile("10").unwrap();
        let first_page = db
            .get_tweets(TweetQuery {
                limit: 20,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            first_page
                .items
                .iter()
                .map(|tweet| tweet.id.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            ["11111", "22222"].into_iter().collect()
        );
        assert!(db.get_tweet("33333").is_err());
        assert!(db.set_active_profile("missing").is_err());

        let (summary, _) = db.import_archive(&first, Path::new("first")).unwrap();
        assert_eq!((summary.imported, summary.existing), (0, 2));
        let connection = db.connect().unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM tweets", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM likes", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            4
        );
    }

    #[test]
    fn missing_active_profile_returns_an_empty_feed() {
        let (_temp, db) = database();
        let page = db
            .get_tweets(TweetQuery {
                limit: 20,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page.total, 0);
        assert!(page.items.is_empty());
    }

    #[test]
    fn migration_preserves_and_adopts_legacy_likes_from_existing_source() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("archive");
        let data = source.join("data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(
            data.join("like.js"),
            r#"window.YTD.like.part0 = [{"like":{"tweetId":"55555","fullText":"preserved"}}];"#,
        )
        .unwrap();
        std::fs::write(
            data.join("account.js"),
            r#"window.YTD.account.part0 = [{"account":{"accountId":"777","username":"AbsoluteMememan","accountDisplayName":"Mr. Mememan"}}];"#,
        )
        .unwrap();

        let path = temp.path().join("legacy.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL
                 );",
            )
            .unwrap();
        for (version, sql) in MIGRATIONS.iter().take(3) {
            connection.execute_batch(sql).unwrap();
            connection
                .execute(
                    "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, 'test')",
                    [version],
                )
                .unwrap();
        }
        connection
            .execute(
                "INSERT INTO tweets(id, text, canonical_url, imported_at)
                 VALUES ('55555', 'preserved', 'https://x.com/i/web/status/55555', 'test')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO likes(tweet_id, archive_text, archive_source)
                 VALUES ('55555', 'preserved', ?1)",
                [source.to_string_lossy().as_ref()],
            )
            .unwrap();
        drop(connection);

        let db = Database::initialize(path).unwrap();
        let state = db.get_profiles().unwrap();
        assert_eq!(state.active_profile_id.as_deref(), Some("777"));
        assert_eq!(state.profiles.len(), 1);
        assert_eq!(
            state.profiles[0].username.as_deref(),
            Some("AbsoluteMememan")
        );
        assert_eq!(state.profiles[0].display_name, "Mr. Mememan");
        assert_eq!(state.profiles[0].post_count, 1);
        assert_eq!(
            db.get_tweet("55555").unwrap().archive_text.as_deref(),
            Some("preserved")
        );
        assert_eq!(
            db.connect()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM likes WHERE profile_id = '__legacy__'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
