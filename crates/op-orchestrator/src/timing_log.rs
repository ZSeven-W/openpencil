//! `OPENPENCIL_DEBUG_TIMING=1` — a wall-clock timeline of one
//! [`Orchestrator::run`](crate::Orchestrator::run).
//!
//! Generation latency is felt as "how long until the first section shows
//! up", and nothing measured it: planning and every sub-agent collect their
//! whole LLM answer before anything lands on the canvas, so where that time
//! goes (transport start-up, time to first token, output length, local
//! post-processing) was guesswork. With the switch set, `run` wraps its LLM
//! client, document sink and progress callback, and prints `[TIMING]` lines
//! on stderr — the channel `OPENPENCIL_DEBUG_PLAN` already uses for its
//! `[PLAN]` lines — with every timestamp relative to the start of the run:
//!
//! ```text
//! [TIMING] +0.0s run start
//! [TIMING] +0.0s progress Planning
//! [TIMING] +0.0s call#1 start phase=plan in=29.1KB
//! [TIMING] +38.2s call#1 end phase=plan first-chunk=4.1s first-text=4.1s total=38.2s text=9.8KB thinking=0B
//! [TIMING] +81.0s insert#1 nodes=34 phase=subtask:hero (first section on canvas)
//! ```
//!
//! `phase=` is a hint taken from the latest progress event; concurrent
//! screen-group workers share it, so on that path it names whichever worker
//! reported last.
//!
//! Without the switch every wrapper forwards untouched — the same stream,
//! the same sink calls, the same progress events reach the host — so the
//! generated design is identical either way.

use std::fmt;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::stream::{BoxStream, Stream};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

use crate::types::{CallRequest, DocSink, LlmChunk, LlmClient, LlmError, Progress};

/// Environment switch read once per run.
pub const TIMING_ENV: &str = "OPENPENCIL_DEBUG_TIMING";

/// Longest progress-event / error preview a `[TIMING]` line carries.
const PREVIEW_CHARS: usize = 160;

/// Whether `OPENPENCIL_DEBUG_TIMING` asks for the timeline. Hosts that want
/// to add transport-level lines (CLI start-up, …) gate them on this too.
pub fn enabled() -> bool {
    switch_on(std::env::var(TIMING_ENV).ok().as_deref())
}

/// Unset, empty and `0` are off; any other value is on.
fn switch_on(value: Option<&str>) -> bool {
    value.is_some_and(|v| !v.trim().is_empty() && v.trim() != "0")
}

type Emit = Box<dyn Fn(&str) + Send + Sync>;

struct TimingState {
    started: Instant,
    emit: Emit,
    calls: AtomicUsize,
    inserts: AtomicUsize,
    phase: Mutex<String>,
    scaffold_done: AtomicBool,
    first_section_seen: AtomicBool,
}

impl TimingState {
    fn line(&self, message: fmt::Arguments<'_>) {
        let stamp = secs(self.started.elapsed());
        (self.emit)(&format!("[TIMING] +{stamp} {message}"));
    }

    fn phase(&self) -> String {
        self.phase.lock().map(|p| p.clone()).unwrap_or_default()
    }

    fn set_phase(&self, phase: String) {
        if let Ok(mut current) = self.phase.lock() {
            *current = phase;
        }
    }

    fn note_insert(&self, nodes: usize) {
        if nodes == 0 {
            return;
        }
        let seq = self.inserts.fetch_add(1, Ordering::Relaxed) + 1;
        let first_section = self.scaffold_done.load(Ordering::Relaxed)
            && !self.first_section_seen.swap(true, Ordering::Relaxed);
        let marker = if first_section {
            " (first section on canvas)"
        } else {
            ""
        };
        self.line(format_args!(
            "insert#{seq} nodes={nodes} phase={}{marker}",
            self.phase()
        ));
    }
}

/// One run's timeline. Inert (pure pass-through) unless built enabled.
pub struct RunTiming {
    state: Option<Arc<TimingState>>,
}

impl RunTiming {
    /// Enabled iff [`enabled`] — the production constructor.
    pub fn from_env() -> Self {
        if enabled() {
            Self::with_emitter(Box::new(|line| eprintln!("{line}")))
        } else {
            Self::disabled()
        }
    }

    /// A timeline that records nothing and wraps nothing.
    pub fn disabled() -> Self {
        Self { state: None }
    }

    /// An enabled timeline writing each line through `emit`.
    pub(crate) fn with_emitter(emit: Emit) -> Self {
        let state = Arc::new(TimingState {
            started: Instant::now(),
            emit,
            calls: AtomicUsize::new(0),
            inserts: AtomicUsize::new(0),
            phase: Mutex::new("start".into()),
            scaffold_done: AtomicBool::new(false),
            first_section_seen: AtomicBool::new(false),
        });
        state.line(format_args!("run start"));
        Self { state: Some(state) }
    }

    /// Wrap the run's LLM client so every call reports its latencies.
    pub fn llm<'a>(&self, inner: &'a dyn LlmClient) -> TimedLlm<'a> {
        TimedLlm {
            inner,
            state: self.state.clone(),
        }
    }

    /// Wrap the run's document sink so content inserts are timestamped.
    pub fn sink<'a>(&self, inner: &'a mut dyn DocSink) -> TimedSink<'a> {
        TimedSink {
            inner,
            state: self.state.clone(),
        }
    }

    /// Record a progress event (call before forwarding it to the host).
    pub fn progress(&self, event: &Progress) {
        let Some(state) = &self.state else {
            return;
        };
        if let Some(phase) = phase_hint(event) {
            state.set_phase(phase);
        }
        if matches!(event, Progress::ScaffoldDone) {
            state.scaffold_done.store(true, Ordering::Relaxed);
        }
        state.line(format_args!("progress {}", preview(&format!("{event:?}"))));
    }
}

impl Drop for RunTiming {
    fn drop(&mut self) {
        if let Some(state) = &self.state {
            state.line(format_args!(
                "run end total={} llm-calls={} inserts={}",
                secs(state.started.elapsed()),
                state.calls.load(Ordering::Relaxed),
                state.inserts.load(Ordering::Relaxed)
            ));
        }
    }
}

/// The phase a progress event moves the run into, if it names one.
fn phase_hint(event: &Progress) -> Option<String> {
    Some(match event {
        Progress::Planning => "plan".into(),
        Progress::PlanCoverageRetry { .. } => "replan".into(),
        Progress::Planned { .. } => "scaffold".into(),
        Progress::SubtaskStarted { id, .. } => format!("subtask:{id}"),
        Progress::SubtaskRetry { id, attempt, .. } => format!("subtask:{id}#{attempt}"),
        Progress::GeometryEcho { id, .. } => format!("echo:{id}"),
        Progress::CleanupDone => "finalize".into(),
        Progress::WorkerScoped(worker) => {
            let inner = phase_hint(&worker.event)?;
            format!("g{}/{inner}", worker.group_idx)
        }
        _ => return None,
    })
}

/// `LlmClient` decorator — see the module doc.
pub struct TimedLlm<'a> {
    inner: &'a dyn LlmClient,
    state: Option<Arc<TimingState>>,
}

impl LlmClient for TimedLlm<'_> {
    fn call(&self, req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let Some(state) = &self.state else {
            return self.inner.call(req);
        };
        let seq = state.calls.fetch_add(1, Ordering::Relaxed) + 1;
        let phase = state.phase();
        let input = kb(req.system_prompt.len() + req.user_prompt.len());
        state.line(format_args!("call#{seq} start phase={phase} in={input}"));
        let inner = self.inner.call(req);
        Box::pin(TimedStream {
            inner,
            state: state.clone(),
            seq,
            phase,
            started: Instant::now(),
            first_chunk: None,
            first_text: None,
            text_bytes: 0,
            thinking_bytes: 0,
            error: None,
            finished: false,
        })
    }
}

struct TimedStream {
    inner: BoxStream<'static, Result<LlmChunk, LlmError>>,
    state: Arc<TimingState>,
    seq: usize,
    phase: String,
    started: Instant,
    first_chunk: Option<Duration>,
    first_text: Option<Duration>,
    text_bytes: usize,
    thinking_bytes: usize,
    error: Option<String>,
    finished: bool,
}

impl TimedStream {
    fn record(&mut self, item: &Result<LlmChunk, LlmError>) {
        let at = self.started.elapsed();
        match item {
            Ok(chunk) => {
                self.first_chunk.get_or_insert(at);
                match chunk {
                    LlmChunk::Text(text) => {
                        if !text.is_empty() {
                            self.first_text.get_or_insert(at);
                        }
                        self.text_bytes += text.len();
                    }
                    LlmChunk::Thinking(text) => self.thinking_bytes += text.len(),
                }
            }
            Err(error) => {
                self.error.get_or_insert_with(|| preview(&error.message));
            }
        }
    }

    fn finish(&mut self, how: &str) {
        if self.finished {
            return;
        }
        self.finished = true;
        let error = self
            .error
            .as_deref()
            .map(|e| format!(" error={e:?}"))
            .unwrap_or_default();
        self.state.line(format_args!(
            "call#{} {how} phase={} first-chunk={} first-text={} total={} text={} thinking={}{error}",
            self.seq,
            self.phase,
            opt_secs(self.first_chunk),
            opt_secs(self.first_text),
            secs(self.started.elapsed()),
            kb(self.text_bytes),
            kb(self.thinking_bytes),
        ));
    }
}

impl Stream for TimedStream {
    type Item = Result<LlmChunk, LlmError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let polled = this.inner.as_mut().poll_next(cx);
        match &polled {
            Poll::Ready(Some(item)) => this.record(item),
            Poll::Ready(None) => this.finish("end"),
            Poll::Pending => {}
        }
        polled
    }
}

impl Drop for TimedStream {
    fn drop(&mut self) {
        // Consumers stop polling on an error or an abort; the drop is then
        // the closest observable end of the call.
        let how = if self.error.is_some() {
            "error"
        } else {
            "dropped"
        };
        self.finish(how);
    }
}

/// `DocSink` decorator — forwards every method, timestamps inserts.
pub struct TimedSink<'a> {
    inner: &'a mut dyn DocSink,
    state: Option<Arc<TimingState>>,
}

impl DocSink for TimedSink<'_> {
    fn state(&self) -> &EditorState {
        self.inner.state()
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        let nodes = self.state.as_ref().map_or(0, |_| inserted_node_count(&cmd));
        let applied = self.inner.apply(cmd);
        if let (true, Some(state)) = (applied, &self.state) {
            state.note_insert(nodes);
        }
        applied
    }

    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        let count = self.state.as_ref().map_or(0, |_| count_nodes(&nodes));
        let roots = self
            .inner
            .insert_subtree_returning_root_ids(nodes, parent_id);
        if let (Some(_), Some(state)) = (&roots, &self.state) {
            state.note_insert(count);
        }
        roots
    }

    fn begin_undo_batch(&mut self) {
        self.inner.begin_undo_batch();
    }

    fn end_undo_batch(&mut self) {
        self.inner.end_undo_batch();
    }

    fn rollback_inserted_roots(&mut self, root_ids: &[String]) {
        if let (false, Some(state)) = (root_ids.is_empty(), &self.state) {
            state.line(format_args!(
                "rollback roots={} phase={}",
                root_ids.len(),
                state.phase()
            ));
        }
        self.inner.rollback_inserted_roots(root_ids);
    }

    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }
}

/// Nodes a command inserts — `InsertSubtree` directly or inside a `Batch`
/// (the concurrent path replays each worker's buffer as one batch).
fn inserted_node_count(cmd: &EditorCommand) -> usize {
    match cmd {
        EditorCommand::InsertSubtree { nodes, .. } => count_nodes(nodes),
        EditorCommand::Batch { commands } => commands.iter().map(inserted_node_count).sum(),
        _ => 0,
    }
}

fn count_nodes(nodes: &[PenNode]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + node.children().map_or(0, |c| count_nodes(c)))
        .sum()
}

/// `12.3s` — the one duration format every `[TIMING]` line uses.
pub fn secs(duration: Duration) -> String {
    format!("{:.1}s", duration.as_secs_f64())
}

fn opt_secs(duration: Option<Duration>) -> String {
    duration.map_or_else(|| "-".into(), secs)
}

fn kb(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    }
}

/// First [`PREVIEW_CHARS`] characters, on one line.
fn preview(text: &str) -> String {
    let flat = text.replace(['\n', '\r'], " ");
    let mut out: String = flat.chars().take(PREVIEW_CHARS).collect();
    if flat.chars().count() > PREVIEW_CHARS {
        out.push('…');
    }
    out
}

#[cfg(test)]
#[path = "timing_log_tests.rs"]
mod tests;
