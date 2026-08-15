//! DS P2-a overlay gate guards — `skills/overlays/deepseek/` is the DS
//! experiment field, and the `model_families` gate is what keeps its teaching
//! from leaking to other model families.
//!
//! Strategic line: output contracts belong in the public corpus, model
//! behaviour adaptation belongs in the DS experiment field. These guards
//! assert the FINAL assembled subtask system_prompt: the overlay's worked
//! example reaches a card intent ONLY under a deepseek-family model id, and
//! never under glm / an unknown model / a non-card intent.

use op_ai_skills::DropReason;

use super::*;
use crate::plan::Region;

/// The overlay skill's canonical opener — the marker phrase the e2e asserts
/// must ride into (deepseek) or stay out of (everyone else) the prompt.
const WORKED_EXAMPLE_MARKER: &str = "CARD ITEM TEMPLATE — WORKED EXAMPLE";

/// A 1080x1440 portrait card-board plan (routes the card budget arm) with
/// one "法则" subtask.
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
        region: Region {
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

/// A deck subtask for the non-card-intent negative arm.
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

fn deck_request(model: &str) -> DesignRequest {
    DesignRequest {
        prompt: "帮我做一个 8 页的融资路演 PPT，深色科技感".into(),
        model: Some(model.into()),
        ..req()
    }
}

/// Card intent + deepseek-family model → the overlay resolves in and its
/// worked example reaches the subtask prompt.
#[test]
fn card_intent_plus_deepseek_model_carries_the_overlay_worked_example() {
    let (call, report) = bsp(
        &card_subtask(),
        &card_plan(),
        &card_request("deepseek-v4-pro"),
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        call.system_prompt.contains(WORKED_EXAMPLE_MARKER),
        "the deepseek overlay worked example never reached the card subtask \
         prompt ({}/{} tokens; included {:?})",
        report.budget_used,
        report.budget_max,
        report
            .included
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
    );
    assert!(
        report
            .included
            .iter()
            .any(|e| e.name == "card-item-template"),
        "the overlay skill must be in the included set for a deepseek model"
    );
    // The overlay rides WITH the public card contract, not instead of it.
    assert!(call.system_prompt.contains("MARGIN OWNERSHIP"));
}

/// The hard wording lines travel with the worked example — they are the
/// model-behaviour adaptation, not corpus fluff.
#[test]
fn deepseek_card_prompt_carries_the_two_hard_wordings() {
    let (call, _report) = bsp(
        &card_subtask(),
        &card_plan(),
        &card_request("deepseek-v4-pro"),
        AbortFlag::new(),
        false,
        false,
    );
    for marker in [
        "NEVER invent a fresh structure, child order, or ornament per item",
        "repeats VERBATIM on every item",
    ] {
        assert!(
            call.system_prompt.contains(marker),
            "deepseek card prompt lost the hard wording {marker:?}"
        );
    }
}

/// Same card intent + a glm model → the overlay stays out; the drop is
/// recorded as a family miss, not an intent miss.
#[test]
fn card_intent_plus_glm_model_omits_the_overlay() {
    let (call, report) = bsp(
        &card_subtask(),
        &card_plan(),
        &card_request("glm-5.2"),
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        !call.system_prompt.contains(WORKED_EXAMPLE_MARKER),
        "the deepseek overlay must not reach a glm model"
    );
    let drop = report
        .dropped
        .iter()
        .find(|d| d.name == "card-item-template")
        .unwrap_or_else(|| {
            panic!(
                "card-item-template must be dropped for glm: {:?}",
                report.dropped
            )
        });
    assert_eq!(drop.reason, DropReason::ModelFamilyMiss);
    // The public card contract still loads — the gate removes the overlay only.
    assert!(call.system_prompt.contains("MARGIN OWNERSHIP"));
}

/// Default "" (no model id known) never admits an overlay — the
/// zero-regression arm of the gate.
#[test]
fn card_intent_without_a_model_omits_the_overlay() {
    let req = DesignRequest {
        model: None,
        ..card_request("ignored")
    };
    let (call, _report) = bsp(
        &card_subtask(),
        &card_plan(),
        &req,
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        !call.system_prompt.contains(WORKED_EXAMPLE_MARKER),
        "an unknown model id must never admit a family-gated overlay"
    );
}

/// A `provider/`-prefixed id normalizes down to the bare family name — the
/// same stripping `resolve_model_profile` applies.
#[test]
fn provider_prefixed_deepseek_id_still_admits_the_overlay() {
    let (call, _report) = bsp(
        &card_subtask(),
        &card_plan(),
        &card_request("anthropic/deepseek-v4-pro"),
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        call.system_prompt.contains(WORKED_EXAMPLE_MARKER),
        "a provider-prefixed deepseek id must still admit the overlay"
    );
}

/// Keyword routing stays in force for gated skills: a deepseek model on a
/// NON-card intent gets no overlay teaching.
#[test]
fn non_card_intent_plus_deepseek_omits_the_overlay() {
    let mut subtask = card_subtask();
    subtask.label = "正文页".into();
    subtask.screen = Some("正文页".into());
    let (call, _report) = bsp(
        &subtask,
        &deck_plan(),
        &deck_request("deepseek-v4-pro"),
        AbortFlag::new(),
        false,
        false,
    );
    assert!(
        !call.system_prompt.contains(WORKED_EXAMPLE_MARKER),
        "a deepseek deck prompt must not carry the card overlay — the \
         keyword gate is not bypassed by the family gate"
    );
}
