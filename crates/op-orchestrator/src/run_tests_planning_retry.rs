//! Single-mode planning failures and the sub-agent 3-attempt tier-gated
//! retry ladder.

use super::*;

// ── single-mode planning tests ───────────────────────────────────────────

/// Planning parse failure → heuristic fallback plan used, run succeeds.
/// (Single-mode planning: one bad response falls straight through to the
/// fallback plan — no mode rotation. The fallback plan has one subtask.)
#[test]
fn planning_parse_failure_uses_fallback_plan() {
    let llm = ScriptedLlm::new(vec![
        // planning attempts 1 + 2 → bad JSON (the retry consumes one more)
        ScriptResponse::Text("not valid json at all".into()),
        ScriptResponse::Text("not valid json at all".into()),
        // fallback plan's single subtask
        ScriptResponse::Text(node_json("section-1")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("fallback after parse failure ok");
    assert!(summary.total_nodes >= 1);
}

/// Planning stream error → heuristic fallback plan used, run succeeds.
#[test]
fn planning_stream_error_uses_fallback_plan() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        // planning attempts 1 + 2 → stream error (non-abort); the retry
        // consumes the second before the heuristic fallback engages.
        ScriptResponse::Fail(LlmError {
            message: "HTTP 500 upstream".into(),
            aborted: false,
        }),
        ScriptResponse::Fail(LlmError {
            message: "HTTP 500 upstream".into(),
            aborted: false,
        }),
        // fallback plan's single subtask
        ScriptResponse::Text(node_json("section-1")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("fallback on stream error ok");
    assert!(summary.total_nodes >= 1);
}

/// Abort during planning stream → `OrchestratorError::Aborted`.
#[test]
fn planning_abort_during_stream_returns_aborted() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
        message: "user aborted".into(),
        aborted: true,
    })]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let abort = AbortFlag::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &abort,
        &stub_providers(),
    ));
    assert!(matches!(result, Err(OrchestratorError::Aborted)));
    // undo batch 在 abort 路径前返回,文档不应已进入批
    assert_eq!(sink.batch_depth, 0);
}

// ── Task C3: sub-agent 3-attempt tier-gated retry ladder ──────────────────

/// Subtask returns zero nodes on attempt 1 but succeeds on attempt 2 →
/// the subtask's nodes land (ladder retries once).
/// Uses Full tier (attempt 2: reduced_complexity=false, minimal_skills=false).
#[test]
fn subtask_retries_on_attempt1_zero_succeeds_on_attempt2() {
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: garbage (0 nodes, retryable)
        ScriptResponse::Text("the model gave garbage".into()),
        // subtask hero — attempt 2: success
        ScriptResponse::Text(node_json("hero")),
        // subtask feat — attempt 1: success
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(), // Full tier → reduced_complexity=false on attempt 2
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("retry succeeded");
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    assert_eq!(sink.batch_depth, 0);
}

/// A self-check quality rejection on attempt 1 must NOT downgrade attempt
/// 2's skill tier, even on a Basic-tier model that would otherwise always
/// narrow to `retryAllowed` on retry — the content was real, just flagged
/// for one geometry issue, so throwing skills away only makes the rest of
/// the design worse. Attempt 2's prompt must also carry the rejection
/// reason so the model can fix exactly that issue.
#[test]
fn self_check_rejection_keeps_full_skills_and_injects_feedback_on_attempt2() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        // hero attempt 1 (Basic tier, full complexity): parses fine, but
        // self-check fatally rejects the mismatched ring — zero nodes land.
        ScriptResponse::Text(radial_reject_script()),
        // hero attempt 2: must stay full complexity despite Basic tier.
        ScriptResponse::Text(node_json("hero")),
        // feat attempt 1: succeeds normally.
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_basic(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("attempt 2 recovers after the self-check rejection");
    assert_eq!(summary.subtasks.len(), 2);

    // Call order: [0] planning, [1] hero attempt 1, [2] hero attempt 2, [3] feat.
    let prompts = llm.system_prompts();
    assert_eq!(prompts.len(), 4, "unexpected call count: {prompts:?}");
    assert_eq!(
        prompts[1], prompts[2],
        "attempt 2 after a self-check rejection must resolve the IDENTICAL \
         (full) skill set attempt 1 used — a Basic-tier model would \
         otherwise narrow this to the retryAllowed set"
    );
}

/// The mirror case: an attempt-1 TRANSPORT failure (not a self-check
/// rejection) on a Basic-tier model must still downgrade attempt 2 to
/// `reduced_complexity`, exactly as before this task — skill downgrade
/// stays reserved for failures that suggest the model is struggling with
/// the full prompt, not for a quality gate on otherwise-fine content.
#[test]
fn transport_failure_still_downgrades_skills_on_attempt2_for_basic_tier() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        // hero attempt 1: a stream error — NOT a self-check rejection.
        ScriptResponse::Fail(LlmError {
            message: "stream disconnected before completion".into(),
            aborted: false,
        }),
        // hero attempt 2: reduced_complexity narrows the skill set.
        ScriptResponse::Text(node_json("hero")),
        // feat attempt 1: succeeds normally.
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_basic(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("attempt 2 recovers after the transport failure");
    assert_eq!(summary.subtasks.len(), 2);

    let prompts = llm.system_prompts();
    assert_eq!(prompts.len(), 4, "unexpected call count: {prompts:?}");
    assert!(
        prompts[2].len() < prompts[1].len(),
        "attempt 2 after a plain transport failure must still narrow to the \
         reduced-complexity skill set on Basic tier (attempt 1: {} chars, \
         attempt 2: {} chars)",
        prompts[1].len(),
        prompts[2].len()
    );
}

const COVERING_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "pricing", "label": "Pricing Table", "region": { "width": 1200, "height": 400 } },
    { "id": "faq", "label": "FAQ", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

fn enumerated_req() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page with hero, pricing table and FAQ".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

fn planning_prompt_count(llm: &ScriptedLlm, brief: &str) -> usize {
    llm.user_prompts()
        .iter()
        .filter(|prompt| prompt.starts_with(brief))
        .count()
}

/// First plan omits enumerated sections → exactly one re-plan carrying the
/// coverage feedback paragraph.
#[test]
fn missing_plan_section_issues_one_replan_with_feedback() {
    let brief = enumerated_req().prompt;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(COVERING_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("pricing")),
        ScriptResponse::Text(node_json("faq")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        enumerated_req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("coverage re-plan run succeeds");

    assert_eq!(planning_prompt_count(&llm, &brief), 2);
    let feedback = crate::plan_coverage::coverage_feedback(&["pricing table".into(), "FAQ".into()]);
    assert!(
        llm.user_prompts()[1].contains(&feedback),
        "second planning prompt must carry coverage feedback, got {}",
        llm.user_prompts()[1]
    );
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::PlanCoverageRetry { missing }
            if missing == &["pricing table".to_string(), "FAQ".to_string()]
    )));
}

/// A plan that already covers every enumerated section is requested once.
#[test]
fn covering_plan_issues_exactly_one_plan_request() {
    let brief = enumerated_req().prompt;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(COVERING_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("pricing")),
        ScriptResponse::Text(node_json("faq")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        enumerated_req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("covering plan run succeeds");

    assert_eq!(planning_prompt_count(&llm, &brief), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::PlanCoverageRetry { .. })));
}

/// A second plan that still misses sections is kept; the gate never loops.
#[test]
fn second_incomplete_plan_is_kept_without_another_retry() {
    let brief = enumerated_req().prompt;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        enumerated_req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("second incomplete plan still runs");

    assert_eq!(planning_prompt_count(&llm, &brief), 2);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Progress::PlanCoverageRetry { .. }))
            .count(),
        1
    );
}

// ── motion50 fix 3: motion clauses / paren fragments must not trip the gate ─

fn motion50_req(prompt: &str) -> DesignRequest {
    DesignRequest {
        prompt: prompt.into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

fn single_subtask_plan_json(label: &str) -> String {
    format!(
        r##"{{"rootFrame": {{ "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{{ "type": "solid", "color": "#FFFFFF" }}] }},
  "subtasks": [
    {{ "id": "s1", "label": "{label}", "region": {{ "width": 1200, "height": 800 }} }}
  ]}}"##
    )
}

/// A one-subtask generation that satisfies the repeated-item completeness
/// gate: three sibling frames (a family of 3) with CJK copy so the output-
/// language gate stays quiet too. Needed because the other-03 covering label
/// must carry the literal required section "三条要点", which the completeness
/// gate parses as a promise of 3 items.
fn three_item_json() -> String {
    r#"I(null, {"type":"frame","name":"Sec","x":0,"y":0,"width":1200,"height":300,"layout":"vertical","children":[
      {"type":"frame","name":"Item","x":0,"y":0,"width":1200,"height":80,"children":[{"type":"text","content":"页面内容","fontSize":16}]},
      {"type":"frame","name":"Item","x":0,"y":0,"width":1200,"height":80,"children":[{"type":"text","content":"页面内容","fontSize":16}]},
      {"type":"frame","name":"Item","x":0,"y":0,"width":1200,"height":80,"children":[{"type":"text","content":"页面内容","fontSize":16}]}
    ]});"#
    .to_string()
}

/// The three motion50 briefs whose coverage retries were pure noise (samples:
/// lane1/web-07, lane0/other-03, app-01/smoke.log): a plan naming every REAL
/// section must pass the gate without a single PlanCoverageRetry.
#[test]
fn motion50_briefs_with_covering_plans_skip_the_coverage_retry() {
    for (brief, covering_label) in [
        (
            "冥想 App 三屏可交互原型（各 375×812）：首页(问候+今日推荐卡+课程列表四条)、呼吸练习屏(大呼吸圆环+计时+暂停)、完成屏(时长与连续天数统计)。交互：首页「开始今日练习」onTap 进呼吸屏，呼吸屏「完成」onTap 进完成屏。动效：呼吸圆环 mount 用 emphasizedDecelerate 缩放淡入 500ms，课程卡 inView fade-up 交错 delayMs 60ms 递增，完成屏统计数字滚动。深色午夜蓝渐变 + 柔光玻璃卡，一个签名瞬间：呼吸圆环外的三层同心光晕。",
            "首页 + 呼吸练习屏 + 完成屏",
        ),
        (
            "开源项目官网（1440 宽长页）：英雄(项目名+一句话+安装命令块+GitHub 星标 count-up)、特性六格、代码示例区、生态 logo、贡献者头像墙、页脚。滚动动效：安装块 mount、特性格 inView 交错、代码区 sticky 随滚动高亮不同行。终端深色 + 等宽字。",
            "英雄 + 特性六格 + 代码示例区 + 生态 logo + 贡献者头像墙 + 页脚",
        ),
        (
            "知识卡片一组三张（1080×1350）：主题「如何做好一次设计评审」，每张含标题、三条要点、底部署名条。动效：标题逐字揭示、要点 inView 交错。高对比撞色 + 大字号，适合社媒。",
            "主题「如何做好一次设计评审」 三条要点 底部署名条",
        ),
    ] {
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(single_subtask_plan_json(covering_label)),
            ScriptResponse::Text(three_item_json()),
        ]);
        let mut sink = VecDocSink::new();
        let mut events = Vec::new();
        let mut on_progress = |p: Progress| events.push(p);
        futures::executor::block_on(Orchestrator::new().run(
            motion50_req(brief),
            &mut sink,
            &llm,
            &mut on_progress,
            &AbortFlag::new(),
            &stub_providers(),
        ))
        .expect("motion50 brief must run");

        assert_eq!(
            planning_prompt_count(&llm, brief),
            1,
            "brief must not trigger a coverage re-plan: {brief}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Progress::PlanCoverageRetry { .. })),
            "PlanCoverageRetry must not fire for a covering plan: {brief}"
        );
    }
}

/// The gate keeps its teeth: a plan that really omits an enumerated section
/// still triggers exactly one coverage retry.
#[test]
fn a_truly_missing_section_still_triggers_the_coverage_retry() {
    let brief = "产品官网（1440）：包含定价三档、关于我们、页脚";
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(single_subtask_plan_json("关于我们 + 页脚")),
        // The re-plan adds the missing section.
        ScriptResponse::Text(single_subtask_plan_json("定价三档 关于我们 页脚")),
        ScriptResponse::Text(node_json("页面内容")),
        ScriptResponse::Text(node_json("页面内容")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        motion50_req(brief),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("coverage re-plan run succeeds");

    assert_eq!(planning_prompt_count(&llm, brief), 2);
    assert!(events
        .iter()
        .any(|event| matches!(event, Progress::PlanCoverageRetry { .. })));
}

// ── motion50 fix 1: provider session/usage quota circuit breaker ────────────

const PLAN3_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } },
    { "id": "foot", "label": "Footer", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

fn limit_fail(message: &str) -> ScriptResponse {
    ScriptResponse::Fail(crate::types::LlmError {
        message: message.into(),
        aborted: false,
    })
}

/// motion50 fix 1 (lane1/web-10): two consecutive subtasks failing on the
/// provider's session/usage quota must trip a run-level circuit breaker — the
/// abort flag is set, the THIRD subtask never reaches the model, and the run
/// ends aborted instead of "success with scaffold only".
#[test]
fn two_consecutive_provider_limit_failures_circuit_break_the_run() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN3_JSON.into()),
        limit_fail("You've hit your session limit · resets 2:50pm (Asia/Shanghai)"),
        limit_fail("You've hit your usage limit · resets 3pm (Asia/Shanghai)"),
        // No further responses on purpose: subtask 3 and the salvage pass
        // must never call the model.
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let abort = AbortFlag::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut |p: Progress| events.push(p),
        &abort,
        &stub_providers(),
    ));
    assert!(
        matches!(result, Err(OrchestratorError::Aborted)),
        "the run must end aborted, got {result:?}"
    );
    assert!(
        abort.is_set(),
        "the circuit breaker must set the abort flag"
    );
    // planning + exactly one call per failed subtask (the quota error is
    // non-retryable, so each burns a single attempt). 4+ calls means the
    // third subtask or the salvage pass still reached the model.
    assert_eq!(
        llm.system_prompts().len(),
        3,
        "expected planning + 2 subtask calls only"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Progress::SubtaskFailed { .. }))
            .count(),
        2
    );
    // The user-facing announcement carries the provider's quota message.
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::RunAborted { reason }
            if reason.contains("session limit") || reason.contains("usage limit")
    )));
}

// ── plan-coverage v2: the covers backfill suppresses the false retry ────────

/// The 0919 GLM-Flash web-11 brief, whose every section the plan genuinely
/// carried while the gate still burned a re-plan on 英雄 / 色板与字阶展示 /
/// 快速开始代码块 / 页脚.
const WEB11_BRIEF: &str = "设计系统文档站首页（1440 宽长页）：英雄（标题+搜索框）、组件网格九个、设计原则三条、色板与字阶展示、快速开始代码块、页脚。交互：组件卡 hover 显示描述，代码块可复制。滚动动效：组件格 inView 交错，色板 mount 逐块展开，代码块 sticky。风格由你决定，要求信息清晰、层级分明。";

const WEB11_COVERS_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1440, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero Section", "elements": "eyebrow, headline, large search input",
      "covers": ["英雄"], "region": { "width": 1440, "height": 400 } },
    { "id": "grid", "label": "Component Grid", "elements": "cards with mini previews",
      "covers": ["组件网格"], "region": { "width": 1440, "height": 400 } },
    { "id": "principles", "label": "Design Principles", "elements": "numbered principle cards",
      "covers": ["设计原则"], "region": { "width": 1440, "height": 400 } },
    { "id": "tokens", "label": "Color Palette & Type Scale", "elements": "swatch ramp rows, type specimens",
      "covers": ["色板与字阶展示"], "region": { "width": 1440, "height": 400 } },
    { "id": "quickstart", "label": "Quick Start Code Block", "elements": "numbered steps, tabbed code panel",
      "covers": ["快速开始代码块"], "region": { "width": 1440, "height": 400 } },
    { "id": "footer", "label": "Footer", "elements": "brand block, link columns, copyright",
      "covers": ["页脚"], "region": { "width": 1440, "height": 400 } }
  ]
}"##;

/// A covering plan that carries the covers backfill is requested exactly
/// once and never emits a PlanCoverageRetry — the 0919 web-11 false positive
/// is gone. (The same script would burn a second planning call pre-fix.)
#[test]
fn web11_covering_plan_with_covers_skips_the_coverage_retry() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(WEB11_COVERS_PLAN_JSON.into()),
        ScriptResponse::Text(WEB11_COVERS_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("页面内容")),
        ScriptResponse::Text(node_json("页面内容")),
        ScriptResponse::Text(node_json("页面内容")),
        ScriptResponse::Text(node_json("页面内容")),
        ScriptResponse::Text(node_json("页面内容")),
        ScriptResponse::Text(node_json("页面内容")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        motion50_req(WEB11_BRIEF),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("web-11 covering run succeeds");

    assert_eq!(
        planning_prompt_count(&llm, WEB11_BRIEF),
        1,
        "a covers-backfilled covering plan must be requested exactly once"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Progress::PlanCoverageRetry { .. })),
        "covers backfill must suppress the PlanCoverageRetry"
    );
}

// ── arena-m02 (0925): the re-plan still drops sections → append them ────────

const ARENA_M02_BRIEF: &str = "健身 App 首页（375×812）：顶部问候与头像、今日目标环形进度、**横向滚动的课程卡片轨道**（六张，超出屏幕裁剪）、本周活动条形图七天、底部导航四标签。";

/// The three-subtask plan the arena-m02 run shipped, returned for BOTH the
/// first plan and the coverage re-plan (the model ignored the feedback).
const ARENA_M02_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "健身首页", "width": 375, "height": 812,
                 "layout": "vertical", "gap": 0 },
  "subtasks": [
    { "id": "greeting", "label": "顶部问候与头像", "region": { "width": 375, "height": 96 },
      "elements": "greeting, supporting line, circular profile avatar" },
    { "id": "weekly-activity", "label": "本周活动条形图", "region": { "width": 375, "height": 220 },
      "elements": "weekly activity bar chart with one vertical bar per day" },
    { "id": "bottom-navigation", "label": "底部导航", "region": { "width": 375, "height": 78 },
      "elements": "icon-and-label bottom navigation tabs: 训练, 课程, 社区, 我的" }
  ]
}"##;

/// Replay of the arena-m02 planning exchange: one coverage re-plan fires, the
/// re-plan drops the same two sections, and the run still plans (and
/// generates) the goal ring and the course rail, in brief order.
#[test]
fn arena_m02_sections_the_replan_still_drops_are_appended_as_subtasks() {
    let mut script = vec![
        ScriptResponse::Text(ARENA_M02_PLAN_JSON.into()),
        ScriptResponse::Text(ARENA_M02_PLAN_JSON.into()),
    ];
    script.extend((0..5).map(|_| ScriptResponse::Text(node_json("页面内容"))));
    let llm = ScriptedLlm::new(script);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    futures::executor::block_on(Orchestrator::new().run(
        motion50_req(ARENA_M02_BRIEF),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("arena-m02 replay run succeeds");

    assert_eq!(planning_prompt_count(&llm, ARENA_M02_BRIEF), 2);
    let planned = events
        .iter()
        .find_map(|event| match event {
            Progress::Planned { subtasks } => Some(subtasks.clone()),
            _ => None,
        })
        .expect("a Planned event");
    let labels: Vec<&str> = planned.iter().map(|(_, label)| label.as_str()).collect();
    assert_eq!(
        labels,
        vec![
            "顶部问候与头像",
            "今日目标环形进度",
            "横向滚动的课程卡片轨道",
            "本周活动条形图",
            "底部导航",
        ]
    );
    for id in ["brief-section-1", "brief-section-2"] {
        assert!(
            events.iter().any(|event| matches!(
                event,
                Progress::SubtaskStarted { id: started, .. } if started == id
            )),
            "appended subtask {id} must actually run"
        );
    }
}
