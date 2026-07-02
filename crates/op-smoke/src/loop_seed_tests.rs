//! Tests for the minimal scaffold seed (`OPENPENCIL_SMOKE_LOOP_SEED=1`).
//!
//! The seed is the structure-providing step that runs BEFORE the agentic
//! loop. These tests prove that — independent of any network / LLM — a
//! fresh `EditorState` becomes a non-empty seeded tree (a page-root frame
//! carrying named empty section stubs) once the seed command is applied.

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorState;

use crate::loop_seed::{build_seed_command, seed_system_prompt_suffix};

/// Pull the single seeded page-root frame out of a state's active children,
/// asserting exactly one top-level node exists.
fn seeded_root(state: &EditorState) -> &jian_ops_schema::node::frame::FrameNode {
    let children = state.active_children();
    assert_eq!(
        children.len(),
        1,
        "seed must produce exactly one top-level page-root frame, got {}",
        children.len()
    );
    match &children[0] {
        PenNode::Frame(f) => f,
        other => panic!("seeded root must be a frame, got: {other:?}"),
    }
}

#[test]
fn seed_produces_page_root_with_named_section_stubs() {
    // A landing-page prompt long enough to trip the planner's 3-section split.
    let prompt = "Design a marketing landing page for a developer productivity \
                  tool with a hero, feature highlights, and a call to action footer";
    let mut state = EditorState::new();
    assert!(
        state.active_children().is_empty(),
        "fresh state must start empty"
    );

    let cmd = build_seed_command(prompt).expect("seed command builds");
    assert!(state.apply(cmd), "seed command must apply to EditorState");

    // The state is now NON-EMPTY: a page-root frame exists.
    let root = seeded_root(&state);
    assert!(
        root.base.name.is_some(),
        "page-root frame must carry a name"
    );

    // The page-root carries ≥ 1 empty named section stub.
    let sections = root
        .children
        .as_ref()
        .expect("page-root must carry section children");
    assert!(
        !sections.is_empty(),
        "page-root must seed at least one section stub"
    );
    for section in sections {
        let PenNode::Frame(sf) = section else {
            panic!("each section stub must be a frame, got: {section:?}");
        };
        assert!(
            sf.base
                .name
                .as_deref()
                .map(|n| !n.is_empty())
                .unwrap_or(false),
            "each section stub must be NAMED"
        );
        // Stubs are EMPTY — the loop fills them, the seed only provides slots.
        let empty = sf.children.as_ref().map(|c| c.is_empty()).unwrap_or(true);
        assert!(empty, "section stub `{:?}` must start empty", sf.base.name);
    }
}

#[test]
fn seed_mobile_prompt_produces_top_summary_and_main() {
    // A mobile prompt routes the heuristic planner to the mobile preset
    // (Top Summary + Main Content), proving the seed reuses the planner's
    // design-type detection rather than a single fixed shape.
    let prompt = "a 390x844 mobile food delivery home screen";
    let mut state = EditorState::new();
    let cmd = build_seed_command(prompt).expect("mobile seed builds");
    assert!(state.apply(cmd), "mobile seed applies");

    let root = seeded_root(&state);
    let names: Vec<String> = root
        .children
        .as_ref()
        .expect("mobile page-root has sections")
        .iter()
        .filter_map(|n| match n {
            PenNode::Frame(f) => f.base.name.clone(),
            _ => None,
        })
        .collect();
    assert!(
        names.iter().any(|n| n == "Top Summary"),
        "mobile seed must include a Top Summary section, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Main Content"),
        "mobile seed must include a Main Content section, got: {names:?}"
    );
}

#[test]
fn seed_system_prompt_suffix_lists_seeded_sections() {
    // The augmented system prompt must tell the model the scaffold exists
    // and name the very sections the seed inserted, so a weak model fills
    // them instead of redrawing the whole design.
    let prompt = "a 390x844 mobile food delivery home screen";
    let suffix = seed_system_prompt_suffix(prompt);
    assert!(
        suffix.contains("Scaffold already created"),
        "suffix must announce the scaffold, got: {suffix}"
    );
    assert!(
        suffix.contains("Top Summary") && suffix.contains("Main Content"),
        "suffix must list the seeded section names, got: {suffix}"
    );
    assert!(
        suffix.to_lowercase().contains("do not recreate"),
        "suffix must warn against recreating the page-root, got: {suffix}"
    );
}
