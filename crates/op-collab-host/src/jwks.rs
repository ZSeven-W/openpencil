//! Native HTTPS adapter for the public collaboration-ticket verifier.
//!
//! The trust root and endpoint are pinned by `op-auth-bridge`. This adapter
//! deliberately accepts no caller-provided credentials, cookies, redirects,
//! or response text in its error surface.

use std::future::Future;
use std::time::Duration;

use op_auth_bridge::{
    CollabJwksFetchError, CollabJwksFetchRequest, CollabJwksFetchResponse, CollabJwksFetcher,
};
use reqwest::header::{
    HeaderMap, ACCEPT, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH,
};
use reqwest::{redirect, Client, Response, StatusCode};

const JWKS_HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(5);
const DEFAULT_JWKS_MAX_AGE_SECONDS: u64 = 60;
const MAX_REPORTED_JWKS_MAX_AGE_SECONDS: u64 = 60 * 60;

/// HTTPS-only fetcher used by `CollabTicketVerifier::production`.
#[derive(Clone, Debug)]
pub(crate) struct NativeCollabJwksFetcher {
    client: Client,
}

impl NativeCollabJwksFetcher {
    pub(crate) fn new() -> Result<Self, CollabJwksFetchError> {
        let client = Client::builder()
            .use_rustls_tls()
            .redirect(redirect::Policy::none())
            .connect_timeout(JWKS_HTTP_TIMEOUT)
            .timeout(JWKS_HTTP_TIMEOUT)
            .user_agent(concat!("OpenPencil/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| CollabJwksFetchError::Unavailable)?;
        Ok(Self { client })
    }
}

impl CollabJwksFetcher for NativeCollabJwksFetcher {
    fn fetch(
        &self,
        request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        self.fetch_cancellable(request, &never_cancelled)
    }

    fn fetch_cancellable(
        &self,
        request: CollabJwksFetchRequest<'_>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        crate::blocking::block_on(self.fetch_async(request, cancelled))
    }
}

impl NativeCollabJwksFetcher {
    async fn fetch_async(
        &self,
        request: CollabJwksFetchRequest<'_>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        // The verifier already validates and pins this endpoint. Keep a local
        // scheme check so future callers cannot accidentally weaken the native
        // transport boundary.
        if !request.endpoint.starts_with("https://") || request.maximum_body_bytes == 0 {
            return Err(CollabJwksFetchError::RejectedResponse);
        }
        ensure_transport_active(cancelled)?;

        let mut fetch = self
            .client
            .get(request.endpoint)
            .header(ACCEPT, "application/json");
        if let Some(etag) = request.etag {
            fetch = fetch.header(IF_NONE_MATCH, etag);
        }
        let response = await_or_cancel(fetch.send(), cancelled)
            .await?
            .map_err(|_| CollabJwksFetchError::Unavailable)?;
        decode_response(response, request.maximum_body_bytes, cancelled).await
    }
}

async fn decode_response(
    mut response: Response,
    maximum_body_bytes: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
    ensure_transport_active(cancelled)?;
    let status = response.status();
    let headers = response.headers();
    let etag = response_etag(headers)?;
    let max_age_seconds = response_max_age(headers);

    if status == StatusCode::NOT_MODIFIED {
        ensure_transport_active(cancelled)?;
        return Ok(CollabJwksFetchResponse::NotModified {
            etag,
            max_age_seconds,
        });
    }
    if status != StatusCode::OK || !content_type_is_json(headers) {
        return Err(CollabJwksFetchError::RejectedResponse);
    }
    if declared_too_large(headers, maximum_body_bytes) {
        return Err(CollabJwksFetchError::ResponseTooLarge);
    }

    let mut body = Vec::with_capacity(maximum_body_bytes.min(16 * 1024));
    while let Some(chunk) = await_or_cancel(response.chunk(), cancelled)
        .await?
        .map_err(|_| CollabJwksFetchError::Unavailable)?
    {
        let remaining = maximum_body_bytes
            .checked_sub(body.len())
            .ok_or(CollabJwksFetchError::ResponseTooLarge)?;
        if chunk.len() > remaining {
            return Err(CollabJwksFetchError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    ensure_transport_active(cancelled)?;
    if body.is_empty() {
        return Err(CollabJwksFetchError::RejectedResponse);
    }
    Ok(CollabJwksFetchResponse::Modified {
        body,
        etag,
        max_age_seconds,
    })
}

pub(crate) async fn await_or_cancel<T>(
    future: impl Future<Output = T>,
    cancelled: &dyn Fn() -> bool,
) -> Result<T, CollabJwksFetchError> {
    ensure_transport_active(cancelled)?;
    tokio::pin!(future);
    tokio::select! {
        biased;
        () = wait_for_cancellation(cancelled) => Err(CollabJwksFetchError::Cancelled),
        output = &mut future => {
            ensure_transport_active(cancelled)?;
            Ok(output)
        }
    }
}

async fn wait_for_cancellation(cancelled: &dyn Fn() -> bool) {
    while !cancelled() {
        tokio::time::sleep(CANCELLATION_POLL_INTERVAL).await;
    }
}

fn never_cancelled() -> bool {
    false
}

fn ensure_transport_active(cancelled: &dyn Fn() -> bool) -> Result<(), CollabJwksFetchError> {
    if cancelled() {
        Err(CollabJwksFetchError::Cancelled)
    } else {
        Ok(())
    }
}

fn response_etag(headers: &HeaderMap) -> Result<Option<String>, CollabJwksFetchError> {
    headers
        .get(ETAG)
        .map(|value| {
            value
                .to_str()
                .map(str::to_owned)
                .map_err(|_| CollabJwksFetchError::RejectedResponse)
        })
        .transpose()
}

fn content_type_is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("application/json"))
}

fn declared_too_large(headers: &HeaderMap, maximum_body_bytes: usize) -> bool {
    headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > maximum_body_bytes as u64)
}

fn response_max_age(headers: &HeaderMap) -> u64 {
    let directives: Vec<_> = headers
        .get_all(CACHE_CONTROL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .collect();
    if directives.iter().any(|directive| {
        directive.eq_ignore_ascii_case("no-store") || directive.eq_ignore_ascii_case("no-cache")
    }) {
        return 0;
    }
    directives
        .into_iter()
        .find_map(parse_max_age_directive)
        .unwrap_or(DEFAULT_JWKS_MAX_AGE_SECONDS)
        .min(MAX_REPORTED_JWKS_MAX_AGE_SECONDS)
}

fn parse_max_age_directive(directive: &str) -> Option<u64> {
    let (name, value) = directive.trim().split_once('=')?;
    if !name.trim().eq_ignore_ascii_case("max-age") {
        return None;
    }
    value.trim().trim_matches('"').parse().ok()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};
    use std::time::Instant;

    use super::*;
    use reqwest::header::HeaderValue;

    struct DropMarker(Arc<AtomicBool>);

    impl Drop for DropMarker {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }

    #[test]
    fn cache_age_parser_is_bounded_and_ignores_unrelated_directives() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=120, immutable"),
        );
        assert_eq!(response_max_age(&headers), 120);

        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=999999999"));
        assert_eq!(
            response_max_age(&headers),
            MAX_REPORTED_JWKS_MAX_AGE_SECONDS
        );

        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
        assert_eq!(response_max_age(&headers), 0);

        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static("max-age=120, no-cache"),
        );
        assert_eq!(response_max_age(&headers), 0);
    }

    #[test]
    fn content_type_requires_json_and_length_is_checked_before_reading() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        headers.insert(CONTENT_LENGTH, HeaderValue::from_static("1025"));
        assert!(content_type_is_json(&headers));
        assert!(declared_too_large(&headers, 1024));

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
        assert!(!content_type_is_json(&headers));
    }

    #[test]
    fn invalid_etag_is_rejected_without_echoing_header_contents() {
        let mut headers = HeaderMap::new();
        headers.insert(ETAG, HeaderValue::from_bytes(b"\x80").unwrap());
        assert_eq!(
            response_etag(&headers),
            Err(CollabJwksFetchError::RejectedResponse)
        );
    }

    #[test]
    fn cancellation_drops_an_in_flight_fetch_future_without_detaching() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let dropped = Arc::new(AtomicBool::new(false));
        let worker_dropped = Arc::clone(&dropped);
        let (started_sender, started_receiver) = mpsc::sync_channel(1);
        let (done_sender, done_receiver) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let pending_fetch = async move {
                let _drop_marker = DropMarker(worker_dropped);
                started_sender.send(()).unwrap();
                std::future::pending::<()>().await;
            };
            let result = crate::blocking::block_on(await_or_cancel(pending_fetch, &|| {
                worker_cancelled.load(Ordering::Acquire)
            }));
            done_sender.send(result).unwrap();
        });

        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("fetch future must start");
        let started_cancelling = Instant::now();
        cancelled.store(true, Ordering::Release);
        assert_eq!(
            done_receiver.recv_timeout(Duration::from_millis(500)),
            Ok(Err(CollabJwksFetchError::Cancelled))
        );
        worker.join().unwrap();
        assert!(started_cancelling.elapsed() < Duration::from_secs(1));
        assert!(dropped.load(Ordering::Acquire));
    }

    #[test]
    fn cancellation_suppresses_a_result_that_becomes_ready_late() {
        let cancelled = AtomicBool::new(false);
        let produced = AtomicUsize::new(0);
        let result = crate::blocking::block_on(await_or_cancel(
            async {
                produced.fetch_add(1, Ordering::SeqCst);
                cancelled.store(true, Ordering::Release);
                42
            },
            &|| cancelled.load(Ordering::Acquire),
        ));

        assert_eq!(produced.load(Ordering::SeqCst), 1);
        assert_eq!(result, Err(CollabJwksFetchError::Cancelled));
    }
}
