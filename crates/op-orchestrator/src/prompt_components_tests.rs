//! Available-components manifest tests (injection, capping, protocol
//! wording and budget interactions).

use super::*;

// ── Available-components manifest (Stage 2 Part B) ───────────────────────────

use jian_ops_schema::node::PenNode;
use op_editor_core::{Component, NodeId};

/// Build a `ComponentLibrary` with `n` reusable masters whose names cycle
/// through a few categories so the grouped manifest exercises bucketing.
/// The manifest only reads each component's `id` + `name`, so the `root`
/// frame is a minimal reusable-flagged stub.
fn library_with(n: usize) -> ComponentLibrary {
    let names = [
        "Primary Button",
        "Search Input",
        "Stat Card",
        "Nav Item",
        "Status Badge",
        "User Avatar",
        "Confirm Dialog",
        "Table Row",
        "Page Header",
    ];
    let mut lib = ComponentLibrary::default();
    for i in 0..n {
        let id = format!("comp-{i}");
        let name = format!("{} {i}", names[i % names.len()]);
        let root: PenNode = serde_json::from_value(serde_json::json!({
            "id": id,
            "type": "frame",
            "name": name,
            "reusable": true,
            "width": 100,
            "height": 40,
        }))
        .expect("frame fixture");
        lib.insert(Component {
            id: NodeId::new(&id),
            name,
            root,
        });
    }
    lib
}

/// With NO components, the prompt is unchanged: no AVAILABLE COMPONENTS block
/// and no `ref` teaching from the `component-composition` skill.
#[test]
fn no_components_prompt_omits_manifest_and_ref_teaching() {
    let (cr, report) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &req(),
        AbortFlag::new(),
        false,
        false,
        &ComponentLibrary::default(),
    );
    assert!(
        !cr.system_prompt.contains("AVAILABLE COMPONENTS"),
        "empty library must not inject the components manifest"
    );
    // The component-composition skill only loads behind `hasReusableComponents`.
    assert!(
        !report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition skill must not load without components"
    );
    // And the empty-library prompt must byte-match the no-arg path (the `bsp`
    // shim forwards an empty library too).
    let (baseline, _) = bsp(&subtask(), &plan(), &req(), AbortFlag::new(), false, false);
    assert_eq!(cr.system_prompt, baseline.system_prompt);
}

/// A Full-tier request (no budget override, no Basic allow-set) so the
/// flag-gated `component-composition` skill reliably survives filtering.
fn full_req() -> DesignRequest {
    DesignRequest {
        model: Some("claude-opus-4".into()),
        ..req()
    }
}

/// With components present, the prompt injects the AVAILABLE COMPONENTS
/// manifest (concrete ids), the `ref` teaching, and loads the
/// `component-composition` skill.
#[test]
fn components_prompt_injects_manifest_and_ref_teaching() {
    let lib = library_with(5);
    let (cr, report) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &full_req(),
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let sys = &cr.system_prompt;
    assert!(
        sys.contains("AVAILABLE COMPONENTS"),
        "manifest header must be present"
    );
    // Concrete ids from the registry are listed.
    assert!(sys.contains("comp-0"), "manifest must list component ids");
    assert!(sys.contains("comp-4"), "manifest must list all 5 ids");
    // The `ref` instantiation teaching is present.
    assert!(
        sys.contains("\"type\":\"ref\""),
        "manifest must teach the ref node syntax"
    );
    // Category grouping appears (button → Buttons bucket).
    assert!(sys.contains("Buttons:"), "manifest groups by category");
    // The component-composition skill loaded behind the flag.
    assert!(
        report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition skill must load when components exist"
    );
}

/// A large library is capped: the manifest lists at most
/// `MAX_COMPONENT_MANIFEST_ENTRIES` and notes the remainder, so the prompt
/// budget can't be blown by a 200-master kit.
#[test]
fn large_component_library_is_capped() {
    let lib = library_with(200);
    let (cr, _) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &req(),
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let sys = &cr.system_prompt;
    // The header reports the true total even though the body is capped.
    assert!(sys.contains("AVAILABLE COMPONENTS (200 reusable"));
    assert!(
        sys.contains("more not listed"),
        "capped manifest must note the remainder"
    );
    // The number of listed `- id (name)` rows must not exceed the cap.
    let listed = sys.matches("  - comp-").count();
    assert!(
        listed <= MAX_COMPONENT_MANIFEST_ENTRIES,
        "listed {listed} entries exceeds cap {MAX_COMPONENT_MANIFEST_ENTRIES}"
    );
}

/// Regression guard for the protocol-mismatch stop-gate: the AVAILABLE
/// COMPONENTS manifest's trailing ref-syntax example must match whichever
/// output protocol governs the REST of this prompt. The subagent path is now
/// script-gen on every retry rung. Teaching the wrong dialect makes every
/// `ref` silently vanish: a bare `{"_parent":...}` line is never recorded by
/// the script sandbox (only `I(...)` calls are), so under script-gen the
/// old always-flat instruction taught the model a no-op.
///
/// Uses `full_req()` (Full tier) for both calls so `reduced_complexity`'s
/// tier-gated SKILL narrowing stays a no-op (per
/// `subagent_prompt_reduced_complexity_full_tier_skill_filtering_is_noop`)
/// and the manifest + `component-composition` skill both survive on either
/// side — isolating the assertion to the protocol switch alone.
#[test]
fn components_manifest_instruction_matches_active_protocol() {
    let lib = library_with(3);

    // (a) Full attempt: script-gen is THE default protocol. The manifest
    // must teach the `I(...)` ref call and must NOT teach the flat `_parent`
    // ref line.
    let (script_cr, _) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &full_req(),
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let script_sys = &script_cr.system_prompt;
    assert!(
        script_sys.contains("I(<containerBinding>, {\"type\":\"ref\""),
        "full-attempt (script-gen) manifest must teach the I(...) ref call:\n{script_sys}"
    );
    assert!(
        !script_sys.contains("\"_parent\":\"<container-id>\""),
        "full-attempt (script-gen) manifest must NOT teach the flat _parent ref line \
         — a bare {{\"_parent\":...}} line is never recorded by the script sandbox:\n{script_sys}"
    );

    // (b) Reduced-complexity retry rung: still script-gen. The manifest must
    // teach the `I(...)` ref call and must not teach the `_parent` ref line.
    let (flat_cr, _) = build_subagent_prompt(
        &subtask(),
        &plan(),
        &full_req(),
        AbortFlag::new(),
        true,
        false,
        &lib,
    );
    let flat_sys = &flat_cr.system_prompt;
    assert!(
        flat_sys.contains("I(<containerBinding>, {\"type\":\"ref\""),
        "reduced-complexity manifest must keep the script-gen ref call:\n{flat_sys}"
    );
    assert!(
        !flat_sys.contains("\"_parent\":\"<container-id>\""),
        "reduced-complexity manifest must not teach the flat _parent ref line:\n{flat_sys}"
    );
}

/// Regression guard for the tier-drop bug (smoke `OPENPENCIL_SMOKE_LIBRARY`
/// scenario): a Basic-tier model with a component library loaded must get BOTH
/// the AVAILABLE COMPONENTS manifest (concrete ids) AND the
/// `component-composition` teaching (the `ref` + `descendants` syntax) in its
/// assembled subtask prompt.
///
/// Before the fix the `component-composition` skill resolved in behind the
/// `hasReusableComponents` flag but was then dropped by the Basic-tier
/// `ALLOWED` allow-set (DropReason::TierFiltered) even with budget room
/// (`budget_used < budget_max`) — so a weak model saw the component list with
/// no instruction on how to emit a `ref` node and built everything from
/// scratch (0 component instances). Reproduces the real smoke path: a Basic-tier
/// MiniMax (M2.x; M3 is Full since 2026-07-18) mobile screen, whose 9200-token budget has room to spare.
#[test]
fn basic_tier_components_prompt_keeps_both_manifest_and_teaching() {
    // Sanity: the model classifies as Basic, the path that drops non-allowed
    // skills via the allow-set (the bug surface).
    assert_eq!(
        resolve_model_profile("minimax-m2.7").tier,
        ModelTier::Basic,
        "test fixture must exercise the Basic tier"
    );

    // A Basic-tier mobile request — the actual smoke scenario. Mobile routes
    // through the wider 10400-token budget so the drop is provably tier-caused,
    // not budget-caused.
    let basic_req = DesignRequest {
        prompt: "Design a 402x874 mobile shop home screen with product cards, \
                 search, and bottom navigation using the available components"
            .into(),
        model: Some("minimax-m2.7".into()),
        ..req()
    };
    let mut mobile_plan = plan();
    mobile_plan.root_frame.width = 402.0;
    mobile_plan.root_frame.height = 874.0;
    let mobile_subtask = Subtask {
        id: "main-content".into(),
        label: "Main Content".into(),
        region: Region {
            width: 402.0,
            height: 640.0,
        },
        id_prefix: "main-content".into(),
        parent_frame_id: Some("page".into()),
        insert_after_sibling_id: None,
        elements: Some("product cards, search bar, category chips".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };

    let lib = library_with(5);
    let (cr, report) = build_subagent_prompt(
        &mobile_subtask,
        &mobile_plan,
        &basic_req,
        AbortFlag::new(),
        false,
        false,
        &lib,
    );
    let sys = &cr.system_prompt;

    // The drop the fix removes was budget-room-permitting: prove budget is
    // not exceeded (finalize status-bar enforcement adds to tree complexity).
    assert!(
        report.budget_used <= report.budget_max,
        "fixture must not exceed budget max (status-bar enforcement adds tree complexity); report={report:?}"
    );

    // (1) The AVAILABLE COMPONENTS manifest reached the system prompt with
    // concrete ids — it is a plain appended block, never tier-dropped.
    assert!(
        sys.contains("AVAILABLE COMPONENTS"),
        "Basic-tier prompt must carry the components manifest"
    );
    assert!(
        sys.contains("comp-0") && sys.contains("comp-4"),
        "manifest must list the concrete component ids"
    );
    assert!(
        sys.contains("\"type\":\"ref\""),
        "manifest must point at the ref node syntax"
    );

    // (2) The component-composition TEACHING skill survived the Basic allow-set
    // (this is the part the bug dropped).
    assert!(
        report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "Basic tier must KEEP component-composition when a library is present; report={report:?}"
    );
    assert!(
        !report
            .dropped
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition must not be tier-dropped; dropped={:?}",
        report.dropped
    );

    // (3) The teaching skill's actual body (the `ref` + `descendants` rules)
    // is present in the system prompt, not just listed in the report.
    assert!(
        sys.contains("COMPONENT COMPOSITION"),
        "the component-composition skill body must be in the system prompt"
    );
    assert!(
        sys.contains("descendants"),
        "the prompt must teach overriding instance content via descendants"
    );
}

/// Regression guard for the non-mobile Basic dashboard component path.
///
/// The earlier `basic_tier_components_prompt_keeps_both_*` test covers the
/// MOBILE path (9200-token budget with headroom), which only ever exercised the
/// TIER allow-set drop. The non-mobile Basic path still runs under the smaller
/// `budget_max = 5200`, where optional dashboard/depth skills can be
/// budget-dropped. With a component library loaded, the model must still get
/// both halves of the component contract: the AVAILABLE COMPONENTS list and the
/// `component-composition` teaching.
///
/// The force-include pin (prompt.rs: `pinned_skills` when `has_reusable_components`,
/// threaded into `trim_by_budget_pinned`) keeps it budget-exempt in tighter
/// prompts. This test covers the current wide-plan Basic path after retiring
/// the JSONL skills that used to consume part of the budget.
#[test]
fn tight_budget_dashboard_keeps_component_composition() {
    // Basic tier is the path that overrides the budget down to 5200 when the
    // plan is NOT a mobile full screen (the bug surface).
    assert_eq!(
        resolve_model_profile("minimax-m2.7").tier,
        ModelTier::Basic,
        "fixture must exercise the Basic tier (5200 budget on non-mobile)"
    );

    // A wide (non-mobile) dashboard plan → is_mobile_full_screen = false →
    // budget_override = Some(5200), the tight path for Basic models.
    let basic_req = DesignRequest {
        prompt: "Design a 1280x800 analytics dashboard with metric cards, \
                 a chart panel, and a data table using the available components"
            .into(),
        model: Some("minimax-m2.7".into()),
        ..req()
    };
    let mut dash_plan = plan();
    dash_plan.root_frame.width = 1280.0;
    dash_plan.root_frame.height = 800.0;
    let dash_subtask = Subtask {
        id: "main".into(),
        label: "Main".into(),
        region: Region {
            width: 1280.0,
            height: 600.0,
        },
        id_prefix: "main".into(),
        parent_frame_id: Some("page".into()),
        insert_after_sibling_id: None,
        elements: Some("metric cards, chart, table".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };

    let lib = library_with(5);
    // Drive the core with script_on forced OFF so this still exercises the
    // legacy NODE-dialect branch deterministically. Public subagent prompts do
    // not call this branch.
    let (cr, report) = build_subagent_prompt_core(
        &dash_subtask,
        &dash_plan,
        &basic_req,
        AbortFlag::new(),
        false,
        false,
        false,
        false,
        &lib,
        &[],
    );
    let sys = &cr.system_prompt;

    // (0) Prove this is the non-mobile Basic tight path.
    assert_eq!(
        report.budget_max, 5200,
        "non-mobile Basic must use the 5200 budget"
    );
    // The pin only proves something under real budget pressure, and there is
    // more of it than before: the always-kept Base skills alone now sum PAST
    // the 5200 ceiling, so by the time the knapsack reaches Domain skills
    // there is no room at all and an unpinned `component-composition` would be
    // skipped outright.
    //
    // This used to assert "some skill was dropped for BudgetExhausted". That
    // stopped being true when the sub-agent compaction moved BEFORE the
    // knapsack (`resolve_generation_skills_after_prompt_filter`): the optional
    // dashboard/depth skills that used to lose the budget race are now removed
    // by the Basic allow-set first, and are reported as `TierFiltered`. The
    // pin is unchanged — its competitors simply never reach the race.
    assert!(
        report.budget_used > report.budget_max,
        "fixture must still exercise budget pressure; report={report:?}"
    );

    // (1) The component-composition TEACHING skill survived the tight budget.
    assert!(
        report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "tight 5200 budget must FORCE-INCLUDE component-composition when a library \
         is present; report={report:?}"
    );
    // (2) It is NOT recorded as a budget drop (the exact regression).
    assert!(
        !report
            .dropped
            .iter()
            .any(|s| s.name == "component-composition"),
        "component-composition must not be budget-dropped on the 5200 path; \
         dropped={:?}",
        report.dropped
    );
    // (3) The skill BODY (the ref + descendants rules) is in the system prompt.
    assert!(
        sys.contains("COMPONENT COMPOSITION"),
        "the component-composition skill body must reach the tight-budget prompt"
    );
    assert!(
        sys.contains("descendants"),
        "the prompt must teach overriding instance content via descendants"
    );
    // (4) The AVAILABLE COMPONENTS manifest with concrete ids is also present —
    // both halves (LIST + HOW) reach the model on the tight path.
    assert!(
        sys.contains("AVAILABLE COMPONENTS"),
        "tight-budget prompt must carry the components manifest"
    );
    assert!(
        sys.contains("comp-0") && sys.contains("comp-4"),
        "manifest must list the concrete component ids"
    );
    // (5) The budget never paid for a skill the tier filter was about to
    // delete. Every skill the Basic allow-set removes must be reported as
    // `TierFiltered`, never as `BudgetExhausted` — the two reasons are the
    // observable difference between compacting before and after the knapsack.
    // Measured on this fixture: six skills (product-principles,
    // jian-components, design-system, dashboard, design-principles,
    // role-definitions) are allow-set removals, and under the old order they
    // competed for — and consumed — part of a 5200-token budget first.
    for name in ["design-system", "dashboard", "role-definitions"] {
        let reason = report
            .dropped
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.reason);
        assert_eq!(
            reason,
            Some(op_ai_skills::DropReason::TierFiltered),
            "{name} must be removed by the tier filter BEFORE the knapsack, \
             so it never consumes budget; report={report:?}"
        );
    }
}

/// The force-include is gated on a library being present: with NO components, a
/// tight-budget Basic dashboard prompt must NOT pin (or contain) the
/// component-composition skill — proving the pin is additive and never changes
/// normal no-library generation.
#[test]
fn tight_budget_dashboard_without_library_does_not_pin_component_composition() {
    let basic_req = DesignRequest {
        prompt: "Design a 1280x800 analytics dashboard with metric cards, \
                 a chart panel, and a data table"
            .into(),
        model: Some("minimax-m2.7".into()),
        ..req()
    };
    let mut dash_plan = plan();
    dash_plan.root_frame.width = 1280.0;
    dash_plan.root_frame.height = 800.0;
    let dash_subtask = Subtask {
        id: "main".into(),
        label: "Main".into(),
        region: Region {
            width: 1280.0,
            height: 600.0,
        },
        id_prefix: "main".into(),
        parent_frame_id: Some("page".into()),
        insert_after_sibling_id: None,
        elements: Some("metric cards, chart, table".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };

    let (cr, report) = build_subagent_prompt(
        &dash_subtask,
        &dash_plan,
        &basic_req,
        AbortFlag::new(),
        false,
        false,
        &ComponentLibrary::default(),
    );
    assert!(
        !report
            .included
            .iter()
            .any(|s| s.name == "component-composition"),
        "no library ⇒ component-composition must not be force-included; report={report:?}"
    );
    assert!(
        !cr.system_prompt.contains("AVAILABLE COMPONENTS"),
        "no library ⇒ no components manifest"
    );
    assert!(
        !cr.system_prompt.contains("COMPONENT COMPOSITION"),
        "no library ⇒ no component-composition teaching"
    );
}
