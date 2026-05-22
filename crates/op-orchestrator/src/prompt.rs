//! Prompt 构造 —— 规划 prompt 与 sub-agent prompt。
//!
//! skill 正文经 `op_ai_skills::resolve_skills` 解析:规划用
//! `Phase::Planning`,sub-agent 用 `Phase::Generation`。两段格式
//! 指令(产 OrchestratorPlan JSON / 产 PenNode JSON 数组)由本
//! 模块附加在 skill 正文之后。
//!
//! 注:格式指令文本是功能性的最小版;逐条对齐 TS
//! `orchestrator-prompt-optimizer.ts` / `orchestrator-sub-agent.ts`
//! 的细则(style-guide 注入、移动端禁 phone-wrapper 等)是后续
//! 细化项。

use crate::compact_prompt::build_compact_planning_prompt;
use crate::compact_skills::apply_skill_filter;
use crate::design_type::{detect_design_type, DesignType};
use crate::model_profile::resolve_model_profile;
use crate::plan::{OrchestratorPlan, Subtask};
use crate::style_guide_context::build_planning_style_guide_context;
use crate::timeouts::{
    apply_profile_to_timeouts, builtin_planning_timeouts, orchestrator_timeouts, sub_agent_timeouts,
};
use crate::types::{AbortFlag, CallRequest, DesignRequest, PlanningMode, PlanningPrompt};
use std::collections::HashMap;

/// sub-agent 阶段要求模型产出的 JSON 形状说明。
const NODE_FORMAT: &str = r#"
Respond with a JSON array of canonical PenNode objects for THIS section only.
Each node is tagged by "type" (frame/group/rectangle/ellipse/line/polygon/path/
text/text_input/image/icon_font). Frames/groups nest children via "children".
Example: [ { "type": "frame", "id": "<prefix>-1", "name": "Card", "x": 0, "y": 0,
"width": 1200, "height": 200, "children": [] } ]
ALL field names are camelCase: cornerRadius, fontSize, fontWeight, justifyContent,
alignItems, clipContent. Geometry fields are x, y, width, height. Never snake_case.
Output ONLY the JSON array."#;

/// Rich 模式 system prompt 末尾后缀 —— verbatim,`orchestrator.ts:1382-1383`。
const RICH_SUFFIX: &str = "\n\n---\nCRITICAL OUTPUT FORMAT ENFORCEMENT:\n\
You MUST output ONLY a single JSON object. Start your response with { and end with }.\n\
Do NOT output any text, explanation, analysis, markdown, or tool calls before or after the JSON.\n\
Do NOT \"explore\" or \"think out loud\". Do NOT use <tool_call> or function calls.\n\
Any pre-design analysis (concept extraction, superfan simulation, etc.) must happen internally — include results as JSON fields, never as prose.\n\
Violating this format will cause a system error.";

/// Minimal 模式后缀 —— verbatim,`orchestrator.ts:1384`。
const MINIMAL_SUFFIX: &str =
    "\n\nOUTPUT ONLY ONE JSON OBJECT. No prose. No markdown. No tool calls.";

fn planning_suffix(mode: PlanningMode) -> &'static str {
    match mode {
        PlanningMode::Rich => RICH_SUFFIX,
        PlanningMode::Minimal => MINIMAL_SUFFIX,
        PlanningMode::Compact => "",
    }
}

/// 丢掉 `landing-page-predesign` skill(除非设计类型是 landing-page)。
/// 见 spec §5.10。
fn filter_planning_skills_for_prompt(
    skills: Vec<op_ai_skills::ResolvedSkill>,
    prompt: &str,
) -> Vec<op_ai_skills::ResolvedSkill> {
    if detect_design_type(prompt).type_ == DesignType::LandingPage {
        return skills;
    }
    skills
        .into_iter()
        .filter(|s| s.meta.name != "landing-page-predesign")
        .collect()
}

/// 规划阶段的 LLM 调用输入。`mode` 决定 prompt 构造方式;返回
/// `PlanningPrompt`(带 compact 的 forced style-guide 名,供 S3b-1b)。
///
/// 超时来源(对齐 TS `callOrchestrator` 中的 `timeouts` 分支):
/// - `Compact` 模式对应 TS `fastTimeout=true`(仅 builtin / Basic 兜底路径),
///   使用 `builtin_planning_timeouts` —— 较短,让规划快速失败。
/// - `Rich` / `Minimal` 使用 `orchestrator_timeouts(prompt_len)`,按
///   prompt 长度分桶后再乘以模型 tier 的 `timeout_multiplier`。
pub fn build_orchestrator_prompt(
    req: &DesignRequest,
    mode: PlanningMode,
    abort: AbortFlag,
) -> PlanningPrompt {
    let model_id = req.model.as_deref().unwrap_or("");
    let profile = resolve_model_profile(model_id);
    let multiplier = profile.timeout_multiplier;

    match mode {
        PlanningMode::Compact => {
            // TS fastTimeout=true path: getBuiltinPlanningTimeouts(model)
            let t = apply_profile_to_timeouts(builtin_planning_timeouts(profile.tier), multiplier);
            let cp = build_compact_planning_prompt(&req.prompt, req.design_md.as_ref());
            PlanningPrompt {
                call_request: CallRequest {
                    system_prompt: cp.system,
                    user_prompt: cp.user_prompt,
                    model: req.model.clone(),
                    provider: req.provider.clone(),
                    timeout: t.hard,
                    abort,
                    no_text_timeout: Some(t.no_text),
                    first_text_timeout: Some(t.first_text),
                },
                forced_style_guide_name: Some(cp.selected_style_guide_name),
                mode,
            }
        }
        PlanningMode::Rich | PlanningMode::Minimal => {
            // TS fastTimeout=false path: getOrchestratorTimeouts(originalLength, model)
            let t = apply_profile_to_timeouts(orchestrator_timeouts(req.prompt.len()), multiplier);
            let ctx = build_planning_style_guide_context(
                &req.prompt,
                req.model.as_deref(),
                mode,
                req.design_md.as_ref(),
            );
            let opts = op_ai_skills::ResolveOptions {
                dynamic_content: HashMap::from([(
                    "availableStyleGuides".to_string(),
                    ctx.available_style_guides,
                )]),
                ..Default::default()
            };
            let agent_ctx =
                op_ai_skills::resolve_skills(op_ai_skills::Phase::Planning, &req.prompt, &opts);
            let skills = filter_planning_skills_for_prompt(agent_ctx.skills, &req.prompt);
            let mut system_prompt = skills
                .iter()
                .map(|s| s.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            system_prompt.push_str(planning_suffix(mode));
            PlanningPrompt {
                call_request: CallRequest {
                    system_prompt,
                    user_prompt: req.prompt.clone(),
                    model: req.model.clone(),
                    provider: req.provider.clone(),
                    timeout: t.hard,
                    abort,
                    no_text_timeout: Some(t.no_text),
                    first_text_timeout: Some(t.first_text),
                },
                forced_style_guide_name: None,
                mode,
            }
        }
    }
}

/// 把解析出的 skill 列表返回(供下游过滤)。
fn resolve_generation_skills(message: &str) -> Vec<op_ai_skills::ResolvedSkill> {
    let ctx = op_ai_skills::resolve_skills(
        op_ai_skills::Phase::Generation,
        message,
        &op_ai_skills::ResolveOptions::default(),
    );
    ctx.skills
}

/// 单个 sub-agent 的 LLM 调用输入。
///
/// * `reduced_complexity` — When `true` and the model is Basic tier,
///   narrows the skill set to the `retryAllowed` 8-skill set (drops
///   `elements` and other non-essential skills).  For Standard/Full
///   tier this is a no-op.  Port of the `reducedComplexity` param in
///   `executeSubAgent` (orchestrator-sub-agent.ts:349).
/// * `minimal_skills` — When `true`, strips the system prompt down to
///   only `schema` + `jsonl-format` (last-ditch fallback for models
///   whose safety scanner times out on the full prompt).  Port of the
///   `minimalSkills` param in `executeSubAgent` (lines 428-431).
pub fn build_subagent_prompt(
    subtask: &Subtask,
    _plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
) -> CallRequest {
    // Resolve the full generation skill set, then apply tier-gated filtering.
    let model_id = req.model.as_deref().unwrap_or("");
    let tier = resolve_model_profile(model_id).tier;
    let resolved = resolve_generation_skills(&subtask.label);
    let filtered = apply_skill_filter(resolved, tier, minimal_skills, reduced_complexity);

    let mut system_prompt = filtered
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    system_prompt.push_str("\n\n");
    system_prompt.push_str(NODE_FORMAT);

    let user_prompt = format!(
        "Overall design: {}\n\nGenerate the section \"{}\" \
         (区块 id 前缀 `{}-`). Target region: {:.0}x{:.0} px.\nPalette: (default)",
        req.prompt, subtask.label, subtask.id_prefix, subtask.region.width, subtask.region.height,
    );

    // Port of getSubAgentTimeouts(preparedPrompt.originalLength, model):
    // `originalLength` = normalized user prompt length; here `req.prompt.len()`
    // is the closest equivalent (we don't have a separate "normalized" form).
    let profile = resolve_model_profile(model_id);
    let t = apply_profile_to_timeouts(
        sub_agent_timeouts(req.prompt.len(), tier),
        profile.timeout_multiplier,
    );

    CallRequest {
        system_prompt,
        user_prompt,
        model: req.model.clone(),
        provider: req.provider.clone(),
        timeout: t.hard,
        abort,
        no_text_timeout: Some(t.no_text),
        first_text_timeout: Some(t.first_text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Region, RootFrameSpec};

    fn req() -> DesignRequest {
        DesignRequest {
            prompt: "a pricing page".into(),
            model: Some("claude".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        }
    }

    fn plan() -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: RootFrameSpec {
                id: "root".into(),
                name: "P".into(),
                width: 1200.0,
                height: 800.0,
                layout: None,
                gap: None,
                padding: None,
                fill: None,
            },
            subtasks: vec![],
            style_guide_name: None,
        }
    }

    #[test]
    fn rich_prompt_has_style_guides_and_suffix() {
        let pp = build_orchestrator_prompt(&req(), PlanningMode::Rich, AbortFlag::new());
        assert_eq!(pp.mode, PlanningMode::Rich);
        assert!(pp.forced_style_guide_name.is_none());
        // style-guide context 经 {{availableStyleGuides}} 注入到 planning skill
        assert!(pp
            .call_request
            .system_prompt
            .contains("Available style guides"));
        // rich 后缀
        assert!(pp
            .call_request
            .system_prompt
            .contains("CRITICAL OUTPUT FORMAT ENFORCEMENT"));
        assert_eq!(pp.call_request.user_prompt, req().prompt);
    }

    #[test]
    fn minimal_prompt_has_short_suffix_no_snippets() {
        let pp = build_orchestrator_prompt(&req(), PlanningMode::Minimal, AbortFlag::new());
        assert!(pp
            .call_request
            .system_prompt
            .contains("OUTPUT ONLY ONE JSON OBJECT"));
        assert!(!pp
            .call_request
            .system_prompt
            .contains("CRITICAL OUTPUT FORMAT ENFORCEMENT"));
    }

    #[test]
    fn compact_prompt_carries_forced_guide_name() {
        let pp = build_orchestrator_prompt(&req(), PlanningMode::Compact, AbortFlag::new());
        assert!(pp.forced_style_guide_name.is_some());
        assert!(pp
            .call_request
            .system_prompt
            .starts_with("You are a UI planning assistant."));
        // compact 不带 rich/minimal 后缀
        assert!(!pp
            .call_request
            .system_prompt
            .contains("CRITICAL OUTPUT FORMAT ENFORCEMENT"));
    }

    #[test]
    fn subagent_prompt_carries_subtask_and_node_format() {
        let st = Subtask {
            id: "hero".into(),
            label: "Hero".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "hero".into(),
            parent_frame_id: Some("root".into()),
            elements: None,
            screen: None,
            generated_root_id: None,
        };
        let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
        assert!(cr.user_prompt.contains("Hero"));
        assert!(cr.user_prompt.contains("hero-"));
        assert!(cr.system_prompt.contains("PenNode"));
    }

    #[test]
    fn subagent_prompt_minimal_skills_only_has_schema_and_jsonl() {
        let st = Subtask {
            id: "hero".into(),
            label: "Hero".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "hero".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
        };
        // minimal_skills=true: the system prompt should contain "schema" skill
        // content and "jsonl-format" skill content, but NOT layout/text-rules etc.
        let cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, true);
        // schema and jsonl-format skills should appear (they always exist)
        assert!(
            cr.system_prompt.contains("PenNode"),
            "NODE_FORMAT suffix should still be appended"
        );
        // The system_prompt should be considerably shorter than a full-skill prompt
        let full_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
        assert!(
            cr.system_prompt.len() < full_cr.system_prompt.len(),
            "minimal_skills prompt should be shorter than full-skill prompt"
        );
    }

    #[test]
    fn subagent_prompt_reduced_complexity_basic_is_shorter_than_full() {
        let st = Subtask {
            id: "hero".into(),
            label: "Hero".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "hero".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
        };
        // req() uses model "claude" which is Full tier — no narrowing.
        // Use a basic-tier model to test narrowing.
        let basic_req = DesignRequest {
            prompt: "a page".into(),
            model: Some("claude-haiku".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        };
        let full_cr =
            build_subagent_prompt(&st, &plan(), &basic_req, AbortFlag::new(), false, false);
        let reduced_cr =
            build_subagent_prompt(&st, &plan(), &basic_req, AbortFlag::new(), true, false);
        assert!(
            reduced_cr.system_prompt.len() <= full_cr.system_prompt.len(),
            "reduced_complexity Basic prompt should be no longer than full-skill prompt"
        );
    }

    #[test]
    fn subagent_prompt_reduced_complexity_full_tier_is_noop() {
        let st = Subtask {
            id: "hero".into(),
            label: "Hero".into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "hero".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
        };
        // req() uses "claude" which maps to Full tier → reduced_complexity is no-op
        let full_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
        let reduced_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), true, false);
        assert_eq!(
            full_cr.system_prompt, reduced_cr.system_prompt,
            "reduced_complexity on Full tier should be a no-op"
        );
    }

    // ── C4: timeout wiring tests ──────────────────────────────────────────────

    /// Rich/Minimal mode sets profile-derived timeouts (not None).
    #[test]
    fn orchestrator_prompt_rich_has_profile_timeouts() {
        let pp = build_orchestrator_prompt(&req(), PlanningMode::Rich, AbortFlag::new());
        assert!(
            pp.call_request.no_text_timeout.is_some(),
            "no_text_timeout must be Some for Rich mode"
        );
        assert!(
            pp.call_request.first_text_timeout.is_some(),
            "first_text_timeout must be Some for Rich mode"
        );
        // Short prompt (< 2200 chars): hard = 300s for Standard/Full tier.
        assert_eq!(
            pp.call_request.timeout,
            std::time::Duration::from_millis(300_000),
            "short-prompt Rich timeout should be 300s"
        );
    }

    /// Compact mode uses builtin_planning_timeouts (60s hard for Full tier, not 300s).
    #[test]
    fn orchestrator_prompt_compact_uses_builtin_timeouts() {
        let pp = build_orchestrator_prompt(&req(), PlanningMode::Compact, AbortFlag::new());
        // builtin: hard=60_000ms for Full tier (multiplier=1.0)
        assert_eq!(
            pp.call_request.timeout,
            std::time::Duration::from_millis(60_000),
            "Compact mode should use builtin planning timeout (60s hard)"
        );
        assert!(
            pp.call_request.no_text_timeout.is_some(),
            "no_text_timeout must be Some for Compact mode"
        );
        assert!(
            pp.call_request.first_text_timeout.is_some(),
            "first_text_timeout must be Some for Compact mode"
        );
    }

    /// A long prompt (>= 4200 chars) yields a larger hard timeout than a short one
    /// for Rich mode.
    #[test]
    fn orchestrator_prompt_long_prompt_has_larger_timeout_than_short() {
        let short_req = DesignRequest {
            prompt: "short".into(), // < 2200 chars
            model: Some("claude-sonnet".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        };
        let long_prompt = "x".repeat(5000); // >= 4200 chars
        let long_req = DesignRequest {
            prompt: long_prompt,
            model: Some("claude-sonnet".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        };
        let short_pp = build_orchestrator_prompt(&short_req, PlanningMode::Rich, AbortFlag::new());
        let long_pp = build_orchestrator_prompt(&long_req, PlanningMode::Rich, AbortFlag::new());
        assert!(
            long_pp.call_request.timeout > short_pp.call_request.timeout,
            "long prompt should yield a larger timeout than short prompt"
        );
    }

    /// timeout_multiplier is applied: deepseek-v4-pro has multiplier=2.0,
    /// so its timeout should be 2× the standard short-bucket orchestrator timeout.
    #[test]
    fn orchestrator_prompt_multiplier_applied() {
        let ds_req = DesignRequest {
            prompt: "a page".into(), // short bucket
            model: Some("deepseek-v4-pro".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        };
        let pp = build_orchestrator_prompt(&ds_req, PlanningMode::Rich, AbortFlag::new());
        // Short bucket base: 300_000ms × 2.0 = 600_000ms
        assert_eq!(
            pp.call_request.timeout,
            std::time::Duration::from_millis(600_000),
            "deepseek-v4-pro multiplier=2.0 should double the short-bucket timeout"
        );
    }

    fn subtask() -> crate::plan::Subtask {
        crate::plan::Subtask {
            id: "s".into(),
            label: "Section".into(),
            region: crate::plan::Region {
                width: 1200.0,
                height: 400.0,
            },
            id_prefix: "s".into(),
            parent_frame_id: None,
            elements: None,
            screen: None,
            generated_root_id: None,
        }
    }

    /// Sub-agent prompt has profile-derived timeouts (not None).
    #[test]
    fn subagent_prompt_has_profile_timeouts() {
        let cr = build_subagent_prompt(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
        assert!(
            cr.no_text_timeout.is_some(),
            "no_text_timeout must be Some for sub-agent"
        );
        assert!(
            cr.first_text_timeout.is_some(),
            "first_text_timeout must be Some for sub-agent"
        );
        // req() has short prompt → Short bucket SA hard = 420_000ms (Full tier, multiplier=1)
        assert_eq!(
            cr.timeout,
            std::time::Duration::from_millis(420_000),
            "short-prompt sub-agent hard timeout should be 420s"
        );
    }

    /// A long prompt (>= 4200 chars) yields a larger sub-agent hard timeout.
    #[test]
    fn subagent_prompt_long_prompt_has_larger_timeout() {
        let short_req = DesignRequest {
            prompt: "design a page".into(),
            model: Some("claude-sonnet".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        };
        let long_req = DesignRequest {
            prompt: "x".repeat(5000),
            model: Some("claude-sonnet".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        };
        let short_cr = build_subagent_prompt(
            &subtask(),
            &plan(),
            &short_req,
            AbortFlag::new(),
            false,
            false,
        );
        let long_cr = build_subagent_prompt(
            &subtask(),
            &plan(),
            &long_req,
            AbortFlag::new(),
            false,
            false,
        );
        assert!(
            long_cr.timeout > short_cr.timeout,
            "long prompt should yield a larger sub-agent timeout"
        );
    }

    /// Basic-tier sub-agent clamps no_text and first_text timeouts.
    #[test]
    fn subagent_prompt_basic_tier_clamps_soft_timeouts() {
        let basic_req = DesignRequest {
            prompt: "a page".into(), // short bucket
            model: Some("claude-haiku".into()),
            provider: None,
            design_md: None,
            concurrency: 1,
        };
        let cr = build_subagent_prompt(
            &subtask(),
            &plan(),
            &basic_req,
            AbortFlag::new(),
            false,
            false,
        );
        // Basic clamp: no_text ≤ 45_000ms, first_text ≤ 75_000ms
        assert!(
            cr.no_text_timeout.unwrap() <= std::time::Duration::from_millis(45_000),
            "Basic tier no_text_timeout should be clamped to ≤ 45s"
        );
        assert!(
            cr.first_text_timeout.unwrap() <= std::time::Duration::from_millis(75_000),
            "Basic tier first_text_timeout should be clamped to ≤ 75s"
        );
    }
}
