//! 公共 trait 与类型 —— 编排器与外界的全部接口。
//!
//! 副作用只从 [`DocSink`](文档变更)与 [`LlmClient`](LLM 调用)
//! 两个 trait 进出;其余全是数据类型。

use futures::stream::BoxStream;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// Apply an `InsertSubtree` and return the post-remap root ids.
    /// `None` = rejected (document unchanged).
    ///
    /// The default implementation routes through `apply` and returns
    /// `Some(vec![])` on success — post-remap ids are unavailable on
    /// buffered / remote sinks where the real `EditorState` is not local.
    /// Override on immediate-apply sinks (e.g. `VecDocSink`) to surface
    /// the real remapped ids from `EditorState::insert_subtree_returning_root_ids`.
    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        let applied = self.apply(EditorCommand::InsertSubtree {
            nodes,
            parent_id: parent_id.clone(),
            page_id: None,
        });
        if applied {
            Some(vec![])
        } else {
            None
        }
    }
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

// ── S3c: Vision-validation traits + supporting types ──────────────────────────

/// 预校验(pre-validation)出口。host 实现把 TS `runPreValidationFixes`
/// 的七个 pass 注入;stub 实现直接返回零修复计数。
///
/// Port of `design-pre-validation.ts:38-45` shape + spec §4.1.
pub trait PreValidator: Send + Sync {
    /// 在文档 sink 上原地运行所有预校验修复 pass,并返回修复统计。
    fn run_pre_validation_fixes(&self, sink: &mut dyn DocSink) -> PreValidationResult;
}

/// 截图出口。host 实现调用 `SkiaEngine.captureRegion`(Rust 侧走
/// `op_pen_loader::editor_state_to_layout_scene` + raster 导出);stub
/// 返回 `None` 表示"跳过视觉轮次"。
///
/// `state` 是**当前已变更**的文档 —— 校验循环在每一轮 apply 修复**之后**
/// 调用本方法,所以 host 必须渲染 `sink.state()` 此刻持有的实时状态。
/// 这与 `PreValidator::run_pre_validation_fixes` 收 sink 同理(后者读
/// `sink.state().doc`),trait 给 provider 一个看到实时状态的入口而不必
/// 在带外共享一份会过期的快照。
///
/// Port of `design-screenshot.ts` shape + spec §4.1.
pub trait ScreenshotProvider: Send + Sync {
    /// 捕捉画布根帧截图,返回 base64 PNG;`None` 表示不可用 / 跳过。
    fn capture_root_frame(&self, state: &EditorState) -> Option<String>;
}

/// 视觉 LLM 调用出口。host 实现使用多模态 LLM;stub 返回
/// `VisionResponse::Skipped`。
///
/// Port of `validateDesignScreenshot` shape in `design-validation.ts:137-228`
/// + spec §4.1.
pub trait VisionLlmClient: Send + Sync {
    /// 执行一次同步视觉校验调用并返回结果。
    fn validate(&self, req: VisionCallRequest) -> VisionResponse;
}

/// `PreValidator::run_pre_validation_fixes` 的返回值 —— 修复统计。
///
/// Port of the TS `{ total, byCategory }` shape.
#[derive(Debug, Clone, Default)]
pub struct PreValidationResult {
    /// 本次运行所有 pass 合计修复的节点数。
    pub total: usize,
    /// 按 pass 类别细分的修复计数(key = pass 名称)。
    pub by_category: BTreeMap<String, usize>,
}

/// `VisionLlmClient::validate` 的输入 —— 单轮视觉校验请求。
///
/// Port of the call-site params in `validateDesignScreenshot`
/// (`design-validation.ts:137`).
#[derive(Debug, Clone)]
pub struct VisionCallRequest {
    /// 视觉 LLM system prompt。
    pub system: String,
    /// 携带节点树 dump 与修复指令的 user message。
    pub message: String,
    /// 画布截图 base64 PNG。
    pub image_base64: String,
    /// 覆盖模型名称;`None` 表示由 host 决定。
    pub model: Option<String>,
    /// 覆盖 provider;`None` 表示由 host 决定。
    pub provider: Option<String>,
    /// 本轮调用的超时时间。
    pub timeout: Duration,
}

/// `VisionLlmClient::validate` 的返回值。
///
/// `Text` = 模型返回了 JSON 文本;`Skipped` = stub / 截图不可用 / host 选择跳过。
#[derive(Debug, Clone)]
pub enum VisionResponse {
    /// 模型返回了完整文本(JSON 格式,待 `parse_validation_response` 解析)。
    Text(String),
    /// 本轮跳过;可选地附带跳过原因(用于日志)。
    Skipped { reason: Option<String> },
}

/// Bundle of the three vision-validation trait objects.
///
/// Passed once to `Orchestrator::run` and threaded to the post-cleanup hook
/// in each of the 3 execution paths (sequential / dashboard / concurrent).
/// All fields are trait-object references — the concrete types live host-side.
///
/// Approach (c) from the D1 spec: a bundle struct, passed as a single param,
/// avoids adding 3 separate fields to the `Orchestrator` struct and keeps the
/// `run` signature manageable.
pub struct ValidationProviders<'a> {
    /// Pre-validation (pure code checks, no LLM).
    pub pre_validator: &'a dyn PreValidator,
    /// Screenshot capture (provides the vision loop's input image).
    pub screenshot: &'a dyn ScreenshotProvider,
    /// Vision LLM caller (multimodal round evaluation).
    pub vision: &'a dyn VisionLlmClient,
    /// System prompt for the vision LLM — resolved by the host from
    /// `op-ai-skills::resolveSkills('validation', '')` (or passed as a
    /// constant from the Rust skill registry).
    pub system_prompt: String,
}

// ── S4: VisualRefProvider ─────────────────────────────────────────────────────

/// Visual-reference rendering outlet.
///
/// host 实现把 HTML 字符串渲染成 base64 PNG 截图;stub 返回 `None`
/// 表示"跳过视觉参考阶段"。
///
/// Port of the `renderHtmlToScreenshot` call-site shape in
/// `visual-ref-orchestrator.ts:108-122` + spec §4.5.
pub trait VisualRefProvider: Send + Sync {
    /// 将 HTML 字符串渲染为给定像素尺寸的截图,返回 base64 PNG;
    /// `None` 表示不可用 / 跳过。
    fn render_html_to_screenshot(&self, html: &str, width: f64, height: f64) -> Option<String>;
}

// ── S4 end ────────────────────────────────────────────────────────────────────

// ── S3c end ───────────────────────────────────────────────────────────────────

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
    /// Planning produced the FULL subtask list — emitted ONCE right after
    /// planning so the UI can show the complete task checklist upfront (TS
    /// parity), instead of revealing tasks one-by-one as each starts. Pairs
    /// `id` with `label` so the UI can mark each row done on `SubtaskDone`.
    Planned {
        subtasks: Vec<(String, String)>,
    },
    ScaffoldDone,
    SubtaskStarted {
        id: String,
        label: String,
    },
    SubtaskDone {
        id: String,
        node_count: usize,
    },
    SubtaskFailed {
        id: String,
        error: String,
    },
    /// Per-subtask skill-load report — emitted right after the sub-agent
    /// prompt is built (from the merged `SkillLoadReport`). `dropped` carries
    /// `(name, reason_display)` pairs for diagnostics; user-facing activity
    /// deliberately omits skill, token-budget, and context-drop internals.
    SubtaskSkills {
        id: String,
        included: Vec<SkillBrief>,
        dropped: Vec<(String, String)>,
        budget_used: u32,
        budget_max: u32,
    },
    /// Emitted on each subtask retry with the reason (e.g. "zero nodes
    /// generated"). `attempt` is the 1-based attempt number being retried into.
    SubtaskRetry {
        id: String,
        attempt: u8,
        reason: String,
    },
    /// Emitted after each sub-agent LLM reply is applied — lets the UI
    /// show a live node count while the subtask is still running.
    SubtaskNodes {
        id: String,
        nodes_so_far: usize,
    },
    CleanupDone,
    // ── S3c: Vision-validation progress variants ─────────────────────────────
    /// 视觉校验阶段开始(pre-validation 将在此之后立即运行)。
    ValidationStarted,
    /// Pre-validation 完成;`applied` = 总修复数,`by_category` = 分类明细。
    ValidationPreCheckDone {
        applied: usize,
        by_category: BTreeMap<String, usize>,
    },
    /// 某轮视觉 LLM 调用开始。`round` ∈ [1, MAX_VALIDATION_ROUNDS]。
    ValidationRoundStarted {
        round: u8,
    },
    /// 某轮视觉 LLM 调用完成并已应用修复。
    ValidationRoundDone {
        round: u8,
        applied: usize,
        quality_score: u8,
    },
    /// 整个视觉校验阶段完成。`total_applied` = pre + 所有轮次之和。
    ValidationDone {
        total_applied: usize,
    },
    // ── S4: Visual-Ref pipeline progress variants ─────────────────────────────
    /// Visual-ref pipeline started (before design-system generation).
    ///
    /// Port of the entry point in `visual-ref-orchestrator.ts:62`.
    VisualRefStarted,
    /// Design system generated and variables seeded.
    ///
    /// Port of stage 1 in `visual-ref-orchestrator.ts:74-90`.
    VisualRefDesignSystem {
        /// Number of `SetVariable*` commands emitted (one per palette/spacing/radius token).
        var_count: usize,
    },
    /// HTML code generated from the design system.
    ///
    /// Port of stage 2 in `visual-ref-orchestrator.ts:92-106`.
    VisualRefHtmlGenerated {
        /// Byte length of the generated HTML string.
        byte_len: usize,
    },
    /// Screenshot step complete (or skipped when `VisualRefProvider` returns `None`).
    ///
    /// Port of stage 3 in `visual-ref-orchestrator.ts:108-122`.
    VisualRefScreenshotReady {
        /// `true` when the screenshot was skipped (provider returned `None`).
        skipped: bool,
    },
    /// Visual-ref pipeline fell back to plain orchestration.
    ///
    /// Emitted when any stage fails or the provider skips. Port of the
    /// fallback path in `visual-ref-orchestrator.ts:124-140`.
    VisualRefFallback {
        /// Human-readable reason for the fallback.
        reason: String,
    },
}

/// A one-line summary of an included skill, surfaced to the chat UI via
/// `Progress::SubtaskSkills`. Mirrors `op_ai_skills::SkillLoadEntry` minus
/// the `category` field (the UI line doesn't display it).
#[derive(Debug, Clone)]
pub struct SkillBrief {
    pub name: String,
    pub token_count: u32,
    pub truncated: bool,
}

impl SkillBrief {
    /// Build a brief from an `op-ai-skills` report entry (name + token_count +
    /// truncated; category is dropped — the UI line doesn't show it).
    pub fn from_entry(e: &op_ai_skills::SkillLoadEntry) -> SkillBrief {
        SkillBrief {
            name: e.name.clone(),
            token_count: e.token_count,
            truncated: e.truncated,
        }
    }
}

/// Short, user-facing word for a `DropReason` (used in the `▸ dropped:` line).
fn drop_reason_display(reason: &op_ai_skills::DropReason) -> &'static str {
    use op_ai_skills::DropReason::*;
    match reason {
        IntentMiss => "intent",
        BudgetExhausted => "budget",
        TierFiltered => "tier",
        MinimalMode => "minimal",
        ReducedComplexity => "reduced",
        Deduped => "dedup",
        ContentMismatch => "mismatch",
    }
}

/// Decompose a merged `SkillLoadReport` into the four payload parts of
/// `Progress::SubtaskSkills` (included briefs, `(name, reason)` drops,
/// budget_used, budget_max).
pub fn report_to_progress_parts(
    report: &op_ai_skills::SkillLoadReport,
) -> (Vec<SkillBrief>, Vec<(String, String)>, u32, u32) {
    let included = report.included.iter().map(SkillBrief::from_entry).collect();
    let dropped = report
        .dropped
        .iter()
        .map(|d| (d.name.clone(), drop_reason_display(&d.reason).to_string()))
        .collect();
    (included, dropped, report.budget_used, report.budget_max)
}

/// 单个 subtask 的执行结果。`error` 带值但 `node_count > 0` 表示
/// "部分产出"(软错误);`node_count == 0` 表示零节点失败。
#[derive(Debug, Clone)]
pub struct SubtaskOutcome {
    pub id: String,
    pub node_count: usize,
    pub error: Option<String>,
    /// Post-remap ids of the roots this subtask inserted (append-mode
    /// cleanup scopes to exactly these — Component 11). Empty on failure
    /// or when the sink is buffered (ids unavailable until replay).
    pub inserted_root_ids: Vec<String>,
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
    /// 所有已运行 subtask 全部失败(零节点)。
    /// 内含第一个非空错误字符串,方便调用方记录或展示。
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

/// 追加上下文 —— 当用户要求扩展已有页面时由 host 填入。
///
/// Port of `AppendContext` in `apps/web/src/services/ai/ai-types.ts:28-37`.
/// 当存在时,编排器跳过创建新根 frame,将生成的区块插入现有目标 frame。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendContext {
    /// 新 sub-agent 区块应插入的 frame id。
    pub target_parent_id: String,
    /// 目标 frame 的宽度(用于给 sub-agent 确定区域尺寸)。
    pub target_width: f64,
    /// 现有顶层区块的标签列表 —— sub-agent 被告知不要重复这些。
    pub existing_section_labels: Vec<String>,
    /// 目标 frame 属于移动页面(宽度 ≤ 480)时为 true。
    pub is_mobile: bool,
}

/// 编排器输入 —— 一次设计请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    /// 追加模式上下文 —— 仅当 host 检测到 append intent 时填入。
    /// Port of `AIDesignRequest.context.appendContext` in `ai-types.ts:51`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub append_context: Option<AppendContext>,
    /// 是否启用后生成视觉校验循环(S3c)。
    /// 对应 TS `VALIDATION_ENABLED` flag(默认 `true`)。
    /// host 可将其设为 `false` 以跳过整个视觉校验阶段。
    /// Port of `VALIDATION_ENABLED` in `ai-runtime-config.ts:109`.
    #[serde(default = "default_validation_enabled")]
    pub validation_enabled: bool,
    /// Vestigial flag — reserved, never read. The visual-ref pipeline (S4)
    /// was removed as dead code (2026-06-06): it ported a TS pipeline upstream
    /// had already deleted (commit 0f12b6e9) and nothing dispatched on this
    /// flag. Kept (not removed) to avoid churning the ~40 `DesignRequest`
    /// construction sites; defaults `false`.
    #[serde(default = "default_visual_ref_enabled")]
    pub visual_ref_enabled: bool,
}

fn default_validation_enabled() -> bool {
    true
}

fn default_visual_ref_enabled() -> bool {
    false
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

    // ── Task A1: AppendContext + DesignRequest.append_context ─────────────────

    /// AppendContext serde round-trips with all 4 fields (camelCase wire names).
    #[test]
    fn append_context_serde_round_trip() {
        let ctx = AppendContext {
            target_parent_id: "frame-abc".into(),
            target_width: 390.0,
            existing_section_labels: vec!["Hero".into(), "Pricing".into()],
            is_mobile: true,
        };
        let json = serde_json::to_string(&ctx).expect("serialize");
        // Wire names are camelCase
        assert!(
            json.contains("targetParentId"),
            "expected targetParentId in {json}"
        );
        assert!(
            json.contains("targetWidth"),
            "expected targetWidth in {json}"
        );
        assert!(
            json.contains("existingSectionLabels"),
            "expected existingSectionLabels in {json}"
        );
        assert!(json.contains("isMobile"), "expected isMobile in {json}");
        let back: AppendContext = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.target_parent_id, "frame-abc");
        assert_eq!(back.target_width, 390.0);
        assert_eq!(back.existing_section_labels, vec!["Hero", "Pricing"]);
        assert!(back.is_mobile);
    }

    /// DesignRequest accepts append_context: None without breaking compilation.
    #[test]
    fn design_request_append_context_none_compiles() {
        let req = DesignRequest {
            prompt: "test".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,
            visual_ref_enabled: false,
        };
        assert!(req.append_context.is_none());
    }

    /// DesignRequest accepts a populated AppendContext.
    #[test]
    fn design_request_append_context_some_compiles() {
        let ctx = AppendContext {
            target_parent_id: "p1".into(),
            target_width: 1200.0,
            existing_section_labels: vec![],
            is_mobile: false,
        };
        let req = DesignRequest {
            prompt: "extend page".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: Some(ctx),
            validation_enabled: true,
            visual_ref_enabled: false,
        };
        assert!(req.append_context.is_some());
    }

    /// DesignRequest without append_context serializes without the field.
    #[test]
    fn design_request_append_context_omitted_from_json_when_none() {
        let req = DesignRequest {
            prompt: "test".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,
            visual_ref_enabled: false,
        };
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(
            !json.contains("appendContext"),
            "appendContext should be omitted when None, got: {json}"
        );
    }

    // ── Task A1 (S3c): vision validation types ────────────────────────────────

    /// `PreValidator` stub returns zero fixes.
    #[test]
    fn skipped_pre_validator_returns_empty_result() {
        use crate::test_support::SkippedPreValidator;
        let mut sink = VecDocSink::new();
        let result = SkippedPreValidator.run_pre_validation_fixes(&mut sink);
        assert_eq!(result.total, 0);
        assert!(result.by_category.is_empty());
    }

    /// `ScreenshotProvider` stub returns `None`.
    #[test]
    fn skipped_screenshot_provider_returns_none() {
        use crate::test_support::SkippedScreenshotProvider;
        let state = op_editor_core::EditorState::new();
        assert!(SkippedScreenshotProvider
            .capture_root_frame(&state)
            .is_none());
    }

    /// `VisionLlmClient` stub returns `VisionResponse::Skipped`.
    #[test]
    fn skipped_vision_llm_client_returns_skipped() {
        use crate::test_support::SkippedVisionLlmClient;
        let req = VisionCallRequest {
            system: "sys".into(),
            message: "msg".into(),
            image_base64: "img".into(),
            model: None,
            provider: None,
            timeout: std::time::Duration::from_millis(5_000),
        };
        let resp = SkippedVisionLlmClient.validate(req);
        assert!(matches!(resp, VisionResponse::Skipped { .. }));
    }

    /// `Progress::Validation*` variants all compile and pattern-match.
    #[test]
    fn progress_validation_variants_compile() {
        use std::collections::BTreeMap;
        let variants = vec![
            Progress::ValidationStarted,
            Progress::ValidationPreCheckDone {
                applied: 3,
                by_category: BTreeMap::new(),
            },
            Progress::ValidationRoundStarted { round: 1 },
            Progress::ValidationRoundDone {
                round: 1,
                applied: 2,
                quality_score: 7,
            },
            Progress::ValidationDone { total_applied: 5 },
        ];
        // Verify all variants are matchable
        for v in variants {
            match v {
                Progress::ValidationStarted => {}
                Progress::ValidationPreCheckDone { .. } => {}
                Progress::ValidationRoundStarted { .. } => {}
                Progress::ValidationRoundDone { .. } => {}
                Progress::ValidationDone { .. } => {}
                _ => {}
            }
        }
    }

    // ── Task A2: VisualRefProvider trait + SkippedVisualRefProvider stub ─────────

    /// `SkippedVisualRefProvider` returns `None` for any input.
    #[test]
    fn skipped_visual_ref_provider_returns_none() {
        use crate::stub_providers::SkippedVisualRefProvider;
        let p = SkippedVisualRefProvider;
        assert!(p
            .render_html_to_screenshot("<html></html>", 1280.0, 800.0)
            .is_none());
        assert!(p.render_html_to_screenshot("", 0.0, 0.0).is_none());
        assert!(p
            .render_html_to_screenshot("<html><body>Hello</body></html>", 390.0, 844.0)
            .is_none());
    }

    /// `Progress::SubtaskSkills` / `SubtaskRetry` + `SkillBrief` compile and match.
    #[test]
    fn progress_skill_variants_compile() {
        let brief = SkillBrief {
            name: "cjk-typography".into(),
            token_count: 800,
            truncated: false,
        };
        assert_eq!(brief.token_count, 800);
        let skills = Progress::SubtaskSkills {
            id: "header".into(),
            included: vec![brief],
            dropped: vec![("examples".into(), "budget".into())],
            budget_used: 5200,
            budget_max: 12000,
        };
        let retry = Progress::SubtaskRetry {
            id: "header".into(),
            attempt: 2,
            reason: "zero nodes generated".into(),
        };
        for v in [skills, retry] {
            match v {
                Progress::SubtaskSkills { .. } => {}
                Progress::SubtaskRetry { .. } => {}
                _ => {}
            }
        }
    }

    /// `Progress::SubtaskNodes` compile and match.
    #[test]
    fn progress_subtask_nodes_compile() {
        let nodes = Progress::SubtaskNodes {
            id: "body".into(),
            nodes_so_far: 12,
        };
        if let Progress::SubtaskNodes { id, nodes_so_far } = nodes {
            assert_eq!(id, "body");
            assert_eq!(nodes_so_far, 12);
        }
    }

    /// `VisualRefProvider` trait is `Send + Sync`.
    #[test]
    fn visual_ref_provider_is_send_sync() {
        use crate::types::VisualRefProvider;
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn VisualRefProvider>();
    }

    /// `op_orchestrator::SkippedVisualRefProvider` resolves from a host-style import.
    #[test]
    fn skipped_visual_ref_provider_resolves_from_crate_root() {
        use crate::{SkippedVisualRefProvider, VisualRefProvider};
        let p: &dyn VisualRefProvider = &SkippedVisualRefProvider;
        assert!(p
            .render_html_to_screenshot("<p>test</p>", 800.0, 600.0)
            .is_none());
    }

    // ── Task A2 end ───────────────────────────────────────────────────────────────

    // ── Task B2: SkillBrief::from_entry + report_to_progress_parts ───────────────

    /// `report_to_progress_parts` maps a `SkillLoadReport` into SubtaskSkills
    /// payload parts: briefs, (name, reason) drops, budget_used, budget_max.
    #[test]
    fn report_to_progress_parts_maps_entries_and_drops() {
        use op_ai_skills::{
            DropReason, DroppedSkill, SkillCategory, SkillLoadEntry, SkillLoadReport,
        };
        let report = SkillLoadReport {
            included: vec![SkillLoadEntry {
                name: "cjk-typography".into(),
                category: SkillCategory::Domain,
                token_count: 800,
                truncated: true,
            }],
            dropped: vec![DroppedSkill {
                name: "examples".into(),
                reason: DropReason::BudgetExhausted,
            }],
            budget_used: 5200,
            budget_max: 12000,
        };
        let (briefs, drops, used, max) = report_to_progress_parts(&report);
        assert_eq!(briefs.len(), 1);
        assert_eq!(briefs[0].name, "cjk-typography");
        assert_eq!(briefs[0].token_count, 800);
        assert!(briefs[0].truncated);
        assert_eq!(drops, vec![("examples".to_string(), "budget".to_string())]);
        assert_eq!(used, 5200);
        assert_eq!(max, 12000);
    }

    /// All 7 `DropReason` variants map to distinct, non-empty display strings.
    #[test]
    fn drop_reason_all_variants_covered() {
        use op_ai_skills::{DropReason, DroppedSkill, SkillLoadReport};
        let reasons = [
            DropReason::IntentMiss,
            DropReason::BudgetExhausted,
            DropReason::TierFiltered,
            DropReason::MinimalMode,
            DropReason::ReducedComplexity,
            DropReason::Deduped,
            DropReason::ContentMismatch,
        ];
        let expected = [
            "intent", "budget", "tier", "minimal", "reduced", "dedup", "mismatch",
        ];
        for (reason, exp) in reasons.iter().zip(expected.iter()) {
            let report = SkillLoadReport {
                included: vec![],
                dropped: vec![DroppedSkill {
                    name: "skill".into(),
                    reason: *reason,
                }],
                budget_used: 0,
                budget_max: 0,
            };
            let (_, drops, _, _) = report_to_progress_parts(&report);
            assert_eq!(drops[0].1, *exp, "reason {reason:?} should map to {exp}");
        }
    }

    // ── Task B2 end ───────────────────────────────────────────────────────────────

    /// `DesignRequest.validation_enabled` defaults to `true` when omitted from JSON.
    #[test]
    fn design_request_validation_enabled_defaults_true() {
        // JSON without `validationEnabled` field
        let json = r#"{"prompt":"test","concurrency":1}"#;
        let req: DesignRequest = serde_json::from_str(json).expect("deserialize");
        assert!(
            req.validation_enabled,
            "validation_enabled should default to true"
        );
    }

    /// `DesignRequest.validation_enabled` round-trips via serde.
    #[test]
    fn design_request_validation_enabled_serde_roundtrip() {
        let req = DesignRequest {
            prompt: "test".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: false,
            visual_ref_enabled: false,
        };
        let json = serde_json::to_string(&req).expect("serialize");
        let back: DesignRequest = serde_json::from_str(&json).expect("deserialize");
        assert!(!back.validation_enabled);
    }

    /// Config consts have the values faithful to `ai-runtime-config.ts:119-123`.
    #[test]
    fn validation_config_consts_match_ts() {
        use crate::validation_config::{
            MAX_VALIDATION_ROUNDS, VALIDATION_NODE_COUNT_THRESHOLD, VALIDATION_QUALITY_THRESHOLD,
            VALIDATION_TIMEOUT_MS,
        };
        assert_eq!(VALIDATION_NODE_COUNT_THRESHOLD, 30);
        assert_eq!(MAX_VALIDATION_ROUNDS, 3);
        assert_eq!(VALIDATION_QUALITY_THRESHOLD, 8);
        assert_eq!(VALIDATION_TIMEOUT_MS, 180_000);
    }
}
