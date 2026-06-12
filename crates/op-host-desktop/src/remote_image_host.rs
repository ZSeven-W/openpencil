//! Background fetcher for remote (`http(s)`) image sources.
//!
//! The platform-free canvas painter
//! (`op_editor_ui::widgets::canvas_viewport_image`) can only RECORD a
//! cache miss for a remote `src`; this desktop-side session drains the
//! recorded misses each frame, fetches the bytes on worker threads
//! (same single-threaded-tokio + reqwest pattern as
//! `update_check.rs` / `iconify_host.rs`), and stores the result back
//! into the painter's shared byte cache — the next paint draws the
//! bitmap. Failures land in the painter's negative cache so a dead
//! URL keeps the dashed placeholder without refetching every frame.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use op_editor_ui::widgets::canvas_viewport_image::{
    mark_remote_image_failed, store_remote_image_bytes, take_remote_image_requests,
};

/// Concurrent fetch cap — a document with dozens of remote images
/// trickles through instead of opening a connection storm.
const MAX_CONCURRENT_FETCHES: usize = 4;
/// Largest remote image accepted (encoded bytes). The painter's byte
/// cache is bounded by entry COUNT, so the per-entry cap bounds the
/// worst-case memory.
const MAX_REMOTE_IMAGE_BYTES: usize = 8 * 1024 * 1024;

type Fetcher = Arc<dyn Fn(&str) -> Result<Vec<u8>, String> + Send + Sync>;

struct FetchJob {
    id: u64,
    rx: Receiver<Result<Vec<u8>, String>>,
}

pub struct RemoteImageSession {
    jobs: Vec<FetchJob>,
    fetcher: Fetcher,
}

impl RemoteImageSession {
    pub fn new() -> Self {
        Self::with_fetcher(Arc::new(fetch_remote_image_blocking))
    }

    /// Test seam — swap the network fetch for a scripted closure.
    fn with_fetcher(fetcher: Fetcher) -> Self {
        Self {
            jobs: Vec::new(),
            fetcher,
        }
    }

    /// Whether fetch workers are still in flight — keeps the event
    /// loop waking so results are drained while the app idles.
    pub fn is_pending(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Per-frame pump: drain landed results into the painter's caches,
    /// then take queued misses up to the concurrency cap and spawn
    /// fetch workers. Returns whether anything changed that warrants
    /// a repaint (stored bytes — a failure only freezes the already
    /// painted placeholder, but a repaint is harmless and keeps the
    /// logic simple).
    pub fn pump(&mut self) -> bool {
        let mut changed = self.poll_jobs();
        let free = MAX_CONCURRENT_FETCHES.saturating_sub(self.jobs.len());
        if free > 0 {
            for (id, url) in take_remote_image_requests(free) {
                self.jobs.push(spawn_fetch(id, url, self.fetcher.clone()));
                changed = true;
            }
        }
        changed
    }

    fn poll_jobs(&mut self) -> bool {
        let mut changed = false;
        let mut i = 0;
        while i < self.jobs.len() {
            match self.jobs[i].rx.try_recv() {
                Ok(Ok(bytes)) => {
                    store_remote_image_bytes(self.jobs[i].id, bytes);
                    self.jobs.swap_remove(i);
                    changed = true;
                }
                Ok(Err(_)) => {
                    mark_remote_image_failed(self.jobs[i].id);
                    self.jobs.swap_remove(i);
                    changed = true;
                }
                Err(TryRecvError::Empty) => i += 1,
                Err(TryRecvError::Disconnected) => {
                    // Worker died without a result — negative-cache so
                    // the id doesn't refetch every frame.
                    mark_remote_image_failed(self.jobs[i].id);
                    self.jobs.swap_remove(i);
                    changed = true;
                }
            }
        }
        changed
    }
}

fn spawn_fetch(id: u64, url: String, fetcher: Fetcher) -> FetchJob {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(fetcher(&url));
    });
    FetchJob { id, rx }
}

/// Blocking HTTPS fetch — only ever runs on a worker thread. `reqwest`
/// is async-only in this workspace, so the worker spins a
/// single-threaded tokio runtime for the one call (`update_check.rs`
/// precedent). Validates the response is a plausible image (content
/// type or magic bytes) and bounded in size.
fn fetch_remote_image_blocking(url: &str) -> Result<Vec<u8>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("openpencil-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        if resp
            .content_length()
            .is_some_and(|len| len > MAX_REMOTE_IMAGE_BYTES as u64)
        {
            return Err("image exceeds the size cap".to_string());
        }
        let content_type_is_image = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim().to_ascii_lowercase().starts_with("image/"))
            .unwrap_or(false);
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        if bytes.is_empty() {
            return Err("empty response body".to_string());
        }
        if bytes.len() > MAX_REMOTE_IMAGE_BYTES {
            return Err("image exceeds the size cap".to_string());
        }
        if !content_type_is_image && !looks_like_image(&bytes) {
            return Err("response is not an image".to_string());
        }
        Ok(bytes.to_vec())
    })
}

/// Magic-byte sniff for the codecs skia decodes (PNG / JPEG / GIF /
/// WebP / BMP) — used when the server omits an `image/*` content type.
fn looks_like_image(bytes: &[u8]) -> bool {
    bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP")
        || bytes.starts_with(b"BM")
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::widgets::canvas_viewport_image::{
        has_cached_image_bytes, has_pending_remote_image_requests, note_remote_image_miss,
        take_remote_image_requests,
    };
    use std::sync::Mutex;

    /// The miss queue is a process-wide static shared with any other
    /// test in this binary — serialize the tests that touch it and
    /// start each from a drained queue.
    static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

    fn lock_registry() -> std::sync::MutexGuard<'static, ()> {
        let guard = REGISTRY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _ = take_remote_image_requests(usize::MAX);
        guard
    }

    /// Pump until the in-flight jobs land (worker threads run the
    /// scripted fetcher; results arrive via the channel).
    fn pump_until_idle(session: &mut RemoteImageSession) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while session.is_pending() {
            session.pump();
            assert!(
                std::time::Instant::now() < deadline,
                "fetch jobs never drained"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn scripted_fetch_stores_bytes_into_the_paint_cache() {
        let _guard = lock_registry();
        let id = 0xFEED_0001_u64;
        note_remote_image_miss(id, "https://example.com/ok.png");

        let mut session = RemoteImageSession::with_fetcher(Arc::new(|url: &str| {
            assert_eq!(url, "https://example.com/ok.png");
            Ok(b"BYTES".to_vec())
        }));
        assert!(session.pump(), "pump takes the queued miss");
        pump_until_idle(&mut session);

        assert!(
            has_cached_image_bytes(id),
            "stored bytes must land in the shared paint cache"
        );
        assert!(!has_pending_remote_image_requests());
    }

    #[test]
    fn failed_fetch_marks_negative_cache_and_never_requeues() {
        let _guard = lock_registry();
        let id = 0xFEED_0002_u64;
        let url = "https://example.com/missing.png";
        note_remote_image_miss(id, url);

        let mut session =
            RemoteImageSession::with_fetcher(Arc::new(|_: &str| Err("404".to_string())));
        session.pump();
        pump_until_idle(&mut session);
        assert!(!has_cached_image_bytes(id));

        // Negative-cached: a repeated paint-time miss is refused, so
        // the host never sees the id again.
        note_remote_image_miss(id, url);
        assert!(!has_pending_remote_image_requests());
        session.pump();
        assert!(!session.is_pending(), "failed id must not refetch");
    }

    #[test]
    fn concurrency_cap_limits_in_flight_jobs() {
        let _guard = lock_registry();
        // Queue more misses than the cap; one pump may take at most
        // MAX_CONCURRENT_FETCHES jobs.
        let base = 0xFEED_0100_u64;
        for i in 0..(MAX_CONCURRENT_FETCHES as u64 + 3) {
            note_remote_image_miss(base + i, "https://example.com/n.png");
        }
        let mut session = RemoteImageSession::with_fetcher(Arc::new(|_: &str| {
            std::thread::sleep(Duration::from_millis(20));
            Ok(b"X".to_vec())
        }));
        session.pump();
        assert!(session.jobs.len() <= MAX_CONCURRENT_FETCHES);
        // Eventually everything drains (later pumps pick up the rest).
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while session.is_pending() || has_pending_remote_image_requests() {
            session.pump();
            if std::time::Instant::now() >= deadline {
                panic!("queued fetches never drained");
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        for i in 0..(MAX_CONCURRENT_FETCHES as u64 + 3) {
            assert!(has_cached_image_bytes(base + i), "id {i} missing");
        }
    }

    #[test]
    fn image_sniff_accepts_known_codecs_only() {
        assert!(looks_like_image(b"\x89PNG\r\n\x1a\nrest"));
        assert!(looks_like_image(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(looks_like_image(b"GIF89a..."));
        assert!(looks_like_image(b"RIFF\x00\x00\x00\x00WEBPVP8 "));
        assert!(!looks_like_image(b"<html><body>nope</body></html>"));
        assert!(!looks_like_image(b""));
    }
}
