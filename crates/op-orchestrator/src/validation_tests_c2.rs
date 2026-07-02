//! C2 tests — `run_post_generation_validation` full loop.
//!
//! Loaded via `#[path = "validation_tests_c2.rs"] mod tests_c2;` in
//! `validation.rs`.

use std::collections::BTreeMap;
use std::sync::Mutex;

use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::{FrameNode, PenNode, PenNodeBase};

use crate::test_support::{
    SkippedPreValidator, SkippedScreenshotProvider, SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{
    AbortFlag, DesignRequest, DocSink, PreValidationResult, PreValidator, Progress,
    ScreenshotProvider, VisionCallRequest, VisionLlmClient, VisionResponse,
};
use crate::validation::run_post_generation_validation;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_request(validation_enabled: bool) -> DesignRequest {
    DesignRequest {
        prompt: "test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled,
        visual_ref_enabled: false,
    }
}

/// Make a leaf frame node with a given id.
fn make_frame_node(id: &str) -> PenNode {
    PenNode::Frame(FrameNode {
        base: PenNodeBase {
            id: id.to_string(),
            ..Default::default()
        },
        container: ContainerProps::default(),
        children: None,
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// Build a `VecDocSink` containing `n` leaf frame nodes so that
/// `count_nodes_in_active_page` returns `n`.
fn sink_with_n_nodes(n: usize) -> VecDocSink {
    let mut sink = VecDocSink::new();
    for i in 0..n {
        sink.state
            .doc
            .children
            .push(make_frame_node(&format!("node-{i}")));
    }
    sink
}

/// Build JSON for a response with one fix for a given node/property and a quality score.
fn json_one_fix(node_id: &str, quality: u8) -> String {
    format!(
        r#"{{"issues":[],"fixes":[{{"nodeId":"{node_id}","property":"fontSize","value":16}}],"structuralFixes":[],"qualityScore":{quality}}}"#
    )
}

/// Build JSON for a response with no fixes.
fn json_no_fixes(quality: u8) -> String {
    format!(r#"{{"issues":[],"fixes":[],"structuralFixes":[],"qualityScore":{quality}}}"#)
}

// ── Stub with configurable pre-fix count ─────────────────────────────────────

struct CountingPreValidator {
    total: usize,
}

impl PreValidator for CountingPreValidator {
    fn run_pre_validation_fixes(&self, _sink: &mut dyn DocSink) -> PreValidationResult {
        let mut by_category = BTreeMap::new();
        if self.total > 0 {
            by_category.insert("test-category".to_string(), self.total);
        }
        PreValidationResult {
            total: self.total,
            by_category,
        }
    }
}

// ── Screenshot provider that returns `None` after `n` calls ──────────────────

struct OnceScreenshotProvider {
    image: Mutex<Option<String>>,
}

impl OnceScreenshotProvider {
    fn new(image: &str) -> Self {
        Self {
            image: Mutex::new(Some(image.to_string())),
        }
    }
}

impl ScreenshotProvider for OnceScreenshotProvider {
    fn capture_root_frame(&self, _state: &op_editor_core::EditorState) -> Option<String> {
        self.image.lock().unwrap().take()
    }
}

/// Always returns the same base64 image string.
struct AlwaysScreenshotProvider {
    image: String,
}

impl AlwaysScreenshotProvider {
    fn new(image: &str) -> Self {
        Self {
            image: image.to_string(),
        }
    }
}

impl ScreenshotProvider for AlwaysScreenshotProvider {
    fn capture_root_frame(&self, _state: &op_editor_core::EditorState) -> Option<String> {
        Some(self.image.clone())
    }
}

// ── Scripted vision client — pops responses in order ─────────────────────────

struct ScriptedVisionClient {
    responses: Mutex<std::collections::VecDeque<VisionResponse>>,
}

impl ScriptedVisionClient {
    fn new(responses: Vec<VisionResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl VisionLlmClient for ScriptedVisionClient {
    fn validate(&self, _req: VisionCallRequest) -> VisionResponse {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(VisionResponse::Skipped { reason: None })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// TC-1: node-count < 30 → no vision rounds, ValidationDone with total_applied = 0.
#[test]
fn tc1_node_count_below_threshold_skips_vision() {
    let mut sink = sink_with_n_nodes(5); // < 30 threshold
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &SkippedScreenshotProvider,
        &SkippedVisionLlmClient,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    assert_eq!(summary.total_applied, 0);
    assert_eq!(summary.rounds_run, 0);

    assert!(
        events
            .iter()
            .any(|p| matches!(p, Progress::ValidationStarted)),
        "missing ValidationStarted"
    );
    assert!(
        events
            .iter()
            .any(|p| matches!(p, Progress::ValidationPreCheckDone { .. })),
        "missing ValidationPreCheckDone"
    );
    assert!(
        events
            .iter()
            .any(|p| matches!(p, Progress::ValidationDone { .. })),
        "missing ValidationDone"
    );
    assert!(
        !events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundStarted { .. })),
        "should not have ValidationRoundStarted for node-count < 30"
    );
}

/// TC-2: node-count < 30 with non-zero pre-fixes → total_applied = pre.total.
#[test]
fn tc2_pre_fixes_count_returned_when_below_threshold() {
    let mut sink = sink_with_n_nodes(5);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let summary = run_post_generation_validation(
        &mut sink,
        &CountingPreValidator { total: 3 },
        &SkippedScreenshotProvider,
        &SkippedVisionLlmClient,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    assert_eq!(summary.total_applied, 3);
    assert_eq!(summary.rounds_run, 0);

    if let Some(Progress::ValidationDone { total_applied }) = events
        .iter()
        .find(|p| matches!(p, Progress::ValidationDone { .. }))
    {
        assert_eq!(*total_applied, 3);
    } else {
        panic!("ValidationDone missing");
    }
}

/// TC-3: screenshot returns `None` on first round → early exit, 0 complete rounds.
/// `ValidationRoundStarted` IS emitted (the TS emits "Capturing…" before the capture),
/// but `ValidationRoundDone` is NOT emitted because the round never completed.
#[test]
fn tc3_screenshot_none_first_round_exits_early() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &SkippedScreenshotProvider, // always returns None
        &SkippedVisionLlmClient,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    assert_eq!(summary.total_applied, 0);
    assert_eq!(summary.rounds_run, 0);

    // No round should have COMPLETED.
    assert!(
        !events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundDone { .. })),
        "no round should have completed when screenshot is None"
    );
    // ValidationDone must still be emitted.
    assert!(
        events
            .iter()
            .any(|p| matches!(p, Progress::ValidationDone { .. })),
        "ValidationDone must be emitted"
    );
}

/// TC-4: one round, vision returns one fix for non-existent node.
/// The round must emit `ValidationRoundStarted` and `ValidationRoundDone`.
#[test]
fn tc4_one_round_emits_progress() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let screenshot = OnceScreenshotProvider::new("img");
    // Vision returns 1 fix for a non-existent node — apply will silently skip it.
    let vision =
        ScriptedVisionClient::new(vec![VisionResponse::Text(json_one_fix("nonexistent", 5))]);

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &screenshot,
        &vision,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    // Node not found → 0 applied, but round ran.
    assert_eq!(summary.rounds_run, 1);

    assert!(
        events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundStarted { round: 1 })),
        "missing ValidationRoundStarted round=1"
    );
    assert!(
        events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundDone { round: 1, .. })),
        "missing ValidationRoundDone round=1"
    );
}

/// TC-5: quality_score >= 8 in round 1 → loop stops after round 1.
#[test]
fn tc5_quality_threshold_stops_loop_early() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let screenshot = AlwaysScreenshotProvider::new("img");
    // Quality 8 = threshold → stop.
    let vision = ScriptedVisionClient::new(vec![
        VisionResponse::Text(json_no_fixes(8)),
        VisionResponse::Text(json_no_fixes(8)),
        VisionResponse::Text(json_no_fixes(8)),
    ]);

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &screenshot,
        &vision,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    assert_eq!(
        summary.rounds_run, 1,
        "should stop after round 1 at quality 8"
    );

    assert!(
        !events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundStarted { round: 2 })),
        "round 2 should not have started"
    );
}

/// TC-6: fix-history dedup — same fix key seen in rounds 1 AND 2 is dropped in round 2.
/// After dedup, fixes_list is empty → break.
#[test]
fn tc6_fix_history_dedup_second_occurrence_dropped() {
    // 40 nodes total, one with known ID "target-node".
    let mut sink = sink_with_n_nodes(39);
    sink.state.doc.children.push(make_frame_node("target-node"));

    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let screenshot = AlwaysScreenshotProvider::new("img");

    // Round 1: 1 fix for target-node:fontSize (quality 5 → no early-stop, fix applied)
    // Round 2: SAME fix again (history count now == 2 → dedup → 0 fixes → break)
    // Round 3: shouldn't be called
    let vision = ScriptedVisionClient::new(vec![
        VisionResponse::Text(json_one_fix("target-node", 5)),
        VisionResponse::Text(json_one_fix("target-node", 5)),
        VisionResponse::Text(json_no_fixes(9)),
    ]);

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &screenshot,
        &vision,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    // Round 1 runs (applies fix or skips), round 2 dedupes → no fixes → breaks.
    assert_eq!(summary.rounds_run, 2, "should run rounds 1 and 2");

    assert!(
        !events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundStarted { round: 3 })),
        "round 3 should not have started due to dedup"
    );
}

/// TC-7: abort fires before the loop → Err(Aborted), no rounds.
#[test]
fn tc7_abort_stops_loop() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();
    abort.set(); // abort before call

    let screenshot = AlwaysScreenshotProvider::new("img");
    let vision = ScriptedVisionClient::new(vec![VisionResponse::Text(json_one_fix("n", 3))]);

    let result = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &screenshot,
        &vision,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    );

    assert!(result.is_err(), "expected Err when abort is set");
    assert!(
        !events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundStarted { .. })),
        "no round should start after abort"
    );
}

/// TC-8: all stub providers with node-count >= 30 → ValidationDone, 0 rounds, 0 applied.
#[test]
fn tc8_all_stubs_large_node_count_zero_rounds() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &SkippedScreenshotProvider,
        &SkippedVisionLlmClient,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    assert_eq!(summary.total_applied, 0);
    assert_eq!(summary.rounds_run, 0);
    assert!(
        events
            .iter()
            .any(|p| matches!(p, Progress::ValidationDone { total_applied: 0 })),
        "ValidationDone(0) expected"
    );
}

/// TC-9: validation_enabled=false → pre-check still runs, no vision rounds.
#[test]
fn tc9_validation_disabled_skips_vision() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let summary = run_post_generation_validation(
        &mut sink,
        &CountingPreValidator { total: 2 },
        &SkippedScreenshotProvider,
        &SkippedVisionLlmClient,
        "sys",
        &make_request(false), // validation_enabled = false
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    assert_eq!(summary.total_applied, 2);
    assert_eq!(summary.rounds_run, 0);

    assert!(
        !events
            .iter()
            .any(|p| matches!(p, Progress::ValidationRoundStarted { .. })),
        "no vision rounds when disabled"
    );
}

/// TC-10: vision returns Skipped on round 1 → ValidationDone, 0 complete rounds.
#[test]
fn tc10_vision_skipped_first_round_exits() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let screenshot = AlwaysScreenshotProvider::new("img");
    let vision = ScriptedVisionClient::new(vec![VisionResponse::Skipped {
        reason: Some("no provider".to_string()),
    }]);

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &screenshot,
        &vision,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    assert_eq!(summary.total_applied, 0);
    assert_eq!(
        summary.rounds_run, 0,
        "skipped on round 1 → 0 complete rounds"
    );
}

/// TC-11: qualityScore=0 with no issues (parse failure) → round runs once, then breaks.
#[test]
fn tc11_quality_zero_no_issues_breaks_after_round1() {
    let mut sink = sink_with_n_nodes(40);
    let mut events: Vec<Progress> = Vec::new();
    let abort = AbortFlag::new();

    let screenshot = AlwaysScreenshotProvider::new("img");
    // Return malformed JSON — parse gives quality_score=0, no issues.
    let vision = ScriptedVisionClient::new(vec![
        VisionResponse::Text("not valid json".to_string()),
        VisionResponse::Text("not valid json".to_string()),
        VisionResponse::Text("not valid json".to_string()),
    ]);

    let summary = run_post_generation_validation(
        &mut sink,
        &SkippedPreValidator,
        &screenshot,
        &vision,
        "sys",
        &make_request(true),
        &mut |p| events.push(p),
        &abort,
    )
    .expect("should succeed");

    // Parse failure → break on round 1 (1 round_started, 1 round_done after the break).
    assert_eq!(
        summary.rounds_run, 1,
        "1 round attempted before break on parse failure"
    );
}
