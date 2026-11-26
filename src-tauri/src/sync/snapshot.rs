//! The snapshot a phone mirrors: a slimmed copy of the PC's database (no raw enrichment JSON,
//! no error text, no archive paths) whose media paths are relative to the media folder, plus
//! the list of media files it references.
//!
//! PC side: [`build`]. Phone side: [`apply`] swaps a verified snapshot in as the phone's
//! database, and [`prune_media`] removes files the PC no longer lists.

use std::{
    collections::{BTreeMap, HashSet},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use anyhow::{bail, Context, Result};
use rusqlite::{functions::FunctionFlags, params, Connection, OptionalExtension};

use super::proto::{Manifest, MediaEntry};
use crate::db::{Database, SCHEMA_VERSION};

/// A built snapshot on the PC.
#[derive(Debug, Clone)]
pub struct Built {
    pub manifest: Manifest,
    pub file: PathBuf,
    pub media: Arc<HashSet<String>>,
}

/// Changes whenever the PC's database does (its file or its write-ahead log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint(Vec<(u64, Option<SystemTime>)>);

pub fn fingerprint(db: &Path) -> Fingerprint {
    let stat = |path: PathBuf| {
        std::fs::metadata(path)
            .map(|m| (m.len(), m.modified().ok()))
            .unwrap_or((0, None))
    };
    let mut wal = db.as_os_str().to_owned();
    wal.push("-wal");
    Fingerprint(vec![stat(db.to_path_buf()), stat(PathBuf::from(wal))])
}

/// Builds a snapshot of `source` into `out_dir` as `snapshot-<hash>.db`.
pub fn build(source: &Path, media_root: &Path, out_dir: &Path) -> Result<Built> {
    std::fs::create_dir_all(out_dir)?;
    let temp = out_dir.join("building.db");
    for stale in [temp.clone(), sibling(&temp, "-journal")] {
        let _ = std::fs::remove_file(stale);
    }
    Database::initialize_with(temp.clone(), false).map_err(anyhow::Error::msg)?;

    let referenced: Arc<Mutex<BTreeMap<String, u64>>> = Arc::default();
    let mut connection = Connection::open(&temp)?;
    register_functions(&connection, media_root.to_path_buf(), referenced.clone())?;
    connection.execute("ATTACH DATABASE ?1 AS src", [source.to_string_lossy()])?;
    let tx = connection.transaction()?;
    tx.execute_batch(
        "DELETE FROM app_settings;
         DELETE FROM profiles;
         INSERT INTO profiles(id, username, display_name, avatar_url, avatar_local_path)
             SELECT id, username, display_name, avatar_url, kb_rel(avatar_local_path) FROM src.profiles;
         INSERT INTO users(id, username, display_name, avatar_url, avatar_local_path)
             SELECT id, username, display_name, avatar_url, kb_rel(avatar_local_path) FROM src.users;
         INSERT INTO tweets(id, author_id, text, created_at, canonical_url, raw_json,
                            enrichment_status, enrichment_error, enriched_at, imported_at, retry_count)
             SELECT id, author_id, text, created_at, canonical_url, NULL,
                    enrichment_status, NULL, enriched_at, imported_at, 0
             FROM src.tweets ORDER BY rowid;
         INSERT INTO likes(profile_id, tweet_id, liked_at, archive_text, archive_source)
             SELECT profile_id, tweet_id, liked_at, archive_text, NULL FROM src.likes ORDER BY rowid;
         INSERT INTO quotes(tweet_id, quoted_id, text, canonical_url, author_name, username)
             SELECT tweet_id, quoted_id, text, canonical_url, author_name, username FROM src.quotes;
         INSERT INTO media(id, tweet_id, kind, remote_url, preview_url, local_path, thumbnail_path,
                           width, height, duration_ms)
             SELECT m.id, m.tweet_id, m.kind,
                    CASE WHEN m.kind = 'video' THEN kb_video(m.id, m.remote_url, t.raw_json)
                         ELSE m.remote_url END,
                    m.preview_url, kb_rel(m.local_path), kb_rel(m.thumbnail_path),
                    m.width, m.height, m.duration_ms
             FROM src.media m JOIN src.tweets t ON t.id = m.tweet_id;
         INSERT INTO playlists(id, profile_id, name, description, created_at, updated_at)
             SELECT id, profile_id, name, description, created_at, updated_at FROM src.playlists;
         INSERT INTO playlist_items(playlist_id, tweet_id, note, added_at)
             SELECT playlist_id, tweet_id, note, added_at FROM src.playlist_items;
         -- Adding items above touched every playlist's updated_at; the PC's dates order the list.
         UPDATE playlists SET updated_at = (SELECT s.updated_at FROM src.playlists s WHERE s.id = playlists.id);
         INSERT INTO app_settings(key, value)
             SELECT key, value FROM src.app_settings WHERE key = 'active_profile_id';
         -- Same content, same bytes: the phone compares snapshots by hash.
         UPDATE schema_migrations SET applied_at = '1970-01-01T00:00:00+00:00';",
    )?;
    let posts: i64 = tx.query_row("SELECT COUNT(DISTINCT tweet_id) FROM likes", [], |r| r.get(0))?;
    tx.commit()?;
    connection.execute_batch("DETACH DATABASE src; VACUUM;")?;
    drop(connection);

    let (hash, size) = hash_file(&temp)?;
    let file = out_dir.join(format!("snapshot-{}.db", hex(&hash)));
    std::fs::rename(&temp, &file)?;
    let media = std::mem::take(&mut *referenced.lock().unwrap());
    let manifest = Manifest {
        schema: SCHEMA_VERSION,
        snapshot: hash,
        snapshot_size: size,
        media: media.iter().map(|(path, size)| MediaEntry { path: path.clone(), size: *size }).collect(),
        posts: posts.max(0) as u64,
        built_at: humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
    };
    Ok(Built { manifest, file, media: Arc::new(media.into_keys().collect()) })
}

fn register_functions(
    connection: &Connection,
    media_root: PathBuf,
    referenced: Arc<Mutex<BTreeMap<String, u64>>>,
) -> Result<()> {
    // kb_rel(path): the path relative to the media folder when it's a file there, else NULL.
    // Every file it returns is recorded for the manifest.
    connection.create_scalar_function("kb_rel", 1, FunctionFlags::SQLITE_UTF8, move |ctx| {
        let Some(path) = ctx.get::<Option<String>>(0)? else { return Ok(None) };
        let Ok(rel) = Path::new(&path).strip_prefix(&media_root) else { return Ok(None) };
        if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
            return Ok(None);
        }
        let Some(rel_text) = rel.to_str().map(|s| s.replace('\\', "/")) else { return Ok(None) };
        match std::fs::metadata(media_root.join(rel)) {
            Ok(meta) if meta.is_file() && meta.len() > 0 => {
                referenced.lock().unwrap().insert(rel_text.clone(), meta.len());
                Ok(Some(rel_text))
            }
            _ => Ok(None),
        }
    })?;
    // kb_video(media id, stored url, raw json): the in-app playback URL the desktop would pick.
    connection.create_scalar_function("kb_video", 3, FunctionFlags::SQLITE_UTF8, |ctx| {
        let id: String = ctx.get(0)?;
        let Some(fallback) = ctx.get::<Option<String>>(1)? else { return Ok(None) };
        let raw: Option<String> = ctx.get(2)?;
        Ok(Some(crate::commands::embedded_video_url(&id, &fallback, raw.as_deref())))
    })?;
    Ok(())
}

/// Makes a verified snapshot the phone's database. The snapshot file is consumed.
pub fn apply(snapshot: &Path, db_path: &Path, media_root: &Path) -> Result<()> {
    let root = format!("{}/", media_root.to_string_lossy().trim_end_matches('/'));
    {
        let connection = Connection::open(snapshot)?;
        let schema: i64 = connection.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0))?;
        if schema > SCHEMA_VERSION {
            bail!("The PC's app is newer than this one. Install the new phone app.");
        }
        connection.execute_batch("PRAGMA journal_mode = DELETE;")?;
        let tx = connection.unchecked_transaction()?;
        for (table, column) in [
            ("media", "local_path"),
            ("media", "thumbnail_path"),
            ("users", "avatar_local_path"),
            ("profiles", "avatar_local_path"),
        ] {
            tx.execute(
                &format!("UPDATE {table} SET {column} = ?1 || {column} WHERE {column} IS NOT NULL"),
                [&root],
            )?;
        }
        // The phone keeps the profile it was showing.
        if let Some(active) = current_profile(db_path) {
            tx.execute(
                "INSERT INTO app_settings(key, value)
                 SELECT 'active_profile_id', id FROM profiles WHERE id = ?1
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                [active],
            )?;
        }
        tx.commit()?;
    }
    std::fs::File::open(snapshot)?.sync_all()?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let _ = std::fs::remove_file(sibling(db_path, suffix));
    }
    std::fs::rename(snapshot, db_path).context("replacing the phone's library")?;
    if let Some(dir) = db_path.parent() {
        if let Ok(dir) = std::fs::File::open(dir) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

fn current_profile(db_path: &Path) -> Option<String> {
    let connection = Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    connection
        .query_row("SELECT value FROM app_settings WHERE key = 'active_profile_id'", params![], |r| r.get(0))
        .optional()
        .ok()
        .flatten()
}

/// Deletes media files (and leftover partial downloads) the manifest no longer lists.
pub fn prune_media(media_root: &Path, keep: &HashSet<String>) -> usize {
    let mut removed = 0;
    for entry in walkdir::WalkDir::new(media_root).min_depth(1).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(media_root) else { continue };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if !keep.contains(&rel) && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    // Empty folders left behind (deepest first).
    for entry in walkdir::WalkDir::new(media_root).min_depth(1).contents_first(true).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_dir() {
            let _ = std::fs::remove_dir(entry.path());
        }
    }
    removed
}

pub fn hash_file(path: &Path) -> Result<([u8; 32], u64)> {
    let mut hasher = blake3::Hasher::new();
    let mut file = std::fs::File::open(path)?;
    let size = std::io::copy(&mut file, &mut hasher)?;
    Ok((*hasher.finalize().as_bytes(), size))
}

pub fn hex(bytes: &[u8]) -> String {
    data_encoding::HEXLOWER.encode(bytes)
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
