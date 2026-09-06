//! `style_guide_context` tests — split out at the 800-line file cap.

use super::*;

#[test]
fn catalog_context_lists_all_guides() {
    let ctx = build_planning_style_guide_context(
        "a fintech dashboard",
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        None,
    );
    assert_eq!(ctx.metadata_count, style_guide_registry().len());
    assert!(ctx.snippet_count > 0); // Rich + full tier → 有 snippet
    assert!(ctx
        .available_style_guides
        .contains("Available style guides"));
}

/// The prompt this pair of tests leans on: proven by
/// `food_delivery_ranks_warm_food_mobile_guide_first` to rank
/// `warm-food-mobile-light` at the top of the automatic ranking.
const FOOD_PROMPT: &str = "a food delivery mobile app";
/// A guide the food prompt would never choose on its own.
const AGAINST_THE_GRAIN: &str = "developer-terminal-dark";

#[test]
fn a_pin_beats_the_prompt_ranking() {
    let ctx = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        Some(AGAINST_THE_GRAIN),
    );

    assert_eq!(ctx.top_guide_names, vec![AGAINST_THE_GRAIN.to_string()]);
    assert_eq!(
        ctx.metadata_count, 1,
        "the menu shrinks to the pinned entry"
    );
    assert!(
        ctx.available_style_guides
            .contains(&format!("\"styleGuideName\": \"{AGAINST_THE_GRAIN}\"")),
        "the pin must be stated as an output directive:\n{}",
        ctx.available_style_guides
    );
    assert!(
        !ctx.available_style_guides
            .contains("warm-food-mobile-light"),
        "a listed alternative is a choice the model can still make:\n{}",
        ctx.available_style_guides
    );
}

#[test]
fn a_stale_pin_falls_back_to_the_ranking_unchanged() {
    let auto = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        None,
    );
    let stale = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        Some("a-guide-that-was-retired"),
    );

    assert_eq!(stale.available_style_guides, auto.available_style_guides);
    assert_eq!(stale.top_guide_names, auto.top_guide_names);
}

#[test]
fn a_blank_pin_is_no_pin() {
    // Trimming matters: the pin arrives from a text-shaped source (the
    // document's editorMeta), and `Some("")` must not shrink the menu to
    // nothing or log a stale-pin warning on every request.
    for blank in ["", "   "] {
        assert!(resolve_pinned_style_guide(Some(blank)).is_none());
    }
    let auto = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        None,
    );
    let blank = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        Some("  "),
    );
    assert_eq!(blank.available_style_guides, auto.available_style_guides);
}

#[test]
fn design_md_outranks_a_pin() {
    let spec = jian_ops_schema::DesignMdSpec {
        raw: String::new(),
        project_name: None,
        visual_theme: Some("calm".into()),
        color_palette: None,
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    };
    let ctx = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        Some(&spec),
        Some(AGAINST_THE_GRAIN),
    );

    assert_eq!(ctx.top_guide_names, vec!["design-md-custom".to_string()]);
}

#[test]
fn a_pin_also_short_circuits_compact_planning() {
    let cp = crate::compact_prompt::build_compact_planning_prompt(
        FOOD_PROMPT,
        None,
        Some(AGAINST_THE_GRAIN),
    );
    assert_eq!(cp.selected_style_guide_name, AGAINST_THE_GRAIN);

    let auto = crate::compact_prompt::build_compact_planning_prompt(FOOD_PROMPT, None, None);
    assert_ne!(auto.selected_style_guide_name, AGAINST_THE_GRAIN);

    let stale =
        crate::compact_prompt::build_compact_planning_prompt(FOOD_PROMPT, None, Some("gone"));
    assert_eq!(
        stale.selected_style_guide_name,
        auto.selected_style_guide_name
    );
}

#[test]
fn minimal_mode_has_no_snippets() {
    let ctx = build_planning_style_guide_context(
        "a fintech dashboard",
        Some("claude-opus"),
        PlanningMode::Minimal,
        None,
        None,
    );
    assert_eq!(ctx.snippet_count, 0);
}

#[test]
fn design_md_branch_skips_catalog() {
    let spec = jian_ops_schema::DesignMdSpec {
        raw: String::new(),
        project_name: None,
        visual_theme: Some("calm".into()),
        color_palette: None,
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    };
    let ctx = build_planning_style_guide_context(
        "a page",
        Some("claude-opus"),
        PlanningMode::Rich,
        Some(&spec),
        None,
    );
    assert_eq!(ctx.metadata_count, 0);
    assert_eq!(ctx.top_guide_names, vec!["design-md-custom".to_string()]);
    assert!(ctx.available_style_guides.contains("custom design system"));
}

#[test]
fn unmatched_prompt_yields_only_tone_tag() {
    // tone 组(dark/light 互斥 if/else)永远 push 一个 tag,故 `tags`
    // 永不为空 —— TS 末尾的 `['minimal','light-mode']` 兜底是死分支。
    // 无关键词命中的 prompt → 只剩 tone tag(light-mode)。
    assert_eq!(infer_tags_from_prompt("xyz123"), vec!["light-mode"]);
}

#[test]
fn tone_and_visual_tags() {
    let t = infer_tags_from_prompt("a dark minimalist dashboard");
    assert!(t.contains(&"dark-mode".to_string()));
    assert!(t.contains(&"minimal".to_string()));
}

#[test]
fn industry_food_pushes_two_tags() {
    let t = infer_tags_from_prompt("a food delivery app");
    assert!(t.contains(&"food".to_string()));
    assert!(t.contains(&"warm-tones".to_string()));
    assert!(t.contains(&"friendly".to_string()));
}

#[test]
fn food_delivery_ranks_warm_food_mobile_guide_first() {
    let tags = infer_tags_from_prompt("a food delivery mobile app");
    let ranked = rank_style_guides_for_prompt(&tags, Platform::Mobile);
    assert_eq!(
        ranked.first().map(|guide| guide.name.as_str()),
        Some("warm-food-mobile-light"),
        "food delivery should prefer the food-specific mobile guide"
    );
}

#[test]
fn developer_pushes_developer_and_monospace() {
    let t = infer_tags_from_prompt("a coding tool");
    assert!(t.contains(&"developer".to_string()));
    assert!(t.contains(&"monospace".to_string()));
}

#[test]
fn no_dedup_source_order() {
    // fintech 可被多组 push;不去重(TS 行为)
    let t = infer_tags_from_prompt("a fintech banking finance app");
    assert!(t.iter().filter(|x| *x == "fintech").count() >= 1);
    // light-mode 总在最前(tone 组最先)
    assert_eq!(t[0], "light-mode");
}

#[test]
fn wallet_app_for_gift_cards_is_apple_wallet_not_fintech() {
    // "gift cards" (plural) is apple-wallet context → must NOT push fintech.
    let t = infer_tags_from_prompt("a wallet app for gift cards");
    assert!(!t.contains(&"fintech".to_string()));
}

#[test]
fn rank_ranks_full_registry_no_filter() {
    let ranked = rank_style_guides_for_prompt(&["fintech".to_string()], Platform::Webapp);
    // 排名不过滤 —— 全 catalog 都在
    assert_eq!(ranked.len(), style_guide_registry().len());
}

#[test]
fn rank_industry_tag_outweighs_plain_tag() {
    // fintech(industry,+30)的 guide 应排在只命中普通 tag(+10)的前面
    let ranked = rank_style_guides_for_prompt(
        &["fintech".to_string(), "minimal".to_string()],
        Platform::Webapp,
    );
    assert!(!ranked.is_empty());
    // 首个的分数 >= 其后任意(降序)
    let s0 = style_guide_prompt_score(
        ranked[0],
        &["fintech".into(), "minimal".into()],
        Platform::Webapp,
    );
    let s1 = style_guide_prompt_score(
        ranked[ranked.len() - 1],
        &["fintech".into(), "minimal".into()],
        Platform::Webapp,
    );
    assert!(s0 >= s1);
}

#[test]
fn metadata_line_is_name_platform_and_lead_aesthetic_no_type_or_color() {
    // Softened (2026-06-23): no `:: tags` (type) and no ` bg:` (color).
    // Re-opened (2026-09-06): the planner picks by mood, so the line carries
    // the guide's lead "Key aesthetics" label — a few words, no hex, no tags.
    let g = style_guide_registry()
        .iter()
        .find(|guide| guide.platform == op_ai_skills::style_guide::Platform::Mobile)
        .expect("a mobile style guide is embedded");
    let line = format_guide_metadata_line(g, PlanningMode::Rich);
    let head = format!("- {} [{}]", g.name, g.platform.as_str());
    assert!(line.starts_with(&head), "{line}");
    let recipes = op_ai_skills::style_guide::signature_recipes(&g.content, 2);
    assert_eq!(recipes.len(), 2, "{line}");
    assert!(line.ends_with(&format!(" · recipes: {} / {}", recipes[0], recipes[1])));
    match op_ai_skills::style_guide::key_aesthetics(&g.content, 1).first() {
        Some(lead) => {
            let label = lead.split(':').next().unwrap_or("").trim();
            assert_eq!(
                line,
                format!(
                    "{head} — {label} · recipes: {} / {}",
                    recipes[0], recipes[1]
                )
            );
        }
        None => assert_eq!(
            line,
            format!("{head} · recipes: {} / {}", recipes[0], recipes[1])
        ),
    }
    assert!(!line.contains(" :: "), "type tags must be dropped: {line}");
    assert!(!line.contains(" bg:"), "bg color must be dropped: {line}");
    assert!(!line.contains('#'), "no hex in the candidate line: {line}");
}

#[test]
fn snippet_drops_color_type_radius_keeps_fonts() {
    // Softened: the snippet suggests font direction only — no colors,
    // no type tags, no radius.
    let g = &style_guide_registry()[0];
    let snip = format_guide_snippet(g);
    assert!(snip.starts_with(&format!("### {} [", g.name)));
    assert!(
        !snip.contains("tags:"),
        "type tags must be dropped:\n{snip}"
    );
    assert!(!snip.contains("colors:"), "colors must be dropped:\n{snip}");
    assert!(!snip.contains("radius:"), "radius must be dropped:\n{snip}");
}

// ─── Imported guides ───────────────────────────────────────────────────
//
// The registry these reach is process-global, so they take a lock and start
// from an empty imported set — see `op_ai_skills::style_guide::user_registry`.

fn exclusive_import_registry() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    op_ai_skills::style_guide::set_user_style_guides(Vec::new());
    guard
}

const IMPORTED_DESIGN_MD: &str = "---\nname: Studio Ochre\n---\n\n\
     # Studio Ochre\n\nWarm ochre surfaces, generous leading, no shadows.\n";

/// The whole point of the Asset Center's import path: pinning a `DESIGN.md`
/// the user brought has to steer planning exactly the way a corpus pin does.
#[test]
fn a_pinned_import_shrinks_the_menu_to_itself() {
    let _guard = exclusive_import_registry();
    let imported = op_ai_skills::style_guide::import_design_md(IMPORTED_DESIGN_MD, "ochre.md")
        .expect("imports");

    let ctx = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        Some(&imported.id),
    );

    assert_eq!(ctx.top_guide_names, vec![imported.id.clone()]);
    assert_eq!(ctx.metadata_count, 1);
    assert!(
        ctx.available_style_guides
            .contains(&format!("\"styleGuideName\": \"{}\"", imported.id)),
        "the import must be named by id, not by display name:\n{}",
        ctx.available_style_guides
    );
    // A corpus guide's snippet is font direction only; an import has no
    // corpus-shaped sections to extract, so its prose is what carries it.
    assert!(
        ctx.available_style_guides.contains("Warm ochre surfaces"),
        "an imported guide's own prose must reach the planner:\n{}",
        ctx.available_style_guides
    );
}

#[test]
fn a_long_import_is_truncated_rather_than_dropped() {
    let _guard = exclusive_import_registry();
    let long = format!(
        "---\nname: Verbose\n---\n\n# Verbose\n\n{}\nTAIL-MARKER\n",
        "Every surface is described at length. ".repeat(300)
    );
    let imported = op_ai_skills::style_guide::import_design_md(&long, "v.md").expect("imports");

    let ctx = build_planning_style_guide_context(
        "a dashboard",
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        Some(&imported.id),
    );
    assert!(ctx.available_style_guides.contains("truncated"));
    assert!(
        !ctx.available_style_guides.contains("TAIL-MARKER"),
        "the planning slot is capped, so the tail must be the part that goes"
    );
}

/// A pin whose import was deleted behaves like any other stale pin.
#[test]
fn a_deleted_import_falls_back_to_the_ranking() {
    let _guard = exclusive_import_registry();
    let auto = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        None,
    );
    let stale = build_planning_style_guide_context(
        FOOD_PROMPT,
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        Some("user:deleted-yesterday"),
    );
    assert_eq!(stale.available_style_guides, auto.available_style_guides);
}

/// Planning names the guide; this is the call that hands the sub-agent the
/// markdown to actually design against. It resolving only the corpus was the
/// break this whole path exists to close.
#[test]
fn a_sub_agent_receives_an_imported_guides_markdown() {
    let _guard = exclusive_import_registry();
    let imported = op_ai_skills::style_guide::import_design_md(IMPORTED_DESIGN_MD, "ochre.md")
        .expect("imports");

    let instruction = crate::prompt::build_style_guide_instruction(
        Some(&imported.id),
        crate::model_profile::ModelTier::Full,
    )
    .expect("an imported guide resolves");
    assert!(instruction.contains("Warm ochre surfaces"));

    assert!(
        crate::prompt::build_style_guide_instruction(
            Some("user:never-imported"),
            crate::model_profile::ModelTier::Full,
        )
        .is_none(),
        "an unknown id must not resolve to some other guide"
    );
}

/// The summary model tiers send a palette list *instead of* the document, so
/// an unextractable guide used to produce "use these EXACT hex colors"
/// followed by nothing — an instruction to obey an empty list.
#[test]
fn a_summary_tier_never_demands_obedience_to_an_empty_palette() {
    let _guard = exclusive_import_registry();
    // A shape none of the field extractors can classify: colours present,
    // roles written in prose the corpus grammar does not know.
    let imported = op_ai_skills::style_guide::import_design_md(
        "# Odd One\n\nThe wash is #101014 and the spark is #6B62F2, used sparingly.\n",
        "odd.md",
    )
    .expect("imports");

    let summary = crate::prompt::build_style_guide_instruction(
        Some(&imported.id),
        crate::model_profile::ModelTier::Basic,
    )
    .expect("resolves");

    assert!(
        summary.contains("#101014") && summary.contains("#6B62F2"),
        "the guide's own colours must reach a summary-tier prompt:\n{summary}"
    );
    let demands_obedience = summary.contains("EXACT hex colors");
    assert!(
        !demands_obedience || summary.contains('#'),
        "an EXACT-colors instruction must be followed by colors:\n{summary}"
    );
}

/// …and a guide with no colours anywhere must not carry the instruction at
/// all, rather than carry it over nothing.
#[test]
fn a_colourless_guide_drops_the_exact_colors_instruction() {
    let _guard = exclusive_import_registry();
    let imported = op_ai_skills::style_guide::import_design_md(
        "# Type Only\n\nGenerous leading, no ornament, one family throughout.\n",
        "type.md",
    )
    .expect("imports");

    let summary = crate::prompt::build_style_guide_instruction(
        Some(&imported.id),
        crate::model_profile::ModelTier::Basic,
    )
    .expect("resolves");
    assert!(
        !summary.contains("EXACT hex colors"),
        "nothing to obey, so nothing should be demanded:\n{summary}"
    );
}

#[test]
fn compact_style_summary_carries_the_guides_key_aesthetics() {
    // Weak tiers used to receive palette, fonts and radii only — the guide's
    // expression layer never reached them, so every guide produced the same
    // neutral template.
    for tier in [
        crate::model_profile::ModelTier::Basic,
        crate::model_profile::ModelTier::Standard,
    ] {
        let text = crate::prompt::build_style_guide_instruction(Some("dark-bold-mobile"), tier)
            .expect("shipped guide resolves");
        assert!(text.contains("Key aesthetics"), "{text}");
        assert!(text.contains("Electric lime"), "{text}");
        assert!(text.contains("Signature recipes"), "{text}");
        assert!(text.contains("Mesh gradient as protagonist"), "{text}");
    }
}

#[test]
fn pinned_mobile_metadata_carries_both_recipe_names() {
    let ctx = build_planning_style_guide_context(
        "a mobile wellness screen",
        Some("claude-opus"),
        PlanningMode::Rich,
        None,
        Some("wellness-green-mobile-light"),
    );
    assert!(
        ctx.available_style_guides.contains(
            "- wellness-green-mobile-light [mobile] — Nature-tinted canvas · recipes: \
             Top-leading protagonist / Dim-and-highlight \"now\""
        ),
        "pinned mobile guide metadata must carry recipes:\n{}",
        ctx.available_style_guides
    );
    assert!(
        ctx.available_style_guides
            .contains("Top-leading protagonist")
            && ctx
                .available_style_guides
                .contains("Dim-and-highlight \"now\""),
        "pinned mobile guide metadata must carry both recipe names:\n{}",
        ctx.available_style_guides
    );
}
