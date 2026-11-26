//! PC → phone sync over real QUIC on this machine: pairing, the first mirror, an update that
//! adds, changes and removes things, and unpairing.

use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use rusqlite::{params, Connection};

use super::{ClientHooks, Node, NodeConfig, Role};
use crate::{
    archive::{ArchiveProfile, ParsedArchive, SparseLike},
    db::{models::TweetQuery, Database},
};

struct Pc {
    dir: PathBuf,
    db: Database,
    node: Node,
}

struct Phone {
    dir: PathBuf,
    node: Node,
    applied: Arc<AtomicUsize>,
}

fn like(id: &str, text: &str) -> SparseLike {
    SparseLike {
        id: id.into(),
        text: Some(text.into()),
        canonical_url: format!("https://x.com/i/web/status/{id}"),
        liked_at: Some("2024-01-01T00:00:00Z".into()),
    }
}

fn archive(likes: Vec<SparseLike>) -> ParsedArchive {
    ParsedArchive {
        profile: ArchiveProfile {
            id: "42".into(),
            username: Some("me".into()),
            display_name: "Me".into(),
            avatar_url: None,
            from_account: true,
        },
        likes,
        failed: 0,
    }
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

async fn pc(root: &Path) -> Pc {
    let dir = root.join("pc");
    let db = Database::initialize(dir.join("knowledge.db")).unwrap();
    db.import_archive(&archive(vec![like("100", "hello rust"), like("200", "second post"), like("300", "third")]), Path::new("a.zip"))
        .unwrap();
    let media = dir.join("media");
    let image = media.join("images/100/0.jpg");
    let avatar = media.join("avatars/7.jpg");
    write(&image, &vec![0xAB; 300_000]);
    write(&avatar, b"avatar");
    let raw = r#"{"tweet":{"media":{"all":[{"id":"v1","formats":[
        {"url":"https://video.twimg.com/low.mp4","bitrate":800000,"container":"mp4"},
        {"url":"https://video.twimg.com/high.mp4","bitrate":9000000,"container":"mp4"}]}]}}}"#;
    let c = Connection::open(db.path()).unwrap();
    c.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    c.execute(
        "INSERT INTO users(id, username, display_name, avatar_local_path) VALUES ('7', 'alice', 'Alice', ?1)",
        [avatar.to_string_lossy()],
    )
    .unwrap();
    c.execute(
        "UPDATE tweets SET author_id = '7', enrichment_status = 'ok', raw_json = ?1, enrichment_error = 'secret' WHERE id = '100'",
        [raw],
    )
    .unwrap();
    c.execute(
        "INSERT INTO media(id, tweet_id, kind, remote_url, local_path) VALUES ('m1', '100', 'image', 'https://pbs.twimg.com/a.jpg', ?1)",
        [image.to_string_lossy()],
    )
    .unwrap();
    c.execute(
        "INSERT INTO media(id, tweet_id, kind, remote_url, local_path) VALUES ('v1', '100', 'video', 'https://video.twimg.com/fallback.mp4', ?1)",
        [media.join("videos/missing.mp4").to_string_lossy()],
    )
    .unwrap();
    c.execute(
        "INSERT INTO playlists(id, profile_id, name, created_at, updated_at) VALUES (1, '42', 'Rust', '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    c.execute("INSERT INTO playlist_items(playlist_id, tweet_id, note) VALUES (1, '100', 'good')", []).unwrap();
    c.execute("UPDATE playlists SET updated_at = '2020-02-02T00:00:00Z' WHERE id = 1", []).unwrap();
    drop(c);
    let node = Node::open(NodeConfig {
        data_dir: dir.join("sync"),
        device_name: "Desk".into(),
        platform: "linux".into(),
        port: 0,
        mdns: false,
        role: Role::Server { db_path: db.path().to_path_buf(), media_root: media },
    })
    .await
    .unwrap();
    Pc { dir, db, node }
}

async fn phone(root: &Path) -> Phone {
    let dir = root.join("phone");
    Database::initialize_with(dir.join("knowledge.db"), false).unwrap();
    let applied = Arc::new(AtomicUsize::new(0));
    let counter = applied.clone();
    let node = Node::open(NodeConfig {
        data_dir: dir.join("sync"),
        device_name: "Pixel".into(),
        platform: "android".into(),
        port: 0,
        mdns: false,
        role: Role::Client {
            db_path: dir.join("knowledge.db"),
            media_root: dir.join("media"),
            hooks: ClientHooks {
                applied: Box::new(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                }),
                busy: Box::new(|_| {}),
            },
        },
    })
    .await
    .unwrap();
    Phone { dir, node, applied }
}

async fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    for _ in 0..300 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("timed out waiting for {what}");
}

fn ids(db: &Database, query: TweetQuery) -> Vec<String> {
    db.get_tweets(TweetQuery { limit: 100, ..query }).unwrap().items.into_iter().map(|t| t.id).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn phone_mirrors_the_pc_and_follows_its_changes() {
    let temp = tempfile::tempdir().unwrap();
    let pc = pc(temp.path()).await;
    let phone = phone(temp.path()).await;

    let invite = pc.node.create_invite().unwrap();
    assert_eq!(phone.node.use_code(&invite.ticket).await.unwrap(), "Desk");
    assert!(phone.node.use_code(&invite.ticket).await.is_err(), "a code works once");
    wait_for("the first sync", || phone.applied.load(Ordering::SeqCst) == 1).await;
    let status = phone.node.status();
    assert_eq!(status.progress.as_ref().unwrap().phase, "upToDate");
    assert_eq!(status.progress.as_ref().unwrap().posts, Some(3));
    wait_for("the PC to hear about it", || pc.node.status().devices[0].last_sync.is_some()).await;
    assert_eq!(pc.node.status().devices[0].name, "Pixel");

    let mirror = Database::initialize_with(phone.dir.join("knowledge.db"), false).unwrap();
    assert_eq!(ids(&mirror, TweetQuery::default()), ids(&pc.db, TweetQuery::default()));
    let search = TweetQuery { search: Some("rust".into()), ..Default::default() };
    assert_eq!(ids(&mirror, search.clone()), ["100"], "full-text search works on the phone");
    let tweet = mirror.get_tweet("100").unwrap();
    let image = tweet.media.iter().find(|m| m.id == "m1").unwrap();
    let local = image.local_path.clone().unwrap();
    assert!(local.starts_with(&*phone.dir.join("media").to_string_lossy()), "{local}");
    assert_eq!(std::fs::read(&local).unwrap().len(), 300_000);
    let video = tweet.media.iter().find(|m| m.id == "v1").unwrap();
    assert_eq!(video.remote_url.as_deref(), Some("https://video.twimg.com/low.mp4"), "playable variant picked on the PC");
    assert_eq!(video.local_path, None, "a missing file isn't referenced");
    assert!(std::fs::read(tweet.author.unwrap().avatar_local_path.unwrap()).unwrap() == b"avatar");
    assert_eq!(tweet.playlist_ids, [1]);
    let playlists = mirror.get_playlists().unwrap();
    assert_eq!(playlists[0].updated_at, "2020-02-02T00:00:00Z", "playlist dates survive the copy");
    let c = Connection::open(mirror.path()).unwrap();
    let leaked: i64 = c
        .query_row("SELECT COUNT(*) FROM tweets WHERE raw_json IS NOT NULL OR enrichment_error IS NOT NULL", [], |r| r.get(0))
        .unwrap();
    assert_eq!(leaked, 0);
    let mode: String = c.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
    assert_eq!(mode, "delete");
    drop(c);

    // The phone's own choice of profile survives a sync.
    let c = Connection::open(pc.db.path()).unwrap();
    c.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    c.execute("INSERT INTO profiles(id, display_name) VALUES ('43', 'Other')", []).unwrap();
    c.execute("INSERT INTO tweets(id, canonical_url, imported_at) VALUES ('400', 'u', 'now')", []).unwrap();
    c.execute("INSERT INTO likes(profile_id, tweet_id) VALUES ('43', '400')", []).unwrap();
    drop(c);
    phone.node.sync_now();
    wait_for("the second profile", || phone.applied.load(Ordering::SeqCst) == 2).await;
    mirror.set_active_profile("43").unwrap();

    // Changes on the PC: a new post, a removed image, a renamed playlist.
    pc.db.import_archive(&archive(vec![like("500", "brand new")]), Path::new("b.zip")).unwrap();
    let c = Connection::open(pc.db.path()).unwrap();
    c.execute("DELETE FROM media WHERE id = 'm1'", []).unwrap();
    c.execute("UPDATE playlists SET name = 'Rustacean' WHERE id = 1", []).unwrap();
    drop(c);
    std::fs::remove_file(pc.dir.join("media/images/100/0.jpg")).unwrap();
    phone.node.sync_now();
    wait_for("the update", || phone.applied.load(Ordering::SeqCst) == 3).await;
    assert_eq!(mirror.get_profiles().unwrap().active_profile_id.as_deref(), Some("43"));
    mirror.set_active_profile("42").unwrap();
    assert!(ids(&mirror, TweetQuery::default()).contains(&"500".to_string()));
    assert_eq!(mirror.get_playlists().unwrap()[0].name, "Rustacean");
    assert!(!Path::new(&local).exists(), "media the PC dropped is removed from the phone");
    assert!(!phone.dir.join("sync/incoming").exists());

    // Nothing changed: no new download.
    phone.node.sync_now();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(phone.applied.load(Ordering::SeqCst), 3);

    // Unpaired on the PC: the phone can't sync any more.
    let phone_id = pc.node.status().devices[0].id.clone();
    pc.node.remove_device(&phone_id).unwrap();
    pc.db.import_archive(&archive(vec![like("600", "after unpairing")]), Path::new("c.zip")).unwrap();
    phone.node.sync_now();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(phone.applied.load(Ordering::SeqCst), 3);
    assert!(!ids(&mirror, TweetQuery::default()).contains(&"600".to_string()));

    pc.node.shutdown().await;
    phone.node.shutdown().await;
}

/// Builds a snapshot of the real library (read-only) and reports its size:
/// `KB_REAL_DATA=~/.local/share/dev.local.xknowledgebase cargo test real_library -- --ignored --nocapture`
#[test]
#[ignore]
fn real_library_snapshot() {
    let data = PathBuf::from(std::env::var("KB_REAL_DATA").expect("KB_REAL_DATA"));
    let temp = tempfile::tempdir().unwrap();
    // Work on a copy so the real database is never touched.
    let copy = temp.path().join("knowledge.db");
    let c = Connection::open(data.join("knowledge.db")).unwrap();
    c.execute("VACUUM INTO ?1", params![copy.to_string_lossy()]).unwrap();
    drop(c);
    let started = std::time::Instant::now();
    let built = super::snapshot::build(&copy, &data.join("media"), &temp.path().join("out")).unwrap();
    let again = super::snapshot::build(&copy, &data.join("media"), &temp.path().join("out2")).unwrap();
    let media_bytes: u64 = built.manifest.media.iter().map(|e| e.size).sum();
    println!(
        "posts {} | snapshot {:.1} MB | media {} files, {:.1} MB | built in {:?} | rebuild same hash: {}",
        built.manifest.posts,
        built.manifest.snapshot_size as f64 / 1e6,
        built.manifest.media.len(),
        media_bytes as f64 / 1e6,
        started.elapsed() / 2,
        built.manifest.snapshot == again.manifest.snapshot
    );
    let source = Database::initialize(copy).unwrap();
    let snap = Database::initialize_with(built.file.clone(), false).unwrap();
    for query in [
        TweetQuery::default(),
        TweetQuery { search: Some("rust".into()), ..Default::default() },
        TweetQuery { media_type: Some("video".into()), ..Default::default() },
    ] {
        let a = source.get_tweets(TweetQuery { limit: 1, ..query.clone() }).unwrap().total;
        let b = snap.get_tweets(TweetQuery { limit: 1, ..query }).unwrap().total;
        assert_eq!(a, b);
    }
}
