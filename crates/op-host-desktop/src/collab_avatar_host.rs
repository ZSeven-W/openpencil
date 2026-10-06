//! Bounded, SSRF-safe background fetcher for verified collaboration avatars.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;

use op_editor_ui::collab_avatar_runtime::{
    complete_collab_avatar_request, take_collab_avatar_requests, CollabAvatarFetchRequest,
};
use op_host_services::profile_avatar_fetch::{
    fetch_account_avatar_blocking, fetch_profile_avatar_blocking,
    ProfileAvatarFetchError as AvatarFetchError,
};

const MAX_CONCURRENT_FETCHES: usize = 3;

type Fetcher =
    Arc<dyn Fn(&CollabAvatarFetchRequest) -> Result<Vec<u8>, AvatarFetchError> + Send + Sync>;

struct FetchJob {
    request: CollabAvatarFetchRequest,
    rx: Receiver<Result<Vec<u8>, AvatarFetchError>>,
}

pub(crate) struct CollabAvatarHost {
    jobs: Vec<FetchJob>,
    fetcher: Fetcher,
}

#[cfg(test)]
static AVATAR_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[cfg(test)]
static AVATAR_TEST_OWNER: std::sync::Mutex<Option<std::thread::ThreadId>> =
    std::sync::Mutex::new(None);

/// Guard over the process-global avatar registry, reentrant per thread.
#[cfg(test)]
pub(crate) struct AvatarTestGuard {
    /// `None` when this thread already held the lock further up its stack.
    inner: Option<std::sync::MutexGuard<'static, ()>>,
}

#[cfg(test)]
impl Drop for AvatarTestGuard {
    fn drop(&mut self) {
        if self.inner.is_some() {
            // Cleared before the mutex is released, so the next owner never
            // sees this thread's id.
            *AVATAR_TEST_OWNER
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = None;
        }
    }
}

/// Serialize everything that touches the process-global avatar registry.
///
/// Rotating the avatar generation **evicts the previous epoch's cached
/// bytes** — verified directly: register an avatar, complete it, rotate, and
/// `cached_collab_avatar_bytes` goes from `Some` to `None`. That is why this
/// exists, and why the *writer* is guarded rather than only the tests that
/// call it: the rotation happens in production code, so no amount of
/// discipline at test call sites can cover it. Guarding the writer covers
/// every collab test there is and every one anybody writes later.
///
/// CAUTION — op-collab-host has its own equivalent guard inside
/// `advance_generation`, but a dependency's `#[cfg(test)]` code is compiled
/// only into THAT crate's own test build, never into this binary's. Any path
/// in THIS crate that drives a rotation (e.g. `CollabRuntime::leave` on a
/// fork-save acknowledgement, `save_session.rs`) must therefore take this
/// lock itself under `#[cfg(test)]` — otherwise a concurrently running test
/// that holds this lock (the image_decode_host avatar test) still sees its
/// cached bytes evicted mid-test, which is exactly the linux-aarch64 CI
/// failure this note comes from.
///
/// Reentrant per thread because several collab tests take this guard and then
/// drive a runtime that rotates — a plain `Mutex` would deadlock them. A
/// nested acquisition on the owning thread is a no-op guard; the outermost
/// one still holds the mutex for the whole test.
#[cfg(test)]
pub(crate) fn lock_avatar_test_registry() -> AvatarTestGuard {
    let current = std::thread::current().id();
    if *AVATAR_TEST_OWNER
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        == Some(current)
    {
        return AvatarTestGuard { inner: None };
    }
    let inner = AVATAR_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *AVATAR_TEST_OWNER
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(current);
    AvatarTestGuard { inner: Some(inner) }
}

impl CollabAvatarHost {
    pub(crate) fn new() -> Self {
        Self::with_fetcher(Arc::new(|request| {
            if request.is_current_account() {
                fetch_account_avatar_blocking(request.url())
            } else {
                fetch_profile_avatar_blocking(request.url())
            }
        }))
    }

    fn with_fetcher(fetcher: Fetcher) -> Self {
        Self {
            jobs: Vec::new(),
            fetcher,
        }
    }

    pub(crate) fn is_pending(&self) -> bool {
        !self.jobs.is_empty()
    }

    /// Drain results, then fill only the bounded free worker slots.
    pub(crate) fn pump(&mut self) -> bool {
        let mut changed = self.poll_jobs();
        let free = MAX_CONCURRENT_FETCHES.saturating_sub(self.jobs.len());
        for request in take_collab_avatar_requests(free) {
            match spawn_fetch(request.clone(), Arc::clone(&self.fetcher)) {
                Some(job) => self.jobs.push(job),
                None => {
                    let _ = complete_collab_avatar_request(&request, None);
                }
            }
            changed = true;
        }
        changed
    }

    fn poll_jobs(&mut self) -> bool {
        let mut changed = false;
        let mut index = 0;
        while index < self.jobs.len() {
            match self.jobs[index].rx.try_recv() {
                Ok(Ok(bytes)) => {
                    let job = self.jobs.swap_remove(index);
                    let _ = complete_collab_avatar_request(&job.request, Some(bytes));
                    changed = true;
                }
                Ok(Err(_)) | Err(TryRecvError::Disconnected) => {
                    let job = self.jobs.swap_remove(index);
                    let _ = complete_collab_avatar_request(&job.request, None);
                    changed = true;
                }
                Err(TryRecvError::Empty) => index += 1,
            }
        }
        changed
    }
}

fn spawn_fetch(request: CollabAvatarFetchRequest, fetcher: Fetcher) -> Option<FetchJob> {
    let (tx, rx) = mpsc::channel();
    let worker_request = request.clone();
    std::thread::Builder::new()
        .name("op-collab-avatar".into())
        .spawn(move || {
            let _ = tx.send(fetcher(&worker_request));
        })
        .ok()?;
    Some(FetchJob { request, rx })
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::collab_avatar_runtime::{
        cached_collab_avatar_bytes, collab_avatar_image, complete_collab_avatar_request,
        register_collab_avatar_url, take_collab_avatar_requests,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    fn png_header() -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[8..12].copy_from_slice(&13_u32.to_be_bytes());
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&16_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&16_u32.to_be_bytes());
        bytes
    }

    fn drain_stale_requests() {
        for request in take_collab_avatar_requests(usize::MAX) {
            let _ = complete_collab_avatar_request(&request, None);
        }
    }

    fn pump_until_idle(host: &mut CollabAvatarHost) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while host.is_pending() {
            host.pump();
            assert!(Instant::now() < deadline, "avatar worker never drained");
            std::thread::yield_now();
        }
    }

    #[test]
    fn scripted_success_and_failure_land_without_network() {
        let _guard = lock_avatar_test_registry();
        drain_stale_requests();
        let url = "https://cdn.example/avatar-host-success.png";
        assert!(register_collab_avatar_url("avatar-host-success", Some(url)));
        assert!(collab_avatar_image("avatar-host-success").is_none());
        let mut host = CollabAvatarHost::with_fetcher(Arc::new(|_| Ok(png_header())));
        assert!(host.pump());
        pump_until_idle(&mut host);
        let image = collab_avatar_image("avatar-host-success").expect("scripted image cached");
        assert!(cached_collab_avatar_bytes(image.image_id).is_some());

        let failed_url = "https://cdn.example/avatar-host-failure.png";
        assert!(register_collab_avatar_url(
            "avatar-host-failure",
            Some(failed_url)
        ));
        assert!(collab_avatar_image("avatar-host-failure").is_none());
        let mut failed =
            CollabAvatarHost::with_fetcher(Arc::new(|_| Err(AvatarFetchError::RequestFailed)));
        failed.pump();
        pump_until_idle(&mut failed);
        assert!(collab_avatar_image("avatar-host-failure").is_none());
        assert!(take_collab_avatar_requests(1).is_empty());
    }

    #[test]
    fn worker_concurrency_is_capped() {
        let _guard = lock_avatar_test_registry();
        drain_stale_requests();
        for index in 0..(MAX_CONCURRENT_FETCHES + 2) {
            let key = format!("avatar-concurrency-{index}");
            let url = format!("https://cdn.example/avatar-concurrency-{index}.png");
            assert!(register_collab_avatar_url(&key, Some(&url)));
            assert!(collab_avatar_image(&key).is_none());
        }
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let fetcher: Fetcher = {
            let active = Arc::clone(&active);
            let peak = Arc::clone(&peak);
            Arc::new(move |_| {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(10));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(png_header())
            })
        };
        let mut host = CollabAvatarHost::with_fetcher(fetcher);
        host.pump();
        assert!(host.jobs.len() <= MAX_CONCURRENT_FETCHES);
        let deadline = Instant::now() + Duration::from_secs(5);
        while host.is_pending()
            || op_editor_ui::collab_avatar_runtime::has_pending_collab_avatar_requests()
        {
            host.pump();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(peak.load(Ordering::SeqCst) <= MAX_CONCURRENT_FETCHES);
    }

    #[test]
    fn url_and_ssrf_guards_reject_unsafe_targets() {
        for url in [
            "http://cdn.example/avatar.png",
            "https://user@cdn.example/avatar.png",
            "https://cdn.example/avatar.png#fragment",
            "https://cdn.example:0/avatar.png",
        ] {
            assert_eq!(
                fetch_profile_avatar_blocking(url),
                Err(AvatarFetchError::UrlNotAllowed)
            );
        }
        for url in [
            "https://127.0.0.1/avatar.png",
            "https://10.0.0.1/avatar.png",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/avatar.png",
            "https://[fe80::1]/avatar.png",
        ] {
            assert_eq!(
                fetch_profile_avatar_blocking(url),
                Err(AvatarFetchError::DialRejected),
                "{url}"
            );
        }
    }
}
