mod archive;
mod commands;
mod db;
mod enrichment;
mod media;

use std::{io, sync::Arc, time::Duration};

use db::Database;
use enrichment::EnrichmentQueue;
use media::MediaCache;
use tauri::Manager;

pub struct Backend {
    pub db: Database,
    pub queue: EnrichmentQueue,
    pub client: reqwest::Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data)?;
            std::fs::create_dir_all(app_data.join("cache"))?;
            let db =
                Database::initialize(app_data.join("knowledge.db")).map_err(io::Error::other)?;
            let client = reqwest::Client::builder()
                .user_agent(concat!("x-knowledge-base/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .build()?;
            let cache = MediaCache::new(app_data.join("media"), client.clone())
                .map_err(io::Error::other)?;
            let concurrency = std::env::var("X_KNOWLEDGE_BASE_CONCURRENCY")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(6)
                .clamp(1, 32);
            let pending = db.pending_ids().map_err(io::Error::other)?;
            let queue = EnrichmentQueue::start(db.clone(), cache, client.clone(), concurrency);
            queue.enqueue(pending);
            app.manage(Arc::new(Backend { db, queue, client }));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_archive,
            commands::get_tweets,
            commands::get_tweet,
            commands::get_profiles,
            commands::set_active_profile,
            commands::retry_enrichment,
            commands::get_import_status,
            commands::load_video,
            commands::play_video_mpv,
            commands::open_original,
        ])
        .run(tauri::generate_context!())
        .expect("error while running X Knowledge Base");
}
