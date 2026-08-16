//! Generation-skill resolution and the style-guide instruction block
//! (including the resolved-style public helper).

use super::*;

// `resolve_generation_skills` (a bare `resolve_skills` wrapper) used to serve
// every sub-agent path except Basic-mobile. It is gone rather than kept for
// symmetry: calling it meant budgeting BEFORE the compaction, which is the
// defect `resolve_generation_skills_after_prompt_filter` below exists to
// avoid, so leaving it in reach would leave the wrong order one call away.

/// 该 plan 是否代表一整屏移动端页面。
///
/// Port of `computeIsMobileFullScreen` (orchestrator-plan-classify.ts:41-58):
/// 窄(≤480)且高(≥480)即整屏;窄而高度为 0/auto 时用 subtask 数 ≥2
/// 区分"整屏多区块页面"与单卡片 Type 0 组件。TS 的 WeakMap memo 是为了
/// 跨 status-bar strip 保持一致——Rust 在 strip 之后的最终 plan 上直接算,
/// 无需 memo。
pub(super) fn is_mobile_full_screen(plan: &OrchestratorPlan) -> bool {
    if plan.root_frame.width > 480.0 {
        return false;
    }
    if plan.root_frame.height >= 480.0 {
        return true;
    }
    plan.subtasks.len() >= 2
}

/// 该 plan 是否代表一块投影幻灯片(16:9 定尺画板)。
///
/// The deck preset is a fixed 1920x1080 board (`design_type::DECK_PRESET`), and
/// `decomposition` mandates that size for every type-4 plan — nothing else in
/// the corpus asks for a root that is both this wide AND fixed-height (landing
/// pages and dashboards carry `height = 0` so they auto-expand). That pair is
/// therefore the deck signature.
///
/// It exists for the same reason `is_mobile_full_screen` feeds the budget
/// override below: the deck teaching (`slides` + `deck-patterns`) is ~4000
/// tokens, so on the plain non-mobile tier budgets (Basic 5200 / Standard 6500,
/// against ~6200 of always-kept Base skills) BOTH are dropped for
/// `BudgetExhausted` and a weak model designs a deck with no deck guidance at
/// all — measured 2026-08-04, before this arm existed.
/// Routed through the single form classifier rather than comparing widths
/// here. The hand-rolled `w >= 1600 && h >= 900` this replaces was a fourth
/// geometric literal alongside the three in `design_form`, and it was LOOSER
/// in the one way that matters: with no aspect gate, a 1920×2000 long page
/// claimed the deck budget and spent it on slide teaching it could not use.
/// Under the classifier that page reads as [`DesignForm::Page`] and keeps the
/// ordinary page budget — see the test that pins that case.
pub(super) fn is_deck_board(plan: &OrchestratorPlan) -> bool {
    crate::design_type::classify_root_form(
        Some(plan.root_frame.width),
        Some(plan.root_frame.height),
    )
    .is_deck_board()
}

/// The portrait-card analogue of [`is_deck_board`]: a 900..=1280 wide,
/// taller-than-wide (aspect <= 2.0) board reads as [`DesignForm::Card`] and
/// claims the card budget arm — the same single-classifier routing so a
/// square (1080x1080) or long page keeps the ordinary page budget.
pub(super) fn is_card_board(plan: &OrchestratorPlan) -> bool {
    crate::design_type::classify_root_form(
        Some(plan.root_frame.width),
        Some(plan.root_frame.height),
    )
    .is_card_board()
}

/// Build the sub-agent style-guide instruction block for the planner-selected
/// guide. Port of `buildSubAgentStyleGuideInstruction`
/// (orchestrator-sub-agent-compact.ts:78-124).
///
/// RUST ADAPTATION: TS emits `$color-*` refs (which it seeds into
/// `doc.variables`); Rust does NOT seed style-guide vars, so refs wouldn't
/// resolve — we emit the guide's concrete HEX values instead. Same effect:
/// the sub-agent uses the selected palette rather than inventing one.
/// Returns `None` when no guide name is set, or it names neither a corpus
/// guide nor an imported one.
pub(crate) fn build_style_guide_instruction(
    style_guide_name: Option<&str>,
    tier: ModelTier,
) -> Option<String> {
    let name = style_guide_name?;
    // Both halves of the catalogue: a pinned import reaches the sub-agent
    // through exactly this call, and resolving only the corpus here would
    // have made pinning one silently do nothing.
    let guide = op_ai_skills::style_guide::find_style_guide(name)?;
    let v = extract_style_guide_values(&guide.content);

    let color_line = |label: &str, hex: &Option<String>| -> Option<String> {
        hex.as_ref().map(|h| format!("- {label}: {h}"))
    };
    let mut colors: Vec<String> = [
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
    // Floor under the summary tiers below, which send this list *instead of*
    // the document. A guide whose fields none of the extractors could fill
    // would otherwise produce "use these EXACT hex colors" followed by
    // nothing — an instruction to obey an empty list, which is worse than
    // sending no palette at all. Roles stay in the guide's own words because
    // the structured pass has already failed and guessing which colour is the
    // background is how the accent ends up behind the whole page.
    if colors.is_empty() {
        colors = op_ai_skills::style_guide::sample_palette(&guide.content, 6)
            .into_iter()
            .map(|sample| {
                if sample.role.is_empty() {
                    format!("- {}", sample.color)
                } else {
                    format!("- {} — {}", sample.color, sample.role)
                }
            })
            .collect();
    }

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
    let has_colors = !colors.is_empty();
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
    // Only when there is something to obey.
    if has_colors {
        lines.push(
            "Use these EXACT hex colors in your fills — do not invent a conflicting palette."
                .into(),
        );
    }
    Some(lines.join("\n"))
}

pub fn build_resolved_style_instruction(
    name: &str,
    params: &op_ai_skills::resolve_style::StyleParams,
) -> Option<String> {
    let guide = match resolve_style(name, params) {
        ResolveOutcome::Hit(guide) => guide,
        ResolveOutcome::Miss { .. } => return None,
    };
    let tokens = &guide.tokens;

    let mut lines = vec![
        format!(
            "RESOLVED STYLE REFERENCE ({} / {})",
            name.trim(),
            params.color_palette.trim()
        ),
        "Bake these reference values directly into node fills, text colors, borders, radii, and font fields. Do NOT create document variables. Do NOT call set_variables.".to_string(),
    ];
    push_resolved_string_tokens(&mut lines, "surface", &tokens.surface);
    push_resolved_string_tokens(&mut lines, "foreground", &tokens.foreground);
    push_resolved_string_tokens(&mut lines, "accent", &tokens.accent);
    push_resolved_string_tokens(&mut lines, "border", &tokens.border);
    for (role, value) in &tokens.rounded {
        lines.push(format!("rounded.{role}={}px", format_design_number(*value)));
    }
    lines.push(format!(
        "typography: headings={}, body={}, captions={}, data={}",
        tokens.typography.headings,
        tokens.typography.body,
        tokens.typography.captions,
        tokens.typography.data
    ));
    for (role, value) in &tokens.on {
        let role = if role.starts_with("on-") {
            role.to_string()
        } else {
            format!("on-{role}")
        };
        lines.push(format!("{role}={value}"));
    }

    Some(lines.join("\n"))
}

pub(super) fn push_resolved_string_tokens(
    lines: &mut Vec<String>,
    prefix: &str,
    values: &std::collections::BTreeMap<String, String>,
) {
    for (role, value) in values {
        lines.push(format!("{prefix}.{role}={value}"));
    }
}

/// Resolve the generation skill set with the sub-agent compaction applied
/// BEFORE the budget knapsack, so the budget is never spent on a skill the
/// compaction is about to delete. This is the order every sub-agent prompt
/// uses; see the call site in `prompt_subagent` for what the other order cost.
///
/// `model_id` drives the DS P2-a overlay gate (the `model_families`
/// frontmatter field): a family-gated skill only enters the candidate set
/// when the normalized model id contains one of its families; an empty id
/// (the default) admits nothing gated, so every caller that does not know
/// its model resolves byte-for-byte as before. Strategic line: output
/// contracts belong in the public corpus, model behaviour adaptation belongs
/// in the DS experiment field (`skills/overlays/`); overlay teaching migrates
/// into the public skills only after ab validation graduates it.
///
/// Mirrors `op_ai_skills::resolve_skills` step for step (phase filter → intent
/// / flag match → dynamic-content injection → `trim_by_budget_pinned`) with
/// the compaction inserted between the third and fourth. **The one thing it
/// does not mirror is memory**: `resolve_skills` derives `{{recentHistory}}`
/// from `opts.memory.generation_history`, and this reimplementation falls back
/// to the empty-history text that derivation produces. That is exact — and
/// only exact — while the caller passes no memory, which the sub-agent path
/// does not (`ResolveOptions { .., ..Default::default() }`). A caller that
/// starts populating `memory` must route through `resolve_skills` or teach
/// this function the same derivation; the history helpers are private to
/// `op-ai-skills`, which is why this note exists instead of a call.
// Nine behaviour flags after `intent` / `model_id` / `opts` — the call site
// owns the derivation of each, and a struct would only move the pile.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_generation_skills_after_prompt_filter(
    intent: &str,
    model_id: &str,
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
    // Model-family gate (DS P2-a overlays) — family-gated skills leave the
    // candidate set before intent matching; the rest of the pipeline is
    // untouched for the survivors.
    let (phase_skills, gated_out) =
        op_ai_skills::resolver::filter_by_model_family(&phase_skills, model_id);
    let matched = filter_by_intent(&phase_skills, intent, &opts.flags);

    let mut dropped: Vec<DroppedSkill> = gated_out
        .iter()
        .map(|gated| DroppedSkill {
            name: gated.meta.name.clone(),
            reason: DropReason::ModelFamilyMiss,
        })
        .collect();
    dropped.extend(
        phase_skills
            .iter()
            .filter(|candidate| !matched.iter().any(|m| m.meta.name == candidate.meta.name))
            .map(|candidate| DroppedSkill {
                name: candidate.meta.name.clone(),
                reason: DropReason::IntentMiss,
            }),
    );

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
