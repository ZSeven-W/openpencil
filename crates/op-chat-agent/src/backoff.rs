//! Shared HTTP throttle / retry ladder + provider client construction for
//! the builtin (API-key) providers — extracted verbatim from
//! `op-host-services/src/chat_builtin_http.rs` (which re-exports everything
//! here) so the agent loop and the mobile hosts dial with identical posture.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::chat_builtin_http::BuiltinHttpError;

/// Design turns build one <=25-op section batch per model turn, plus repair
/// turns after layoutIssues feedback. 28 sits in the requested 24-32 window:
/// enough for roughly 8-10 sections with fixes, without letting a bad loop run
/// indefinitely. Plain chat keeps `chat_canvas_tools::MAX_TOOL_TURNS`.
pub const DESIGN_LOOP_MAX_TURNS: usize = 28;

/// A <=25-op batch plus one concise tool call fits comfortably in 4-6k output
/// tokens. 6144 keeps section batches complete without encouraging the model to
/// emit a whole screen in one turn.
pub const DESIGN_LOOP_MAX_OUTPUT_TOKENS: u32 = 6_144;

const BUILTIN_HTTP_DEFAULT_MIN_GAP: Duration = Duration::from_millis(350);
// 5 retries at 1/2/4/8/16s covers ~31s — enough to outlast a per-minute
// account frequency window (GLM "AccountRateLimitExceeded" killed a run
// that 3 retries / 7s could not ride out, measured 2026-07-12).
pub const BUILTIN_HTTP_MAX_RETRIES: u32 = 5;
/// Extra inter-request gap added after each 429, halved after each
/// success — the throttle adapts to the account's real ceiling instead of
/// hammering at the configured floor.
static BUILTIN_HTTP_ADAPTIVE_GAP_MS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
const ADAPTIVE_GAP_START_MS: u64 = 1_000;
const ADAPTIVE_GAP_MAX_MS: u64 = 5_000;
const BUILTIN_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const BUILTIN_HTTP_READ_IDLE_TIMEOUT: Duration = Duration::from_secs(180);
const RETRY_AFTER_MAX: Duration = Duration::from_secs(30);
const BACKOFF_MAX: Duration = Duration::from_secs(8);

static BUILTIN_HTTP_LAST_REQUEST: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

pub fn builtin_http_min_gap() -> Duration {
    std::env::var("OPENPENCIL_BUILTIN_HTTP_MIN_GAP_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(BUILTIN_HTTP_DEFAULT_MIN_GAP)
}

pub fn throttle_wait(last: Option<Instant>, now: Instant, min_gap: Duration) -> Duration {
    last.and_then(|last| last.checked_add(min_gap))
        .map(|next_allowed| next_allowed.saturating_duration_since(now))
        .unwrap_or(Duration::ZERO)
}

fn widen_adaptive_gap() {
    use std::sync::atomic::Ordering;
    let current = BUILTIN_HTTP_ADAPTIVE_GAP_MS.load(Ordering::Relaxed);
    let next = (current * 2).clamp(ADAPTIVE_GAP_START_MS, ADAPTIVE_GAP_MAX_MS);
    BUILTIN_HTTP_ADAPTIVE_GAP_MS.store(next, Ordering::Relaxed);
}

fn relax_adaptive_gap() {
    use std::sync::atomic::Ordering;
    let current = BUILTIN_HTTP_ADAPTIVE_GAP_MS.load(Ordering::Relaxed);
    if current > 0 {
        BUILTIN_HTTP_ADAPTIVE_GAP_MS.store(current / 2, Ordering::Relaxed);
    }
}

async fn throttle_builtin_http_request(base_min_gap: Duration) {
    let adaptive = Duration::from_millis(
        BUILTIN_HTTP_ADAPTIVE_GAP_MS.load(std::sync::atomic::Ordering::Relaxed),
    );
    let min_gap = base_min_gap + adaptive;
    if min_gap.is_zero() {
        return;
    }

    let wait = {
        let last_request = BUILTIN_HTTP_LAST_REQUEST.get_or_init(|| Mutex::new(None));
        let mut last = last_request
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();
        let wait = throttle_wait(*last, now, min_gap);
        *last = now.checked_add(wait);
        wait
    };

    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

pub fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
        || status == reqwest::StatusCode::from_u16(529).expect("529 is a valid HTTP status")
}

pub fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let seconds = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_secs(seconds).min(RETRY_AFTER_MAX))
}

pub fn backoff_delay(attempt: u32) -> Duration {
    let delay = 1_u64.checked_shl(attempt).unwrap_or(u64::MAX);
    Duration::from_secs(delay).min(BACKOFF_MAX)
}

/// Default retry/throttle knobs for callers without a per-provider
/// profile (the design tool-loop's own HTTP path).
pub fn default_backoff_knobs() -> (u32, Duration) {
    (BUILTIN_HTTP_MAX_RETRIES, builtin_http_min_gap())
}

pub async fn send_with_backoff(
    label: &str,
    url: &str,
    max_retries: u32,
    min_gap: Duration,
    build: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, BuiltinHttpError> {
    for attempt in 0..=max_retries {
        throttle_builtin_http_request(min_gap).await;
        match build().send().await {
            Ok(resp) if resp.status().is_success() => {
                relax_adaptive_gap();
                return Ok(resp);
            }
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    widen_adaptive_gap();
                }
                if is_retryable_status(status) && attempt < max_retries {
                    let delay =
                        parse_retry_after(resp.headers()).unwrap_or_else(|| backoff_delay(attempt));
                    drop(resp);
                    tokio::time::sleep(delay).await;
                    continue;
                }
                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    // Must contain the literal substring "http 429" (case-
                    // insensitively) — `op_orchestrator::retry::is_non_retryable`
                    // pattern-matches on it to stop the subtask retry ladder
                    // at attempt 1 instead of burning two more full LLM calls
                    // on a rate limit that will still be in effect seconds
                    // later. The prior wording "(429)" (no "http") silently
                    // missed that classifier — a genuinely exhausted 429 was
                    // misread as retryable and got attempts 2/3 anyway.
                    return Err(BuiltinHttpError::RateLimited {
                        label: label.to_string(),
                        max_retries,
                    });
                }
                // Provider error bodies are untrusted and can echo request
                // headers. Never relay them into chat/SSE where credentials
                // could be reflected back into logs or browser responses.
                return Err(BuiltinHttpError::HttpStatus {
                    label: label.to_string(),
                    status,
                });
            }
            Err(e) => {
                if e.is_timeout() {
                    return Err(BuiltinHttpError::Timeout {
                        label: label.to_string(),
                        url: url.to_string(),
                        message: e.to_string(),
                    });
                }
                if attempt < max_retries {
                    tokio::time::sleep(backoff_delay(attempt)).await;
                    continue;
                }
                return Err(BuiltinHttpError::Transport {
                    label: label.to_string(),
                    url: url.to_string(),
                    message: e.to_string(),
                });
            }
        }
    }
    unreachable!("backoff loop always returns before exhausting range")
}

/// reqwest client with connect + read-idle timeouts. A bare `Client::new()`
/// has NO timeout, so a hung LLM endpoint (connection opens but the server
/// never streams, or stalls mid-response) makes the blocking provider iterator
/// — and therefore the orchestrator's planning / sub-agent call — hang forever
/// (desktop pinned on "Planning…", with Stop unable to interrupt the already
/// in-flight request). These deadlines surface the stall as an error so the
/// planning loop falls back instead of hanging.
///
/// The deadline is per-read, NOT per-request: a reasoning model can spend more
/// than a whole-request budget on a single large generation while the stream
/// stays alive, so an overall `.timeout()` severs live work (measured: a 1M-ctx
/// reasoning model aborted mid-design at the old 300s cap). A read-idle
/// deadline still trips on the stall this guard exists for, because a wedged
/// endpoint stops producing bytes.
///
/// Reports [`crate::provider_dial::ProviderDialError::ClientBuild`] rather
/// than a local variant: this IS the `Trusted` half of `provider_dial`'s
/// dial, and the pinned `PublicOnly` half produces the same sentence for the
/// same reqwest failure — one variant keeps them from drifting apart.
pub fn builtin_http_client() -> Result<reqwest::Client, crate::provider_dial::ProviderDialError> {
    builtin_http_client_builder().build().map_err(|error| {
        crate::provider_dial::ProviderDialError::ClientBuild {
            message: error.to_string(),
        }
    })
}

/// Shared builder so pinned (DNS-screened) clients keep the same redirect
/// and timeout posture as the default provider client.
pub fn builtin_http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(BUILTIN_HTTP_CONNECT_TIMEOUT)
        .read_timeout(BUILTIN_HTTP_READ_IDLE_TIMEOUT)
}

/// Apply the provider-specific low-reasoning control for structured design
/// turns. The two supported wire shapes are deliberately centralized here so
/// classic streaming and the tool-executing agent loop cannot drift or send
/// mutually-exclusive fields together. (Moved verbatim from
/// `op-host-services/src/chat_builtin_http_wire.rs`, which re-exports it.)
pub fn apply_reasoning_wire_control(body: &mut Value, model: &str, reduce_reasoning: bool) {
    if !reduce_reasoning {
        return;
    }
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    match op_orchestrator::reasoning_wire_control(model) {
        Some(op_orchestrator::ReasoningWireControl::ThinkingDisabled) => {
            obj.remove("reasoning_effort");
            obj.insert("thinking".into(), serde_json::json!({ "type": "disabled" }));
        }
        Some(op_orchestrator::ReasoningWireControl::ReasoningEffortLow) => {
            obj.remove("thinking");
            obj.insert("reasoning_effort".into(), Value::String("low".into()));
        }
        None => {}
    }
}

/// Anthropic-wire twin of [`apply_reasoning_wire_control`].
///
/// The empty-canvas postmortem: the OpenAI-compat agent loop applied the
/// low-reasoning control and the Anthropic loop silently did not, so a
/// reasoning model driven over its Anthropic-compatible endpoint (the
/// DeepSeek preset's alternate API format) burned the whole 6144-token turn
/// budget on hidden thinking — `{}`-sized read tools still fit in the
/// leftovers, `batch_design` never did, and every design run ended as a row
/// of green read cards over an empty canvas.
///
/// Wire shapes differ from the OpenAI-compat body, hence a separate entry
/// point rather than a blind reuse:
/// - `ThinkingDisabled` families (DeepSeek / GLM / MiniMax / Kimi K2.5-2.6)
///   accept the Anthropic-shape `thinking: {"type": "disabled"}` — the same
///   field the native Anthropic API documents, verified against
///   `api.deepseek.com/anthropic`.
/// - `ReasoningEffortLow` (Kimi K3) is an OpenAI-wire-only top-level field;
///   no equivalent Anthropic-wire control is documented, and K3's reasoning
///   cannot be disabled at all — so it is deliberately a no-op here rather
///   than a guessed field that would 400 the whole run.
pub fn apply_reasoning_wire_control_anthropic(
    body: &mut Value,
    model: &str,
    reduce_reasoning: bool,
) {
    if !reduce_reasoning {
        return;
    }
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    if matches!(
        op_orchestrator::reasoning_wire_control(model),
        Some(op_orchestrator::ReasoningWireControl::ThinkingDisabled)
    ) {
        obj.insert("thinking".into(), serde_json::json!({ "type": "disabled" }));
    }
}
