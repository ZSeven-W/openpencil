//! Budget guards for planning skills whose content is AUGMENTED at runtime.
//!
//! `op-ai-skills` has `no_skill_silently_exceeds_its_own_budget`, but that
//! test measures the file on disk — and `style-guide-selector.md` carries a
//! `{{availableStyleGuides}}` placeholder that the orchestrator substitutes
//! with the whole style-guide catalog before the trimmer ever sees it. The
//! static corpus was 210 tokens against a 500 budget; the injected catalog
//! took the real prompt to 819 tokens, and Step 1 of `trim_by_budget_pinned`
//! silently chopped 1287 chars off the tail — the selection rules that tell
//! the model what to DO with the catalog — on every planning call
//! (2026-07-28 production log).
//!
//! The corpus test cannot catch this: `op-ai-skills` is below the
//! orchestrator in the dependency graph and cannot see the catalog. So the
//! guard has to live here, on the side that owns the augmentation.

use std::collections::HashMap;

use jian_ops_schema::{DesignMdColor, DesignMdSpec, DesignMdTypography};
use op_ai_skills::budget::estimate_tokens;
use op_ai_skills::resolver::inject_dynamic_content;

use super::*;
use crate::plan::{Region, RootFrameSpec};

/// Every placeholder the orchestrator substitutes into a PLANNING skill.
/// Adding a runtime-augmented planning key without adding it here is what
/// this guard exists to make impossible — a new key with no worst-case entry
/// leaves the same blind spot `{{availableStyleGuides}}` sat in.
const AUGMENTED_PLANNING_KEYS: [&str; 1] = ["availableStyleGuides"];

/// A design.md spec filled to the size limits `build_design_md_style_policy`
/// enforces (200/300/400-char truncations, 10 palette rows, 6 surface rows),
/// so the design.md branch is measured at ITS worst case too — that branch
/// interpolates USER content, and a fixed budget has to cover it.
fn saturated_design_md() -> DesignMdSpec {
    let color = |i: usize| DesignMdColor {
        name: format!("Palette Color Number {i}"),
        hex: format!("#0000{i:02}"),
        role: format!("surface role {i} — card, panel and sidebar backgrounds"),
    };
    DesignMdSpec {
        raw: String::new(),
        project_name: Some("A Rather Long Project Name For Measurement".into()),
        visual_theme: Some("v".repeat(400)),
        color_palette: Some((0..20).map(color).collect()),
        typography: Some(DesignMdTypography {
            font_family: Some("f".repeat(120)),
            headings: Some("h".repeat(120)),
            body: Some("b".repeat(120)),
            scale: Some("s".repeat(400)),
        }),
        component_styles: Some("c".repeat(600)),
        layout_principles: Some("l".repeat(800)),
        generation_notes: Some("n".repeat(800)),
    }
}

/// The worst-case value of `{{availableStyleGuides}}` over every branch that
/// can produce one: both catalog planning modes crossed with every model tier
/// (the tier sets the snippet count), plus the design.md branch.
fn worst_case_style_guide_context() -> (String, String) {
    // One model id per tier — `snippet_limit` is the only tier-sensitive
    // input, and it is what makes the catalog branch grow.
    let models = ["claude-opus", "glm-4", "minimax-m3", ""];
    let prompts = [
        "a fintech dashboard",
        "a dark minimalist mobile music app landing page",
        "xyz123",
    ];
    let design_md = saturated_design_md();
    let mut worst = (String::new(), String::new());
    for mode in [PlanningMode::Rich, PlanningMode::Minimal] {
        for model in models {
            for prompt in prompts {
                for spec in [None, Some(&design_md)] {
                    let ctx =
                        build_planning_style_guide_context(prompt, Some(model), mode, spec, None);
                    if ctx.available_style_guides.chars().count() > worst.0.chars().count() {
                        let label = format!(
                            "mode={mode:?} model={model:?} design_md={} prompt={prompt:?}",
                            spec.is_some()
                        );
                        worst = (ctx.available_style_guides, label);
                    }
                }
            }
        }
    }
    worst
}

#[test]
fn runtime_augmented_planning_skills_fit_their_own_budget() {
    let (context, label) = worst_case_style_guide_context();
    let dynamic = HashMap::from([("availableStyleGuides".to_string(), context.clone())]);

    let mut offenders = Vec::new();
    for skill in op_ai_skills::get_skills_by_phase(op_ai_skills::Phase::Planning) {
        let augmented = inject_dynamic_content(&skill.content, &dynamic);
        if augmented == skill.content {
            continue; // no placeholder — the corpus test already covers it
        }
        let actual = estimate_tokens(&augmented);
        if actual > skill.meta.budget {
            offenders.push(format!(
                "{} (budget={}, augmented={actual}, over by {}) — worst case {label}",
                skill.meta.name,
                skill.meta.budget,
                actual - skill.meta.budget
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "planning skills are truncated AFTER runtime augmentation, silently dropping \
         their tail from every planning prompt: {offenders:?}"
    );
}

#[test]
fn every_augmented_placeholder_has_a_worst_case_in_this_guard() {
    // The guard is only as good as its key list. Any `{{placeholder}}` a
    // planning skill declares must be one this file measures — otherwise a
    // new augmented key reopens exactly the hole this file closes.
    for skill in op_ai_skills::get_skills_by_phase(op_ai_skills::Phase::Planning) {
        for (index, _) in skill.content.match_indices("{{") {
            let rest = &skill.content[index + 2..];
            let Some(end) = rest.find("}}") else { continue };
            let key = &rest[..end];
            assert!(
                AUGMENTED_PLANNING_KEYS.contains(&key),
                "planning skill {:?} interpolates {{{{{key}}}}}, which this budget guard \
                 does not measure — add its worst case to AUGMENTED_PLANNING_KEYS",
                skill.meta.name
            );
        }
    }
}

/// `decomposition` is the skill that decides how many slides a deck gets, and
/// the budget trimmer cuts from the END — so a skill that overruns loses its
/// tail silently while still looking present in the prompt. Asserting the
/// LAST line of the corpus file reaches the assembled system prompt is what
/// proves the whole skill arrived, and the deck rules in its middle with it.
#[test]
fn decomposition_reaches_the_planning_prompt_with_its_tail_intact() {
    let request = DesignRequest {
        prompt: "帮我做一个 12 页的产品培训课件 PPT".into(),
        ..req()
    };
    let pp = build_orchestrator_prompt(&request, PlanningMode::Rich, AbortFlag::new());
    let system_prompt = &pp.call_request.system_prompt;

    let body = &op_ai_skills::get_skill_by_name("decomposition")
        .expect("decomposition registered")
        .content;
    let last_line = body
        .trim_end()
        .lines()
        .next_back()
        .expect("decomposition is not empty");
    assert!(
        system_prompt.contains(last_line),
        "decomposition's last line {last_line:?} never reached the planning prompt — \
         the skill is over its {} budget (measured {}) and its tail was trimmed",
        op_ai_skills::get_skill_by_name("decomposition")
            .expect("registered")
            .meta
            .budget,
        estimate_tokens(body)
    );

    // The de-anchored slide-count teaching sits mid-file; assert it directly
    // so a future edit can't drop it while keeping the tail.
    for rule in ["SLIDE COUNT", "HARD constraint"] {
        assert!(
            system_prompt.contains(rule),
            "planning prompt lost the slide-count rule {rule:?}"
        );
    }
}

#[test]
fn no_planning_skill_is_dropped_or_truncated_by_the_phase_budget() {
    // End-to-end through the real resolver: the per-skill cap AND the phase
    // total. The landing-page prompt is deliberate — `landing-page-predesign`
    // is the phase's only Domain skill, so it is the one that gets squeezed
    // out when the base skills eat the whole total.
    let (context, label) = worst_case_style_guide_context();
    let opts = op_ai_skills::ResolveOptions {
        dynamic_content: HashMap::from([("availableStyleGuides".to_string(), context)]),
        ..Default::default()
    };
    let prompt = "a marketing landing page for a fintech product";
    let ctx = op_ai_skills::resolve_skills(op_ai_skills::Phase::Planning, prompt, &opts);

    let truncated: Vec<&str> = ctx
        .report
        .included
        .iter()
        .filter(|entry| entry.truncated)
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(
        truncated.is_empty(),
        "truncated planning skills {truncated:?} (worst case {label}); \
         used {}/{} tokens",
        ctx.report.budget_used,
        ctx.report.budget_max
    );

    let starved: Vec<&str> = ctx
        .report
        .dropped
        .iter()
        .filter(|entry| entry.reason == op_ai_skills::DropReason::BudgetExhausted)
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(
        starved.is_empty(),
        "planning skills dropped for budget {starved:?}; used {}/{} tokens — the phase \
         total must cover the base skills (which are budget-exempt and therefore always \
         counted) plus every intent-matched Domain skill",
        ctx.report.budget_used,
        ctx.report.budget_max
    );
}

// ── Generation-phase hard-rule delivery guards (2026-08-14 DS corpus pass) ──
//
// Three corpus rules landed on 2026-08-14: SIBLING ISOMORPHISM (layout.md),
// MARGIN FLOOR (slides.md / deck-contract.md, plus one generic sentence in
// layout.md), and NODE NAMING + IMAGE SLOT CONTRACT (schema.md). These guards
// exist because the 0814 run proved the "corpus was edited, model never saw
// it" failure mode is alive on the generation path: a Basic non-mobile
// subtask resolved schema/layout/text-rules/overflow/icon-catalog against
// budget 5200 with `cjk-typography` squeezed into a 69-token truncated tail.
// A rule sitting in a file on disk says nothing about what reached the model,
// so each guard below reads the FINAL assembled subtask system_prompt on the
// 0814 tier/intent shapes and asserts the rule's marker phrase verbatim.

/// Rule 1 — same-list siblings are structurally isomorphic. `layout` is a
/// Base skill (always kept regardless of budget), so the failure this guard
/// exists for is its own per-skill cap silently cutting the tail —
/// `no_skill_silently_exceeds_its_own_budget` catches that at the corpus
/// level, this one catches it in the assembled prompt.
#[test]
fn basic_tier_subtask_prompt_carries_the_sibling_isomorphism_rule() {
    let (call, report) = bsp(
        &subtask(),
        &plan(),
        &DesignRequest {
            model: Some("glm-4.6".into()), // Basic arm — the 0814 budget fixture tier
            ..req()
        },
        AbortFlag::new(),
        false,
        false,
    );
    assert_eq!(
        report.budget_max, 5200,
        "fixture must reproduce the 0814 Basic non-mobile budget arm"
    );
    assert!(
        call.system_prompt.contains("SIBLING ISOMORPHISM"),
        "the layout sibling-isomorphism rule never reached the subtask prompt — \
         `layout` was dropped or truncated ({}/{} tokens; included {:?})",
        report.budget_used,
        report.budget_max,
        report
            .included
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
    );
}

/// Rule 2 — the margin floor. The two deck carriers are verified with deck
/// INTENT so the deck budget arm is what loads them, and each carrier has its
/// own marker so one file delivering while the other was dropped still fails.
#[test]
fn deck_intent_prompt_carries_the_margin_floor_rule() {
    let (call, report) = bsp(
        &deck_subtask(),
        &deck_plan(),
        &deck_request("glm-4.6"),
        AbortFlag::new(),
        false,
        false,
    );
    assert_eq!(
        report.budget_max,
        op_ai_skills::Phase::Generation.default_budget(),
        "deck fixture must exercise the deck budget arm, got {}",
        report.budget_max
    );
    for marker in [
        "MARGIN FLOOR",
        "1080-wide card roots ≥48px",  // slides.md carrier
        "the safe margin Law 3 locks", // deck-contract.md carrier
    ] {
        assert!(
            call.system_prompt.contains(marker),
            "deck prompt is missing {marker:?} — the margin-floor rule was dropped \
             or truncated ({}/{} tokens; included {:?})",
            report.budget_used,
            report.budget_max,
            report
                .included
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
        );
    }
}

/// Rule 2's generic one-liner lives in `layout` (a Base skill), so it must
/// reach every subtask prompt — verified on the plain 0814 Basic 5200 path,
/// where `slides` / `deck-contract` are keyword-gated OFF and `layout` is the
/// only skill that can carry the phrase.
#[test]
fn basic_tier_subtask_prompt_carries_the_layout_margin_floor_sentence() {
    let (call, report) = bsp(
        &subtask(),
        &plan(),
        &DesignRequest {
            model: Some("glm-4.6".into()),
            ..req()
        },
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        call.system_prompt.contains("MARGIN FLOOR"),
        "the generic margin-floor sentence in `layout` never reached the plain \
         subtask prompt ({}/{} tokens)",
        report.budget_used,
        report.budget_max,
    );
}

/// Rule 3 — every node named, image slots are `image` nodes. `schema` is a
/// Base skill on the 0814 Basic 5200 path; assert both contract markers reach
/// the assembled prompt.
#[test]
fn basic_tier_subtask_prompt_carries_the_node_name_and_image_slot_contract() {
    let (call, report) = bsp(
        &subtask(),
        &plan(),
        &DesignRequest {
            model: Some("glm-4.6".into()),
            ..req()
        },
        AbortFlag::new(),
        false,
        false,
    );
    for marker in ["NODE NAMING", "IMAGE SLOT CONTRACT"] {
        assert!(
            call.system_prompt.contains(marker),
            "the schema {marker:?} rule never reached the subtask prompt — \
             `schema` was dropped or truncated ({}/{} tokens; included {:?})",
            report.budget_used,
            report.budget_max,
            report
                .included
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
        );
    }
}

// ── DS P1.5: card domain corpus delivery guards ─────────────────────────────
//
// The card corpus (`domains/cards.md`) is the FIRST domain skill a card
// board has ever had — the 0815 fixture proved "the file does not exist" and
// "the file exists but the model never saw it" are the same failure on the
// generation path. The two guards below assert the FINAL assembled subtask
// system_prompt: the card rules reach a card intent, and do NOT reach a
// non-card intent — keyword routing is a gate, not a resident.

/// A card request ("知识卡片") resolves `cards` and its three hard rules
/// reach the model verbatim. Deliberately the Basic tier: the card budget
/// arm (the same Generation default the deck arm reads) is what makes the
/// rules survivable there — the plain 5200 arm's always-kept Base skills
/// alone resolve ~5440 tokens.
#[test]
fn card_intent_prompt_carries_the_card_contract_rules() {
    let (call, report) = bsp(
        &card_subtask(),
        &card_plan(),
        &card_request("glm-4.6"),
        AbortFlag::new(),
        false,
        false,
    );
    assert_eq!(
        report.budget_max,
        op_ai_skills::Phase::Generation.default_budget(),
        "card fixture must exercise the card budget arm, got {}",
        report.budget_max
    );
    for marker in [
        "MARGIN OWNERSHIP",
        "ITEM TEMPLATE",
        "ORNAMENT DISCIPLINE",
        "VERTICAL RHYTHM", // DS P2-b D: the card vertical-rhythm rule
    ] {
        assert!(
            call.system_prompt.contains(marker),
            "card prompt is missing {marker:?} — `cards` was dropped or truncated \
             ({}/{} tokens; included {:?})",
            report.budget_used,
            report.budget_max,
            report
                .included
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
        );
    }
}

/// The card budget arm keys off the PORTRAIT artboard, never the wording —
/// same single-classifier routing as the deck arm.
#[test]
fn the_card_budget_arm_keys_off_the_portrait_artboard() {
    assert!(is_card_board(&card_plan()));
    // A square (1080x1080) is not a card board and keeps the page budget.
    let mut square = card_plan();
    square.root_frame.height = 1080.0;
    assert!(!is_card_board(&square));
    // A phone screen is its own contract, not a card.
    let mut mobile = card_plan();
    mobile.root_frame.width = 390.0;
    mobile.root_frame.height = 844.0;
    assert!(!is_card_board(&mobile));
}

/// The gate direction that matters: a NON-card intent (a deck subtask whose
/// label/screen are not covers) must not carry the card rules — they are
/// keyword-gated, not always-on budget residents.
#[test]
fn a_non_card_intent_prompt_omits_the_card_contract_rules() {
    let mut deck_subtask = deck_subtask();
    deck_subtask.label = "正文页".into();
    deck_subtask.screen = Some("正文页".into());
    let (call, _report) = bsp(
        &deck_subtask,
        &deck_plan(),
        &deck_request("glm-4.6"),
        AbortFlag::new(),
        false,
        false,
    );
    for marker in ["MARGIN OWNERSHIP", "ITEM TEMPLATE"] {
        assert!(
            !call.system_prompt.contains(marker),
            "deck prompt must not carry the card rule {marker:?} — the keyword \
             gate is not routing"
        );
    }
}

/// Card fixtures: a 1080x1440 portrait board plan plus a "法则" subtask, and
/// the card-series request the routing test needs (知识卡片 is a keyword the
/// card domain skill must fire on).
fn card_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "card".into(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn card_subtask() -> crate::plan::Subtask {
    crate::plan::Subtask {
        id: "rule-01".into(),
        label: "法则 01".into(),
        region: crate::plan::Region {
            width: 1080.0,
            height: 600.0,
        },
        id_prefix: "rule-01".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn card_request(model: &str) -> DesignRequest {
    DesignRequest {
        prompt: "帮我做一套知识卡片：如何早起".into(),
        model: Some(model.into()),
        ..req()
    }
}

/// Deck fixtures (mirror `prompt_deck_skill_tests`) — the fixed 1920×1080
/// projector artboard that routes the deck budget arm and loads the deck
/// keyword-gated skills.
fn deck_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "deck".into(),
            name: "Deck".into(),
            width: 1920.0,
            height: 1080.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn deck_subtask() -> crate::plan::Subtask {
    crate::plan::Subtask {
        id: "cover".into(),
        label: "封面".into(),
        region: Region {
            width: 1920.0,
            height: 1080.0,
        },
        id_prefix: "cover".into(),
        parent_frame_id: None,
        elements: None,
        screen: Some("封面".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn deck_request(model: &str) -> DesignRequest {
    DesignRequest {
        prompt: "帮我做一个 8 页的融资路演 PPT，深色科技感".into(),
        model: Some(model.into()),
        ..req()
    }
}
