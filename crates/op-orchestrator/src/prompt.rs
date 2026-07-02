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
use crate::compact_skills::{apply_skill_filter, SkillNamed};
use crate::design_md_policy::build_design_md_style_policy;
use crate::design_type::{detect_design_type, DesignType};
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::plan::{OrchestratorPlan, Subtask};
use crate::style_guide_context::build_planning_style_guide_context;
use crate::timeouts::{
    apply_profile_to_timeouts, builtin_planning_timeouts, orchestrator_timeouts, sub_agent_timeouts,
};
use crate::types::{AbortFlag, CallRequest, DesignRequest, PlanningMode, PlanningPrompt};
use op_ai_skills::style_guide::{
    extract_style_guide_values, select_style_guide, style_guide_registry, SelectOptions,
};
use op_ai_skills::{
    budget::trim_by_budget_pinned,
    get_skills_by_phase,
    resolver::{filter_by_intent, inject_dynamic_content},
    DropReason, DroppedSkill, Phase, ResolveOptions, ResolvedSkill, SkillLoadEntry,
    SkillLoadReport,
};
use op_editor_core::ComponentLibrary;
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

/// manifest 模式（`OPENPENCIL_MANIFEST=1`）的输出协议说明——细则与目录
/// 在 `element-manifest` skill 里，这里只钉住最终输出契约。
const MANIFEST_FORMAT: &str = r#"
OUTPUT PROTOCOL: ELEMENT MANIFEST JSONL. Respond with one {"el":"<kind>",...}
JSON object per line, exactly as the ELEMENT MANIFEST section above specifies.
Use catalog kinds for everything they cover; group with {"el":"section"} lines
and "in" line-number references; NEVER emit id / parent_id / pageId fields.
Output ONLY the JSONL lines."#;

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

const PLANNING_QUALITY_GUARDRAILS: &str = r#"

PLANNING QUALITY GUARDRAILS:
- Do not plan the same predictable mobile stack of search + categories + orange promo + two cards unless the request explicitly asks for that exact convention.
- Mobile top rhythm: planned header/title/search/primary-content sections should be compact; avoid allocating a huge empty band between the title and first useful module.
- Plan one signature moment in the first viewport: a crafted hero/product composition, editorial crop, distinctive category rail, refined data module, or other domain-specific focal idea.
- Bottom navigation is optional. If planned, it should be integrated with the page flow, not a detached floating pill, nested rounded capsule, or extra footer band.
- Product-card favorite/heart controls must be inside their card/image; never plan them as protruding decorative badges."#;

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
            system_prompt.push_str(PLANNING_QUALITY_GUARDRAILS);
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

/// Compose the intent string for per-subtask skill matching. Component 1:
/// keyword triggers must see the ORIGINAL request, not just the short subtask
/// label, or domain/knowledge skills (mobile-app, cjk-typography, food cues)
/// never fire. Order: original prompt, then label, then screen/element hints.
pub(crate) fn subtask_intent(req: &DesignRequest, subtask: &Subtask) -> String {
    let mut s = String::new();
    s.push_str(&req.prompt);
    s.push('\n');
    s.push_str(&subtask.label);
    if let Some(screen) = subtask.screen.as_ref() {
        s.push_str("\nscreen: ");
        s.push_str(screen);
    }
    if let Some(elements) = subtask.elements.as_ref() {
        s.push('\n');
        s.push_str(elements);
    }
    s
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct ExplicitDesignTokens {
    radius: Option<f64>,
    spacing: Option<f64>,
}

fn find_keyword_positions(chars: &[char], keyword: &str) -> Vec<(usize, usize)> {
    let needle: Vec<char> = keyword.chars().collect();
    if needle.is_empty() || needle.len() > chars.len() {
        return Vec::new();
    }
    chars
        .windows(needle.len())
        .enumerate()
        .filter_map(|(idx, window)| {
            if window == needle.as_slice() {
                Some((idx, idx + needle.len()))
            } else {
                None
            }
        })
        .collect()
}

fn keyword_distance(start: usize, end: usize, ranges: &[(usize, usize)]) -> Option<usize> {
    ranges
        .iter()
        .map(|&(k_start, k_end)| {
            if end <= k_start {
                k_start - end
            } else {
                start.saturating_sub(k_end)
            }
        })
        .min()
}

fn format_design_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

fn extract_explicit_design_tokens(prompt: &str) -> ExplicitDesignTokens {
    let lower = prompt.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let radius_ranges: Vec<(usize, usize)> = [
        "圆角",
        "corner radius",
        "cornerradius",
        "border radius",
        "border-radius",
        "radius",
        "corner",
    ]
    .into_iter()
    .flat_map(|keyword| find_keyword_positions(&chars, keyword))
    .collect();
    let spacing_ranges: Vec<(usize, usize)> = ["间距", "spacing", "gap"]
        .into_iter()
        .flat_map(|keyword| find_keyword_positions(&chars, keyword))
        .collect();

    let mut out = ExplicitDesignTokens::default();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        let end = i;
        let raw_value: String = chars[start..end].iter().collect();
        let Ok(value) = raw_value.parse::<f64>() else {
            continue;
        };
        if !(0.0..=128.0).contains(&value) {
            continue;
        }

        let radius_distance = keyword_distance(start, end, &radius_ranges);
        let spacing_distance = keyword_distance(start, end, &spacing_ranges);
        match (radius_distance, spacing_distance) {
            (Some(r), Some(s)) if r <= 12 || s <= 12 => {
                if r <= s {
                    out.radius.get_or_insert(value);
                } else {
                    out.spacing.get_or_insert(value);
                }
            }
            (Some(r), _) if r <= 12 => {
                out.radius.get_or_insert(value);
            }
            (_, Some(s)) if s <= 12 => {
                out.spacing.get_or_insert(value);
            }
            _ => {}
        }
    }
    out
}

fn explicit_design_token_instruction(tokens: ExplicitDesignTokens) -> Option<String> {
    if tokens.radius.is_none() && tokens.spacing.is_none() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(radius) = tokens.radius {
        parts.push(format!(
            "cornerRadius must be {}px for ordinary rounded components",
            format_design_number(radius)
        ));
    }
    if let Some(spacing) = tokens.spacing {
        parts.push(format!(
            "layout gap/spacing must be {}px",
            format_design_number(spacing)
        ));
    }
    Some(format!(
        "EXPLICIT USER DESIGN TOKENS: {}. These values override style-guide radius/spacing ranges, role defaults, and generic mobile guidance. Do not use larger rounded-card/search ranges or mixed 16/20/24px internal gaps unless geometry strictly requires it.",
        parts.join("; ")
    ))
}

/// Resolve the generation-phase skill set against a message, returning the
/// full AgentContext (skills + load report). The report is held for B3b's
/// IntentMiss/BudgetExhausted merge.
fn resolve_generation_skills(
    message: &str,
    opts: &op_ai_skills::ResolveOptions,
) -> op_ai_skills::AgentContext {
    op_ai_skills::resolve_skills(op_ai_skills::Phase::Generation, message, opts)
}

/// 该 plan 是否代表一整屏移动端页面。
///
/// Port of `computeIsMobileFullScreen` (orchestrator-plan-classify.ts:41-58):
/// 窄(≤480)且高(≥480)即整屏;窄而高度为 0/auto 时用 subtask 数 ≥2
/// 区分"整屏多区块页面"与单卡片 Type 0 组件。TS 的 WeakMap memo 是为了
/// 跨 status-bar strip 保持一致——Rust 在 strip 之后的最终 plan 上直接算,
/// 无需 memo。
fn is_mobile_full_screen(plan: &OrchestratorPlan) -> bool {
    if plan.root_frame.width > 480.0 {
        return false;
    }
    if plan.root_frame.height >= 480.0 {
        return true;
    }
    plan.subtasks.len() >= 2
}

/// Build the sub-agent style-guide instruction block for the planner-selected
/// guide. Port of `buildSubAgentStyleGuideInstruction`
/// (orchestrator-sub-agent-compact.ts:78-124).
///
/// RUST ADAPTATION: TS emits `$color-*` refs (which it seeds into
/// `doc.variables`); Rust does NOT seed style-guide vars, so refs wouldn't
/// resolve — we emit the guide's concrete HEX values instead. Same effect:
/// the sub-agent uses the selected palette rather than inventing one.
/// Returns `None` when no guide name is set or it isn't in the registry.
fn build_style_guide_instruction(
    style_guide_name: Option<&str>,
    tier: ModelTier,
) -> Option<String> {
    let name = style_guide_name?;
    let opts = SelectOptions {
        name: Some(name.to_string()),
        ..Default::default()
    };
    let guide = select_style_guide(style_guide_registry(), &opts)?;
    let v = extract_style_guide_values(&guide.content);

    let color_line = |label: &str, hex: &Option<String>| -> Option<String> {
        hex.as_ref().map(|h| format!("- {label}: {h}"))
    };
    let colors: Vec<String> = [
        color_line("Background", &v.colors.background),
        color_line("Surface", &v.colors.surface),
        color_line("Accent", &v.colors.accent),
        color_line("Text", &v.colors.text_primary),
        color_line("Secondary text", &v.colors.text_secondary),
        color_line("Muted text", &v.colors.text_muted),
        color_line("Border", &v.colors.border),
    ]
    .into_iter()
    .flatten()
    .collect();

    // Full tier: the whole guide + an exact-hex palette appendix.
    if tier == ModelTier::Full {
        let mut s = format!(
            "VISUAL STYLE GUIDE (follow these specifications exactly):\n{}",
            guide.content.trim()
        );
        if !colors.is_empty() {
            s.push_str(
                "\n\nPALETTE — use these EXACT hex colors, do NOT invent a conflicting palette:\n",
            );
            s.push_str(&colors.join("\n"));
        }
        return Some(s);
    }

    // Standard/Basic: a compact summary.
    let mut lines = vec![format!("VISUAL STYLE GUIDE SUMMARY ({name}):")];
    let tags: Vec<String> = guide.tags.iter().take(6).cloned().collect();
    if !tags.is_empty() {
        lines.push(format!("- Tags: {}", tags.join(", ")));
    }
    lines.extend(colors);
    if let Some(f) = &v.typography.display_font {
        lines.push(format!("- Heading font: {f}"));
    }
    if let Some(f) = &v.typography.body_font {
        lines.push(format!("- Body font: {f}"));
    }
    if let Some(r) = v.radius.card {
        lines.push(format!("- Card radius: {r}"));
    }
    if let Some(r) = v.radius.button {
        lines.push(format!("- Button radius: {r}"));
    }
    lines.push(
        "Use these EXACT hex colors in your fills — do not invent a conflicting palette.".into(),
    );
    Some(lines.join("\n"))
}

fn resolve_generation_skills_after_prompt_filter(
    intent: &str,
    opts: &ResolveOptions,
    tier: ModelTier,
    is_mobile_screen: bool,
    design_system_covered: bool,
    minimal_skills: bool,
    reduced_complexity: bool,
) -> (
    Vec<ResolvedSkill>,
    SkillLoadReport,
    Vec<(String, DropReason)>,
) {
    let total_budget = opts
        .budget_override
        .unwrap_or_else(|| Phase::Generation.default_budget());
    let phase_skills: Vec<op_ai_skills::SkillEntry> = get_skills_by_phase(Phase::Generation)
        .into_iter()
        .cloned()
        .collect();
    let matched = filter_by_intent(&phase_skills, intent, &opts.flags);

    let mut dropped: Vec<DroppedSkill> = phase_skills
        .iter()
        .filter(|candidate| !matched.iter().any(|m| m.meta.name == candidate.meta.name))
        .map(|candidate| DroppedSkill {
            name: candidate.meta.name.clone(),
            reason: DropReason::IntentMiss,
        })
        .collect();

    let mut dynamic = opts.dynamic_content.clone();
    dynamic
        .entry("recentHistory".to_string())
        .or_insert_with(|| "No recent history.".to_string());
    let injected: Vec<op_ai_skills::SkillEntry> = matched
        .into_iter()
        .map(|mut skill| {
            skill.content = inject_dynamic_content(&skill.content, &dynamic);
            skill
        })
        .collect();

    let (filtered_entries, filter_drops) = apply_skill_filter(
        injected,
        tier,
        is_mobile_screen,
        design_system_covered,
        minimal_skills,
        reduced_complexity,
    );
    // Honor caller-pinned skills (force-included, budget-exempt) — same
    // mechanism `resolve_skills` uses on the non-mobile path. Empty by default,
    // so a no-library mobile generation is unchanged.
    let pinned: Vec<&str> = opts.pinned_skills.iter().map(String::as_str).collect();
    let trimmed = trim_by_budget_pinned(&filtered_entries, total_budget, intent, &pinned);

    for entry in &filtered_entries {
        if !trimmed.iter().any(|kept| kept.meta.name == entry.meta.name) {
            dropped.push(DroppedSkill {
                name: entry.meta.name.clone(),
                reason: DropReason::BudgetExhausted,
            });
        }
    }

    let included: Vec<SkillLoadEntry> = trimmed
        .iter()
        .map(|skill| SkillLoadEntry {
            name: skill.meta.name.clone(),
            category: skill.meta.category,
            token_count: skill.token_count,
            truncated: skill.truncated,
        })
        .collect();
    let budget_used = included.iter().map(|entry| entry.token_count).sum();
    let report = SkillLoadReport {
        included,
        dropped,
        budget_used,
        budget_max: total_budget,
    };

    (trimmed, report, filter_drops)
}

/// Max component entries listed in the AVAILABLE COMPONENTS manifest. A large
/// harvested library (shadcn, an imported design kit) can hold hundreds of
/// masters; listing them all would blow the prompt budget, so the manifest
/// caps at this many (grouped by category, alphabetical within a category) and
/// notes the remainder.
const MAX_COMPONENT_MANIFEST_ENTRIES: usize = 60;

/// Best-effort category bucket for a component, derived from its name. Pencil /
/// shadcn kits name components like "Primary Button", "Card", "Nav Item",
/// "Input"; bucketing by a recognised keyword groups the manifest so the model
/// scans a short, readable list instead of a flat dump.
fn component_category(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    let has = |kw: &str| n.contains(kw);
    if has("button") || has("btn") || has("cta") {
        "Buttons"
    } else if has("input") || has("field") || has("textarea") || has("select") || has("search") {
        "Inputs"
    } else if has("card") || has("tile") || has("panel") {
        "Cards"
    } else if has("nav") || has("tab") || has("menu") || has("sidebar") || has("breadcrumb") {
        "Navigation"
    } else if has("badge") || has("chip") || has("tag") || has("pill") || has("label") {
        "Badges"
    } else if has("avatar") || has("icon") || has("image") || has("logo") {
        "Media"
    } else if has("modal") || has("dialog") || has("popover") || has("tooltip") || has("toast") {
        "Overlays"
    } else if has("table") || has("row") || has("list") || has("cell") {
        "Tables & Lists"
    } else if has("header") || has("footer") || has("hero") || has("section") {
        "Layout"
    } else {
        "Other"
    }
}

/// Stable category order so the manifest reads consistently across runs.
const COMPONENT_CATEGORY_ORDER: &[&str] = &[
    "Buttons",
    "Inputs",
    "Cards",
    "Navigation",
    "Badges",
    "Media",
    "Overlays",
    "Tables & Lists",
    "Layout",
    "Other",
];

/// Build the AVAILABLE COMPONENTS manifest block for the generation prompt.
///
/// Returns `None` when the library is empty — so a no-component document's
/// prompt is byte-for-byte unchanged (the block + the `component-composition`
/// skill flag only fire when masters exist). When present, the block lists
/// `id (Name)` entries grouped by category, capped at
/// [`MAX_COMPONENT_MANIFEST_ENTRIES`], plus a one-line instruction pointing the
/// model at the `ref` + `descendants` syntax taught by the
/// `component-composition` skill.
///
/// `manifest_on` branches the one-line instantiation instruction by which
/// output protocol is active. The default raw/loop path emits a bare PenNode
/// (`{"type":"ref",…}`); the element-manifest path emits an `el` line
/// (`{"el":"ref",…}`). Telling a manifest-arm model to emit raw-node syntax
/// contradicts the dominant `el`-line contract, so it emits rich `el` kinds and
/// 0 component refs — the instruction must match the live protocol.
fn available_components_manifest(
    components: &ComponentLibrary,
    manifest_on: bool,
) -> Option<String> {
    if components.is_empty() {
        return None;
    }
    // Bucket components by category, preserving registry order within a bucket.
    let mut by_category: HashMap<&'static str, Vec<(&str, &str)>> = HashMap::new();
    for c in &components.components {
        by_category
            .entry(component_category(&c.name))
            .or_default()
            .push((c.id.as_str(), c.name.as_str()));
    }

    let total = components.len();
    let ref_syntax = if manifest_on {
        "an `el:\"ref\"` line"
    } else {
        "a `ref` node"
    };
    let mut lines = vec![format!(
        "AVAILABLE COMPONENTS ({total} reusable components in this document — \
         PREFER instantiating these with {ref_syntax} over building from scratch):"
    )];
    let mut listed = 0usize;
    'outer: for cat in COMPONENT_CATEGORY_ORDER {
        let Some(entries) = by_category.get(*cat) else {
            continue;
        };
        if entries.is_empty() {
            continue;
        }
        lines.push(format!("{cat}:"));
        for (id, name) in entries {
            if listed >= MAX_COMPONENT_MANIFEST_ENTRIES {
                lines.push(format!(
                    "  …and {} more not listed (ask only for the ids above).",
                    total - listed
                ));
                break 'outer;
            }
            lines.push(format!("  - {id} ({name})"));
            listed += 1;
        }
    }
    // Branch the instantiation example by the active output protocol so the
    // model is never told to emit a syntax that contradicts its line contract.
    // The example is COMPLETE (not just the envelope) so this block is
    // self-sufficient: even if the component-composition skill were ever trimmed
    // out, the model still has a usable, copy-pasteable instruction.
    let instruction = if manifest_on {
        "To use one, emit it as a manifest line — `el:\"ref\"`, the component id, nest it \
         under a section with `in:<line>`, and override its text/fill via `descendants` \
         (NO `id` field — the system assigns ids). Example:\n  \
         {\"el\":\"section\",\"role\":\"actions\"}\n  \
         {\"el\":\"ref\",\"in\":1,\"ref\":\"<id from above>\",\"descendants\":{\"<descendant-id>\":{\"content\":\"Get started\"}}}\n\
         Only build an element by hand when no component above fits."
    } else {
        "To use one, emit a single node — `type:\"ref\"`, the component id, its `_parent`, and \
         override its text/fill via `descendants` (it needs no `children`). Example:\n  \
         {\"_parent\":\"<container-id>\",\"id\":\"<your-id>\",\"type\":\"ref\",\"ref\":\"<id from above>\",\"descendants\":{\"<descendant-id>\":{\"content\":\"Get started\"}}}\n\
         Only build an element by hand when no component above fits."
    };
    lines.push(instruction.to_string());
    Some(lines.join("\n"))
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
/// * `components` — the document's reusable-component registry. When
///   non-empty it injects an AVAILABLE COMPONENTS manifest + raises the
///   `hasReusableComponents` flag (loads the `component-composition`
///   skill). Empty (the default path) ⇒ prompt unchanged.
pub fn build_subagent_prompt(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    components: &ComponentLibrary,
) -> (CallRequest, SkillLoadReport) {
    // Element-manifest protocol (spec 2026-06-10-element-manifest-v2): only on
    // the full first attempt — the retry ladder (reduced/minimal) falls back
    // to the smaller raw-JSONL prompt, and `parse_manifest` returning `None`
    // on such output routes parsing back through `parse_nodes`.
    let manifest_on =
        crate::manifest::manifest_enabled_for_model(req.model.as_deref().unwrap_or(""))
            && !reduced_complexity
            && !minimal_skills;
    build_subagent_prompt_with_manifest(
        subtask,
        plan,
        req,
        abort,
        reduced_complexity,
        minimal_skills,
        manifest_on,
        components,
    )
}

/// Env-independent core of [`build_subagent_prompt`] — `manifest_on` is a
/// parameter so tests can exercise the manifest wiring without touching
/// the process-global `OPENPENCIL_MANIFEST` variable.
#[allow(clippy::too_many_arguments)]
fn build_subagent_prompt_with_manifest(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    manifest_on: bool,
    components: &ComponentLibrary,
) -> (CallRequest, SkillLoadReport) {
    // Resolve the full generation skill set, then apply tier-gated filtering.
    let model_id = req.model.as_deref().unwrap_or("");
    let tier = resolve_model_profile(model_id).tier;

    // Available reusable components → AVAILABLE COMPONENTS manifest +
    // `hasReusableComponents` flag (loads the `component-composition` skill).
    // `None` when the registry is empty, so the default no-component path is
    // byte-for-byte unchanged.
    let component_manifest = available_components_manifest(components, manifest_on);
    let has_reusable_components = component_manifest.is_some();

    // design.md payload for the `{{designMdContent}}` template. If the
    // structured policy summary is empty (a bare-minimum design.md with only
    // free-form text), fall back to the raw markdown so the sub-agent still
    // sees the spec. Port of orchestrator-sub-agent.ts:379-384.
    let design_md_content = req
        .design_md
        .as_ref()
        .map(|spec| {
            let structured = build_design_md_style_policy(spec);
            let structured = structured.trim();
            if structured.is_empty() {
                spec.raw.trim().to_string()
            } else {
                structured.to_string()
            }
        })
        .unwrap_or_default();
    let has_design_md = !design_md_content.is_empty();
    // Rust `OrchestratorPlan` carries only the style-guide NAME (the TS
    // `selectedStyleGuideContent` content field has no Rust equivalent yet),
    // so `style_guide_name.is_some()` is the faithful proxy for "a guide was
    // selected". Port of the flag block in orchestrator-sub-agent.ts:396-416.
    let no_style_guide_match = plan.style_guide_name.is_none() && !has_design_md;

    let mut flags = HashMap::new();
    flags.insert("isBasicTier".to_string(), tier == ModelTier::Basic);
    flags.insert("hasDesignMd".to_string(), has_design_md);
    // No existing-document variable context is wired into `DesignRequest`
    // (TS sources this from `request.context.variables`), so this is always
    // false on the Rust path today.
    flags.insert("hasVariables".to_string(), false);
    flags.insert("noStyleGuideMatch".to_string(), no_style_guide_match);
    // Element-tools (N-tool) path is not ported to Rust (feature-flag off in
    // TS production); `elements`/`elements-cookbook` therefore stay gated off.
    flags.insert("hasMcpTools".to_string(), false);
    flags.insert("hasManifest".to_string(), manifest_on);
    flags.insert("hasReusableComponents".to_string(), has_reusable_components);

    let mut dynamic_content = HashMap::new();
    if has_design_md {
        dynamic_content.insert("designMdContent".to_string(), design_md_content);
    }
    if manifest_on {
        // The catalog is generated from the embedded TS schemas so the
        // prompt can never drift from the builders.
        dynamic_content.insert(
            "elementManifestCatalog".to_string(),
            op_mcp::element_manifest::manifest_catalog(),
        );
    }

    let is_mobile_screen = is_mobile_full_screen(plan);
    // Look up the planner-selected style guide by name and build a block that
    // injects its palette/fonts into the sub-agent prompt (port of
    // `buildSubAgentStyleGuideInstruction`). When present this REPLACES the
    // generic `design-system` skill.
    let style_guide_instruction =
        build_style_guide_instruction(plan.style_guide_name.as_deref(), tier);
    // `design-system` is dropped when ANOTHER styling source already covers it:
    // the `design-md` skill (`has_design_md`), the `style-defaults` skill (loads
    // on `noStyleGuideMatch`), OR the style-guide instruction block just built.
    // Keeping it alongside any of those would inject design-system's conflicting
    // "output ONLY a JSON token object" header redundantly (Codex review).
    let design_system_covered =
        has_design_md || no_style_guide_match || style_guide_instruction.is_some();
    let explicit_tokens = extract_explicit_design_tokens(&req.prompt);
    let explicit_token_instruction = explicit_design_token_instruction(explicit_tokens);

    // Mobile UI guardrails live in the flag-gated `mobile-ui` skill rather than
    // inline prompt strings (user direction 2026-06-23: control generation via
    // skills, not hardcoded text in prompt.rs). Set the gate flag — matching the
    // old inline `plan.root_frame.width <= 480.0` condition exactly — and inject
    // the few dynamic values those rules reference.
    let is_mobile_layout = plan.root_frame.width <= 480.0;
    flags.insert("isMobileScreen".to_string(), is_mobile_layout);
    if is_mobile_layout {
        let mobile_spacing = explicit_tokens.spacing.map(format_design_number);
        let mobile_radius = explicit_tokens.radius.map(format_design_number);
        let rhythm = if let Some(spacing) = mobile_spacing.as_deref() {
            format!(
                "MOBILE VERTICAL RHYTHM: Keep every section root height=\"fit_content\" and do not insert blank spacer frames or empty bands to fill the planned region. Use {spacing}px as the default gap/spacing for header-to-search, search-to-next-section, module, grid, card, and internal component rhythm. Do not mix 16/20/24/32px gaps when the user explicitly requested {spacing}px spacing."
            )
        } else {
            "MOBILE VERTICAL RHYTHM: Keep every section root height=\"fit_content\" and do not insert blank spacer frames or empty bands to fill the planned region. Header-to-search spacing should be 12px, search-to-next-section 12-16px, major module gaps 16-24px, and internal gaps usually 8/12px.".to_string()
        };
        dynamic_content.insert("mobileRhythm".to_string(), rhythm);
        dynamic_content.insert(
            "mobileSearchRadius".to_string(),
            mobile_radius.as_deref().unwrap_or("8-12").to_string(),
        );
        dynamic_content.insert(
            "mobileGridGap".to_string(),
            mobile_spacing.as_deref().unwrap_or("12").to_string(),
        );
    }

    // Tier-scaled budget override (orchestrator-sub-agent.ts:414-415).
    // Basic mobile carries the `mobile-app` + `mobile-ui` domain skills after the
    // compact filter; the `mobile-ui` rules used to be appended to the user prompt
    // (uncounted) and now live in a budgeted skill, so the budget grows by ~its
    // size — the TOTAL prompt is unchanged, the rules just moved user→system.
    let budget_override = match tier {
        ModelTier::Basic if is_mobile_layout || is_mobile_screen => Some(9200),
        ModelTier::Basic => Some(5200),
        ModelTier::Standard if is_mobile_layout => Some(9500),
        ModelTier::Standard => Some(6500),
        ModelTier::Full => None,
    };

    // Force-include the component-instance teaching whenever a reusable-component
    // library is loaded. When a library is present the model already receives the
    // AVAILABLE COMPONENTS *list*; the `component-composition` skill carries the
    // HOW-to-instantiate teaching (`ref` + `descendants` syntax) — without it the
    // model gets the catalog but no usable instruction and emits 0 refs. On the
    // tight non-mobile Basic budget (5200, of which base skills already use
    // ~3900) the skill is otherwise dropped by BudgetExhausted, so we pin it
    // (budget-exempt) here. Empty on every no-library path ⇒ no change there.
    let pinned_skills = if has_reusable_components {
        vec!["component-composition".to_string()]
    } else {
        Vec::new()
    };
    let opts = ResolveOptions {
        flags,
        dynamic_content,
        budget_override,
        pinned_skills,
        ..Default::default()
    };
    let intent = subtask_intent(req, subtask);
    let (mut filtered, resolve_report, filter_drops) =
        if tier == ModelTier::Basic && is_mobile_screen {
            resolve_generation_skills_after_prompt_filter(
                &intent,
                &opts,
                tier,
                is_mobile_screen,
                design_system_covered,
                minimal_skills,
                reduced_complexity,
            )
        } else {
            let agent_ctx = resolve_generation_skills(&intent, &opts);
            let resolved = agent_ctx.skills;
            let (filtered, filter_drops) = apply_skill_filter(
                resolved,
                tier,
                is_mobile_screen,
                design_system_covered,
                minimal_skills,
                reduced_complexity,
            );
            (filtered, agent_ctx.report, filter_drops)
        };
    // The manifest protocol REPLACES the raw-JSONL output format —
    // carrying both would contradict (and burn Basic-tier budget).
    if manifest_on {
        filtered.retain(|s| {
            s.skill_name() != "jsonl-format" && s.skill_name() != "jsonl-format-simplified"
        });
    }
    // Pull the element-manifest skill out of priority order and append
    // it LAST, right above the output contract — recency wins in long
    // prompts. ab-v9: deepseek's Full-tier prompt runs ~8k tokens and
    // it kept hand-rolling past a top-of-prompt catalog (58% vs the
    // Basic-tier models' 85% whose compact prompts kept it close).
    let manifest_skill = if manifest_on {
        filtered
            .iter()
            .position(|s| s.skill_name() == "element-manifest")
            .map(|idx| filtered.remove(idx))
    } else {
        None
    };

    let mut system_prompt = filtered
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if let Some(skill) = &manifest_skill {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(skill.content.as_str());
    }
    system_prompt.push_str("\n\n");
    // Flat `_parent` for ALL tiers — validated that Basic-tier models
    // (MiniMax M2.7/M3 are Basic) emit clean `_parent` trees with it. The
    // `jsonl-format` + `jsonl-format-simplified` skills both teach `_parent`,
    // so this agrees with whichever skill the tier loads (no contradiction).
    // Manifest mode swaps in the element-manifest contract instead.
    system_prompt.push_str(if manifest_on {
        MANIFEST_FORMAT
    } else {
        NODE_FORMAT
    });
    // Append the selected style guide's palette/fonts so the sub-agent follows
    // it instead of inventing a conflicting one.
    if let Some(sg) = &style_guide_instruction {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(sg);
    }
    if let Some(instruction) = &explicit_token_instruction {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(instruction);
        system_prompt
            .push_str("\nThis instruction overrides style-guide examples when they conflict.");
    }
    // Available-components manifest LAST — recency wins, and the model should
    // consult the concrete id list right before producing nodes. The
    // `component-composition` skill (loaded via `hasReusableComponents`) carries
    // the `ref` + `descendants` syntax; this block carries the actual ids.
    if let Some(manifest) = &component_manifest {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(manifest);
    }

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
    let explicit_user_token_block = explicit_token_instruction
        .as_ref()
        .map(|instruction| format!("{instruction}\n\n"))
        .unwrap_or_default();

    // Deterministic element nomination (ab-v9.1 de-randomization lever):
    // weak models miss catalog kinds whose names sit literally in the
    // brief (setting_row 10/10, toolbar 5/10 despite the word "toolbar").
    // Token-matching the subtask's own text pulls that recognition step
    // out of the model's attention. Multi-section plans scan only the
    // subtask-local text so one section's nouns don't pollute another's
    // hints; single-section plans scan the whole brief too.
    let element_hints = if manifest_on {
        let mut hint_src = subtask.label.clone();
        if let Some(items) = subtask.elements.as_ref() {
            hint_src.push(' ');
            hint_src.push_str(items);
        }
        if plan.subtasks.len() <= 1 {
            hint_src.push(' ');
            hint_src.push_str(&req.prompt);
        }
        crate::manifest_hints::nominate_kinds(
            &hint_src,
            &op_mcp::element_manifest::known_element_kinds(),
        )
    } else {
        Vec::new()
    };

    // Three constraints differ by output protocol: the raw-JSONL path has
    // the model author its own root frame + ids; the manifest path forbids
    // exactly that (system-assigned ids, system-owned section root).
    let (root_rule, nesting_rule, output_rule) = if manifest_on {
        (
            "Do NOT create a page wrapper or root frame -- emit element/section manifest lines only; the system wraps them under this section's root automatically.".to_string(),
            "Group elements with {\"el\":\"section\"} lines and `in` line-number references (see ELEMENT MANIFEST rules). Lines without `in` stack vertically in output order.".to_string(),
            "Output ONLY manifest JSONL lines in the EXACT format the system prompt specifies above -- no prose, no id fields, no extra wrapping.".to_string(),
        )
    } else {
        (
            format!("Root frame: id=\"{}-root\", width=\"fill_container\", height=\"fit_content\", layout=\"vertical\". NEVER use fixed pixel height on root -- let content determine height.", subtask.id_prefix),
            "ALL nodes must be descendants of the root frame -- every non-root node must be nested under its parent (row -> its cards -> each card's content), in whichever format the system prompt specifies. No floating/orphan nodes; a flat sibling list with no parent links collapses into a vertical stack.".to_string(),
            format!("IDs prefix=\"{}-\". Output ONLY the structured nodes in the EXACT format the system prompt specifies above -- no prose, no extra wrapping.", subtask.id_prefix),
        )
    };
    let mut user_prompt = format!(
        "Page sections:\n{}\n\n\
Generate ONLY \"{}\" (~{:.0}px of content).{}\n\
Overall design: {}\n\n\
{}\
CRITICAL LAYOUT CONSTRAINTS:\n\
- {}\n\
- Target content amount: ~{:.0}px tall. Generate enough elements to fill this area.\n\
- DENSITY: Do NOT pack the area edge-to-edge. Prefer fewer, stronger modules with visible negative space; most sections should have 3-5 primary rows/cards at most.\n\
- VISUAL HIERARCHY: Each section must have one clear focal element, secondary supporting text, and quieter metadata. Avoid equal-weight blocks competing for attention.\n\
- SPACING CONSISTENCY: Use a single outer content gutter and consistent internal gaps. Do not create nested wrappers with conflicting padding or content touching edges.\n\
- CRAFT POLISH: Add refinement through restrained 1px low-contrast borders, tonal surfaces, small state badges, and subtle shadows. Avoid template-like thick outlines, giant pills, or flat blocks with no micro-detail.\n\
- MEDIA CONSISTENCY: Use photographic images sparingly and keep them visually consistent in subject, crop, tone, and radius. For food/category UI, prefer cohesive icon or illustration tiles over random unrelated photos.\n\
- ICON SCALE: Icons support content; keep most icons 16-22px inside 36-48px controls. Avoid oversized circular icon bubbles or repeated identical icon treatments unless the design brief calls for them.\n\
- ACCENT DISCIPLINE: Reserve saturated accent color for one primary CTA or promo plus small highlights. Do not apply it to every icon, label, border, and large surface at once.\n\
- SIGNATURE MOMENT: Give the first viewport one memorable focal module that feels custom to the brief: distinctive composition, branded surface, expressive hero image/illustration, or an editorial product moment. Supporting modules must stay quieter.\n\
- WOW FACTOR: Make the design feel bespoke through domain-specific composition, confident cropping, custom icon rhythm, and one precise visual idea. Do not rely on generic tinted wrappers, heavy shadows, emoji, or repeated rounded boxes to look polished.\n\
- COMPOSITIONAL CONTRAST: Create interest through scale contrast, asymmetric balance, layered depth, and one clear focal path. Do not make every card, chip, icon, and CTA the same visual weight.\n\
- PREMIUM DETAIL: Use high-quality details such as precise alignment, consistent radii, quiet dividers, tonal badges, subtle image masks, and purposeful shadows. Prefer one polished detail over many loud decorations.\n\
- NO DECORATION SPAM: Do not add random blobs, repeated icon circles, excessive gradients, oversized badges, or unrelated photos just to make the design look busy. Every visual flourish must support the product story.\n\
- {}\n\
- NEVER set x or y on children inside layout frames.\n\
- Use \"fill_container\" for children that stretch, \"fit_content\" for shrink-wrap sizing.\n\
- SECTION BACKGROUND: do NOT set fill on your section root frame. Only set fill on cards, buttons, chips, badges, and other visually distinct components.\n\
- TYPOGRAPHY HIERARCHY: Do NOT make every text bold. Use 700 only for primary headings, 600 for buttons/key labels, 500 for short chips/nav labels, and 400 for body text, placeholders, subtitles, metadata, and captions.\n\
- ICONS: use icon_font with lucide iconFontName; never use path nodes for icons.\n\
- {}",
        section_list,
        subtask.label,
        subtask.region.height,
        my_elements,
        req.prompt,
        explicit_user_token_block,
        root_rule,
        subtask.region.height,
        nesting_rule,
        output_rule,
    );

    if !element_hints.is_empty() {
        user_prompt.push_str(&format!(
            "\n\nELEMENT HINTS: this section's brief matches these catalog kinds: {}.\n\
- Check each hinted kind in the catalog and declare it as its own {{\"el\":...}} line unless it clearly does not fit this section.\n\
- Most specific wins: when a composite hinted kind already contains a smaller hinted one (a setting_row already has its switch), declare ONLY the composite.\n\
- NEVER hand-compose a hinted kind out of section/text/icon_font lines.",
            element_hints.join(", ")
        ));
    }

    // (Mobile UI guardrails now load from the `mobile-ui` skill — see the
    // `isMobileScreen` flag + dynamic-content setup above.)

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

    // Assemble the per-subtask skill-load report from the FINAL skill set
    // (post tier/dedup/manifest filtering). `budget_max` reflects the tier
    // budget override (None == Full tier's 8000 default).
    let budget_max = budget_override.unwrap_or(8000);
    let included: Vec<SkillLoadEntry> = filtered
        .iter()
        .chain(manifest_skill.iter())
        .map(|s| SkillLoadEntry {
            name: s.skill_name().to_string(),
            category: s.meta.category,
            token_count: s.token_count,
            truncated: s.truncated,
        })
        .collect();
    let budget_used: u32 = included.iter().map(|e| e.token_count).sum();
    // Merge resolve-time drops (IntentMiss + BudgetExhausted from B0) with
    // tier/dedup/mode drops captured by apply_skill_filter (B3b).
    let mut dropped = resolve_report.dropped;
    dropped.extend(
        filter_drops
            .into_iter()
            .map(|(name, reason)| op_ai_skills::DroppedSkill { name, reason }),
    );
    let report = SkillLoadReport {
        included,
        dropped,
        budget_used,
        budget_max,
    };

    (
        CallRequest {
            system_prompt,
            user_prompt,
            model: req.model.clone(),
            provider: req.provider.clone(),
            timeout: t.hard,
            abort,
            no_text_timeout: Some(t.no_text),
            first_text_timeout: Some(t.first_text),
        },
        report,
    )
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
