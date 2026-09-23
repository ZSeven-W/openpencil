//! Workbench (self-hosted) image generation: submit → poll → fetch.
//!
//! The Workbench backend is not OpenAI-compatible — a generation is a JOB:
//! `POST /workbench-api/qwen-image` (unique `Idempotency-Key`) returns a job
//! id, `GET /workbench-api/qwen-image/:id` reports
//! `queued|running|succeeded|failed|canceled`, and a succeeded job carries
//! `artifacts[0].url` (`/media/<token>`, relative to the same base). Auth is
//! `Authorization: Bearer <key>` everywhere. The single GPU is shared with
//! the LLM, so a busy machine answers `{"error":"qwen_mode_active"}` (or a
//! plain 409) — mapped to [`ImageGenerateError::Busy`] so callers can treat
//! it as a transient, degradable failure rather than a config fault.

use std::time::Duration;

use op_editor_core::agent_settings::{ImageGenProfile, ImageTestStatus};

use crate::web_image_generate::{read_provider_body, ImageGenerateError};

const PROVIDER: &str = "Workbench";
/// Diffusion steps requested per job. The machine clamps to its own model
/// default when the value is absent, so a mid-range explicit value keeps
/// latency predictable without pretending to tune quality.
const DEFAULT_STEPS: i64 = 24;
/// Default wall-clock budget for one submit+poll cycle.
const DEFAULT_TIMEOUT_SECS: u64 = 180;

/// Polling cadence + total budget for one Workbench job. The defaults ride
/// the spec's 2 s interval and a 180 s timeout; `OPENPENCIL_WORKBENCH_TIMEOUT_SECS`
/// overrides the budget for slow machines, and tests inject both.
pub struct WorkbenchPolling {
    pub interval: Duration,
    pub timeout: Duration,
}

impl Default for WorkbenchPolling {
    fn default() -> Self {
        let timeout = std::env::var("OPENPENCIL_WORKBENCH_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|secs| *secs > 0)
            .map(Duration::from_secs)
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        Self {
            interval: Duration::from_secs(2),
            timeout,
        }
    }
}

/// Workbench jobs in flight per process. The machine runs one diffusion job
/// at a time on its single GPU, so more concurrency only adds poll traffic:
/// it rate-limits per key (measured `RateLimit-Policy: 240;w=60`), and an
/// enrichment pass with eleven slots polling every 2 s blew through that and
/// dropped every slot to stock search. Two keeps the GPU fed back-to-back.
const MAX_CONCURRENT_JOBS: usize = 2;

fn workbench_permits() -> &'static tokio::sync::Semaphore {
    static PERMITS: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
    PERMITS.get_or_init(|| tokio::sync::Semaphore::new(MAX_CONCURRENT_JOBS))
}

/// Attempts for a transient transport failure on a job that is already
/// accepted — consecutive dropped polls, or the artifact download.
const TRANSIENT_ATTEMPTS: u32 = 3;

/// Longest single wait between rate-limited submissions.
const MAX_RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(20);

/// `Retry-After` in whole seconds, when the machine sends one.
fn retry_after(resp: &reqwest::Response) -> Option<Duration> {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(|secs| Duration::from_secs(secs.min(MAX_RATE_LIMIT_BACKOFF.as_secs())))
}

/// Mint a fresh idempotency key: 16 random bytes, hex-encoded. Every
/// submission must carry a unique key (the machine rejects replays), and the
/// key is derived from the OS RNG rather than anything user-identifiable.
fn new_idempotency_key() -> String {
    let mut bytes = [0u8; 16];
    // The key is a request nonce, not a credential — an RNG failure has no
    // honest fallback, so refuse to submit rather than reuse a key.
    getrandom::fill(&mut bytes)
        .map(|()| bytes.iter().map(|byte| format!("{byte:02x}")).collect())
        .unwrap_or_default()
}

/// Submit one t2i job and poll it to a terminal state. Returns the FIRST
/// artifact's ABSOLUTE url (the API reports `/media/<token>` relative to the
/// configured base) so the caller downloads it through the same path as any
/// other provider's remote result.
pub async fn generate_workbench(
    client: &reqwest::Client,
    prompt: &str,
    profile: &ImageGenProfile,
    width: Option<f64>,
    height: Option<f64>,
    polling: WorkbenchPolling,
) -> Result<String, ImageGenerateError> {
    // Queue BEFORE the job's budget starts: waiting for a permit is not the
    // machine being slow, and must not eat the submit+poll timeout.
    let _permit = workbench_permits()
        .acquire()
        .await
        .map_err(|_| ImageGenerateError::Busy { provider: PROVIDER })?;
    let base = profile
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|base| !base.is_empty())
        .ok_or(ImageGenerateError::MissingBaseUrl)?
        .trim_end_matches('/')
        .to_string();
    let size = workbench_size(width, height);
    let body = serde_json::json!({
        "mode": "t2i",
        "prompt": prompt,
        "width": size.0,
        "height": size.1,
        "outputSizing": "explicit",
        "steps": DEFAULT_STEPS,
        "transparent": false,
        "model": profile.model,
    });
    // One budget covers the whole cycle: rate-limit waits spend it too, so a
    // throttled slot still degrades to search on time instead of hanging.
    let deadline = tokio::time::Instant::now() + polling.timeout;
    let mut backoff = polling.interval.max(Duration::from_millis(1));
    let (status, body) = loop {
        let resp = client
            .post(format!("{base}/workbench-api/qwen-image"))
            .bearer_auth(profile.api_key.trim())
            // Fresh per attempt: a throttled submission was never accepted,
            // and the machine rejects a replayed key.
            .header("Idempotency-Key", new_idempotency_key())
            .json(&body)
            .send()
            .await
            .map_err(|e| ImageGenerateError::Request {
                provider: PROVIDER,
                message: e.to_string(),
            })?;
        let retry_after = retry_after(&resp);
        let (status, text) = read_provider_body(PROVIDER, resp).await?;
        // The machine rate-limits submissions per key. An enrichment pass
        // fans out one job per image slot, so the burst is expected: wait it
        // out (honouring `Retry-After`) rather than failing every slot after
        // the first few straight to stock search.
        if status.as_u16() == 429 || text.contains("rate_limited") {
            let wait = retry_after.unwrap_or(backoff);
            if tokio::time::Instant::now() + wait >= deadline {
                return Err(ImageGenerateError::Busy { provider: PROVIDER });
            }
            tokio::time::sleep(wait).await;
            backoff = (backoff * 2).min(MAX_RATE_LIMIT_BACKOFF);
            continue;
        }
        break (status, text);
    };
    if body.contains("qwen_mode_active") {
        return Err(ImageGenerateError::Busy { provider: PROVIDER });
    }
    if status.as_u16() == 409 {
        return Err(ImageGenerateError::Busy { provider: PROVIDER });
    }
    if !status.is_success() {
        return Err(ImageGenerateError::Provider(
            crate::web_image_generate::provider_error(PROVIDER, status, &body),
        ));
    }
    let submitted: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| ImageGenerateError::ResponseParse {
            provider: PROVIDER,
            message: e.to_string(),
        })?;
    let id = job_object(&submitted)
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or(ImageGenerateError::MissingPredictionId { provider: PROVIDER })?
        .to_string();

    let mut poll_failures = 0;
    loop {
        tokio::time::sleep(polling.interval).await;
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Err(ImageGenerateError::Timeout { provider: PROVIDER });
        }
        let poll_timeout = Duration::from_secs(15).min(deadline - now);
        let sent = client
            .get(format!("{base}/workbench-api/qwen-image/{id}"))
            .bearer_auth(profile.api_key.trim())
            .timeout(poll_timeout)
            .send()
            .await;
        // A dropped poll (connection reset, TLS eof on a flaky tailnet) says
        // nothing about the job, which keeps running on the machine. Tolerate
        // a short run of them before calling the transport broken.
        let sent = match sent {
            Err(e)
                if !e.is_timeout()
                    && poll_failures + 1 < TRANSIENT_ATTEMPTS
                    && tokio::time::Instant::now() < deadline =>
            {
                poll_failures += 1;
                continue;
            }
            other => other,
        };
        let resp = sent.map_err(|e| {
            // The per-poll timeout is clipped to the remaining budget, so
            // a request cut off by it IS the overall budget running out —
            // report that, not a transport fault (both degrade, but the
            // reason in the fallback log line should be the true one).
            if e.is_timeout() && tokio::time::Instant::now() + Duration::from_millis(5) >= deadline
            {
                ImageGenerateError::Timeout { provider: PROVIDER }
            } else {
                ImageGenerateError::PollRequest {
                    provider: PROVIDER,
                    message: e.to_string(),
                }
            }
        })?;
        poll_failures = 0;
        let poll_retry_after = retry_after(&resp);
        let (status, body) = read_provider_body(PROVIDER, resp).await?;
        // A throttled poll says nothing about the job — it is still running
        // on the machine. Wait out the window and poll again.
        if status.as_u16() == 429 || body.contains("rate_limited") {
            let wait = poll_retry_after.unwrap_or(polling.interval * 2);
            if tokio::time::Instant::now() + wait >= deadline {
                return Err(ImageGenerateError::Timeout { provider: PROVIDER });
            }
            tokio::time::sleep(wait).await;
            continue;
        }
        if !status.is_success() {
            return Err(ImageGenerateError::PollStatus {
                provider: PROVIDER,
                status: status.as_u16(),
                body: body.chars().take(200).collect::<String>(),
            });
        }
        let polled: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| ImageGenerateError::PollParse {
                provider: PROVIDER,
                message: e.to_string(),
            })?;
        let job = job_object(&polled);
        match job.get("status").and_then(serde_json::Value::as_str) {
            Some("succeeded") => {
                let url = job
                    .get("artifacts")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|artifacts| artifacts.first())
                    .and_then(|artifact| artifact.get("url"))
                    .and_then(serde_json::Value::as_str)
                    .ok_or(ImageGenerateError::OutputMissing { provider: PROVIDER })?;
                let absolute = if url.starts_with("http://") || url.starts_with("https://") {
                    url.to_string()
                } else {
                    format!("{base}/{}", url.trim_start_matches('/'))
                };
                // `/media/<token>` answers 401 without the workbench key, so
                // the generic unauthenticated image fetcher can never read
                // it. Download here, with the key, and hand back a `data:`
                // URL — the key never leaves this function or enters the
                // document.
                // The artifact is already rendered — a dropped connection
                // must not throw the GPU's work away. Retry a few times.
                let mut attempt = 1;
                loop {
                    match download_artifact(client, &absolute, profile.api_key.trim()).await {
                        Err(ImageGenerateError::DownloadFailed)
                            if attempt < TRANSIENT_ATTEMPTS
                                && tokio::time::Instant::now() < deadline =>
                        {
                            attempt += 1;
                            tokio::time::sleep(polling.interval * 2).await;
                        }
                        result => return result,
                    }
                }
            }
            Some("failed" | "canceled") => {
                let message = job
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown error")
                    .to_string();
                return Err(ImageGenerateError::Upstream {
                    provider: PROVIDER,
                    message,
                });
            }
            _ => {}
        }
    }
}

/// Settings "Test" probe: `GET /workbench-api/qwen-image/status`, mapping
/// `ready`/`online`/`error` onto the shared [`ImageTestStatus`].
pub async fn test_workbench(
    client: &reqwest::Client,
    profile: &ImageGenProfile,
) -> ImageTestStatus {
    let Some(base) = profile
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|base| !base.is_empty())
        .map(|base| base.trim_end_matches('/').to_string())
    else {
        return ImageTestStatus::Invalid;
    };
    let resp = match client
        .get(format!("{base}/workbench-api/qwen-image/status"))
        .bearer_auth(profile.api_key.trim())
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return ImageTestStatus::Invalid,
    };
    let (status, body) = match read_provider_body(PROVIDER, resp).await {
        Ok(read) => read,
        Err(_) => return ImageTestStatus::Invalid,
    };
    if !status.is_success() {
        return ImageTestStatus::Invalid;
    }
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) else {
        return ImageTestStatus::Invalid;
    };
    let ready = json.get("ready").and_then(serde_json::Value::as_bool);
    let online = json.get("online").and_then(serde_json::Value::as_bool);
    let error = json
        .get("error")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|error| !error.is_empty());
    if ready == Some(true) && online == Some(true) && error.is_none() {
        ImageTestStatus::Valid
    } else {
        ImageTestStatus::Invalid
    }
}

/// The workbench wraps both the submit and the poll reply in `{"job": {…}}`
/// (verified against a live machine); older builds answered with the job
/// object at the top level. Read either shape.
fn job_object(value: &serde_json::Value) -> &serde_json::Value {
    value
        .get("job")
        .filter(|job| job.is_object())
        .unwrap_or(value)
}

/// Whether a self-hosted endpoint should be dialled directly, bypassing any
/// HTTP(S)_PROXY the process inherited. A workbench usually sits on a LAN or
/// a tailnet (100.64.0.0/10); routed through a desktop's system proxy it
/// answers 502, so the generation never runs and silently degrades to
/// search. Public hosts keep whatever proxy the user configured.
pub fn bypasses_proxy(base_url: &str) -> bool {
    let Some(host) = reqwest::Url::parse(base_url.trim())
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
    else {
        return false;
    };
    if host == "localhost" || host.ends_with(".local") || host.ends_with(".ts.net") {
        return true;
    }
    let host = host.trim_start_matches('[').trim_end_matches(']');
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => {
            let [a, b, ..] = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || (a == 100 && (64..=127).contains(&b))
        }
        Ok(std::net::IpAddr::V6(ip)) => ip.is_loopback(),
        Err(_) => false,
    }
}

/// Largest artifact accepted from a workbench (a 2048² PNG is ~8 MB).
const MAX_ARTIFACT_BYTES: usize = 24 * 1024 * 1024;

async fn download_artifact(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
) -> Result<String, ImageGenerateError> {
    use base64::Engine as _;
    let resp = client
        .get(url)
        .bearer_auth(api_key)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|_| ImageGenerateError::DownloadFailed)?;
    if !resp.status().is_success() {
        return Err(ImageGenerateError::DownloadFailed);
    }
    let header_mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        })
        .filter(|mime| mime.starts_with("image/"));
    let bytes = resp
        .bytes()
        .await
        .map_err(|_| ImageGenerateError::DownloadFailed)?;
    if bytes.is_empty() || bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(ImageGenerateError::DownloadFailed);
    }
    let mime = header_mime.unwrap_or_else(|| {
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            "image/jpeg".into()
        } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
            "image/webp".into()
        } else {
            "image/png".into()
        }
    });
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    ))
}

/// Map a design slot's box onto a resolution the workbench accepts: every
/// edge a multiple of 32 in 256..=2048 (it answers `invalid_width` /
/// `invalid_height` / `resolution_must_be_multiple_of_32` otherwise — a
/// 1440×120 hero band or a 180×240 thumbnail is never valid as-is). The
/// slot's aspect ratio is kept (capped at 4:1), the long edge is generated
/// at 1024, and the image is scaled/cropped into the slot afterwards.
pub(crate) fn workbench_size(width: Option<f64>, height: Option<f64>) -> (i64, i64) {
    const LONG_EDGE: f64 = 1024.0;
    let aspect = match (width, height) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 && w.is_finite() && h.is_finite() => w / h,
        _ => 1.0,
    }
    .clamp(0.25, 4.0);
    let (w, h) = if aspect >= 1.0 {
        (LONG_EDGE, LONG_EDGE / aspect)
    } else {
        (LONG_EDGE * aspect, LONG_EDGE)
    };
    let snap = |edge: f64| ((edge / 32.0).round() as i64 * 32).clamp(256, 2048);
    (snap(w), snap(h))
}

#[cfg(test)]
#[path = "web_image_generate_workbench_tests.rs"]
mod tests;
