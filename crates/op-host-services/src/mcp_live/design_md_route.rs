//! Extension-only intelligent `design.md` generation route.
//!
//! The connection thread validates a small, content-free evidence corpus and
//! queues it for the desktop host. The request never snapshots or mutates the
//! live document. The desktop owns provider selection and replies later through
//! [`DesignMdResponder`].

use super::design_md_output::{append_evidence_appendix, validate_markdown};
use super::*;
use std::sync::atomic::AtomicBool;
use std::sync::OnceLock;
use std::time::Instant;

pub(crate) const DESIGN_MD_PATH: &str = "/api/generate/design-md";
const DESIGN_MD_JOB_PREFIX: &str = "/api/generate/design-md/";
const DESIGN_MD_RETRY_AFTER_MS: u64 = 750;
const DESIGN_MD_JOB_TTL: Duration = Duration::from_secs(130);
const DESIGN_MD_RESULT_TTL: Duration = Duration::from_secs(30);
const MAX_COMPLETED_JOBS: usize = 2;
static DESIGN_MD_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static DESIGN_MD_JOB: OnceLock<Mutex<Option<DesignMdJob>>> = OnceLock::new();
static DESIGN_MD_COMPLETED: OnceLock<Mutex<VecDeque<CompletedDesignMdJob>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMdResponseError {
    NoModel,
    WorkerSpawn,
    ProviderError,
    EmptyOutput,
    InvalidOutput,
    OutputTooLarge,
    Timeout,
}

#[derive(Debug)]
struct DesignMdSlotLease;

impl Drop for DesignMdSlotLease {
    fn drop(&mut self) {
        DESIGN_MD_IN_FLIGHT.store(false, Ordering::Release);
    }
}

type DesignMdResult = Result<String, DesignMdResponseError>;

#[derive(Debug)]
enum DesignMdJobState {
    Pending,
    Expired,
    Cancelled,
}

#[derive(Debug)]
struct DesignMdJob {
    id: String,
    owner_origin: Option<String>,
    deadline: Instant,
    canceled: Arc<AtomicBool>,
    state: DesignMdJobState,
    provenance: crate::design_md_evidence::DesignMdEvidenceProvenance,
    lease: Option<Arc<DesignMdSlotLease>>,
    watchdog_stop: SyncSender<()>,
}

#[cfg(test)]
static FORCE_WATCHDOG_SPAWN_FAILURE: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct CompletedDesignMdJob {
    id: String,
    owner_origin: Option<String>,
    expires_at: Instant,
    result: DesignMdResult,
}

/// Completion handle for an asynchronous desktop LLM worker.
///
/// The global single-flight lease is shared with the active job and this
/// handle. Cancellation removes the job's copy, but cannot open a second paid
/// turn until the desktop worker observes cancellation and drops its copy.
#[derive(Debug)]
pub struct DesignMdResponder {
    job_id: String,
    canceled: Arc<AtomicBool>,
    _lease: Arc<DesignMdSlotLease>,
}

impl DesignMdResponder {
    pub fn success(self, markdown: String) -> bool {
        finish_job(&self.job_id, Ok(markdown))
    }

    pub fn error(self, error: DesignMdResponseError) -> bool {
        finish_job(&self.job_id, Err(error))
    }

    pub fn is_cancelled(&self) -> bool {
        self.canceled.load(Ordering::Acquire)
    }

    /// Shared cancellation flag for `ChatProvider::send_cancellable`. The
    /// route sets it at job expiry/cancellation; the desktop may also set it while
    /// shutting down the worker.
    pub fn cancellation_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.canceled)
    }
}

impl Drop for DesignMdResponder {
    fn drop(&mut self) {
        let _ = finish_job(&self.job_id, Err(DesignMdResponseError::ProviderError));
    }
}

/// One validated request waiting for the desktop host's provider worker.
/// Prompts contain only the compact, typed evidence produced by
/// `design_md_evidence`; no live-document state is captured.
#[derive(Debug)]
pub struct PendingDesignMdRequest {
    system_prompt: String,
    user_prompt: String,
    responder: DesignMdResponder,
}

impl PendingDesignMdRequest {
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    pub fn user_prompt(&self) -> &str {
        &self.user_prompt
    }

    /// True when the job expired or the caller cancelled it before the UI
    /// started paid work.
    pub fn is_cancelled(&self) -> bool {
        self.responder.is_cancelled()
    }

    pub fn into_parts(self) -> (String, String, DesignMdResponder) {
        (self.system_prompt, self.user_prompt, self.responder)
    }
}

pub(crate) fn is_design_md_path(path: &str) -> bool {
    path == DESIGN_MD_PATH || design_md_job_id(path).is_some()
}

pub(super) fn write_preflight<S: std::io::Write>(
    stream: &mut S,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    write_design_http(stream, "204 No Content", "", cors_origin)
}

pub(super) fn serve_design_md<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    req: &crate::mcp_serve::HttpRequest,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    if req.query.is_some() {
        return write_route_error(
            stream,
            "400 Bad Request",
            "invalidRequest",
            "This route does not accept query parameters.",
            cors_origin,
        );
    }
    if admission::is_unpaired_extension_origin(req.origin.as_deref()) {
        return write_route_error(
            stream,
            "403 Forbidden",
            "extensionNotPaired",
            "This extension is not paired with OpenPencil.",
            cors_origin,
        );
    }
    match (req.method.as_str(), design_md_job_id(&req.path)) {
        ("POST", None) if req.path == DESIGN_MD_PATH => {
            start_design_md_job(stream, req_tx, wake_ui, req, cors_origin)
        }
        ("GET", Some(job_id)) => poll_design_md_job(stream, req, job_id, cors_origin),
        ("DELETE", Some(job_id)) => cancel_design_md_job(stream, req, job_id, cors_origin),
        _ => write_route_error(
            stream,
            "400 Bad Request",
            "invalidRequest",
            "Use POST to start, GET to poll, or DELETE to cancel a design.md job.",
            cors_origin,
        ),
    }
}

fn start_design_md_job<S: std::io::Read + std::io::Write>(
    stream: &mut S,
    req_tx: &Sender<UiRequest>,
    wake_ui: &UiWake,
    req: &crate::mcp_serve::HttpRequest,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    if !content_type_is_strict_json(req.content_type.as_deref()) {
        return write_route_error(
            stream,
            "400 Bad Request",
            "invalidRequest",
            "Content-Type must be application/json.",
            cors_origin,
        );
    }
    let (sanitized, provenance) =
        match crate::design_md_evidence::sanitize_design_md_evidence_with_provenance(&req.body) {
            Ok(value) => value,
            Err(error) => {
                let error = error.to_string();
                return write_route_error(
                    stream,
                    "400 Bad Request",
                    "invalidRequest",
                    &error,
                    cors_origin,
                );
            }
        };
    let job_id = match make_job_id() {
        Some(id) => id,
        None => {
            return write_route_error(
                stream,
                "503 Service Unavailable",
                "workerSpawn",
                "OpenPencil could not create a design analysis job.",
                cors_origin,
            )
        }
    };
    let (system_prompt, user_prompt) =
        crate::design_md_evidence::build_design_md_evidence_prompts(&sanitized, &provenance);
    let Some((canceled, lease, watchdog_rx)) = install_job(
        job_id.clone(),
        request_owner(req),
        provenance,
        Instant::now(),
    ) else {
        return write_route_error(
            stream,
            "429 Too Many Requests",
            "busy",
            "Another design.md generation is already running.",
            cors_origin,
        );
    };
    if spawn_job_watchdog(job_id.clone(), watchdog_rx, DESIGN_MD_JOB_TTL).is_err() {
        remove_job(&job_id, true);
        return write_route_error(
            stream,
            "503 Service Unavailable",
            "workerSpawn",
            "OpenPencil could not start the design analysis watchdog.",
            cors_origin,
        );
    }
    let pending = PendingDesignMdRequest {
        system_prompt,
        user_prompt,
        responder: DesignMdResponder {
            job_id: job_id.clone(),
            canceled: Arc::clone(&canceled),
            _lease: lease,
        },
    };
    if req_tx
        .send(UiRequest::GenerateDesignMd { request: pending })
        .is_err()
    {
        remove_job(&job_id, true);
        return write_route_error(
            stream,
            "503 Service Unavailable",
            "workerSpawn",
            "OpenPencil is not accepting design analysis requests.",
            cors_origin,
        );
    }
    wake_ui();
    let response = write_pending(stream, &job_id, cors_origin);
    if response.is_err() {
        remove_job(&job_id, true);
    }
    response
}

fn try_acquire_slot() -> Option<DesignMdSlotLease> {
    DESIGN_MD_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .ok()
        .map(|_| DesignMdSlotLease)
}

fn job_store() -> &'static Mutex<Option<DesignMdJob>> {
    DESIGN_MD_JOB.get_or_init(|| Mutex::new(None))
}

fn completed_jobs() -> &'static Mutex<VecDeque<CompletedDesignMdJob>> {
    // A tiny idempotency window lets a poll retry after the first HTTP reply
    // was written but lost. Memory remains bounded by count, output cap, and
    // `DESIGN_MD_RESULT_TTL`.
    DESIGN_MD_COMPLETED.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn install_job(
    id: String,
    owner_origin: Option<String>,
    provenance: crate::design_md_evidence::DesignMdEvidenceProvenance,
    now: Instant,
) -> Option<(
    Arc<AtomicBool>,
    Arc<DesignMdSlotLease>,
    std::sync::mpsc::Receiver<()>,
)> {
    let mut guard = job_store()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    cleanup_completed(now);
    if let Some(job) = guard.as_mut() {
        expire_job(job, now);
        if matches!(job.state, DesignMdJobState::Pending) {
            return None;
        }
    }
    let lease = Arc::new(try_acquire_slot()?);
    let canceled = Arc::new(AtomicBool::new(false));
    let (watchdog_stop, watchdog_rx) = mpsc::sync_channel(1);
    if let Some(previous) = guard.as_mut() {
        let _ = previous.watchdog_stop.try_send(());
    }
    *guard = Some(DesignMdJob {
        id,
        owner_origin,
        deadline: now + DESIGN_MD_JOB_TTL,
        canceled: Arc::clone(&canceled),
        state: DesignMdJobState::Pending,
        provenance,
        lease: Some(Arc::clone(&lease)),
        watchdog_stop,
    });
    Some((canceled, lease, watchdog_rx))
}

fn finish_job(job_id: &str, result: DesignMdResult) -> bool {
    let mut guard = job_store()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let Some(mut job) = guard.take() else {
        return false;
    };
    if job.id != job_id {
        *guard = Some(job);
        return false;
    }
    expire_job(&mut job, Instant::now());
    if !matches!(job.state, DesignMdJobState::Pending) || job.canceled.load(Ordering::Acquire) {
        *guard = Some(job);
        return false;
    }
    let result = match result {
        Ok(markdown) => finalize_markdown(markdown, &job.provenance),
        Err(error) => Err(error),
    };
    let _ = job.watchdog_stop.try_send(());
    let _ = job.lease.take();
    push_completed(CompletedDesignMdJob {
        id: job.id,
        owner_origin: job.owner_origin,
        expires_at: Instant::now() + DESIGN_MD_RESULT_TTL,
        result,
    });
    drop(guard);
    true
}

fn finalize_markdown(
    markdown: String,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> DesignMdResult {
    let markdown = validate_markdown(markdown, provenance)?;
    let markdown = append_evidence_appendix(markdown, provenance);
    if markdown.len() > crate::design_md_evidence::MAX_DESIGN_MD_OUTPUT_BYTES {
        Err(DesignMdResponseError::OutputTooLarge)
    } else {
        Ok(markdown)
    }
}

fn expire_job(job: &mut DesignMdJob, now: Instant) {
    if now < job.deadline
        || matches!(
            job.state,
            DesignMdJobState::Expired | DesignMdJobState::Cancelled
        )
    {
        return;
    }
    job.canceled.store(true, Ordering::Release);
    let _ = job.watchdog_stop.try_send(());
    let _ = job.lease.take();
    job.state = DesignMdJobState::Expired;
}

fn cleanup_completed(now: Instant) {
    let mut completed = completed_jobs()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    completed.retain(|job| job.expires_at > now);
}

fn push_completed(job: CompletedDesignMdJob) {
    let mut completed = completed_jobs()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    completed.retain(|existing| existing.expires_at > Instant::now());
    completed.push_back(job);
    while completed.len() > MAX_COMPLETED_JOBS {
        completed.pop_front();
    }
}

fn remove_job(job_id: &str, cancel: bool) {
    let mut guard = job_store()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if guard.as_ref().is_some_and(|job| job.id == job_id) {
        if cancel {
            if let Some(job) = guard.as_mut() {
                job.canceled.store(true, Ordering::Release);
                let _ = job.watchdog_stop.try_send(());
                let _ = job.lease.take();
            }
        }
        *guard = None;
    }
    drop(guard);
    if cancel {
        let mut completed = completed_jobs()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        completed.retain(|job| job.id != job_id);
    }
}

fn request_owner(req: &crate::mcp_serve::HttpRequest) -> Option<String> {
    req.origin.as_deref().map(str::trim).map(str::to_string)
}

fn request_owns_job(req: &crate::mcp_serve::HttpRequest, job: &DesignMdJob) -> bool {
    request_matches_owner(req, job.owner_origin.as_deref())
}

fn request_matches_owner(req: &crate::mcp_serve::HttpRequest, owner_origin: Option<&str>) -> bool {
    match req.origin.as_deref().map(str::trim) {
        // Chrome's privileged extension GET/DELETE may omit Origin. The
        // 128-bit random job id is the capability in that wire shape.
        None => true,
        Some(origin) => owner_origin == Some(origin),
    }
}

fn make_job_id() -> Option<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn spawn_job_watchdog(
    job_id: String,
    stop_rx: std::sync::mpsc::Receiver<()>,
    timeout: Duration,
) -> std::io::Result<()> {
    #[cfg(test)]
    if FORCE_WATCHDOG_SPAWN_FAILURE.load(Ordering::Acquire) {
        return Err(std::io::Error::other("forced watchdog spawn failure"));
    }
    thread::Builder::new()
        .name("op-design-md-watchdog".into())
        .stack_size(256 * 1024)
        .spawn(move || {
            if matches!(
                stop_rx.recv_timeout(timeout),
                Err(RecvTimeoutError::Timeout)
            ) {
                expire_job_by_id(&job_id, Instant::now());
            }
        })
        .map(drop)
}

fn expire_job_by_id(job_id: &str, now: Instant) {
    let mut guard = job_store()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if let Some(job) = guard.as_mut().filter(|job| job.id == job_id) {
        expire_job(job, now);
    }
}

fn design_md_job_id(path: &str) -> Option<&str> {
    let id = path.strip_prefix(DESIGN_MD_JOB_PREFIX)?;
    (id.len() == 32
        && id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')))
    .then_some(id)
}

#[derive(Clone)]
enum JobPoll {
    Pending,
    Finished(DesignMdResult),
    Expired,
    Cancelled,
    NotFound,
}

fn job_poll(req: &crate::mcp_serve::HttpRequest, job_id: &str, now: Instant) -> JobPoll {
    {
        let mut guard = job_store()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(job) = guard.as_mut().filter(|job| job.id == job_id) {
            expire_job(job, now);
            if !request_owns_job(req, job) {
                return JobPoll::NotFound;
            }
            return match &job.state {
                DesignMdJobState::Pending => JobPoll::Pending,
                DesignMdJobState::Expired => JobPoll::Expired,
                DesignMdJobState::Cancelled => JobPoll::Cancelled,
            };
        }
    }
    cleanup_completed(now);
    let completed = completed_jobs()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    completed
        .iter()
        .find(|job| job.id == job_id && request_matches_owner(req, job.owner_origin.as_deref()))
        .map(|job| JobPoll::Finished(job.result.clone()))
        .unwrap_or(JobPoll::NotFound)
}

fn poll_design_md_job<S: std::io::Write>(
    stream: &mut S,
    req: &crate::mcp_serve::HttpRequest,
    job_id: &str,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    let poll = job_poll(req, job_id, Instant::now());
    let response = match &poll {
        JobPoll::Pending => write_pending(stream, job_id, cors_origin),
        JobPoll::Finished(Ok(markdown)) => write_design_http(
            stream,
            "200 OK",
            &serde_json::json!({"ok":true,"markdown":markdown,"intelligent":true}).to_string(),
            cors_origin,
        ),
        JobPoll::Finished(Err(error)) => write_response_error(stream, *error, cors_origin),
        JobPoll::Expired => write_route_error(
            stream,
            "410 Gone",
            "expired",
            "The design analysis job expired.",
            cors_origin,
        ),
        JobPoll::Cancelled => write_route_error(
            stream,
            "410 Gone",
            "cancelled",
            "The design analysis job was cancelled.",
            cors_origin,
        ),
        JobPoll::NotFound => write_route_error(
            stream,
            "404 Not Found",
            "notFound",
            "Design analysis job not found.",
            cors_origin,
        ),
    };
    if response.is_ok() && matches!(poll, JobPoll::Expired | JobPoll::Cancelled) {
        remove_job(job_id, false);
    }
    response
}

fn cancel_design_md_job<S: std::io::Write>(
    stream: &mut S,
    req: &crate::mcp_serve::HttpRequest,
    job_id: &str,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    let mut guard = job_store()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let Some(job) = guard
        .as_mut()
        .filter(|job| job.id == job_id && request_owns_job(req, job))
    else {
        drop(guard);
        cleanup_completed(Instant::now());
        let mut completed = completed_jobs()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let found = completed
            .iter()
            .position(|job| {
                job.id == job_id && request_matches_owner(req, job.owner_origin.as_deref())
            })
            .is_some_and(|index| completed.remove(index).is_some());
        drop(completed);
        if !found {
            return write_route_error(
                stream,
                "404 Not Found",
                "notFound",
                "Design analysis job not found.",
                cors_origin,
            );
        }
        let body = serde_json::json!({"ok":true,"status":"cancelled"}).to_string();
        return write_design_http(stream, "200 OK", &body, cors_origin);
    };
    job.canceled.store(true, Ordering::Release);
    let _ = job.watchdog_stop.try_send(());
    let _ = job.lease.take();
    job.state = DesignMdJobState::Cancelled;
    drop(guard);
    let body = serde_json::json!({"ok":true,"status":"cancelled"}).to_string();
    let response = write_design_http(stream, "200 OK", &body, cors_origin);
    if response.is_ok() {
        remove_job(job_id, false);
    }
    response
}

fn write_pending<S: std::io::Write>(
    stream: &mut S,
    job_id: &str,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    let body = serde_json::json!({
        "ok": true,
        "status": "pending",
        "jobId": job_id,
        "retryAfterMs": DESIGN_MD_RETRY_AFTER_MS,
    })
    .to_string();
    write_design_http(stream, "202 Accepted", &body, cors_origin)
}

fn content_type_is_strict_json(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let mut parts = value.split(';');
    if !parts
        .next()
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
    {
        return false;
    }
    let parameters: Vec<&str> = parts.filter(|part| !part.trim().is_empty()).collect();
    parameters.is_empty()
        || (parameters.len() == 1
            && parameters[0].split_once('=').is_some_and(|(name, value)| {
                name.trim().eq_ignore_ascii_case("charset")
                    && value.trim().eq_ignore_ascii_case("utf-8")
            }))
}

fn write_response_error<S: std::io::Write>(
    stream: &mut S,
    error: DesignMdResponseError,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    let (status, code, message) = response_error_metadata(error);
    write_route_error(stream, status, code, message, cors_origin)
}

fn response_error_metadata(
    error: DesignMdResponseError,
) -> (&'static str, &'static str, &'static str) {
    match error {
        DesignMdResponseError::NoModel => (
            "503 Service Unavailable",
            "noModel",
            "No compatible AI model is configured.",
        ),
        DesignMdResponseError::WorkerSpawn => (
            "503 Service Unavailable",
            "workerSpawn",
            "The design analysis worker could not start.",
        ),
        DesignMdResponseError::ProviderError => (
            "502 Bad Gateway",
            "providerError",
            "The AI provider could not complete design analysis.",
        ),
        DesignMdResponseError::EmptyOutput => (
            "502 Bad Gateway",
            "emptyOutput",
            "The AI provider returned an empty design document.",
        ),
        DesignMdResponseError::InvalidOutput => (
            "502 Bad Gateway",
            "invalidOutput",
            "The AI provider returned an invalid design document.",
        ),
        DesignMdResponseError::OutputTooLarge => (
            "502 Bad Gateway",
            "outputTooLarge",
            "The generated design document is too large.",
        ),
        DesignMdResponseError::Timeout => (
            "504 Gateway Timeout",
            "timeout",
            "Design analysis timed out.",
        ),
    }
}

fn write_route_error<S: std::io::Write>(
    stream: &mut S,
    status: &str,
    code: &str,
    error: &str,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    let body = serde_json::json!({"ok": false, "code": code, "error": error}).to_string();
    write_design_http(stream, status, &body, cors_origin)
}

fn write_design_http<S: std::io::Write>(
    stream: &mut S,
    status: &str,
    body: &str,
    cors_origin: Option<&str>,
) -> Result<(), McpLiveError> {
    let cors_line = cors_origin
        .map(|origin| format!("Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n"))
        .unwrap_or_default();
    let response = format!(
        "HTTP/1.1 {status}\r\n\
         {cors_line}\
         Access-Control-Allow-Methods: POST, GET, DELETE, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         Cache-Control: no-store\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{}",
        body.len(),
        body
    );
    std::io::Write::write_all(stream, response.as_bytes()).map_err(|error| {
        McpLiveError::from(crate::mcp_serve::McpServeError::Io(format!(
            "http write: {error}"
        )))
    })?;
    std::io::Write::flush(stream).map_err(|error| {
        McpLiveError::from(crate::mcp_serve::McpServeError::Io(format!(
            "http flush: {error}"
        )))
    })
}

#[cfg(test)]
#[path = "design_md_route_tests.rs"]
mod tests;
