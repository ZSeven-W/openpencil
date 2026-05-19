//! host 侧的编排器接线 —— `op_orchestrator` 的两个 trait 在 desktop
//! 上的实现,以及把一次设计请求跑起来的入口。
//!
//! `op-orchestrator` 是纯 crate,副作用只走 `DocSink`(文档变更)
//! 与 `LlmClient`(LLM 调用)两个 trait。本模块提供:
//!
//! - [`DesktopLlmClient`] —— 用 `agent::QueryEngine` 实现 `LlmClient`,
//!   每次 `call` 新建一个引擎(隔离上下文,见 spec §3.2)。
//! - [`DesktopDocSink`] —— 用 `op_editor_core::EditorState` 实现
//!   `DocSink`。
//! - [`run_design_request`] —— intent gate 之后的入口:跑
//!   `Orchestrator::run()`。
//!
//! ## 仍待 host 作者接的线(casement 阻塞,本文件未经编译)
//!
//! 1. **intent gate**:`chat_runtime.rs` 收到用户消息后先过
//!    `op_orchestrator::classify_intent`;`Design` → 本模块,`Chat`
//!    → 现有 `BuiltInProvider` 单发路径。本模块不动 `chat_runtime.rs`
//!    的线程模型,留给 host 作者接。
//! 2. **线程模型**:`Orchestrator::run()` 是 `async` 且要 `&mut
//!    EditorState`;桌面文档在 `WidgetHostNative`(UI 线程)。最简
//!    正确接法是在文档所在处 `block_on(run(...))`,LLM 的 async 由
//!    `DesktopLlmClient` 内部 `shared_runtime().spawn` 消化。
//! 3. **undo 批界**:`DesktopDocSink::begin/end_undo_batch` 目前是
//!    no-op —— `op-editor-core` 的 History batch API 待确认后接上
//!    (见各方法的注释)。

use std::sync::Arc;

use agent::abort::AbortController;
use agent::provider::Provider;
use agent::query::QueryEngine;
use agent::stream::Event;
use futures::channel::mpsc;
use futures::StreamExt;
use op_editor_core::{EditorCommand, EditorState};
use op_orchestrator::{
    AbortFlag, CallRequest, DesignRequest, DocSink, LlmChunk, LlmClient, LlmError, Orchestrator,
    OrchestratorError, Progress, RunSummary,
};

/// `LlmClient` 的 desktop 实现。每次 `call` 新建一个 `QueryEngine`
/// —— 规划与各 sub-agent 因此拿到互相隔离的对话上下文(不复用
/// `BuiltInProvider` 那个累积历史的共享引擎)。
pub struct DesktopLlmClient {
    provider: Arc<dyn Provider>,
    /// 缺省模型 —— `CallRequest.model` 为 `None` 时用它。
    default_model: String,
}

impl DesktopLlmClient {
    pub fn new(provider: Arc<dyn Provider>, default_model: impl Into<String>) -> Self {
        Self {
            provider,
            default_model: default_model.into(),
        }
    }
}

impl LlmClient for DesktopLlmClient {
    fn call(&self, req: CallRequest) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();

        // 调用前已中止 —— 直接给一个 aborted 错误流。
        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }

        let provider = self.provider.clone();
        let model = req.model.clone().unwrap_or_else(|| self.default_model.clone());
        let system = req.system_prompt.clone();
        let user = req.user_prompt.clone();

        crate::chat_runtime::shared_runtime().spawn(async move {
            let engine = QueryEngine::new(provider, model).with_system(system);
            let abort = AbortController::new();
            let stream = match engine.run(user, abort).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.unbounded_send(Err(LlmError {
                        message: e.to_string(),
                        aborted: false,
                    }));
                    return;
                }
            };
            let mut stream = stream;
            while let Some(item) = stream.next().await {
                let sent = match item {
                    Ok(Event::TextDelta { delta }) => tx.unbounded_send(Ok(LlmChunk::Text(delta))),
                    Ok(Event::Thinking { delta }) => {
                        tx.unbounded_send(Ok(LlmChunk::Thinking(delta)))
                    }
                    Ok(Event::Result { .. }) => break,
                    Ok(Event::Error { code, message }) => tx.unbounded_send(Err(LlmError {
                        message: format!("{code}: {message}"),
                        aborted: false,
                    })),
                    // ToolUse / ToolResult / Usage / 其它 —— 编排器
                    // 只要文本,静默跳过。
                    Ok(_) => Ok(()),
                    Err(e) => tx.unbounded_send(Err(LlmError {
                        message: e.to_string(),
                        aborted: false,
                    })),
                };
                if sent.is_err() {
                    break; // 接收端已丢弃
                }
            }
        });

        Box::pin(rx)
    }
}

/// `DocSink` 的 desktop 实现 —— 直接操作 host 的 `EditorState`。
/// 要求 `Orchestrator::run()` 在文档所在线程上执行(见模块注释)。
pub struct DesktopDocSink<'a> {
    state: &'a mut EditorState,
}

impl<'a> DesktopDocSink<'a> {
    pub fn new(state: &'a mut EditorState) -> Self {
        Self { state }
    }
}

impl DocSink for DesktopDocSink<'_> {
    fn state(&self) -> &EditorState {
        self.state
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.state.apply(cmd)
    }

    fn begin_undo_batch(&mut self) {
        // TODO(host): 接 op-editor-core 的 History batch 模式,使整轮
        // 编排合并为一次 undo。未接前每条 InsertSubtree 是独立 undo
        // 步 —— 功能正确,只是 undo 粒度偏细。
    }

    fn end_undo_batch(&mut self) {
        // TODO(host): 同 begin_undo_batch。
    }
}

/// intent gate 之后的入口 —— 对一次设计请求跑编排器。
///
/// `on_progress` 把 [`Progress`] 转成 UI 能消费的形式(step 标签等)。
/// 调用方负责在文档所在线程上 `block_on` 本 future。
pub async fn run_design_request(
    prompt: String,
    model: Option<String>,
    provider_id: Option<String>,
    state: &mut EditorState,
    llm: &dyn LlmClient,
    on_progress: &mut dyn FnMut(Progress),
    abort: &AbortFlag,
) -> Result<RunSummary, OrchestratorError> {
    let request = DesignRequest {
        prompt,
        model,
        provider: provider_id,
    };
    let mut sink = DesktopDocSink::new(state);
    Orchestrator::new()
        .run(request, &mut sink, llm, on_progress, abort)
        .await
}
