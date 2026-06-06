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
Respond with THIS section's canonical PenNode objects in the FLAT _parent format:
output ONE JSON object per line (NO enclosing [ ] array), each tagged by "type"
(frame/group/rectangle/ellipse/line/polygon/path/text/text_input/image/icon_font)
and carrying "_parent" — null for the section root, else the id of its parent
node (which MUST appear on an earlier line).
EVERY non-root node MUST set "_parent". Do NOT emit a flat list of siblings with
no _parent links, and do NOT rely on a "children" array — a flat list renders
BROKEN: a horizontal row whose items are not _parent-linked to it collapses into
a vertical stack. Express the WHOLE tree through _parent (row -> its cards -> each
card's texts/icons).
Example (a horizontal row of two cards inside a section):
{"_parent":null,"id":"<prefix>-root","type":"frame","name":"Section","width":"fill_container","height":"fit_content","layout":"vertical","gap":16}
{"_parent":"<prefix>-root","id":"<prefix>-row","type":"frame","name":"Row","width":"fill_container","height":"fit_content","layout":"horizontal","gap":16}
{"_parent":"<prefix>-row","id":"<prefix>-card1","type":"frame","name":"Card","width":"fill_container","height":"fill_container","layout":"vertical","cornerRadius":12}
{"_parent":"<prefix>-card1","id":"<prefix>-card1-title","type":"text","name":"Title","content":"Revenue","fontSize":14}
{"_parent":"<prefix>-row","id":"<prefix>-card2","type":"frame","name":"Card","width":"fill_container","height":"fill_container","layout":"vertical","cornerRadius":12}
ALL field names are camelCase: cornerRadius, fontSize, fontWeight, justifyContent,
alignItems, clipContent. Geometry fields are x, y, width, height. Never snake_case.
Output ONLY the JSON lines."#;

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
    plan: &OrchestratorPlan,
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
    // Flat `_parent` for ALL tiers — validated that Basic-tier models
    // (MiniMax M2.7/M3 are Basic) emit clean `_parent` trees with it. The
    // `jsonl-format` + `jsonl-format-simplified` skills both teach `_parent`,
    // so this agrees with whichever skill the tier loads (no contradiction).
    system_prompt.push_str(NODE_FORMAT);

    let section_list = plan
        .subtasks
        .iter()
        .map(|st| {
            let marker = if st.id == subtask.id { " <- YOU" } else { "" };
            let elements = st
                .elements
                .as_ref()
                .map(|items| format!(" [{items}]"))
                .unwrap_or_default();
            format!(
                "- {}{} ({:.0}x{:.0}){}",
                st.label, elements, st.region.width, st.region.height, marker
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let my_elements = subtask
        .elements
        .as_ref()
        .map(|items| {
            format!("\nYOUR ELEMENTS: {items}\nDo NOT generate elements listed in other sections.")
        })
        .unwrap_or_default();

    let mut user_prompt = format!(
        "Page sections:\n{}\n\n\
Generate ONLY \"{}\" (~{:.0}px of content).{}\n\
Overall design: {}\n\n\
CRITICAL LAYOUT CONSTRAINTS:\n\
- Root frame: id=\"{}-root\", width=\"fill_container\", height=\"fit_content\", layout=\"vertical\". NEVER use fixed pixel height on root -- let content determine height.\n\
- Target content amount: ~{:.0}px tall. Generate enough elements to fill this area.\n\
- ALL nodes must be descendants of the root frame -- every non-root node must be nested under its parent (row -> its cards -> each card's content), in whichever format the system prompt specifies. No floating/orphan nodes; a flat sibling list with no parent links collapses into a vertical stack.\n\
- NEVER set x or y on children inside layout frames.\n\
- Use \"fill_container\" for children that stretch, \"fit_content\" for shrink-wrap sizing.\n\
- SECTION BACKGROUND: do NOT set fill on your section root frame. Only set fill on cards, buttons, chips, badges, and other visually distinct components.\n\
- TYPOGRAPHY HIERARCHY: Do NOT make every text bold. Use 700 only for primary headings, 600 for buttons/key labels, 500 for short chips/nav labels, and 400 for body text, placeholders, subtitles, metadata, and captions.\n\
- ICONS: use icon_font with lucide iconFontName; never use path nodes for icons.\n\
- IDs prefix=\"{}-\". Output ONLY the structured nodes in the EXACT format the system prompt specifies above -- no prose, no extra wrapping.",
        section_list,
        subtask.label,
        subtask.region.height,
        my_elements,
        req.prompt,
        subtask.id_prefix,
        subtask.region.height,
        subtask.id_prefix,
    );

    if plan.root_frame.width <= 480.0 {
        user_prompt.push_str(
            "\n\nMOBILE STATUS BAR: A status bar (time, signal, wifi, battery) has already been pre-inserted as the first child of the root page frame. Do NOT generate any status bar, system chrome, or OS-level indicators. Start your content directly.",
        );
        user_prompt.push_str(
            "\n\nNO PHONE MOCKUP WRAPPER: The whole design IS a mobile screen. Do NOT wrap your section in a phone-shaped frame. Your section root must use width=\"fill_container\" and contain only this section's content.",
        );
        user_prompt.push_str(
            "\n\nMOBILE WIDTH SAFETY: Every visible child must stay inside the 390px screen width. Do not create horizontal rows, chips, cards, or buttons that overflow outside the root; wrap, shrink, or clip horizontal lists instead.",
        );
        user_prompt.push_str(
            "\nMOBILE SECTION INSETS: Every non-chrome section root must keep horizontal padding of about 24px. Headings, cards, chips, and lists must not touch the screen edge. Use width=\"fill_container\" inside padded sections instead of fixed 390px widths.",
        );
        user_prompt.push_str(
            "\nMOBILE SEARCH ACTIONS: Search filter/sliders actions must be visible square controls, not loose white icons. Put them in a 52-56px by 52-56px rounded button using the accent color with a high-contrast icon.",
        );
        user_prompt.push_str(
            "\nNO BLANK PLACEHOLDERS: Do not use empty gray image placeholders in app UI. If no real image asset is available, use a square colored food/icon tile with icon_font instead.",
        );
        user_prompt.push_str(
            "\nMOBILE ROW STRUCTURE: For category chips, segmented controls, tab bars, and bottom navigation, use either multiple horizontal rows inside a vertical wrapper or equal-width children in a fill_container row. Never create one fit_content horizontal row whose total child width can exceed the screen.",
        );
        user_prompt.push_str(
            "\nMOBILE NAV SURFACE: Bottom navigation and tab bars must sit on the current page palette, full width at the bottom, 62-72px tall. Do not create a separate white footer band, oversized rounded pill, or extra side margins. Never use black or safe-dark fills for nav bars unless the whole root frame background is dark.",
        );
    }

    // Port of orchestrator-sub-agent.ts:739-748 — APPEND MODE prompt injection.
    if let Some(labels) = subtask.existing_section_labels.as_ref() {
        if !labels.is_empty() {
            let existing = labels
                .iter()
                .map(|n| format!("\"{n}\""))
                .collect::<Vec<_>>()
                .join(", ");
            user_prompt.push_str(&format!(
                "\n\nAPPEND MODE: The page already contains these sibling sections (read-only, already on canvas): {existing}.\n\
- Your root frame will be inserted as a NEW sibling at the end of that list.\n\
- Do NOT re-emit any of the sections listed above. Do NOT emit any status bar or system chrome — that is also already on the page.\n\
- Do NOT wrap your output in a phone mockup or a full-page container.\n\
- Internal headings/titles within YOUR new section are fine — only the top-level sibling sections above are off-limits.\n\
- Match the visual style (colors, cornerRadius, padding, gap) already established by those existing siblings.\n\
- Output ONLY this one new section — a single root frame with its content."
            ));
        }
    }

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
#[path = "prompt_tests.rs"]
mod tests;
