//! 测试桩 —— crate 内各测试模块共用。仅在 `cfg(test)` 下编译。

use crate::types::{CallRequest, DocSink, LlmChunk, LlmClient, LlmError};
use futures::stream::BoxStream;
use futures::Stream;
use op_editor_core::{EditorCommand, EditorState};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

/// 录命令 + 持内存 `EditorState` 的 `DocSink` 实现。
pub(crate) struct VecDocSink {
    pub state: EditorState,
    pub applied: Vec<EditorCommand>,
    pub batch_depth: i32,
}

impl VecDocSink {
    pub(crate) fn new() -> Self {
        Self {
            state: EditorState::new(),
            applied: Vec::new(),
            batch_depth: 0,
        }
    }
}

impl DocSink for VecDocSink {
    fn state(&self) -> &EditorState {
        &self.state
    }
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.applied.push(cmd.clone());
        self.state.apply(cmd)
    }
    fn begin_undo_batch(&mut self) {
        self.batch_depth += 1;
    }
    fn end_undo_batch(&mut self) {
        self.batch_depth -= 1;
    }
}

/// 一次脚本化的 LLM 响应。
pub(crate) enum ScriptResponse {
    /// 一段成功文本(作为单个 `Text` chunk 返回)。
    Text(String),
    /// 一次失败。
    Fail(LlmError),
}

/// 按脚本逐次返回响应的 `LlmClient` 桩 —— 每次 `call` 弹一条。
pub(crate) struct ScriptedLlm {
    responses: Mutex<VecDeque<ScriptResponse>>,
}

impl ScriptedLlm {
    pub(crate) fn new(responses: Vec<ScriptResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl LlmClient for ScriptedLlm {
    fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let next = self.responses.lock().unwrap().pop_front();
        let items: Vec<Result<LlmChunk, LlmError>> = match next {
            Some(ScriptResponse::Text(t)) => vec![Ok(LlmChunk::Text(t))],
            Some(ScriptResponse::Fail(e)) => vec![Err(e)],
            None => vec![Err(LlmError {
                message: "scripted LLM exhausted".into(),
                aborted: false,
            })],
        };
        Box::pin(futures::stream::iter(items))
    }
}

/// Shared in-flight counters for [`CountingLlm`].
struct CallCounters {
    /// Current number of in-flight calls (a call is "in-flight" from the
    /// moment `call` returns until its stream yields the text chunk).
    current: AtomicUsize,
    /// Maximum observed concurrent in-flight count.
    max_concurrent: AtomicUsize,
}

impl CallCounters {
    /// Record one new in-flight call: bump `current` and update `max_concurrent`.
    fn enter(&self) {
        let new_val = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        let mut cur_max = self.max_concurrent.load(Ordering::SeqCst);
        while new_val > cur_max {
            match self.max_concurrent.compare_exchange(
                cur_max,
                new_val,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => cur_max = actual,
            }
        }
    }

    /// Record one call completing: decrement `current`.
    fn leave(&self) {
        self.current.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A `Stream` that **yields `Poll::Pending` once** before producing its single
/// text chunk.  The initial pending poll is what lets `futures::future::join_all`
/// poll a *different* worker — so multiple `CountingLlm` calls genuinely overlap
/// (their `current` counter is > 1 simultaneously) instead of running strictly
/// one-at-a-time.
///
/// Lifecycle vs the counter:
/// - The call's `enter()` is recorded by `CountingLlm::call` BEFORE the stream
///   is built, so the call counts as in-flight immediately.
/// - The stream's first poll registers a self-wake and returns `Pending` — the
///   executor is now free to poll another worker, raising `current`.
/// - The stream's second poll calls `leave()` (the call completes) and returns
///   the text chunk.
/// - The third poll returns `None` (stream end).
struct YieldingChunkStream {
    yielded_once: bool,
    /// `Some` until the text chunk has been emitted.
    text: Option<String>,
    counters: Arc<CallCounters>,
}

impl Stream for YieldingChunkStream {
    type Item = Result<LlmChunk, LlmError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if !this.yielded_once {
            // First poll: yield control so join_all can poll another worker.
            this.yielded_once = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        match this.text.take() {
            Some(t) => {
                // Second poll: the call completes — decrement the in-flight count.
                this.counters.leave();
                Poll::Ready(Some(Ok(LlmChunk::Text(t))))
            }
            None => Poll::Ready(None),
        }
    }
}

/// An `LlmClient` that tracks the maximum number of concurrent in-flight calls.
///
/// Used in Task B2 tests to verify the semaphore genuinely caps concurrency.
/// Each scripted response is a `String` (node JSON text — always succeeds).
///
/// Crucially the returned stream **yields once** before producing its chunk
/// (see [`YieldingChunkStream`]), so under `join_all` multiple workers' calls
/// genuinely overlap — `max_concurrent` reflects real wall-clock overlap, not a
/// vacuous always-1.
pub(crate) struct CountingLlm {
    responses: Mutex<VecDeque<String>>,
    counters: Arc<CallCounters>,
}

impl CountingLlm {
    pub(crate) fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            counters: Arc::new(CallCounters {
                current: AtomicUsize::new(0),
                max_concurrent: AtomicUsize::new(0),
            }),
        }
    }

    /// Returns the maximum number of concurrent in-flight calls observed.
    pub(crate) fn max_concurrent(&self) -> usize {
        self.counters.max_concurrent.load(Ordering::SeqCst)
    }
}

impl LlmClient for CountingLlm {
    fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        // Record the call as in-flight immediately (before the stream is polled).
        self.counters.enter();

        let text = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_default();

        Box::pin(YieldingChunkStream {
            yielded_once: false,
            text: Some(text),
            counters: Arc::clone(&self.counters),
        })
    }
}
