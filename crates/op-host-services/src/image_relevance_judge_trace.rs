//! Explicitly enabled, payload-free diagnostics for image-judge decisions.

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use op_image_enrich::net::{JudgedCandidate, RelevanceVerdict};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const TRACE_PATH_ENV: &str = "OP_IMAGE_JUDGE_TRACE_PATH";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_STRING_BYTES: usize = 4096;
const REDACTED: &str = "[redacted]";
const EVIDENCE_FIELDS: &[&str] = &[
    "schema_version",
    "checks",
    "appearance_contract",
    "aesthetic_preferences",
    "soft_deviations",
    "hard_conflicts",
    "verdict",
    "reason",
];
const WRITE_QUEUE_CAPACITY: usize = 32;
static WRITER: OnceLock<Option<SyncSender<TraceWrite>>> = OnceLock::new();
static BATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TraceWrite {
    path: PathBuf,
    line: Vec<u8>,
}

struct BatchState {
    path: PathBuf,
    id: String,
    route: &'static str,
    query: String,
    intent: String,
    sender: SyncSender<TraceWrite>,
}

/// Clones identify the same batch; concurrent records stay separate JSON lines.
#[derive(Clone, Default)]
pub(super) struct JudgeTraceBatch {
    state: Option<Arc<BatchState>>,
}

pub(super) struct TraceResponse<'a> {
    pub(super) attempt: usize,
    pub(super) content: Option<&'a str>,
    pub(super) reasoning: Option<&'a str>,
    pub(super) candidate: &'a JudgedCandidate,
    pub(super) elapsed: Duration,
    pub(super) error: Option<&'a str>,
}

impl JudgeTraceBatch {
    pub(super) fn from_env(route: &'static str, query: &str, intent: &str) -> Self {
        let path = std::env::var(TRACE_PATH_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| PathBuf::from(value.trim()));
        Self::with_path(path, route, query, intent)
    }

    fn with_path(path: Option<PathBuf>, route: &'static str, query: &str, intent: &str) -> Self {
        let Some(path) = path.filter(|path| !path.as_os_str().is_empty()) else {
            return Self::default();
        };
        let Some(sender) = writer_sender() else {
            return Self::default();
        };
        Self::with_sender(Some(path), route, query, intent, sender.clone())
    }

    fn with_sender(
        path: Option<PathBuf>,
        route: &'static str,
        query: &str,
        intent: &str,
        sender: SyncSender<TraceWrite>,
    ) -> Self {
        let state = path
            .filter(|path| !path.as_os_str().is_empty())
            .map(|path| {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                Arc::new(BatchState {
                    path,
                    id: format!(
                        "{}-{nanos}-{}",
                        std::process::id(),
                        BATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                    ),
                    route,
                    query: safe_string(query),
                    intent: safe_string(intent),
                    sender,
                })
            });
        Self { state }
    }

    pub(super) fn record(&self, index: usize, thumb: &[u8], response: TraceResponse<'_>) {
        let Some(state) = &self.state else { return };
        let TraceResponse {
            attempt,
            content,
            reasoning,
            candidate,
            elapsed,
            error,
        } = response;
        let verdict = match candidate.verdict {
            RelevanceVerdict::On => "on",
            RelevanceVerdict::Weak => "weak",
            RelevanceVerdict::Off => "off",
        };
        let record = json!({
            "pid": std::process::id(),
            "batch_id": state.id,
            "route": safe_string(state.route),
            "query": state.query,
            "intent": state.intent,
            "index": index,
            "candidate_index": candidate.index,
            "thumb_sha256": format!("{:x}", Sha256::digest(thumb)),
            "attempt": attempt,
            "elapsed_ms": elapsed.as_millis().min(u64::MAX as u128) as u64,
            "host_verdict": verdict,
            "host_reason": safe_string(&candidate.reason),
            "error_detail": error.map(safe_string),
            "content_evidence": evidence_json(content),
            "reasoning_evidence": evidence_json(reasoning),
        });
        let Ok(mut line) = serde_json::to_vec(&record) else {
            return;
        };
        line.push(b'\n');
        // Filesystem latency must never consume the caller's judge deadline or
        // keep its model permit alive. Saturation/disconnection drops diagnostics.
        let _ = state.sender.try_send(TraceWrite {
            path: state.path.clone(),
            line,
        });
    }
}

fn writer_sender() -> Option<&'static SyncSender<TraceWrite>> {
    WRITER
        .get_or_init(|| {
            let (sender, receiver) = sync_channel::<TraceWrite>(WRITE_QUEUE_CAPACITY);
            std::thread::Builder::new()
                .name("op-image-judge-trace".into())
                .spawn(move || {
                    for record in receiver {
                        let _ = append_record(&record);
                    }
                })
                .ok()
                .map(|_detached| sender)
        })
        .as_ref()
}

fn append_record(record: &TraceWrite) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    // Reject special files before opening them. Check again after opening to
    // close replacement races; nonblocking/no-follow Unix flags also prevent
    // a swapped FIFO from waiting forever for a reader during open().
    reject_special_path(&record.path)?;
    if let Some(parent) = record.path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let mut file = options.open(&record.path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "trace requires a regular file",
        ));
    }
    file.write_all(&record.line)
}

fn reject_special_path(path: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    #[cfg(windows)]
    if matches!(
        path.components().next(),
        Some(std::path::Component::Prefix(prefix))
            if matches!(prefix.kind(), std::path::Prefix::DeviceNS(_) | std::path::Prefix::Verbatim(_))
    ) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "trace requires a regular file",
        ));
    }
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
        Ok(_) => Err(Error::new(
            ErrorKind::InvalidInput,
            "trace requires a regular file",
        )),
    }
}

fn evidence_json(text: Option<&str>) -> Option<Value> {
    let text = text?.trim();
    if text.len() > MAX_RESPONSE_BYTES {
        return None;
    }
    let value = serde_json::from_str::<Value>(text).ok().or_else(|| {
        let start = text.find('{')?;
        let end = text.rfind('}')?;
        serde_json::from_str(text.get(start..=end)?).ok()
    })?;
    let object = value.as_object()?;
    let allowed = object
        .iter()
        .filter(|(key, _)| EVIDENCE_FIELDS.contains(&key.as_str()));
    let mut budget = 512;
    let sanitized: serde_json::Map<String, Value> = allowed
        .map(|(key, value)| (key.clone(), safe_value(value, 0, &mut budget)))
        .collect();
    (!sanitized.is_empty()).then_some(Value::Object(sanitized))
}

fn safe_value(value: &Value, depth: usize, budget: &mut usize) -> Value {
    if depth >= 12 || *budget == 0 {
        return Value::Null;
    }
    *budget -= 1;
    match value {
        Value::String(text) => Value::String(safe_string(text)),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .take(128)
                .map(|value| safe_value(value, depth + 1, budget))
                .collect(),
        ),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .take(128)
                .filter(|(key, _)| !sensitive_key(key) && safe_string(key) == **key)
                .map(|(key, value)| (key.clone(), safe_value(value, depth + 1, budget)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn sensitive_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect();
    [
        "apikey",
        "token",
        "password",
        "secret",
        "credential",
        "authorization",
        "authentication",
        "privatekey",
        "cookie",
        "base64",
        "requestbody",
        "rawresponse",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn safe_string(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let compact: String = lower
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace())
        .collect();
    let credential = [
        "api_key",
        "api-key",
        "api key",
        "apikey",
        "access_token",
        "access token",
        "accesstoken",
        "client_secret",
        "client secret",
        "clientsecret",
        "authorization",
        "bearer ",
        "password",
        "private_key",
        "private key",
        "credential",
        "secret_key",
        "token=",
        "token:",
        "token\":",
        "key=",
        "key:",
        "sk-",
        "sk_",
        "ghp_",
        "gho_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "aiza",
        "akia",
        "-----begin",
    ]
    .iter()
    .any(|marker| lower.contains(marker) || compact.contains(marker));
    let encoded = text
        .split(|ch: char| !ch.is_ascii_alphanumeric() && !matches!(ch, '+' | '/' | '=' | '_' | '-'))
        .any(|word| word.len() >= 64);
    let jwt = text.split_whitespace().any(|word| {
        let parts: Vec<_> = word
            .trim_matches(|ch: char| matches!(ch, '"' | '\'' | ',' | ';'))
            .split('.')
            .collect();
        parts.len() == 3
            && parts.iter().all(|part| {
                part.len() >= 8
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            })
    });
    if text.len() > MAX_STRING_BYTES
        || credential
        || encoded
        || jwt
        || compact.contains("data:")
        || lower.contains("\\u")
        || lower.contains("\\x")
    {
        REDACTED.to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
#[path = "image_relevance_judge_trace_tests.rs"]
mod tests;
