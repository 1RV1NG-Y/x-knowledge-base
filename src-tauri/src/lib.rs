mod archive;
mod commands;
mod db;
mod enrichment;
mod media;
mod sync;

use std::{
    io,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use db::Database;
use enrichment::EnrichmentQueue;
use media::MediaCache;
use tauri::{Emitter, Manager};

pub struct Backend {
    pub db: Database,
    pub queue: EnrichmentQueue,
    pub client: reqwest::Client,
}

/// The sync node, once it has started (or why it couldn't).
#[derive(Clone, Default)]
pub struct SyncState(pub Arc<Mutex<Option<Result<sync::Node, String>>>>);

/// The phone build is a read-only mirror of the PC: no imports, no enrichment, its library
/// replaced by each sync. `KB_PHONE=1` runs a desktop build that way, for testing.
pub fn is_phone() -> bool {
    cfg!(target_os = "android") || std::env::var_os("KB_PHONE").is_some_and(|v| v == "1")
}

#[cfg(target_os = "android")]
struct PhoneControls(tauri::plugin::PluginHandle<tauri::Wry>);

#[cfg(target_os = "android")]
fn phone_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("phone")
        .setup(|app, api| {
            app.manage(PhoneControls(
                api.register_android_plugin("dev.local.xknowledgebase", "PhonePlugin")?,
            ));
            Ok(())
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut context = tauri::generate_context!();
    // A shared GTK app ID makes a second launch hand off to the running instance and exit,
    // so dev builds skip it and can run alongside the installed app.
    #[cfg(debug_assertions)]
    {
        context.config_mut().app.enable_gtk_app_id = false;
    }

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());
    #[cfg(target_os = "android")]
    {
        builder = builder
            .plugin(tauri_plugin_barcode_scanner::init())
            .plugin(phone_plugin());
    }

    builder
        .manage(SyncState::default())
        .setup(|app| {
            let phone = is_phone();
            // KB_DATA_DIR lets a test copy run beside the real app with its own data.
            let app_data = match std::env::var_os("KB_DATA_DIR") {
                Some(dir) => PathBuf::from(dir),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&app_data)?;
            std::fs::create_dir_all(app_data.join("cache"))?;
            let db_path = app_data.join("knowledge.db");
            let db = Database::initialize_with(db_path.clone(), !phone).map_err(io::Error::other)?;
            let client = reqwest::Client::builder()
                .user_agent(concat!("x-knowledge-base/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(Duration::from_secs(10))
                .timeout(Duration::from_secs(30))
                .build()?;
            let media_root = app_data.join("media");
            let cache = MediaCache::new(media_root.clone(), client.clone())
                .map_err(io::Error::other)?;
            let concurrency = std::env::var("X_KNOWLEDGE_BASE_CONCURRENCY")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(6)
                .clamp(1, 32);
            let queue = EnrichmentQueue::start(db.clone(), cache, client.clone(), concurrency);
            if !phone {
                queue.enqueue(db.pending_ids().map_err(io::Error::other)?);
            }
            app.manage(Arc::new(Backend { db, queue, client }));
            start_sync(app.handle().clone(), app_data, db_path, media_root, phone);

            // WebKitGTK eases every wheel tick over a long animation, which makes the
            // feed lag behind the wheel. Scroll directly instead; touchpads keep their
            // precise deltas and kinetic momentum.
            #[cfg(target_os = "linux")]
            if let Some(window) = app.get_webview_window("main") {
                window.with_webview(|webview| {
                    use webkit2gtk::{SettingsExt, WebViewExt};
                    if let Some(settings) = webview.inner().settings() {
                        settings.set_enable_smooth_scrolling(false);
                    }
                })?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_archive,
            commands::get_tweets,
            commands::get_tweet,
            commands::get_profiles,
            commands::set_active_profile,
            commands::get_playlists,
            commands::create_playlist,
            commands::update_playlist,
            commands::delete_playlist,
            commands::set_playlist_membership,
            commands::retry_enrichment,
            commands::get_import_status,
            commands::load_video,
            commands::play_video_mpv,
            commands::open_original,
            commands::sync::sync_status,
            commands::sync::sync_invite,
            commands::sync::sync_use_code,
            commands::sync::sync_remove_device,
            commands::sync::sync_now,
        ])
        .run(context)
        .expect("error while running X Knowledge Base");
}

/// Starts the sync node in the background. A failure (port busy, another copy of the app
/// holding the sync identity) is shown in the sync screen; the rest of the app works.
fn start_sync(
    app: tauri::AppHandle,
    app_data: PathBuf,
    db_path: PathBuf,
    media_root: PathBuf,
    phone: bool,
) {
    let state = app.state::<SyncState>().inner().clone();
    tauri::async_runtime::spawn(async move {
        let role = if phone {
            let ui = app.clone();
            #[cfg(target_os = "android")]
            let awake = app.clone();
            sync::Role::Client {
                db_path,
                media_root,
                hooks: sync::ClientHooks {
                    applied: Box::new(move || {
                        let _ = ui.emit("library-synced", ());
                    }),
                    busy: Box::new(move |_on| {
                        #[cfg(target_os = "android")]
                        {
                            let controls = awake.state::<PhoneControls>().0.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = controls
                                    .run_mobile_plugin_async::<serde_json::Value>(
                                        "keepAwake",
                                        serde_json::json!({ "on": _on }),
                                    )
                                    .await;
                            });
                        }
                    }),
                },
            }
        } else {
            sync::Role::Server { db_path, media_root }
        };
        let port = std::env::var("KB_SYNC_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(sync::DEFAULT_PORT);
        let result = sync::Node::open(sync::NodeConfig {
            data_dir: app_data.join("sync"),
            device_name: sync::device_name(),
            platform: if cfg!(target_os = "android") { "android".into() } else { std::env::consts::OS.into() },
            port,
            mdns: true,
            role,
        })
        .await
        .map_err(|e| format!("{e:#}"));
        if let Err(error) = &result {
            eprintln!("phone sync is off: {error}");
        }
        if let Ok(node) = &result {
            let mut events = node.subscribe();
            let ui = app.clone();
            tauri::async_runtime::spawn(async move {
                use tokio::sync::broadcast::error::RecvError;
                // Downloads report progress many times a second; the UI needs a few.
                loop {
                    match events.recv().await {
                        Ok(_) | Err(RecvError::Lagged(_)) => {
                            tokio::time::sleep(Duration::from_millis(250)).await;
                            while events.try_recv().is_ok() {}
                            let _ = ui.emit("sync-status", ());
                        }
                        Err(RecvError::Closed) => break,
                    }
                }
            });
        }
        *state.0.lock().unwrap() = Some(result);
        let _ = app.emit("sync-status", ());
    });
}
