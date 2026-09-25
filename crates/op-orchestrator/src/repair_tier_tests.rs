//! Tests for [`super`] — the tier classification and the policy it drives.

use super::*;

fn template_state() -> EditorState {
    let mut state = EditorState::starter();
    state.editor_ui.scene_template_center.generate_basis = Some("minimal-keynote".to_string());
    state
}

#[test]
fn contract_passes_run_for_authored_input_too() {
    let policy = RepairTierPolicy::for_document(&template_state());
    for pass in [
        TieredPass::GeometryValidation,
        TieredPass::TextCollision,
        TieredPass::TextContrast,
        TieredPass::HorizontalOverflow,
    ] {
        assert_eq!(pass.tier(), RepairTier::Contract, "{pass:?}");
        assert!(
            policy.runs_pass(pass),
            "{pass:?} proves a defect — it must run at any provenance"
        );
    }
}

#[test]
fn intent_passes_defer_to_authored_input() {
    let policy = RepairTierPolicy::for_document(&template_state());
    for pass in [
        TieredPass::ThemeVariablePolarity,
        TieredPass::WrapperDoubleInset,
        TieredPass::StructuralWrapperTransparency,
        TieredPass::SurfaceColorDiscipline,
        TieredPass::VariableBinding,
        TieredPass::TreeHeuristics,
    ] {
        assert_eq!(pass.tier(), RepairTier::Intent, "{pass:?}");
        assert!(!policy.runs_pass(pass), "{pass:?}");
    }
}

#[test]
fn ordinary_generated_output_runs_both_tiers() {
    let policy = RepairTierPolicy::for_document(&EditorState::starter());
    assert!(policy.runs(RepairTier::Contract));
    assert!(policy.runs(RepairTier::Intent));
    assert!(policy.runs_pass(TieredPass::WrapperDoubleInset));
    assert_eq!(policy.deferring_to(), None);
    assert_eq!(policy.intent_skip_note(), None);
}

#[test]
fn the_skip_names_the_template_it_is_deferring_to() {
    let policy = RepairTierPolicy::for_document(&template_state());
    let note = policy.intent_skip_note().expect("a skip is recorded");
    assert!(note.contains("intent-tier passes skipped"), "{note}");
    assert!(note.contains("minimal-keynote"), "{note}");
    // The user must be able to tell that the other half still ran.
    assert!(note.contains("contract-tier"), "{note}");
}

#[test]
fn the_ledger_carries_the_decision_but_counts_no_repair() {
    let mut summary = RepairSummary::default();
    let policy = RepairTierPolicy::for_document(&template_state());

    note_intent_tier_skip(&mut summary, &policy);

    assert_eq!(summary.notes().len(), 1);
    assert!(summary.notes()[0].contains("minimal-keynote"));
    // A note is not an edit: the credential must not claim a repair for it.
    assert_eq!(summary.total_repairs(), 0);
}

#[test]
fn a_run_that_skipped_nothing_leaves_no_note() {
    let mut summary = RepairSummary::default();
    note_intent_tier_skip(&mut summary, &RepairTierPolicy::all());
    assert!(summary.notes().is_empty());
}

#[test]
fn the_decision_is_stated_once_however_many_roots_reach_the_gate() {
    let mut summary = RepairSummary::default();
    let policy = RepairTierPolicy::for_document(&template_state());
    for _ in 0..4 {
        note_intent_tier_skip(&mut summary, &policy);
    }
    assert_eq!(summary.notes().len(), 1);
}

#[test]
fn all_runs_everything() {
    let policy = RepairTierPolicy::all();
    assert!(policy.runs(RepairTier::Intent));
    assert!(policy.runs(RepairTier::Contract));
}

#[test]
fn an_imported_website_defers_the_intent_tier() {
    let mut state = EditorState::starter();
    state.editor_ui.home.imported_from = Some("https://acme.example/pricing".to_string());
    let policy = RepairTierPolicy::for_document(&state);
    assert!(policy.runs(RepairTier::Contract));
    assert!(!policy.runs(RepairTier::Intent));
    let note = policy.intent_skip_note().expect("the skip is recorded");
    assert!(
        note.contains("acme.example/pricing via imported-site"),
        "{note}"
    );
}
