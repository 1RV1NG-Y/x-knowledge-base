pub mod fxtwitter;

use std::{
    collections::HashSet,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use tokio::sync::{mpsc, Semaphore};

use crate::{
    db::{
        models::{Author, JobStatus, Media, Quote},
        Database,
    },
    media::MediaCache,
};
use fxtwitter::{FxTwitter, TweetEnricher};

#[derive(Debug, Clone)]
pub struct NormalizedTweet {
    pub id: String,
    pub text: String,
    pub created_at: Option<String>,
    pub canonical_url: String,
    pub author: Option<Author>,
    pub media: Vec<Media>,
    pub quote: Option<Quote>,
    pub raw_json: String,
}

#[derive(Clone)]
pub struct EnrichmentQueue {
    sender: mpsc::UnboundedSender<String>,
    queued: Arc<Mutex<HashSet<String>>>,
    status: Arc<RwLock<JobStatus>>,
}

impl EnrichmentQueue {
    pub fn start(
        db: Database,
        cache: MediaCache,
        client: reqwest::Client,
        concurrency: usize,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let queued = Arc::new(Mutex::new(HashSet::new()));
        let status = Arc::new(RwLock::new(JobStatus::default()));
        tauri::async_runtime::spawn(dispatch(
            receiver,
            db,
            cache,
            FxTwitter::new(client),
            concurrency.clamp(1, 32),
            queued.clone(),
            status.clone(),
        ));
        Self {
            sender,
            queued,
            status,
        }
    }

    pub fn enqueue<I>(&self, ids: I)
    where
        I: IntoIterator<Item = String>,
    {
        let mut newly_queued = Vec::new();
        {
            let mut queued = self
                .queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for id in ids {
                if queued.insert(id.clone()) {
                    newly_queued.push(id);
                }
            }
        }
        if newly_queued.is_empty() {
            return;
        }
        {
            let mut status = self
                .status
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if matches!(status.phase.as_str(), "idle" | "complete") {
                status.processed = 0;
                status.total = 0;
                status.failed = 0;
            }
            status.phase = "enriching".into();
            status.total += newly_queued.len() as u64;
            status.message = format!(
                "Enriching {} posts",
                status.total.saturating_sub(status.processed)
            );
        }
        for id in newly_queued {
            if self.sender.send(id.clone()).is_err() {
                self.queued
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&id);
                let mut status = self
                    .status
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                status.failed += 1;
                status.message = "The enrichment queue stopped unexpectedly".into();
            }
        }
    }

    pub fn begin_import(&self, path: &str) {
        let mut status = self
            .status
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        status.phase = "importing".into();
        status.processed = 0;
        status.total = 0;
        status.failed = 0;
        status.message = format!("Reading archive: {path}");
    }

    pub fn import_failed(&self, message: String) {
        let mut status = self
            .status
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        status.phase = "failed".into();
        status.failed = 1;
        status.message = message;
    }

    pub fn import_complete(&self, imported: u64, existing: u64, failed: u64) {
        let mut status = self
            .status
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        status.phase = "complete".into();
        status.processed = imported + existing;
        status.total = imported + existing;
        status.failed = failed;
        status.message = format!("Imported {imported} new and {existing} existing likes");
    }

    pub fn status(&self) -> JobStatus {
        self.status
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

async fn dispatch(
    mut receiver: mpsc::UnboundedReceiver<String>,
    db: Database,
    cache: MediaCache,
    enricher: FxTwitter,
    concurrency: usize,
    queued: Arc<Mutex<HashSet<String>>>,
    status: Arc<RwLock<JobStatus>>,
) {
    let semaphore = Arc::new(Semaphore::new(concurrency));
    while let Some(id) = receiver.recv().await {
        let Ok(permit) = semaphore.clone().acquire_owned().await else {
            break;
        };
        let db = db.clone();
        let cache = cache.clone();
        let enricher = enricher.clone();
        let queued = queued.clone();
        let status = status.clone();
        tauri::async_runtime::spawn(async move {
            let success = process(&id, &db, &cache, &enricher).await;
            drop(permit);
            queued
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&id);
            let mut job = status
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            job.processed += 1;
            if !success {
                job.failed += 1;
            }
            if job.processed >= job.total {
                job.phase = "complete".into();
                job.message = if job.failed == 0 {
                    format!("Enriched {} posts", job.processed)
                } else {
                    format!("Finished with {} unavailable or failed posts", job.failed)
                };
            } else {
                job.message = format!("Enriching posts: {} / {}", job.processed, job.total);
            }
        });
    }
}

async fn process(id: &str, db: &Database, cache: &MediaCache, enricher: &FxTwitter) -> bool {
    const MAX_ATTEMPTS: u32 = 3;
    for attempt in 1..=MAX_ATTEMPTS {
        match enricher.enrich(id).await {
            Ok(mut tweet) => {
                cache.cache(&mut tweet).await;
                let save_db = db.clone();
                let save_result =
                    tokio::task::spawn_blocking(move || save_db.save_enriched(&tweet)).await;
                match save_result {
                    Ok(Ok(())) => return true,
                    Ok(Err(error)) => {
                        let failure_db = db.clone();
                        let failure_id = id.to_string();
                        let message = format!("Could not store enrichment result: {error}");
                        let _ = tokio::task::spawn_blocking(move || {
                            failure_db.mark_failure(&failure_id, "failed", &message, attempt)
                        })
                        .await;
                        return false;
                    }
                    Err(error) => {
                        let failure_db = db.clone();
                        let failure_id = id.to_string();
                        let message = format!("Enrichment storage task failed: {error}");
                        let _ = tokio::task::spawn_blocking(move || {
                            failure_db.mark_failure(&failure_id, "failed", &message, attempt)
                        })
                        .await;
                        return false;
                    }
                }
            }
            Err(error) => {
                if error.transient && attempt < MAX_ATTEMPTS {
                    let delay = error
                        .retry_after
                        .unwrap_or_else(|| Duration::from_secs(1_u64 << attempt));
                    tokio::time::sleep(delay.min(Duration::from_secs(60))).await;
                    continue;
                }
                let status = error.kind.status().to_string();
                let message = error.message;
                let id = id.to_string();
                let db = db.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    db.mark_failure(&id, &status, &message, attempt)
                })
                .await;
                return false;
            }
        }
    }
    false
}
