//! Short-request async job protocol for intelligent `design.md` generation.
//!
//! MV3 service workers cannot rely on a single HTTP request staying alive
//! while a model thinks. The host therefore acknowledges a POST immediately,
//! and the extension polls a short GET until the final Markdown is ready.

use serde::Deserialize;
use serde_json::Value;

use crate::js_text::{js_trim, truncate_utf16};

pub const START_TIMEOUT_MS: u32 = 10_000;
pub const POLL_TIMEOUT_MS: u32 = 10_000;
pub const POLL_INTERVAL_MS: u32 = 1_000;
pub const TOTAL_BUDGET_MS: u32 = 120_000;
pub const MAX_START_REPLY_BYTES: usize = 8 * 1024;
pub const MAX_PENDING_REPLY_BYTES: usize = MAX_START_REPLY_BYTES;
pub const MAX_MARKDOWN_BYTES: usize = 512 * 1024;
pub const MAX_POLL_REPLY_BYTES: usize = MAX_MARKDOWN_BYTES * 2 + 4 * 1024;
pub const MAX_POLL_ATTEMPTS: u32 = 160;
pub const MAX_TOTAL_REPLY_BYTES: usize = MAX_START_REPLY_BYTES
    + MAX_POLL_REPLY_BYTES
    + MAX_PENDING_REPLY_BYTES * MAX_POLL_ATTEMPTS as usize;

const JOB_ID_BYTES: usize = 32;
const MIN_RETRY_AFTER_MS: u64 = 100;
const MAX_RETRY_AFTER_MS: u64 = 5_000;
const MAX_DETAIL_UNITS: usize = 200;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PendingEnvelope {
    ok: bool,
    status: String,
    job_id: String,
    retry_after_ms: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FinalEnvelope {
    ok: bool,
    markdown: String,
    intelligent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartReply {
    Pending {
        job_id: String,
        retry_after_ms: u32,
    },
    Failed {
        code: DesignMdFailure,
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollReply {
    Pending {
        retry_after_ms: u32,
    },
    Ready {
        markdown: String,
    },
    Failed {
        code: DesignMdFailure,
        detail: String,
    },
}

/// Backward-compatible final-reply shape retained for one release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesignMdReply {
    Ready {
        markdown: String,
    },
    Failed {
        code: DesignMdFailure,
        detail: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMdFailure {
    ExtensionNotPaired,
    Unsupported,
    Busy,
    NoModel,
    Timeout,
    GenerationFailed,
}

impl DesignMdFailure {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExtensionNotPaired => "extensionNotPaired",
            Self::Unsupported => "unsupported",
            Self::Busy => "busy",
            Self::NoModel => "noModel",
            Self::Timeout => "timeout",
            Self::GenerationFailed => "generationFailed",
        }
    }
}

pub fn classify_start_reply(status: u16, text: &str) -> StartReply {
    if status == 202 {
        return match pending_envelope(text, MAX_START_REPLY_BYTES) {
            Some((job_id, retry_after_ms)) => StartReply::Pending {
                job_id,
                retry_after_ms,
            },
            None => start_failed("The design generator returned an invalid job"),
        };
    }
    let (code, detail) = failure(status, text);
    StartReply::Failed { code, detail }
}

pub fn classify_poll_reply(status: u16, text: &str) -> PollReply {
    if status == 202 {
        return match pending_envelope(text, MAX_PENDING_REPLY_BYTES) {
            Some((_job_id, retry_after_ms)) => PollReply::Pending { retry_after_ms },
            None => poll_failed("The design generator returned an invalid job status"),
        };
    }
    if status == 200 {
        return match smart_markdown(text, false) {
            Some(markdown) => PollReply::Ready { markdown },
            None => poll_failed("The design generator returned an invalid document"),
        };
    }
    let (code, detail) = poll_failure(status, text);
    PollReply::Failed { code, detail }
}

/// Legacy synchronous classifier. New code uses [`classify_poll_reply`].
pub fn classify_reply(status: u16, text: &str) -> DesignMdReply {
    if status == 200 {
        return match smart_markdown(text, true) {
            Some(markdown) => DesignMdReply::Ready { markdown },
            None => DesignMdReply::Failed {
                code: DesignMdFailure::GenerationFailed,
                detail: "The design generator returned an invalid document".to_owned(),
            },
        };
    }
    let (code, detail) = failure(status, text);
    DesignMdReply::Failed { code, detail }
}

pub fn is_valid_job_id(job_id: &str) -> bool {
    job_id.len() == JOB_ID_BYTES
        && job_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn pending_envelope(text: &str, max_bytes: usize) -> Option<(String, u32)> {
    if text.len() > max_bytes {
        return None;
    }
    let envelope: PendingEnvelope = serde_json::from_str(text).ok()?;
    if !envelope.ok || envelope.status != "pending" {
        return None;
    }
    if !is_valid_job_id(&envelope.job_id) {
        return None;
    }
    if !(MIN_RETRY_AFTER_MS..=MAX_RETRY_AFTER_MS).contains(&envelope.retry_after_ms) {
        return None;
    }
    Some((envelope.job_id, envelope.retry_after_ms as u32))
}

fn smart_markdown(text: &str, allow_plain: bool) -> Option<String> {
    if text.len() > MAX_POLL_REPLY_BYTES {
        return None;
    }
    let markdown = if let Ok(envelope) = serde_json::from_str::<FinalEnvelope>(text) {
        if !envelope.ok || !envelope.intelligent {
            return None;
        }
        envelope.markdown
    } else if allow_plain {
        text.to_owned()
    } else {
        return None;
    };
    let markdown = js_trim(&markdown);
    if markdown.is_empty()
        || markdown.len() > MAX_MARKDOWN_BYTES
        || !crate::design_md_validate::is_valid(markdown)
    {
        return None;
    }
    Some(markdown.to_owned())
}

fn failure(status: u16, text: &str) -> (DesignMdFailure, String) {
    let text = if text.len() <= MAX_PENDING_REPLY_BYTES {
        text
    } else {
        ""
    };
    let code = reply_code(text).unwrap_or(match status {
        403 => DesignMdFailure::ExtensionNotPaired,
        404 => DesignMdFailure::Unsupported,
        429 => DesignMdFailure::Busy,
        503 => DesignMdFailure::NoModel,
        504 => DesignMdFailure::Timeout,
        _ => DesignMdFailure::GenerationFailed,
    });
    (code, reply_detail(status, text))
}

fn poll_failure(status: u16, text: &str) -> (DesignMdFailure, String) {
    let (mut code, detail) = failure(status, text);
    // Once POST returned a job, a missing poll route is that job disappearing
    // (wrong owner, result TTL cleanup, or a forged id), not an old host that
    // lacks the feature. Expiry consumed the generation budget and is surfaced
    // as the existing timeout role.
    if status == 404 {
        code = DesignMdFailure::GenerationFailed;
    } else if status == 410 {
        code = DesignMdFailure::Timeout;
    }
    (code, detail)
}

fn start_failed(detail: &str) -> StartReply {
    StartReply::Failed {
        code: DesignMdFailure::GenerationFailed,
        detail: detail.to_owned(),
    }
}

fn poll_failed(detail: &str) -> PollReply {
    PollReply::Failed {
        code: DesignMdFailure::GenerationFailed,
        detail: detail.to_owned(),
    }
}

fn reply_detail(status: u16, text: &str) -> String {
    if text.len() <= 64 * 1024 {
        if let Ok(value) = serde_json::from_str::<Value>(text) {
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                if let Some(detail) = safe_detail(error) {
                    return detail;
                }
            }
        }
    }
    safe_detail(text).unwrap_or_else(|| format!("HTTP {status}"))
}

fn reply_code(text: &str) -> Option<DesignMdFailure> {
    if text.len() > 64 * 1024 {
        return None;
    }
    let value = serde_json::from_str::<Value>(text).ok()?;
    let code = value.get("code")?.as_str()?;
    Some(match code {
        "extensionNotPaired" => DesignMdFailure::ExtensionNotPaired,
        "unsupported" => DesignMdFailure::Unsupported,
        "busy" => DesignMdFailure::Busy,
        "noModel" | "modelUnavailable" => DesignMdFailure::NoModel,
        "timeout" => DesignMdFailure::Timeout,
        _ => DesignMdFailure::GenerationFailed,
    })
}

fn safe_detail(value: &str) -> Option<String> {
    let value = js_trim(value);
    if value.is_empty() {
        return None;
    }
    let capped = truncate_utf16(value, MAX_DETAIL_UNITS);
    Some(
        capped
            .chars()
            .map(|ch| if ch.is_control() { ' ' } else { ch })
            .collect(),
    )
}
