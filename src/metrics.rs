use std::{
    sync::Arc,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use once_cell::sync::Lazy;

use crate::audio;

pub static METRICS: Lazy<Arc<Metrics>> = Lazy::new(|| Arc::new(Metrics::new()));

#[derive(Debug)]
pub struct Metrics {
    start: Instant,
    ready: AtomicBool,
    active_voice_calls: AtomicUsize,
    connected_guilds: AtomicUsize,
    total_queue_len: AtomicUsize,
    downloads_bytes: AtomicU64,
    downloads_files: AtomicU64,
}

impl Metrics {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            ready: AtomicBool::new(false),
            active_voice_calls: AtomicUsize::new(0),
            connected_guilds: AtomicUsize::new(0),
            total_queue_len: AtomicUsize::new(0),
            downloads_bytes: AtomicU64::new(0),
            downloads_files: AtomicU64::new(0),
        }
    }

    pub fn set_ready(&self, v: bool) {
        self.ready.store(v, Ordering::Relaxed);
    }
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn inc_connections(&self) {
        self.active_voice_calls.fetch_add(1, Ordering::Relaxed);
        self.connected_guilds.fetch_add(1, Ordering::Relaxed);
    }
    pub fn dec_connections(&self) {
        let _ = self
            .active_voice_calls
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                Some(x.saturating_sub(1))
            });
        let _ = self
            .connected_guilds
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                Some(x.saturating_sub(1))
            });
    }

    pub fn inc_queue(&self, n: usize) {
        self.total_queue_len.fetch_add(n, Ordering::Relaxed);
    }
    pub fn dec_queue(&self, n: usize) {
        let _ = self
            .total_queue_len
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                Some(x.saturating_sub(n))
            });
    }

    pub fn set_downloads(&self, files: u64, bytes: u64) {
        self.downloads_files.store(files, Ordering::Relaxed);
        self.downloads_bytes.store(bytes, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            uptime_secs: self.start.elapsed().as_secs(),
            ready: self.is_ready(),
            active_voice_calls: self.active_voice_calls.load(Ordering::Relaxed),
            connected_guilds: self.connected_guilds.load(Ordering::Relaxed),
            total_queue_len: self.total_queue_len.load(Ordering::Relaxed),
            downloads_bytes: self.downloads_bytes.load(Ordering::Relaxed),
            downloads_files: self.downloads_files.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub uptime_secs: u64,
    pub ready: bool,
    pub active_voice_calls: usize,
    pub connected_guilds: usize,
    pub total_queue_len: usize,
    pub downloads_bytes: u64,
    pub downloads_files: u64,
}

pub fn spawn_download_size_scanner() {
    // Periodically scan DOWNLOAD_FOLDER or cache fallback for file count and total size.
    tokio::spawn(async {
        loop {
            let mut files: u64 = 0;
            let mut bytes: u64 = 0;
            if let Ok(root) = audio::resolved_download_base_dir() {
                // Iterative DFS to avoid recursive async
                let mut stack = vec![root];
                while let Some(dir) = stack.pop() {
                    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
                        while let Ok(Some(ent)) = rd.next_entry().await {
                            match ent.file_type().await {
                                Ok(ft) if ft.is_file() => {
                                    files += 1;
                                    if let Ok(meta) = ent.metadata().await {
                                        bytes = bytes.saturating_add(meta.len());
                                    }
                                }
                                Ok(ft) if ft.is_dir() => {
                                    stack.push(ent.path());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            METRICS.set_downloads(files, bytes);
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}
