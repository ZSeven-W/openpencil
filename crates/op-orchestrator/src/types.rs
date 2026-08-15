//! 公共 trait 与类型 —— 编排器与外界的全部接口。
//!
//! 副作用只从 [`DocSink`](文档变更)与 [`LlmClient`](LLM 调用)
//! 两个 trait 进出;其余全是数据类型。

use crate::plan::Subtask;
use futures::stream::BoxStream;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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

/// Run-scoped budget for the `geometry_echo` in-loop self-correction step
/// (`concurrent::run_subtask_retry_ladder`) — caps the TOTAL number of
/// subtasks across one `Orchestrator::run()` call that may spend an extra
/// LLM call self-correcting a geometry violation, so a slow model or a
/// systematically-violating design can't compound retry cost across the
/// whole run. Cheap, cloneable `Arc<AtomicUsize>` semantics — same shape as
/// [`AbortFlag`], threaded the same way (by reference) down to every place
/// that runs a subtask.
#[derive(Debug, Clone)]
pub struct GeometryEchoBudget(Arc<AtomicUsize>);

impl GeometryEchoBudget {
    pub fn new(cap: usize) -> Self {
        Self(Arc::new(AtomicUsize::new(cap)))
    }

    /// Consume one unit of budget iff any remains. Returns `true` when the
    /// caller may proceed with an echo retry — `false` means the run-wide
    /// cap is already exhausted (or was `0` to begin with, e.g. the
    /// `OPENPENCIL_GEOMETRY_ECHO=0` rollback valve — see
    /// [`geometry_echo_cap`]).
    pub(crate) fn try_consume(&self) -> bool {
        self.0
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
    }
}

/// `min(subtask_count, 6)` unless `env_value == Some("0")` (the
/// `OPENPENCIL_GEOMETRY_ECHO=0` rollback safety valve — geometry echo
/// defaults ON), in which case `0`. A zero-budget [`GeometryEchoBudget`]
/// degrades to a no-op exactly like "no budget left", so disabling the env
/// var needs no separate branch anywhere the budget is consumed.
///
/// Takes the env value as a plain `Option<&str>` (rather than reading
/// `std::env::var` itself) so this — the actual cap LOGIC — stays a pure,
/// directly-testable function; only the one-line call site in `run.rs`
/// touches the real process environment (untested glue, matching this
/// crate's `is_self_check_rejection` / thin-adapter convention elsewhere).
pub(crate) fn geometry_echo_cap(subtask_count: usize, env_value: Option<&str>) -> usize {
    if env_value == Some("0") {
        0
    } else {
        subtask_count.min(6)
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
    /// Emitted once, right before a `geometry_echo` in-loop self-correction
    /// retry starts (`concurrent::run_subtask_retry_ladder`'s tail) — a
    /// fact line so the progress panel shows this step actually working,
    /// not a silent extra LLM call. `issue_count` is how many diagnostic
    /// lines `geometry_validation::geometry_diagnostics_for_roots` found.
    GeometryEcho {
        id: String,
        issue_count: usize,
    },
    /// Emitted ONCE, right before `RunSummary` is returned, whenever the
    /// "promise-delivery" invariant check (`unfilled_screens::detect_unfilled_screens`)
    /// finds a scaffolded screen that never received real content — the
    /// classic-path honest report (postmortem 0718-1-glm-1: a silently
    /// delivered blank screen). `names` are already marked on the canvas
    /// (`" (unfilled)"` suffix) by the time this fires. Never emitted when
    /// nothing is unfilled.
    UnfilledScreens {
        names: Vec<String>,
    },
    /// Emitted after each sub-agent LLM reply is applied — lets the UI
    /// show a live node count while the subtask is still running.
    SubtaskNodes {
        id: String,
        nodes_so_far: usize,
    },
    /// Emitted ONCE, right before the screen-group concurrent phase starts
    /// (D-lite "three-piece" visibility fix, 2026-07-17) — a user-facing
    /// confirmation that `group_count` screens are about to run against
    /// `workers` overlapping workers, so ⚡Nx's effect is legible instead of
    /// silent. Never emitted on the sequential path (`workers == 1` never
    /// fires this — see `run.rs`'s `effective_concurrency` branch).
    ConcurrentGroupsStarted {
        group_count: usize,
        workers: u32,
    },
    /// The dual of [`Progress::ConcurrentGroupsStarted`] — emitted ONCE when
    /// the plan has ≥2 screen groups but the SEQUENTIAL phase-3 loop ran
    /// anyway (`effective_concurrency == 1`). Self-diagnostic
    /// (sequential-execution root-cause hunt, 2026-07-17): `requested_workers`
    /// is the raw, unclamped `DesignRequest.concurrency` this turn carried —
    /// `1` proves the ⚡Nx picker's value never reached the orchestrator for
    /// this turn; any value `> 1` here would mean `effective_concurrency`
    /// itself has a bug (its contract guarantees `> 1` whenever
    /// `group_count > 1 && clamp_concurrency(requested_workers) > 1`).
    ScreenGroupsSequential {
        group_count: usize,
        requested_workers: u32,
    },
    /// Progress emitted by one screen-group agent in the concurrent classic
    /// orchestrator path. The boxed inner event keeps the recursive enum
    /// finite-sized while preserving the ordinary [`Progress`] vocabulary for
    /// subtask lifecycle updates.
    ///
    /// `group_idx` identifies the screen group, not a semaphore worker slot:
    /// three screen groups running with a concurrency limit of two still have
    /// three stable identities and three independent progress streams.
    WorkerScoped(WorkerEvent),
    CleanupDone,
    /// The user-visible quality credential for the classic path — emitted
    /// once, right after [`Progress::CleanupDone`], carrying what the cleanup
    /// passes actually checked and repaired
    /// (`crate::repair_summary::RepairSummary`, flattened to wire strings so
    /// hosts render it without depending on this crate's enum).
    ///
    /// A separate variant rather than a payload on `CleanupDone` so existing
    /// `CleanupDone` matchers (ordering assertions, the "polishing" narration)
    /// keep working untouched.
    ///
    /// Deliberately carries NO leftover-issue count: the promise-delivery
    /// check (`Progress::UnfilledScreens`) runs later in the pipeline, so
    /// anything claimed here about remaining work would be a guess. Renderers
    /// must omit that clause rather than assume "none".
    QualityChecked {
        /// Check-family keys that ran, in display order.
        checks: Vec<String>,
        /// `(check family, document edits applied)` for families that
        /// repaired something.
        repairs: Vec<(String, usize)>,
        /// One already-rendered line per applied edit, in application order
        /// (`crate::repair_record::RepairRecord::line`). The itemized half of
        /// the same tally `repairs` counts — see `RepairSummary`, where both
        /// are derived from one list so they cannot disagree.
        records: Vec<String>,
        /// Statements about the run that are not edits — today, the
        /// deliberate skip of a whole pass tier for authored template input
        /// (`crate::repair_tier`). Never counted as repairs; rendered ahead
        /// of them, because "this was not run" outranks "this was".
        notes: Vec<String>,
    },
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

/// Stable context attached to one screen group's progress events.
#[derive(Debug, Clone)]
pub struct WorkerEvent {
    pub group_idx: usize,
    pub screen: String,
    pub identity: crate::agent_identity::AgentIdentity,
    pub event: Box<Progress>,
}

impl Progress {
    /// Wrap an ordinary progress event with its screen-group identity.
    pub fn worker_scoped(
        group_idx: usize,
        screen: impl Into<String>,
        identity: crate::agent_identity::AgentIdentity,
        event: Progress,
    ) -> Self {
        Self::WorkerScoped(WorkerEvent {
            group_idx,
            screen: screen.into(),
            identity,
            event: Box::new(event),
        })
    }
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
        ModelFamilyMiss => "family",
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
    /// The persisted subtask spec, present ONLY on a zero-node failure —
    /// carries `region`/`elements`/`screen`/`parent_frame_id` through to the
    /// host so a failed row's manual "Retry" button (progress-panel remedy,
    /// see `crate::retry_subtask`) can re-run this EXACT subtask later,
    /// instead of a re-derived approximation. `None` on success (nothing to
    /// retry) — avoids cloning the spec on the common path.
    pub subtask: Option<Subtask>,
}

/// `run()` 成功返回的汇总。
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub root_frame_id: String,
    pub subtasks: Vec<SubtaskOutcome>,
    pub total_nodes: usize,
    /// Names of top-level "screen" roots (`unfilled_screens::detect_unfilled_screens`)
    /// that never received real content by the time the run finished — the
    /// "promise-delivery" invariant's classic-path honest report. Empty on
    /// the common path. Each name is also already marked on the canvas
    /// itself (`unfilled_screens::mark_unfilled_screens`'s " (unfilled)"
    /// suffix) before this summary is built, so a caller that only reads
    /// this field and one that only looks at the canvas see the same story.
    pub unfilled_screens: Vec<String>,
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

/// Existing-canvas facts for a request that creates sibling screens.
///
/// Unlike [`AppendContext`], this does not target an existing parent. It
/// carries the artboard contract and the exact screens the user promised so
/// planning failure can still fan out into correctly-sized top-level roots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuationContext {
    pub screen_width: f64,
    pub screen_height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    pub screen_names: Vec<String>,
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
    /// Sibling-screen continuation facts derived from the live canvas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_context: Option<ContinuationContext>,
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
    /// Style guide the user pinned in the Asset Center, by `name`.
    ///
    /// When it names a guide the registry still carries, planning uses that
    /// one instead of ranking the catalog against the prompt — the pin is the
    /// user overriding the inference, so a prompt that reads "fintech" must
    /// not pull the fintech guide out from under a pinned brutalist one. A
    /// name the registry has dropped falls back to the ranking with a log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_style_guide: Option<String>,
}

/// Mirrors the serde defaults exactly, so a request built through `Default`
/// and one decoded from `{"prompt":"…"}` are the same request. Its purpose is
/// to let call sites name only the fields they care about — a request struct
/// with a dozen fields is otherwise re-listed in full at every site, and each
/// new field becomes a mechanical edit across all of them.
impl Default for DesignRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            continuation_context: None,
            validation_enabled: default_validation_enabled(),
            visual_ref_enabled: default_visual_ref_enabled(),
            pinned_style_guide: None,
        }
    }
}

fn default_validation_enabled() -> bool {
    true
}

fn default_visual_ref_enabled() -> bool {
    false
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
