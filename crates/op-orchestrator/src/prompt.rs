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

/// 解析 generation 阶段 skill 列表,带 flag/dynamic/budget 选项(供下游过滤)。
fn resolve_generation_skills(
    message: &str,
    opts: &op_ai_skills::ResolveOptions,
) -> Vec<op_ai_skills::ResolvedSkill> {
    op_ai_skills::resolve_skills(op_ai_skills::Phase::Generation, message, opts).skills
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
    // Element-manifest protocol (spec 2026-06-10-element-manifest-v2): only on
    // the full first attempt — the retry ladder (reduced/minimal) falls back
    // to the smaller raw-JSONL prompt, and `parse_manifest` returning `None`
    // on such output routes parsing back through `parse_nodes`.
    let manifest_on = crate::manifest::manifest_enabled() && !reduced_complexity && !minimal_skills;
    build_subagent_prompt_with_manifest(
        subtask,
        plan,
        req,
        abort,
        reduced_complexity,
        minimal_skills,
        manifest_on,
    )
}

/// Env-independent core of [`build_subagent_prompt`] — `manifest_on` is a
/// parameter so tests can exercise the manifest wiring without touching
/// the process-global `OPENPENCIL_MANIFEST` variable.
fn build_subagent_prompt_with_manifest(
    subtask: &Subtask,
    plan: &OrchestratorPlan,
    req: &DesignRequest,
    abort: AbortFlag,
    reduced_complexity: bool,
    minimal_skills: bool,
    manifest_on: bool,
) -> CallRequest {
    // Resolve the full generation skill set, then apply tier-gated filtering.
    let model_id = req.model.as_deref().unwrap_or("");
    let tier = resolve_model_profile(model_id).tier;

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

    // Tier-scaled budget override (orchestrator-sub-agent.ts:414-415).
    let budget_override = match tier {
        ModelTier::Basic => Some(5200),
        ModelTier::Standard => Some(6500),
        ModelTier::Full => None,
    };

    let opts = op_ai_skills::ResolveOptions {
        flags,
        dynamic_content,
        budget_override,
        ..Default::default()
    };
    let resolved = resolve_generation_skills(&subtask.label, &opts);
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
    let mut filtered = apply_skill_filter(
        resolved,
        tier,
        is_mobile_screen,
        design_system_covered,
        minimal_skills,
        reduced_complexity,
    );
    // The manifest protocol REPLACES the raw-JSONL output format —
    // carrying both would contradict (and burn Basic-tier budget).
    if manifest_on {
        filtered.retain(|s| {
            s.skill_name() != "jsonl-format" && s.skill_name() != "jsonl-format-simplified"
        });
    }

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
CRITICAL LAYOUT CONSTRAINTS:\n\
- {}\n\
- Target content amount: ~{:.0}px tall. Generate enough elements to fill this area.\n\
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
        root_rule,
        subtask.region.height,
        nesting_rule,
        output_rule,
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
