//! Bounded, SSRF-safe background fetcher for verified collaboration avatars.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use op_editor_ui::collab_avatar_runtime::{
    complete_collab_avatar_request, take_collab_avatar_requests, CollabAvatarFetchRequest,
    MAX_AVATAR_ENCODED_BYTES,
};
use reqwest::header::{ACCEPT, LOCATION};

const MAX_CONCURRENT_FETCHES: usize = 3;
const MAX_REDIRECTS: usize = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AvatarFetchError {
    UrlNotAllowed,
    DialRejected,
    RequestFailed,
    TimedOut,
    HttpStatus,
    RedirectInvalid,
    TooManyRedirects,
    TooLarge,
    EmptyBody,
}

type Fetcher = Arc<dyn Fn(&str) -> Result<Vec<u8>, AvatarFetchError> + Send + Sync>;

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

/// Serialize desktop tests that rotate the process-global avatar generation.
#[cfg(test)]
pub(crate) fn lock_avatar_test_registry() -> std::sync::MutexGuard<'static, ()> {
    AVATAR_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

impl CollabAvatarHost {
    pub(crate) fn new() -> Self {
        Self::with_fetcher(Arc::new(fetch_avatar_blocking))
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
    let url = request.url().to_string();
    std::thread::Builder::new()
        .name("op-collab-avatar".into())
        .spawn(move || {
            let _ = tx.send(fetcher(&url));
        })
        .ok()?;
    Some(FetchJob { request, rx })
}

fn fetch_avatar_blocking(url: &str) -> Result<Vec<u8>, AvatarFetchError> {
    op_host_services::chat_runtime::block_on_anywhere(fetch_avatar(url))
}

async fn fetch_avatar(url: &str) -> Result<Vec<u8>, AvatarFetchError> {
    let mut url = parse_avatar_url(url)?;
    for redirect_count in 0..=MAX_REDIRECTS {
        // `public_https_client` resolves + screens every address, disables
        // proxies, and pins the socket while the request URL retains the
        // original hostname for TLS certificate verification and SNI.
        let client = with_timeout(
            REQUEST_TIMEOUT,
            op_host_services::public_https_client::public_https_client(&url),
        )
        .await?
        .map_err(|_| AvatarFetchError::DialRejected)?;
        let mut response = client
            .get(url.clone())
            .header(ACCEPT, "image/webp,image/png,image/jpeg,image/gif")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|_| AvatarFetchError::RequestFailed)?;

        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                return Err(AvatarFetchError::TooManyRedirects);
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(AvatarFetchError::RedirectInvalid)?;
            url = parse_avatar_url(
                url.join(location)
                    .map_err(|_| AvatarFetchError::RedirectInvalid)?
                    .as_str(),
            )?;
            continue;
        }
        if !response.status().is_success() {
            return Err(AvatarFetchError::HttpStatus);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_AVATAR_ENCODED_BYTES as u64)
        {
            return Err(AvatarFetchError::TooLarge);
        }
        let mut bytes = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(0)
                .min(MAX_AVATAR_ENCODED_BYTES as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AvatarFetchError::RequestFailed)?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_AVATAR_ENCODED_BYTES {
                return Err(AvatarFetchError::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(AvatarFetchError::EmptyBody);
        }
        return Ok(bytes);
    }
    Err(AvatarFetchError::TooManyRedirects)
}

async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, AvatarFetchError>
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| AvatarFetchError::TimedOut)
}

fn parse_avatar_url(value: &str) -> Result<reqwest::Url, AvatarFetchError> {
    let url = reqwest::Url::parse(value).map_err(|_| AvatarFetchError::UrlNotAllowed)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port() == Some(0)
    {
        return Err(AvatarFetchError::UrlNotAllowed);
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::collab_avatar_runtime::{
        cached_collab_avatar_bytes, collab_avatar_image, complete_collab_avatar_request,
        register_collab_avatar_url, take_collab_avatar_requests,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    fn png_header() -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
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
            assert_eq!(parse_avatar_url(url), Err(AvatarFetchError::UrlNotAllowed));
        }
        for url in [
            "https://127.0.0.1/avatar.png",
            "https://10.0.0.1/avatar.png",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/avatar.png",
            "https://[fe80::1]/avatar.png",
        ] {
            assert_eq!(
                fetch_avatar_blocking(url),
                Err(AvatarFetchError::DialRejected),
                "{url}"
            );
        }
    }

    #[test]
    fn resolver_deadline_cancels_a_hanging_dial_future() {
        let result = op_host_services::chat_runtime::block_on_anywhere(with_timeout(
            Duration::from_millis(1),
            std::future::pending::<()>(),
        ));
        assert_eq!(result, Err(AvatarFetchError::TimedOut));
    }
}
