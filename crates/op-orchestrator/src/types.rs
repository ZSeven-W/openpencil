//! 公共 trait 与类型 —— 编排器与外界的全部接口。
//!
//! 副作用只从 [`DocSink`](文档变更)与 [`LlmClient`](LLM 调用)
//! 两个 trait 进出;其余全是数据类型。

use futures::stream::BoxStream;
use op_editor_core::{EditorCommand, EditorState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 文档出口。写经 [`apply`](DocSink::apply);读经
/// [`state`](DocSink::state)。host 实现把 apply 与存盘 / 重绘 /
/// undo 批界绑在一处(对齐 `op-mcp` 的 applier 模式)。
pub trait DocSink: Send {
    /// 只读访问当前文档 —— cleanup 判定空 scaffold、未来 S3c
    /// 校验都要读。
    fn state(&self) -> &EditorState;
    /// 应用一条编辑命令;返回 `false` 表示命令被拒(文档未变)。
    fn apply(&mut self, cmd: EditorCommand) -> bool;
    /// 开启一个 undo 批 —— 批内的所有 apply 合并为一次 undo。
    fn begin_undo_batch(&mut self);
    /// 关闭当前 undo 批。
    fn end_undo_batch(&mut self);
}

/// LLM 调用出口。每次 [`call`](LlmClient::call) 是一次独立、无累积
/// 上下文的 LLM turn —— host 实现应为每次调用新建引擎,使规划与
/// 各 sub-agent 拿到隔离上下文。
pub trait LlmClient: Send + Sync {
    fn call(&self, req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>>;
}

/// 一次 LLM 调用的完整输入。字段一次定全,S3b 并发 / 流式不必
/// 改 trait 签名。
#[derive(Debug, Clone)]
pub struct CallRequest {
    pub system_prompt: String,
    pub user_prompt: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub timeout: Duration,
    pub abort: AbortFlag,
    /// 从请求开始到收到第一个文本 chunk 的超时;`None` 表示不设。
    /// Port of `noTextTimeoutMs` in the TS timeout profiles.
    pub no_text_timeout: Option<Duration>,
    /// 从第一个文本 chunk 到"真正内容"出现的超时;`None` 表示不设。
    /// Port of `firstTextTimeoutMs` in the TS timeout profiles.
    pub first_text_timeout: Option<Duration>,
}

/// 流元素 —— 区分文本与思考,与 TS 的 text/thinking/error 三分
/// 一致。错误走 `Result` 的 `Err(LlmError)`,不混入 chunk。
#[derive(Debug, Clone)]
pub enum LlmChunk {
    Text(String),
    Thinking(String),
}

/// 一次 LLM 调用的失败。`aborted` 区分用户中止与真实错误。
#[derive(Debug, Clone)]
pub struct LlmError {
    pub message: String,
    pub aborted: bool,
}

/// 廉价可克隆的中止句柄(`Arc<AtomicBool>` 语义)。
#[derive(Debug, Clone, Default)]
pub struct AbortFlag(Arc<AtomicBool>);

impl AbortFlag {
    pub fn new() -> Self {
        Self::default()
    }
    /// 置位 —— 之后所有 `is_set` 返回 `true`。
    pub fn set(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    /// 是否已被中止。
    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// 规划 prompt 的构造模式 —— TS rich/minimal/compact 三档。
/// Plan B 的格式化器与 Plan C 的 `build_orchestrator_prompt` 共用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningMode {
    Rich,
    Minimal,
    Compact,
}

/// `build_orchestrator_prompt` 的产物 —— 比裸 `CallRequest` 多带
/// compact 模式的 `forced_style_guide_name`(S3b-1b 回填 plan 用)。
#[derive(Debug, Clone)]
pub struct PlanningPrompt {
    pub call_request: CallRequest,
    /// compact 模式预选的 styleGuideName;rich/minimal 为 None。
    pub forced_style_guide_name: Option<String>,
    pub mode: PlanningMode,
}

/// 用户消息的意图分类 —— 决定走编排器还是普通聊天。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Design,
    Chat,
}

/// `run()` 的进度回调载荷。字段从 S3a 定全,S3b/S3c 只填新分支。
#[derive(Debug, Clone)]
pub enum Progress {
    Planning,
    ScaffoldDone,
    SubtaskStarted { id: String, label: String },
    SubtaskDone { id: String, node_count: usize },
    SubtaskFailed { id: String, error: String },
    CleanupDone,
}

/// 单个 subtask 的执行结果。`error` 带值但 `node_count > 0` 表示
/// "部分产出"(软错误);`node_count == 0` 表示零节点失败。
#[derive(Debug, Clone)]
pub struct SubtaskOutcome {
    pub id: String,
    pub node_count: usize,
    pub error: Option<String>,
}

/// `run()` 成功返回的汇总。
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub root_frame_id: String,
    pub subtasks: Vec<SubtaskOutcome>,
    pub total_nodes: usize,
}

/// `run()` 的失败。
#[derive(Debug, Clone)]
pub enum OrchestratorError {
    /// 用户中途取消。
    Aborted,
    /// 跑完但未产出任何真实内容。
    NoContent,
    /// 并发路径:所有 screen-group worker 全部失败(零节点)。
    /// 内含第一个非空错误字符串,方便调用方记录或展示。
    /// Port of `orchestrator-sub-agent.ts:321-325` throw path.
    AllFailed(String),
    /// 内部错误(意外情况)。
    Internal(String),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrchestratorError::Aborted => write!(f, "orchestration aborted by user"),
            OrchestratorError::NoContent => write!(f, "orchestration produced no content"),
            OrchestratorError::AllFailed(m) => write!(f, "orchestration failed: {m}"),
            OrchestratorError::Internal(m) => write!(f, "orchestration internal error: {m}"),
        }
    }
}

impl std::error::Error for OrchestratorError {}

/// 编排器输入 —— 一次设计请求。
#[derive(Debug, Clone)]
pub struct DesignRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    /// 当前文档的 design.md(若有)—— 规划 prompt 据此走 design.md 分支。
    pub design_md: Option<jian_ops_schema::DesignMdSpec>,
    /// 并发度:允许同时运行的 screen-group worker 数。
    /// 调用方应传 store-clamped 值 [1,6];crate 内部防御性 clamp。
    /// 默认为 1(顺序执行)。Port of TS `request.concurrency ?? 1`.
    pub concurrency: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::VecDocSink;

    #[test]
    fn vec_doc_sink_implements_docsink() {
        let mut sink = VecDocSink::new();
        sink.begin_undo_batch();
        sink.end_undo_batch();
        assert_eq!(sink.batch_depth, 0);
        assert!(sink.applied.is_empty());
        let _ = sink.state();
    }

    #[test]
    fn abort_flag_sets_and_reads() {
        let flag = AbortFlag::new();
        assert!(!flag.is_set());
        let clone = flag.clone();
        flag.set();
        assert!(clone.is_set());
    }
}
