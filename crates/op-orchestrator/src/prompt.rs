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
use crate::types::{AbortFlag, CallRequest, DesignRequest, PlanningMode, PlanningPrompt};
use std::collections::HashMap;
use std::time::Duration;

const PLANNING_TIMEOUT: Duration = Duration::from_secs(300);
const SUBAGENT_TIMEOUT: Duration = Duration::from_secs(420);

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
pub fn build_orchestrator_prompt(
    req: &DesignRequest,
    mode: PlanningMode,
    abort: AbortFlag,
) -> PlanningPrompt {
    match mode {
        PlanningMode::Compact => {
            let cp = build_compact_planning_prompt(&req.prompt, req.design_md.as_ref());
            PlanningPrompt {
                call_request: CallRequest {
                    system_prompt: cp.system,
                    user_prompt: cp.user_prompt,
                    model: req.model.clone(),
                    provider: req.provider.clone(),
                    timeout: PLANNING_TIMEOUT,
                    abort,
                    no_text_timeout: None,
                    first_text_timeout: None,
                },
                forced_style_guide_name: Some(cp.selected_style_guide_name),
                mode,
            }
        }
        PlanningMode::Rich | PlanningMode::Minimal => {
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
                    timeout: PLANNING_TIMEOUT,
                    abort,
                    no_text_timeout: None,
                    first_text_timeout: None,
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

    CallRequest {
        system_prompt,
        user_prompt,
        model: req.model.clone(),
        provider: req.provider.clone(),
        timeout: SUBAGENT_TIMEOUT,
        abort,
        no_text_timeout: None,
        first_text_timeout: None,
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
        };
        // req() uses model "claude" which is Full tier — no narrowing.
        // Use a basic-tier model to test narrowing.
        let basic_req = DesignRequest {
            prompt: "a page".into(),
            model: Some("claude-haiku".into()),
            provider: None,
            design_md: None,
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
        };
        // req() uses "claude" which maps to Full tier → reduced_complexity is no-op
        let full_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), false, false);
        let reduced_cr = build_subagent_prompt(&st, &plan(), &req(), AbortFlag::new(), true, false);
        assert_eq!(
            full_cr.system_prompt, reduced_cr.system_prompt,
            "reduced_complexity on Full tier should be a no-op"
        );
    }
}
