//! Tests for the `spawn_agents` sub-agent runtime (Task 3.1).
//!
//! The full GUI loop can't be unit-tested (winit + a live provider), but
//! the pure pieces can: the scoped-prompt builder, the spec parser, the
//! nested-spawn guard + stash, and the per-sub indicator/identity wiring
//! against the process-global `agent_indicators` registry.

use super::*;
use op_editor_core::{agent_indicators, EditorState};
use op_host_services::design_agent_tools::execute_design_tool;

/// A real styleguide name from the embedded corpus.
const REAL_STYLEGUIDE: &str = "ai-product-dark";

fn spec(prompt: &str, containers: &[&str], styleguide: &str, guidelines: &[&str]) -> SpawnSpec {
    SpawnSpec {
        prompt: prompt.to_string(),
        container_nodes: containers.iter().map(|s| s.to_string()).collect(),
        styleguide_name: styleguide.to_string(),
        guideline_names: guidelines.iter().map(|s| s.to_string()).collect(),
    }
}

/// Insert one top-level frame into `state` via the real design tool path,
/// returning its id (so a test can badge it under a sub's epoch).
fn insert_frame(state: &mut EditorState) -> String {
    use op_editor_core::pen_node_ext::PenNodeExt;
    let before: std::collections::HashSet<String> = state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    let (result, mutated) = execute_design_tool(
        state,
        "batch_design",
        r#"{"operations":"root=I(null,{type:'frame',width:120,height:80})"}"#,
    );
    assert!(!result.is_error, "batch_design failed: {}", result.content);
    assert!(mutated, "batch_design must insert a frame");
    state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .find(|id| !before.contains(id))
        .expect("a new frame id appeared")
}

// ---------------------------------------------------------------------------
// build_sub_agent_prompt
// ---------------------------------------------------------------------------

#[test]
fn build_prompt_includes_protocol_styleguide_guideline_and_scoping() {
    let s = spec(
        "Design the hero section",
        &["n10", "n11"],
        REAL_STYLEGUIDE,
        &["web-app"],
    );
    let prompt = build_sub_agent_prompt(&s);

    // Design-agent protocol prompt markers (verbatim from the corpus).
    assert!(
        prompt.contains("batch_design"),
        "must include the design-agent protocol prompt"
    );
    assert!(
        prompt.contains("get_style_guide"),
        "design-agent protocol markers must be present"
    );

    // The sub's assignment + prompt text.
    assert!(
        prompt.contains("Design the hero section"),
        "must include the sub's prompt"
    );

    // Container-node scoping text.
    assert!(
        prompt.contains("n10, n11"),
        "must scope the sub to its container nodes"
    );

    // Resolved styleguide content (the full markdown is injected; every
    // guide carries an explicit Style Scope section).
    assert!(
        prompt.contains("## Style Scope"),
        "must inject the resolved styleguide content"
    );

    // Resolved guideline content (web-app composes product+design
    // principles; canonical phrase from product-principles).
    assert!(
        prompt.contains("PURPOSE FIRST"),
        "must inject the resolved web-app guideline content"
    );
}

#[test]
fn build_prompt_omits_container_scope_when_no_containers() {
    let s = spec("Fill the nav", &[], REAL_STYLEGUIDE, &[]);
    let prompt = build_sub_agent_prompt(&s);
    assert!(prompt.contains("Fill the nav"));
    assert!(
        !prompt.contains("only build inside container node(s)"),
        "no container scope line when the spec lists no containers"
    );
}

#[test]
fn build_prompt_tolerates_unknown_styleguide_and_guideline() {
    // Unknown names resolve to None and are simply skipped — the prompt
    // still carries the protocol + the sub's assignment.
    let s = spec("Build a card", &["n1"], "no-such-guide", &["no-such-topic"]);
    let prompt = build_sub_agent_prompt(&s);
    assert!(prompt.contains("Build a card"));
    assert!(prompt.contains("batch_design"));
}

// ---------------------------------------------------------------------------
// parse_spawn_args
// ---------------------------------------------------------------------------

#[test]
fn parse_args_accepts_array_config_form() {
    // The model emits `config` as a nested JSON array.
    let args = r#"{"config":[{"prompt":"Hero","styleguideName":"brand","containerNodes":["n1"],"guidelineNames":["web-app"]}]}"#;
    let specs = parse_spawn_args(args).expect("array config parses");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].prompt, "Hero");
    assert_eq!(specs[0].styleguide_name, "brand");
    assert_eq!(specs[0].container_nodes, vec!["n1"]);
    assert_eq!(specs[0].guideline_names, vec!["web-app"]);
}

#[test]
fn parse_args_accepts_string_config_form() {
    // Some transports pass `config` already stringified.
    let args = r#"{"config":"[{\"prompt\":\"Footer\",\"styleguideName\":\"brand\"}]"}"#;
    let specs = parse_spawn_args(args).expect("string config parses");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].prompt, "Footer");
}

#[test]
fn parse_args_two_items_returns_two_specs() {
    let args = r#"{"config":[
        {"prompt":"Hero","styleguideName":"brand"},
        {"prompt":"Footer","styleguideName":"brand"}
    ]}"#;
    let specs = parse_spawn_args(args).expect("two items parse");
    assert_eq!(specs.len(), 2);
    assert_eq!(specs[1].prompt, "Footer");
}

#[test]
fn parse_args_missing_config_errors() {
    let err = parse_spawn_args(r#"{"foo":"bar"}"#).unwrap_err();
    assert!(
        err.contains("non-empty config array"),
        "missing config must error: {err}"
    );
}

#[test]
fn parse_args_invalid_json_errors() {
    let err = parse_spawn_args("not json").unwrap_err();
    assert!(
        err.contains("must be a JSON object"),
        "invalid json must error: {err}"
    );
}

// ---------------------------------------------------------------------------
// Nested-spawn guard + stash
// ---------------------------------------------------------------------------

#[test]
fn stash_top_level_succeeds_nested_refused() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // Clear any leftover stash.
    let _ = take_pending_spawn();

    // Nested (a sub calling spawn_agents) is refused, leaves no stash.
    let refused = stash_pending_spawn(vec![spec("X", &[], "brand", &[])], true);
    assert!(!refused, "nested spawn must be refused");
    assert!(
        take_pending_spawn().is_none(),
        "a refused nested spawn must not stash specs"
    );

    // Top-level stashes; the host picks it up exactly once.
    let ok = stash_pending_spawn(
        vec![spec("A", &[], "brand", &[]), spec("B", &[], "brand", &[])],
        false,
    );
    assert!(ok, "top-level spawn must stash");
    let taken = take_pending_spawn().expect("specs were stashed");
    assert_eq!(taken.len(), 2);
    assert!(take_pending_spawn().is_none(), "the stash is consumed once");
}

// ---------------------------------------------------------------------------
// Per-sub identity + indicator wiring (no live provider needed)
// ---------------------------------------------------------------------------

#[test]
fn distinct_identities_and_epoch_scoped_frame_badges() {
    use op_orchestrator::agent_identity::assign_agent_identities;

    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    // Two specs → two distinct identities (colour + name).
    let identities = assign_agent_identities(2);
    assert_eq!(identities.len(), 2);
    assert_ne!(identities[0].color, identities[1].color);
    assert_ne!(identities[0].name, identities[1].name);

    // Build two indicators as `launch_sub_agents` would (one epoch each).
    let mut state = EditorState::new();
    let epoch0 = agent_indicators::begin();
    let initial0 = collect_top_level_frame_ids(&state);
    let ind0 = DesignLoopIndicator {
        epoch: epoch0,
        color: identities[0].color.clone(),
        name: identities[0].name.clone(),
        initial_frame_ids: initial0,
    };

    // Sub-0 builds a frame; register it under sub-0's epoch/identity.
    let frame_id = insert_frame(&mut state);
    register_new_frames(&ind0, &state);

    let snap = agent_indicators::snapshot();
    let tag = snap
        .frames
        .get(&frame_id)
        .expect("the new frame is badged under sub-0's epoch");
    assert_eq!(tag.color, identities[0].color, "badge uses sub-0's colour");
    assert_eq!(tag.name, identities[0].name, "badge uses sub-0's name");

    // Ending sub-0's epoch retires the badge so a later run starts clean.
    agent_indicators::end_if_epoch(epoch0);
    assert!(
        agent_indicators::snapshot().frames.is_empty(),
        "ending the epoch clears sub-0's badges"
    );
    agent_indicators::clear();
}

#[test]
fn lazy_epoch_keeps_each_active_subs_badges_live_in_sequence() {
    // Regression: beginning all epochs up-front would leave only the LAST
    // sub's epoch live (begin() bumps one global epoch + clears), so an
    // earlier sub's badges would be dropped by the epoch-scoped registry.
    // The lazy-begin model (one epoch live at a time, sequential) fixes
    // this. Simulate the pump's begin → badge → end cadence for two subs.
    use op_orchestrator::agent_identity::assign_agent_identities;

    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    agent_indicators::clear();

    let ids = assign_agent_identities(2);
    let mut state = EditorState::new();

    // Sub-0 becomes active: begin its epoch, snapshot, build a frame, badge.
    let epoch0 = agent_indicators::begin();
    let initial0 = collect_top_level_frame_ids(&state);
    let ind0 = DesignLoopIndicator {
        epoch: epoch0,
        color: ids[0].color.clone(),
        name: ids[0].name.clone(),
        initial_frame_ids: initial0,
    };
    let frame0 = insert_frame(&mut state);
    register_new_frames(&ind0, &state);

    // Sub-0's frame must be badged with sub-0's colour (epoch is live).
    let tag0 = agent_indicators::snapshot()
        .frames
        .get(&frame0)
        .cloned()
        .expect("sub-0 frame badged while sub-0 is active");
    assert_eq!(tag0.color, ids[0].color);

    // Sub-0 finishes: end its epoch (retires badges), THEN sub-1 begins.
    agent_indicators::end_if_epoch(epoch0);
    let epoch1 = agent_indicators::begin();
    assert!(
        epoch1 > epoch0,
        "each active sub gets a fresh, higher epoch"
    );
    let initial1 = collect_top_level_frame_ids(&state); // includes frame0
    let ind1 = DesignLoopIndicator {
        epoch: epoch1,
        color: ids[1].color.clone(),
        name: ids[1].name.clone(),
        initial_frame_ids: initial1,
    };
    let frame1 = insert_frame(&mut state);
    register_new_frames(&ind1, &state);

    // Sub-1 badges only ITS new frame, with sub-1's colour.
    let snap = agent_indicators::snapshot();
    let tag1 = snap
        .frames
        .get(&frame1)
        .cloned()
        .expect("sub-1 frame badged while sub-1 is active");
    assert_eq!(tag1.color, ids[1].color);
    assert!(
        !snap.frames.contains_key(&frame0),
        "sub-0's badge retired when its epoch ended (sub-1 doesn't re-tag it)"
    );

    agent_indicators::end_if_epoch(epoch1);
    agent_indicators::clear();
}

// ---------------------------------------------------------------------------
// Sequential pump bookkeeping (counter + clear) without a live provider
// ---------------------------------------------------------------------------

#[test]
fn pump_with_finished_subs_decrements_then_clears_agents_running() {
    use op_host_native::WidgetHostNative;

    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let mut host = WidgetHostNative::new();
    // Seed the N/M header as `launch_sub_agents` would for two subs.
    host.editor_state_mut().chat.agents_running = (2, 2);

    // Two subs whose sessions are already `None` (finished) — drives the
    // advance + decrement + clear path without a live ChatSession. The
    // indicator is begun lazily by the pump, so it starts as `None`.
    use op_orchestrator::agent_identity::AgentIdentity;
    let mut subs = vec![
        SubAgentSession {
            session: None,
            identity: AgentIdentity {
                color: "#FF6B6B".into(),
                name: "Kiki".into(),
            },
            indicator: None,
        },
        SubAgentSession {
            session: None,
            identity: AgentIdentity {
                color: "#4ECDC4".into(),
                name: "Mochi".into(),
            },
            indicator: None,
        },
    ];
    let mut active = 0usize;

    // Frame 1: sub-0 is already finished → advance to 1, header → (1, 2).
    let changed = pump_sub_agents(&mut host, &mut subs, &mut active, None);
    assert!(changed);
    assert_eq!(active, 1);
    assert_eq!(host.editor_state().chat.agents_running, (1, 2));
    assert_eq!(subs.len(), 2, "subs stay until all finish");

    // Frame 2: sub-1 finished → advance past the end → clear + (0, 0).
    let changed = pump_sub_agents(&mut host, &mut subs, &mut active, None);
    assert!(changed);
    assert!(subs.is_empty(), "all subs done → cleared");
    assert_eq!(active, 0);
    assert_eq!(host.editor_state().chat.agents_running, (0, 0));

    // Idle: nothing to pump.
    assert!(!pump_sub_agents(&mut host, &mut subs, &mut active, None));
    agent_indicators::clear();
}
