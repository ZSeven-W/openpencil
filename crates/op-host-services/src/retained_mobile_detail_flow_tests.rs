//! Opt-in engine regression of an untouched, saved mobile prototype.
//!
//! Required env: OPENPENCIL_QA_DETAIL_FLOW_{DOCUMENT,START,TARGET,OUTPUT}.
//! MODE is `all` (default), `flow`, or `scroll`; TAP is required for flow,
//! LAST for scroll. Optional FOOTER/SCROLL select exact target-page nodes.
//! ROUNDTRIP=1 additionally saves and reloads a new copy through doc_io.
//! OUTPUT must not exist. No model, binding repair, or OS input is used.
//! PNGs are native raster exports of the live Preview scene, not OS captures
//! or device-frame paint_framed screenshots. Device pin geometry is checked
//! separately. A failure still writes a receipt and fails the ignored test.

use op_editor_core::EditorState;
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};
use op_editor_ui::Rect;
use op_preview_core::{device_frame::compute_frame_geometry, PreviewSession};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

struct Case {
    input: PathBuf,
    output: PathBuf,
    start: String,
    target: String,
    tap: Option<String>,
    last: Option<String>,
    footer: Option<String>,
    scroll: Option<String>,
    mode: String,
    roundtrip: bool,
}

fn optional(name: &str) -> Option<String> {
    std::env::var(format!("OPENPENCIL_QA_DETAIL_FLOW_{name}"))
        .ok()
        .filter(|v| !v.trim().is_empty())
}

fn required(name: &str) -> String {
    optional(name).unwrap_or_else(|| panic!("OPENPENCIL_QA_DETAIL_FLOW_{name} is required"))
}

impl Case {
    fn from_env() -> Self {
        Self {
            input: optional("DOCUMENT").unwrap_or_default().into(),
            output: required("OUTPUT").into(),
            start: optional("START").unwrap_or_default(),
            target: optional("TARGET").unwrap_or_default(),
            tap: optional("TAP"),
            last: optional("LAST"),
            footer: optional("FOOTER"),
            scroll: optional("SCROLL"),
            mode: optional("MODE").unwrap_or_else(|| "all".into()),
            roundtrip: optional("ROUNDTRIP").as_deref() == Some("1"),
        }
    }

    fn flow(&self) -> bool {
        self.mode != "scroll"
    }

    fn scroll(&self) -> bool {
        self.mode != "flow"
    }
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn node<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
    if value["id"] == id {
        return Some(value);
    }
    ["children", "pages"].into_iter().find_map(|key| {
        value[key]
            .as_array()
            .into_iter()
            .flatten()
            .find_map(|child| node(child, id))
    })
}

fn live<'a>(scene: &'a LayoutScene, id: &str) -> &'a SceneNode {
    scene
        .active_page()
        .and_then(|page| page.find(id))
        .unwrap_or_else(|| panic!("node {id} is absent from the mounted scene"))
}

fn rect_value(rect: Rect) -> Value {
    json!([rect.origin.x, rect.origin.y, rect.size.x, rect.size.y])
}

fn bottom(rect: Rect) -> f32 {
    rect.origin.y + rect.size.y
}

fn valid(rect: Rect) -> bool {
    [rect.origin.x, rect.origin.y, rect.size.x, rect.size.y]
        .into_iter()
        .all(f32::is_finite)
        && rect.size.x > 0.0
        && rect.size.y > 0.0
}

fn inside(inner: Rect, outer: Rect) -> bool {
    valid(inner)
        && valid(outer)
        && inner.origin.x >= outer.origin.x - 1.0
        && inner.origin.y >= outer.origin.y - 1.0
        && inner.origin.x + inner.size.x <= outer.origin.x + outer.size.x + 1.0
        && bottom(inner) <= bottom(outer) + 1.0
}

fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.origin.x.max(b.origin.x);
    let y = a.origin.y.max(b.origin.y);
    Rect::xywh(
        x,
        y,
        (a.origin.x + a.size.x).min(b.origin.x + b.size.x) - x,
        bottom(a).min(bottom(b)) - y,
    )
}

fn visible_rect(root: &SceneNode, id: &str, clip: Rect) -> Option<Rect> {
    if root.hidden
        || !root.opacity.is_finite()
        || root.opacity <= 0.0
        || !root.composite_opacity.is_finite()
        || root.composite_opacity <= 0.0
    {
        return None;
    }
    let clip = if root.clip_content {
        intersect(clip, root.bounds)
    } else {
        clip
    };
    if root.id == id {
        return Some(intersect(clip, root.bounds));
    }
    root.children
        .iter()
        .find_map(|child| visible_rect(child, id, clip))
}

fn enter(state: &EditorState) -> PreviewSession {
    PreviewSession::enter(
        &state.doc,
        (800.0, 1000.0),
        &state.ui.variables.active_theme,
        state.ui.active_page_index,
        state.editor_ui.preserve_authored_geometry,
        false,
        std::rc::Rc::new(jian_skia::SkiaMeasure::new()),
        0,
    )
    .expect("production Preview entry")
}

fn settle(session: &mut PreviewSession, first_ms: u64) -> bool {
    let mut switched = false;
    // Sample the real scene at every bounded engine-clock tick, as paint does.
    // Clock advancement alone never observes newly visible inView motion;
    // the final assertion snapshot would otherwise start its first frame.
    for offset in [0, 500, 1000] {
        session.pump(first_ms + offset);
        switched |= session.reconcile(first_ms + offset).switched;
        let _ = session.preview_scene_for_test();
    }
    switched
}

fn mounted(session: &PreviewSession) -> Value {
    let entries = session.screen_switcher_entries();
    let scene = session.preview_scene_for_test();
    let root = scene.active_page().and_then(|page| page.children.first());
    let root_visible = root.is_some_and(|node| {
        visible_rect(node, &node.id, node.bounds).is_some_and(|clip| inside(node.bounds, clip))
    });
    json!({
        "root":session.framed_root().map(|(id, _)| id),
        "root_fully_visible":root_visible,
        "root_bounds":root.map(|node| rect_value(node.bounds)),
        "mounted_route":session.current_screen_index().and_then(|i| entries.get(i)).map(|e| &e.0),
        "runtime_state":session.debug_snapshot(),
        "warnings":session.warnings(),
    })
}

fn assert_mounted(session: &PreviewSession, root: &str) {
    assert_eq!(
        session.framed_root().map(|(id, _)| id),
        Some(root.to_owned()),
        "expected root must be mounted, not merely requested by the router"
    );
    let scene = session.preview_scene_for_test();
    assert!(
        scene
            .active_page()
            .unwrap()
            .children
            .iter()
            .any(|n| n.id == root),
        "target must be a live page root"
    );
    let root_node = live(&scene, root);
    assert!(
        visible_rect(root_node, root, root_node.bounds)
            .is_some_and(|clip| inside(root_node.bounds, clip)),
        "mounted root must have valid bounds and positive visible opacity"
    );
}

fn capture(session: &PreviewSession, output: &Path, name: &str, receipt: &mut Value) {
    let (root, _) = session.framed_root().expect("a mounted root to capture");
    let scene = session.preview_scene_for_test();
    let png = crate::export::render_node_raster_bytes(
        &scene,
        &root,
        crate::export::RasterFormat::Png,
        1.0,
    )
    .expect("native raster capture of live scene");
    let path = output.join(name);
    std::fs::write(&path, &png).unwrap();
    receipt["captures"].as_array_mut().unwrap().push(json!({
        "file":path,"root":root,"sha256":sha(&png),"kind":"live-preview-scene-native-raster-export"
    }));
}

fn stage(receipt: &mut Value, name: &str) {
    receipt["current_stage"] = json!(name);
    receipt["stages"][name] = json!("running");
}

fn setup_root(session: &mut PreviewSession, raw: &Value, root: &str, now: u64) -> Value {
    let initial = mounted(session);
    if session.framed_root().is_some_and(|(id, _)| id == root) {
        return json!({"method":"already-mounted","before":initial});
    }
    let path = node(raw, root)
        .and_then(|n| n["screen"].as_str())
        .expect("setup requires an authored screen route; never synthesize a binding");
    session.navigate_to_screen(path);
    assert!(
        session.reconcile(now).switched,
        "setup screen switch failed"
    );
    settle(session, now + 1);
    assert_mounted(session, root);
    json!({"method":"explicit-screen-switcher-setup-only","before":initial,"path":path,
        "business_navigation_evidence":false})
}

fn flow(case: &Case, raw: &Value, session: &mut PreviewSession, receipt: &mut Value) {
    stage(receipt, "flow");
    let tap_id = case.tap.as_deref().expect("TAP is required for flow");
    assert_ne!(
        case.start, case.target,
        "flow must navigate to another screen"
    );
    for id in [&case.start, &case.target] {
        assert!(node(raw, id).and_then(|v| v["screen"].as_str()).is_some(),
            "{id} has no saved screen marker; preview auto-wiring is not persisted binding evidence");
    }
    let start = node(raw, &case.start).unwrap();
    let tap = node(start, tap_id).expect("tap node must belong to the saved starting root");
    let actions = tap["events"]["onTap"].clone();
    receipt["flow"]["saved_on_tap"] = actions.clone();
    assert!(
        actions.as_array().is_some_and(|a| !a.is_empty()),
        "selected node has no saved onTap actions; do not repair the artwork to pass"
    );
    let scene = session.preview_scene_for_test();
    let target = live(&scene, tap_id);
    let bounds = target.bounds;
    let start_scene = live(&scene, &case.start);
    let visible = visible_rect(start_scene, tap_id, start_scene.bounds)
        .expect("tap target or its ancestor is hidden");
    assert!(
        inside(bounds, visible),
        "tap target must be fully visible through its ancestor clips"
    );
    let point = (
        bounds.origin.x + bounds.size.x / 2.0,
        bounds.origin.y + bounds.size.y / 2.0,
    );
    receipt["flow"]["before"] = mounted(session);
    receipt["flow"]["point"] = json!([point.0, point.1]);
    let trace_start = session
        .trace_entries()
        .last()
        .map_or(0, |entry| entry.sequence);
    session.set_now_ms(4000);
    let consumed = session.dispatch_tap(point.0, point.1);
    let switched = settle(session, 4500);
    receipt["flow"]["consumed"] = json!(consumed);
    receipt["flow"]["reconcile_switched"] = json!(switched);
    receipt["flow"]["after"] = mounted(session);
    let trace = session.trace_entries();
    let authored_navigation_executed = trace.iter().any(|entry| {
        entry.sequence > trace_start
            && entry.node_id.as_deref() == Some(tap_id)
            && entry.event.as_deref() == Some("onTap")
            && matches!(entry.action.as_deref(), Some("push" | "replace" | "pop"))
            && entry.detail["phase"] == "result"
            && entry.detail["ok"] == true
    });
    receipt["flow"]["saved_tap_navigation_executed"] = json!(authored_navigation_executed);
    receipt["flow"]["trace"] = json!(trace);
    let mounted_target = session
        .framed_root()
        .is_some_and(|(id, _)| id == case.target);
    if !mounted_target {
        capture(session, &case.output, "02-navigation-failed.png", receipt);
    }
    assert!(consumed, "tap was not consumed by the runtime");
    assert!(authored_navigation_executed,
        "the selected node's saved onTap navigation did not execute; another handler cannot satisfy this check");
    assert!(switched, "tap did not commit a screen switch via reconcile");
    assert_mounted(session, &case.target);
    let scene = session.preview_scene_for_test();
    assert!(
        !live(&scene, &case.target).children.is_empty(),
        "mounted target is empty"
    );
    if let Some(last) = &case.last {
        assert!(
            live(&scene, &case.target).find(last).is_some(),
            "requested target content is absent"
        );
    }
    receipt["stages"]["flow"] = json!("passed");
}

fn scroll(case: &Case, raw: &Value, session: &mut PreviewSession, receipt: &mut Value) {
    stage(receipt, "scroll");
    let last_id = case.last.as_deref().expect("LAST is required for scroll");
    let root = node(raw, &case.target).expect("saved target root");
    let scroll_id = if let Some(id) = &case.scroll {
        id.clone()
    } else {
        let candidates: Vec<_> = root["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["role"] == "scroll-area")
            .collect();
        assert_eq!(
            candidates.len(),
            1,
            "use SCROLL to disambiguate the actual scroll container"
        );
        candidates[0]["id"].as_str().unwrap().to_owned()
    };
    let viewport_source = node(root, &scroll_id).expect("scroll node belongs to target");
    let last_source = node(viewport_source, last_id).expect("LAST must be a descendant of SCROLL");
    assert_ne!(
        last_id, scroll_id,
        "LAST must identify a row, not the whole viewport"
    );
    assert!(
        last_source["children"]
            .as_array()
            .is_some_and(|children| !children.is_empty()),
        "LAST must select a complete row/card container, not only its final text label"
    );
    let (footer_id, pin_rect) = session
        .pinned_nav_candidate(true)
        .expect("target has no pinned footer candidate");
    if let Some(expected) = &case.footer {
        assert_eq!(&footer_id, expected, "unexpected footer was detected");
    }
    assert!(
        node(viewport_source, &footer_id).is_none(),
        "footer must be outside scroll content"
    );
    let before = session.preview_scene_for_test();
    let board = live(&before, &case.target).bounds;
    let viewport = live(&before, &scroll_id).bounds;
    let footer = live(&before, &footer_id).bounds;
    let last_before = live(&before, last_id).bounds;
    let footer_clip = visible_rect(live(&before, &case.target), &footer_id, board);
    let footer_visible = footer_clip.is_some_and(valid);
    let footer_fully_visible = footer_clip.is_some_and(|clip| inside(footer, clip));
    receipt["scroll"] = json!({"viewport_id":scroll_id,"last_id":last_id,"footer_id":footer_id,
        "last_node_type":last_source["type"],"last_node_name":last_source["name"],
        "expected_footer":case.footer,"footer_selection":if case.footer.is_some(){"explicit"}else{"production-pin-candidate"},
        "before":{"root":rect_value(board),"viewport":rect_value(viewport),
            "footer":rect_value(footer),"last":rect_value(last_before),
            "footer_visible":footer_visible,"footer_fully_visible":footer_fully_visible,
            "footer_clip":footer_clip.map(rect_value)}});
    assert!(
        footer_fully_visible,
        "fixed footer must be visible and complete through ancestor clips before scrolling"
    );
    assert!(
        inside(footer, board),
        "fixed footer is outside the target board"
    );
    assert_eq!(pin_rect, footer);
    let status = session.pinned_status_bar_candidate(true);
    let geometry = compute_frame_geometry(
        op_editor_core::PreviewDeviceKind::Phone,
        Rect::xywh(0.0, 0.0, 480.0, 960.0),
        board,
        Some(footer),
        status.as_ref().map(|(_, r)| *r),
    );
    let strip = geometry.pinned.as_ref().expect("device pinned strip").strip;
    assert!(inside(strip, geometry.frame) && (bottom(strip) - bottom(geometry.frame)).abs() <= 1.0);
    let mut visible = intersect(viewport, board);
    visible.size.y = visible.size.y.min(footer.origin.y - visible.origin.y);
    if let Some((_, status_rect)) = &status {
        visible = intersect(
            visible,
            Rect::xywh(
                board.origin.x,
                bottom(*status_rect),
                board.size.x,
                bottom(board) - bottom(*status_rect),
            ),
        );
    }
    receipt["scroll"]["before"]["visible"] = rect_value(visible);
    receipt["scroll"]["device_footer_strip"] = rect_value(strip);
    receipt["scroll"]["device_frame"] = rect_value(geometry.frame);
    assert!(
        valid(visible),
        "no visible scroll region remains above the footer"
    );
    assert!(
        !inside(last_before, visible),
        "LAST is already visible; scroll regression is not applicable (use MODE=flow)"
    );
    let point = (
        visible.origin.x + visible.size.x / 2.0,
        visible.origin.y + visible.size.y / 2.0,
    );
    session.set_now_ms(7000);
    let consumed = session.dispatch_wheel(point.0, point.1, 0.0, -10000.0);
    settle(session, 7500);
    let after = session.preview_scene_for_test();
    let last_after = live(&after, last_id).bounds;
    let footer_after = live(&after, &footer_id).bounds;
    let root_after = live(&after, &case.target);
    let footer_clip_after = visible_rect(root_after, &footer_id, root_after.bounds);
    let footer_visible_after = footer_clip_after.is_some_and(valid);
    let footer_fully_visible_after =
        footer_clip_after.is_some_and(|clip| inside(footer_after, clip));
    let visible_after = intersect(visible, live(&after, &scroll_id).bounds);
    let row_clip = visible_rect(live(&after, &case.target), last_id, visible_after);
    let last_visible = row_clip.is_some_and(|clip| inside(last_after, clip));
    let stationary = footer_after == footer;
    let status_stationary = status
        .as_ref()
        .map(|(id, rect)| live(&after, id).bounds == *rect);
    let candidate_stable = session
        .pinned_nav_candidate(true)
        .is_some_and(|(id, rect)| id == footer_id && rect == footer);
    receipt["scroll"]["after"] = json!({"last":rect_value(last_after),"footer":rect_value(footer_after),"visible":rect_value(visible_after),
        "footer_visible":footer_visible_after,"footer_fully_visible":footer_fully_visible_after,
        "footer_clip":footer_clip_after.map(rect_value)});
    receipt["scroll"]["wheel_consumed"] = json!(consumed);
    receipt["scroll"]["last_row_fully_visible"] = json!(last_visible);
    receipt["scroll"]["footer_stationary"] = json!(stationary);
    receipt["scroll"]["status_stationary"] = json!(status_stationary);
    receipt["scroll"]["status_check"] = json!(if status.is_some() {
        "checked"
    } else {
        "not-applicable-no-status-candidate"
    });
    receipt["scroll"]["pinned_footer_stable"] = json!(candidate_stable);
    capture(session, &case.output, "03-scrolled-bottom.png", receipt);
    assert_mounted(session, &case.target);
    assert!(
        consumed && last_visible && stationary && candidate_stable && footer_fully_visible_after,
        "scroll must be consumed, reveal the whole last row and leave the expected footer fixed and fully visible"
    );
    assert!(
        status_stationary.unwrap_or(true),
        "detected status bar moved"
    );
    assert!(
        last_after.origin.y < last_before.origin.y,
        "content did not actually scroll"
    );
    receipt["stages"]["scroll"] = json!("passed");
}

fn roundtrip(case: &Case, state: &EditorState, receipt: &mut Value) {
    stage(receipt, "roundtrip");
    let path = case.output.join("engine-save-reload.op");
    assert!(
        !path.exists(),
        "never overwrite an existing verification copy"
    );
    let meta = op_pen_loader::EditorMeta::from_state(state);
    crate::doc_io::save_to_path(state, &path).expect("production save to new verification copy");
    let reopened = crate::doc_io::load_editor_state_with_report(&path, op_i18n::Locale::EnUs)
        .expect("production reload of verification copy");
    let doc_equal = reopened.state.doc == state.doc;
    let meta_equal = op_pen_loader::EditorMeta::from_state(&reopened.state) == meta;
    receipt["roundtrip"] = json!({"file":path,"sha256":sha(&std::fs::read(&path).unwrap()),
        "canonical_document_equal":doc_equal,"editor_meta_equal":meta_equal,
        "scope":"production doc_io API; not OS Save/Open UI", "reload_report":format!("{:?}", reopened.report)});
    assert!(
        doc_equal && meta_equal,
        "save/reload changed canonical document or editor metadata"
    );
    let mut reopened_session = enter(&reopened.state);
    reopened_session.begin_lifecycle(0);
    settle(&mut reopened_session, 0);
    let expected = if case.flow() {
        &case.start
    } else {
        &case.target
    };
    let raw = serde_json::to_value(&reopened.state.doc).unwrap();
    receipt["roundtrip"]["preview_setup"] = setup_root(&mut reopened_session, &raw, expected, 1500);
    assert_mounted(&reopened_session, expected);
    capture(
        &reopened_session,
        &case.output,
        "04-save-reload.png",
        receipt,
    );
    receipt["stages"]["roundtrip"] = json!("passed");
}

fn run(case: &Case, source: &[u8], receipt: &mut Value) {
    stage(receipt, "load");
    assert!(
        matches!(case.mode.as_str(), "all" | "flow" | "scroll"),
        "MODE must be all, flow, or scroll"
    );
    let raw: Value = serde_json::from_slice(source).expect("saved JSON document");
    let loaded = crate::doc_io::load_editor_state_with_report(&case.input, op_i18n::Locale::EnUs)
        .expect("production document load");
    receipt["load_report"] = json!(format!("{:?}", loaded.report));
    assert!(
        !loaded.report.rewrite_blocked_by_schema_warning,
        "current build cannot safely validate a malformed or newer schema"
    );
    receipt["editor_meta"] = json!(op_pen_loader::EditorMeta::from_state(&loaded.state));
    let original_doc = loaded.state.doc.clone();
    receipt["stages"]["load"] = json!("passed");
    stage(receipt, "preview_setup");
    let mut session = enter(&loaded.state);
    session.begin_lifecycle(0);
    settle(&mut session, 0);
    let initial_root = if case.flow() {
        &case.start
    } else {
        &case.target
    };
    receipt["preview_setup"] = setup_root(&mut session, &raw, initial_root, 1500);
    assert_mounted(&session, initial_root);
    capture(&session, &case.output, "01-start.png", receipt);
    receipt["stages"]["preview_setup"] = json!("passed");
    if case.flow() {
        flow(case, &raw, &mut session, receipt);
    }
    assert_mounted(&session, &case.target);
    capture(&session, &case.output, "02-target-first.png", receipt);
    if case.scroll() {
        scroll(case, &raw, &mut session, receipt);
    }
    assert_eq!(
        loaded.state.doc, original_doc,
        "Preview must not mutate author data"
    );
    if case.roundtrip {
        roundtrip(case, &loaded.state, receipt);
    }
}

#[test]
#[ignore = "requires OPENPENCIL_QA_DETAIL_FLOW_*; real saved artifact, engine only"]
fn retained_mobile_detail_navigation_scroll_and_roundtrip() {
    let case = Case::from_env();
    std::fs::create_dir(&case.output)
        .expect("OUTPUT must be a new directory under an existing parent");
    let mut receipt = json!({
        "status":"running","scope":"headless native Preview engine regression; no OS input",
        "mode":case.mode,"input":case.input,
        "selectors":{"start":case.start,"tap":case.tap,"target":case.target,"last":case.last,
            "footer":case.footer,"scroll":case.scroll},
        "visual_quality":"not-evaluated","business_effects":"not-tested; host capabilities disabled",
        "fonts":"existing test-process/platform fonts; no bundled-catalog or user-font-store initialization; not desktop typography parity",
        "capture_scope":"native raster of live scene; device-frame pin geometry checked separately",
        "settle_frames":{"offsets_ms":[0,500,1000],"observe_scene_each_tick":true},
        "stages":{"config":"pending","load":"pending","preview_setup":"pending",
            "flow":if case.flow(){"pending"}else{"not-requested"},
            "scroll":if case.scroll(){"pending"}else{"not-requested"},
            "roundtrip":if case.roundtrip{"pending"}else{"not-requested"}},
        "flow":{},"captures":[]
    });
    let mut source = None;
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        stage(&mut receipt, "config");
        assert!(!case.input.as_os_str().is_empty(), "DOCUMENT is required");
        assert!(
            !case.start.is_empty() && !case.target.is_empty(),
            "START and TARGET are required"
        );
        assert!(
            matches!(case.mode.as_str(), "all" | "flow" | "scroll"),
            "MODE must be all, flow, or scroll"
        );
        assert!(
            !case.flow() || case.tap.is_some(),
            "TAP is required for flow"
        );
        assert!(
            !case.scroll() || case.last.is_some(),
            "LAST is required for scroll"
        );
        receipt["stages"]["config"] = json!("passed");
        stage(&mut receipt, "load");
        let bytes = std::fs::read(&case.input).expect("input document bytes");
        receipt["input_sha256"] = json!(sha(&bytes));
        source = Some(bytes);
        run(&case, source.as_ref().unwrap(), &mut receipt)
    }));
    let current = std::fs::read(&case.input);
    let unchanged = source
        .as_ref()
        .map(|original| current.as_ref().is_ok_and(|bytes| bytes == original));
    receipt["source_unchanged"] = json!(unchanged);
    receipt["input_sha256_after"] = json!(current.as_ref().ok().map(|bytes| sha(bytes)));
    if let Err(error) = &outcome {
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic");
        receipt["failure"] = json!(message);
        let current_stage = receipt["current_stage"]
            .as_str()
            .unwrap_or("unknown")
            .to_owned();
        receipt["stages"][current_stage] = json!("failed");
    }
    receipt["status"] = json!(if outcome.is_ok() && unchanged == Some(true) {
        "passed-requested-engine-checks"
    } else {
        "failed"
    });
    std::fs::write(
        case.output.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    assert_ne!(
        unchanged,
        Some(false),
        "original artwork bytes changed during regression"
    );
    if let Err(error) = outcome {
        std::panic::resume_unwind(error);
    }
}

#[path = "retained_mobile_initial_state_tests.rs"]
mod initial_state;
