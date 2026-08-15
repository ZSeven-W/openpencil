//! `enrich_images` tool tests — injected stub search backends, no network.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use op_image_enrich::{ImageSearchTarget, SEARCH_FAILED_PLACEHOLDER_SRC as SENTINEL};
use op_mcp::{McpTool, ToolOutcome};

use super::enrich_images_tool::{
    run_enrich_sync, EnrichImagesTool, EnrichSummary, ImageSearchBackend,
};

/// A scripted backend: `query -> url`, with a call log so tests can prove a
/// Generate prompt never reached a search.
#[derive(Default)]
struct StubBackend {
    urls: HashMap<String, String>,
    calls: Mutex<Vec<String>>,
}

impl StubBackend {
    fn with(query: &str, url: &str) -> Self {
        Self {
            urls: HashMap::from([(query.to_string(), url.to_string())]),
            ..Self::default()
        }
    }
}

impl ImageSearchBackend for StubBackend {
    fn search(&self, target: &ImageSearchTarget) -> Option<String> {
        self.calls.lock().unwrap().push(target.query.clone());
        self.urls.get(&target.query).cloned()
    }
}

/// A backend that sleeps before answering, to drive the deadline path.
struct SleepingBackend {
    delay: Duration,
    url: String,
}

impl ImageSearchBackend for SleepingBackend {
    fn search(&self, _target: &ImageSearchTarget) -> Option<String> {
        std::thread::sleep(self.delay);
        Some(self.url.clone())
    }
}

/// One Search-bound empty image node + one Generate-bound image node.
const MIXED_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "root",
      "name": "Landing",
      "width": 390,
      "height": 844,
      "layout": "vertical",
      "children": [
        { "type": "image", "id": "photo", "name": "Hero", "src": "", "imageSearchQuery": "mountain lake", "width": 320, "height": 180 },
        { "type": "image", "id": "art", "name": "Art", "src": "", "imagePrompt": "paint a moonlit forest", "width": 320, "height": 180 }
      ]
    }
  ]
}"##;

/// An empty image-fill slot on a Frame (the 0814 shape).
const EMPTY_FILL_FIXTURE: &str = r##"{
  "version": "1.0",
  "children": [
    {
      "type": "frame",
      "id": "slot",
      "name": "Banner",
      "width": 320,
      "height": 180,
      "imageSearchQuery": "forest trail",
      "fill": [{ "type": "image", "url": "" }]
    }
  ]
}"##;

fn load(source: &str) -> EditorState {
    crate::doc_io::load_editor_state_from_source(source, op_editor_core::Locale::EnUs)
        .expect("load enrich fixture")
}

fn find<'a>(nodes: &'a [PenNode], id: &str) -> Option<&'a PenNode> {
    op_editor_core::walkers::find_node(nodes, &NodeId::new(id.to_string()))
}

fn summary_of(json: &str) -> EnrichSummary {
    let value: serde_json::Value = serde_json::from_str(json).expect("enrich summary json");
    EnrichSummary {
        targets: value["targets"].as_u64().unwrap_or_default() as usize,
        resolved: value["resolved"].as_u64().unwrap_or_default() as usize,
        failed: value["failed"].as_u64().unwrap_or_default() as usize,
        unresolved: value["unresolved"].as_u64().unwrap_or_default() as usize,
    }
}

#[test]
fn enrich_fills_search_slots_and_fails_generate_without_searching() {
    let live = load(MIXED_FIXTURE);
    let backend = Arc::new(StubBackend::with(
        "mountain lake",
        "data:image/png;base64,AA==",
    ));
    let backend_dyn: Arc<dyn ImageSearchBackend> = backend.clone();
    let tool = EnrichImagesTool::for_test(&live, backend_dyn);

    let mut args = BTreeMap::new();
    args.insert("timeout_seconds".to_string(), "30".to_string());
    let outcome = tool.call(&args);
    let (json, commands) = match outcome {
        ToolOutcome::OkJsonWithCommand(json, EditorCommand::Batch { commands }) => (json, commands),
        other => panic!("expected a batch outcome, got {other:?}"),
    };
    assert_eq!(
        summary_of(&json),
        EnrichSummary {
            targets: 2,
            resolved: 1,
            failed: 1,
            unresolved: 0,
        }
    );

    // The Generate slot must never have been offered to the search backend.
    let calls: Vec<String> = backend.calls.lock().unwrap().clone();
    assert_eq!(calls, ["mountain lake".to_string()], "search-only contract");

    // The host applier path: apply the recorded writes.
    let mut live = live;
    assert!(live.apply(EditorCommand::Batch { commands }));

    let photo = find(live.active_children(), "photo").expect("photo survives");
    let PenNode::Image(photo) = photo else {
        panic!("photo must stay an image node");
    };
    assert_eq!(photo.src, "data:image/png;base64,AA==");

    let art = find(live.active_children(), "art").expect("art survives");
    let PenNode::Image(art) = art else {
        panic!("art must stay an image node");
    };
    assert_eq!(art.src, SENTINEL, "explicit Generate must fail visibly");
}

#[test]
fn enrich_lands_url_on_empty_image_fill_slot() {
    let live = load(EMPTY_FILL_FIXTURE);
    let backend = Arc::new(StubBackend::with(
        "forest trail",
        "data:image/jpeg;base64,AQ==",
    ));
    let tool = EnrichImagesTool::for_test(&live, backend);

    let outcome = tool.call(&BTreeMap::new());
    let (json, commands) = match outcome {
        ToolOutcome::OkJsonWithCommand(json, EditorCommand::Batch { commands }) => (json, commands),
        other => panic!("expected a batch outcome, got {other:?}"),
    };
    assert_eq!(
        summary_of(&json),
        EnrichSummary {
            targets: 1,
            resolved: 1,
            failed: 0,
            unresolved: 0,
        }
    );

    let mut live = live;
    assert!(live.apply(EditorCommand::Batch { commands }));

    let slot = find(live.active_children(), "slot").expect("slot survives");
    let PenNode::Frame(frame) = slot else {
        panic!("slot must stay a frame");
    };
    let Some([PenFill::Image(body)]) = frame.container.fill.as_deref() else {
        panic!("slot must carry exactly one image fill");
    };
    assert_eq!(body.url, "data:image/jpeg;base64,AQ==");
}

#[test]
fn enrich_timeout_leaves_unstarted_targets_unresolved() {
    // Two targets, one backend slower than the budget: the first target's
    // search started before the deadline and may land; the second is never
    // started and counts unresolved.
    let source = r##"{
      "version": "1.0",
      "children": [
        { "type": "image", "id": "one", "src": "", "imageSearchQuery": "lake one", "width": 120, "height": 80 },
        { "type": "image", "id": "two", "src": "", "imageSearchQuery": "lake two", "width": 120, "height": 80 }
      ]
    }"##;
    let live = load(source);
    let backend = Arc::new(SleepingBackend {
        delay: Duration::from_millis(1200),
        url: "data:image/jpeg;base64,AQ==".to_string(),
    });
    let tool = EnrichImagesTool::for_test(&live, backend);

    let mut args = BTreeMap::new();
    args.insert("timeout_seconds".to_string(), "1".to_string());
    let outcome = tool.call(&args);
    let json = match outcome {
        ToolOutcome::OkJsonWithCommand(json, _) => json,
        other => panic!("expected a batch outcome, got {other:?}"),
    };
    assert_eq!(
        summary_of(&json),
        EnrichSummary {
            targets: 2,
            resolved: 1,
            failed: 0,
            unresolved: 1,
        }
    );
}

#[test]
fn enrich_expired_deadline_marks_every_target_unresolved() {
    // Drive the core loop directly with a deadline already in the past: no
    // search may start, every target stays empty and counts unresolved.
    let mut state = load(MIXED_FIXTURE);
    let backend = StubBackend::default();
    let run = run_enrich_sync(
        &mut state,
        None,
        Instant::now() - Duration::from_secs(1),
        &backend,
    );
    assert_eq!(
        run.summary,
        EnrichSummary {
            targets: 2,
            resolved: 0,
            failed: 0,
            unresolved: 2,
        }
    );
    assert!(
        backend.calls.lock().unwrap().is_empty(),
        "an expired deadline must not start any search"
    );
}

#[test]
fn enrich_timeout_parsing_rejects_and_clamps() {
    let live = load(EMPTY_FILL_FIXTURE);
    let backend = Arc::new(StubBackend::default());
    let tool = EnrichImagesTool::for_test(&live, backend);

    for (raw, expected_err) in [("0", true), ("soon", true), ("-1", true)] {
        let mut args = BTreeMap::new();
        args.insert("timeout_seconds".to_string(), raw.to_string());
        match tool.call(&args) {
            ToolOutcome::Err(op_mcp::ToolErrorCode::InvalidArgument, _) => {
                assert!(expected_err, "timeout_seconds {raw:?} must reject");
            }
            other => panic!("timeout_seconds {raw:?} must reject, got {other:?}"),
        }
    }
    // Above the cap clamps to MAX_TIMEOUT_SECONDS (600) instead of rejecting.
    let mut args = BTreeMap::new();
    args.insert("timeout_seconds".to_string(), "9999".to_string());
    let outcome = tool.call(&args);
    assert!(
        matches!(
            outcome,
            ToolOutcome::OkJsonWithCommand(_, _) | ToolOutcome::OkJson(_)
        ),
        "an over-cap timeout must clamp and succeed"
    );
}

#[test]
fn enrich_rejects_unknown_root_ids() {
    let live = load(EMPTY_FILL_FIXTURE);
    let backend = Arc::new(StubBackend::default());
    let tool = EnrichImagesTool::for_test(&live, backend);

    let mut args = BTreeMap::new();
    args.insert("root_ids".to_string(), "nope".to_string());
    match tool.call(&args) {
        ToolOutcome::Err(op_mcp::ToolErrorCode::InvalidArgument, message) => {
            assert!(
                message.contains("nope"),
                "message must name the bad id: {message}"
            );
        }
        other => panic!("unknown root must reject, got {other:?}"),
    }
}

#[test]
fn enrich_scopes_to_requested_roots() {
    // Two roots; only the one named in root_ids holds the target.
    let source = r##"{
      "version": "1.0",
      "children": [
        { "type": "frame", "id": "a", "name": "A", "width": 200, "height": 200,
          "children": [{ "type": "image", "id": "img_a", "src": "", "imageSearchQuery": "lake a", "width": 120, "height": 80 }] },
        { "type": "frame", "id": "b", "name": "B", "width": 200, "height": 200,
          "children": [{ "type": "image", "id": "img_b", "src": "", "imageSearchQuery": "lake b", "width": 120, "height": 80 }] }
      ]
    }"##;
    let live = load(source);
    let backend = Arc::new(StubBackend::with("lake b", "data:image/png;base64,AQ=="));
    let tool = EnrichImagesTool::for_test(&live, backend);

    let mut args = BTreeMap::new();
    args.insert("root_ids".to_string(), "b".to_string());
    let outcome = tool.call(&args);
    let (json, commands) = match outcome {
        ToolOutcome::OkJsonWithCommand(json, EditorCommand::Batch { commands }) => (json, commands),
        other => panic!("expected a batch outcome, got {other:?}"),
    };
    assert_eq!(
        summary_of(&json),
        EnrichSummary {
            targets: 1,
            resolved: 1,
            failed: 0,
            unresolved: 0,
        }
    );

    let mut live = live;
    assert!(live.apply(EditorCommand::Batch { commands }));
    let img_b = find(live.active_children(), "img_b").expect("img_b survives");
    let PenNode::Image(img_b) = img_b else {
        panic!("img_b must stay an image node");
    };
    assert_eq!(img_b.src, "data:image/png;base64,AQ==");
    let img_a = find(live.active_children(), "img_a").expect("img_a survives");
    let PenNode::Image(img_a) = img_a else {
        panic!("img_a must stay an image node");
    };
    assert_eq!(img_a.src, "", "the out-of-scope slot must stay untouched");
}
