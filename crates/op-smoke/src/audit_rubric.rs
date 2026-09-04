//! Rubric metrics for the `OPENPENCIL_SMOKE_AUDIT` gate — the dimensions the
//! 07-04 G3 A/B missed. Geometry issues alone declared the loop the winner
//! while its real output shipped without mobile chrome and with a degraded
//! node vocabulary; these deterministic metrics make "chrome completeness",
//! "vocabulary richness", and "content density" first-class columns so the
//! next loop-vs-orchestrator comparison weighs what a viewer actually sees.
//!
//! M3 addition (interactive-preview plan, Track A follow-up): `screenCount` /
//! `hasEntryScreen` / `navBoundTabs` / `navTotalTabs` / `popBound` /
//! `appModeReady` quantify whether a generated document actually enters
//! PreviewSession's routed multi-screen App Mode — `wire_screen_navigation`
//! (Track A) now auto-wires `screen` markers + nav-tab bindings, while the
//! cleanup-only interaction backfill persists fact-proven back/card actions
//! at the end of every generation turn. This is the deterministic check that
//! the bindings actually landed, not just that the passes exist. The
//! nav-container / has-events predicates are reused directly from
//! `op_orchestrator::wire_screen_navigation` (bumped `pub` for this;
//! op-smoke already depends on `op-orchestrator`) rather than reimplemented,
//! so the rubric can never silently disagree with what the nav pass wired.
//!
//! M3 addition (content-completeness follow-up): `completeness` closes a
//! blind spot the geometry + chrome + interactivity metrics above all share
//! — they only ever look at what LANDED in the document, never at what was
//! PLANNED. A dashboard that permanently lost its revenue-chart and
//! activity-table subtasks (all 3 retry-ladder attempts exhausted, see
//! `concurrent::run_subtask_retry_ladder`) still audits `structurallyClean:
//! true` / `issueCount: 0` — geometry has nothing to complain about in the
//! content that's actually there. `RunSummary::subtasks` (one
//! `SubtaskOutcome` per planned subtask, win or lose) is the one place that
//! still remembers the subtasks that never delivered, so this section is
//! keyed off of it rather than the document.

use std::collections::BTreeMap;

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, PenNodeExt};
use op_orchestrator::wire_screen_navigation::{collect_nav_containers, subtree_has_events};
use op_orchestrator::RunSummary;
use serde_json::{json, Value};

/// Mobile-screen width band (390 reference ± the 320-480 devices we seed).
const MOBILE_WIDTH_RANGE: std::ops::RangeInclusive<f64> = 320.0..=480.0;

/// Build the rubric JSON for an audited document. `run_summary` is `Some`
/// only when this audit is running right after a generation this same
/// process drove (it has a live `RunSummary` to report against); a
/// pure-render audit that just loaded an existing `.op` off disk
/// (`OPENPENCIL_SMOKE_AUDIT`'s standalone path) never had an orchestrator
/// run to begin with, so it passes `None` and the `completeness` section is
/// omitted entirely — reporting a fabricated 0/0 would be worse than saying
/// nothing.
pub fn rubric_report(state: &EditorState, run_summary: Option<&RunSummary>) -> Value {
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut text_nodes = 0usize;
    let mut icon_nodes = 0usize;
    let mut image_fills = 0usize;
    let mut screens: Vec<Value> = Vec::new();

    for root in state.active_children() {
        if !matches!(root, PenNode::Frame(_)) || root.children().is_none_or(|c| c.is_empty()) {
            continue;
        }
        collect_counts(
            root,
            &mut kinds,
            &mut text_nodes,
            &mut icon_nodes,
            &mut image_fills,
        );
        let mobile = root
            .width_px()
            .is_some_and(|w| MOBILE_WIDTH_RANGE.contains(&w));
        let has_status_bar = subtree_any(root, &is_status_bar);
        let has_bottom_nav = subtree_any(root, &is_bottom_nav);
        screens.push(json!({
            "name": root.base().name.as_deref().unwrap_or("?"),
            "width": root.width_px(),
            "mobile": mobile,
            "hasStatusBar": has_status_bar,
            "hasBottomNav": has_bottom_nav,
            // Chrome completeness is only judged where chrome is expected.
            "chromeComplete": if mobile {
                Value::Bool(has_status_bar && has_bottom_nav)
            } else {
                Value::Null
            },
        }));
    }

    // Vocabulary richness = distinct node kinds beyond the frame+text floor a
    // degraded generation collapses to (07-05 regression shipped 0/0/0
    // path/rectangle/text_input).
    let richness = kinds
        .keys()
        .filter(|k| !matches!(k.as_str(), "frame" | "text"))
        .count();

    let mut report = json!({
        "screens": screens,
        "nodeKinds": kinds,
        "vocabularyRichness": richness,
        "textNodes": text_nodes,
        "iconNodes": icon_nodes,
        "imageFills": image_fills,
        "interactivity": interactivity_report(state.active_children()),
    });
    if let Some(summary) = run_summary {
        if let Value::Object(map) = &mut report {
            map.insert("completeness".to_string(), completeness_report(summary));
        }
    }
    report
}

/// Whether every PLANNED subtask actually delivered content. Delivered ⟺
/// `node_count > 0` — provably equivalent to `error.is_none()` at both
/// `SubtaskOutcome` construction sites (`subagent.rs`'s `fail()` closure
/// always pairs `node_count: 0` with `error: Some(..)`; the success tail
/// always pairs `error: None` with the real count), so either predicate
/// agrees. `node_count > 0` is used directly since "content actually
/// landed" is the more literal signal and doesn't depend on every future
/// caller keeping the error field wired correctly.
fn completeness_report(summary: &RunSummary) -> Value {
    let planned = summary.subtasks.len();
    let delivered = summary.subtasks.iter().filter(|s| s.node_count > 0).count();
    let permanent_failures: Vec<&str> = summary
        .subtasks
        .iter()
        .filter(|s| s.node_count == 0)
        .map(|s| s.id.as_str())
        .collect();
    json!({
        "plannedSubtasks": planned,
        "deliveredSubtasks": delivered,
        "permanentFailures": permanent_failures,
        "complete": permanent_failures.is_empty(),
    })
}

/// App-Mode readiness metrics over the whole active page (not just the
/// mobile-shaped screens the chrome-completeness loop above filters to —
/// `screen` markers and nav wiring are equally meaningful on a desktop-shaped
/// top-level frame).
fn interactivity_report(nodes: &[PenNode]) -> Value {
    let mut screen_count = 0usize;
    let mut has_entry_screen = false;
    let mut nav_bound_tabs = 0usize;
    let mut nav_total_tabs = 0usize;
    let mut pop_bound = 0usize;

    for node in nodes {
        // `screen` only ever marks a top-level frame (Track A contract point
        // 3), so this check stays at THIS level — never recursed.
        if let PenNode::Frame(frame) = node {
            if let Some(path) = frame.screen.as_deref() {
                screen_count += 1;
                if path == "/" {
                    has_entry_screen = true;
                }
            }
        }
        // Nav containers and back buttons, unlike `screen`, can sit anywhere
        // inside a screen's subtree, so these two walk the whole subtree.
        let mut nav_containers = Vec::new();
        collect_nav_containers(node, &mut nav_containers);
        for nav in nav_containers {
            for item in nav.children().into_iter().flatten() {
                nav_total_tabs += 1;
                if subtree_has_events(item) {
                    nav_bound_tabs += 1;
                }
            }
        }
        pop_bound += count_pop_bound(node);
    }

    let app_mode_ready = screen_count >= 2 && has_entry_screen && nav_bound_tabs > 0;

    json!({
        "screenCount": screen_count,
        "hasEntryScreen": has_entry_screen,
        "navBoundTabs": nav_bound_tabs,
        "navTotalTabs": nav_total_tabs,
        "popBound": pop_bound,
        "appModeReady": app_mode_ready,
    })
}

/// Count nodes with a bound `pop` action anywhere in `node`'s subtree.
/// Serializes ONCE for the whole subtree (`serde_json::to_value` already
/// recurses into `children`), then walks the resulting `Value` tree instead
/// of re-serializing at every node — the same subtree would otherwise be
/// serialized once per node it contains (O(n²) on a per-node JSON check).
fn count_pop_bound(node: &PenNode) -> usize {
    let Ok(value) = serde_json::to_value(node) else {
        return 0;
    };
    count_pop_bound_value(&value)
}

fn count_pop_bound_value(value: &Value) -> usize {
    let mut count = usize::from(has_pop_action(value));
    if let Some(children) = value.get("children").and_then(Value::as_array) {
        for child in children {
            count += count_pop_bound_value(child);
        }
    }
    count
}

/// Whether a node's (already-serialized) `events.onTap` includes a bound
/// `pop` navigation action. The cleanup interaction backfill only ever writes
/// `{"pop": null}`; checked generically (rather than repeating its strict
/// geometry/icon-data predicate) so a hand-authored pop binding on any node
/// counts too — this metric is "did a pop binding land", not "did the strict
/// back-control fact match".
fn has_pop_action(value: &Value) -> bool {
    value
        .get("events")
        .and_then(|e| e.get("onTap"))
        .and_then(Value::as_array)
        .is_some_and(|actions| actions.iter().any(|a| a.get("pop").is_some()))
}

fn collect_counts(
    node: &PenNode,
    kinds: &mut BTreeMap<String, usize>,
    text_nodes: &mut usize,
    icon_nodes: &mut usize,
    image_fills: &mut usize,
) {
    let kind = node_kind(node);
    *kinds.entry(kind.to_string()).or_insert(0) += 1;
    match kind {
        "text" => *text_nodes += 1,
        "icon_font" => *icon_nodes += 1,
        _ => {}
    }
    if has_image_fill(node) {
        *image_fills += 1;
    }
    for child in node.children().into_iter().flatten() {
        collect_counts(child, kinds, text_nodes, icon_nodes, image_fills);
    }
}

fn node_kind(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
        // Widget-kind additions (text_area / select / switch / …) — count
        // them under one bucket; the rubric's vocabulary signal only needs
        // "beyond frame+text", not a per-widget census.
        _ => "widget",
    }
}

/// An `image` node, an `imagePrompt`-tagged frame, or a fill carrying an
/// image source all count as one populated image slot.
fn has_image_fill(node: &PenNode) -> bool {
    if matches!(node, PenNode::Image(_)) {
        return true;
    }
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    if value
        .get("imagePrompt")
        .and_then(Value::as_str)
        .is_some_and(|p| !p.is_empty())
    {
        return true;
    }
    value
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| {
            fills.iter().any(|f| {
                f.get("type").and_then(Value::as_str) == Some("image")
                    || f.get("src")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
            })
        })
}

fn subtree_any(node: &PenNode, pred: &dyn Fn(&PenNode) -> bool) -> bool {
    if pred(node) {
        return true;
    }
    node.children()
        .into_iter()
        .flatten()
        .any(|child| subtree_any(child, pred))
}

fn is_status_bar(node: &PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        || node
            .base()
            .name
            .as_deref()
            .is_some_and(|n| n.to_ascii_lowercase().contains("status bar"))
}

fn is_bottom_nav(node: &PenNode) -> bool {
    if node.base().role.as_deref() == Some("bottom-tab-bar") {
        return true;
    }
    node.base().name.as_deref().is_some_and(|n| {
        let n = n.to_ascii_lowercase();
        n.contains("tab bar") || n.contains("tab-bar") || n.contains("bottom nav")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_from(nodes: serde_json::Value) -> EditorState {
        let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
            "version": "1.0",
            "children": nodes,
        }))
        .expect("doc");
        EditorState::from_document(doc)
    }

    #[test]
    fn mobile_screen_with_full_chrome_scores_complete() {
        let state = state_from(serde_json::json!([{
            "type": "frame", "id": "r", "name": "Home", "width": 390, "height": 844,
            "children": [
                { "type": "frame", "id": "sb", "name": "Status Bar", "role": "status-bar",
                  "width": "fill_container", "height": 62 },
                { "type": "icon_font", "id": "i", "name": "home", "iconFontName": "home",
                  "width": 20, "height": 20 },
                { "type": "frame", "id": "nav", "name": "bottom-tab-bar", "role": "bottom-tab-bar",
                  "width": "fill_container", "height": 72 }
            ]
        }]));
        let rubric = rubric_report(&state, None);
        let screen = &rubric["screens"][0];
        assert_eq!(screen["mobile"], serde_json::json!(true));
        assert_eq!(
            screen["chromeComplete"],
            serde_json::json!(true),
            "{rubric}"
        );
        assert_eq!(rubric["iconNodes"], serde_json::json!(1));
        assert!(
            rubric["vocabularyRichness"].as_u64().unwrap() >= 1,
            "{rubric}"
        );
    }

    /// Two screens, both Track-A-wired: "home" is the entry (`screen: "/"`)
    /// with a fully-bound nav; "profile" carries a bound back button and a
    /// nav with ONE unbound tab (so `navBoundTabs` < `navTotalTabs`, proving
    /// the count isn't just "any binding present").
    fn app_mode_ready_doc() -> serde_json::Value {
        serde_json::json!([
            { "type": "frame", "id": "home", "name": "Home", "screen": "/",
              "width": 390, "height": 844, "children": [
                { "type": "frame", "id": "nav-home", "name": "Bottom Nav", "role": "bottom-tab-bar",
                  "width": "fill_container", "height": 72, "children": [
                    { "type": "frame", "id": "tab-home", "width": 80, "height": 40,
                      "events": { "onTap": [ { "replace": "\"/\"" } ] },
                      "children": [ { "type": "text", "id": "tab-home-lbl", "content": "Home" } ] },
                    { "type": "frame", "id": "tab-profile", "width": 80, "height": 40,
                      "events": { "onTap": [ { "replace": "\"/profile\"" } ] },
                      "children": [ { "type": "text", "id": "tab-profile-lbl", "content": "Profile" } ] }
                  ] }
              ] },
            { "type": "frame", "id": "profile", "name": "Profile", "screen": "/profile",
              "width": 390, "height": 844, "children": [
                { "type": "frame", "id": "back", "name": "back", "width": 24, "height": 24,
                  "events": { "onTap": [ { "pop": null } ] } },
                { "type": "frame", "id": "nav-profile", "name": "Bottom Nav", "role": "bottom-tab-bar",
                  "width": "fill_container", "height": 72, "children": [
                    { "type": "frame", "id": "tab-home-2", "width": 80, "height": 40,
                      "children": [ { "type": "text", "id": "tab-home-2-lbl", "content": "Home" } ] },
                    { "type": "frame", "id": "tab-profile-2", "width": 80, "height": 40,
                      "events": { "onTap": [ { "replace": "\"/profile\"" } ] },
                      "children": [ { "type": "text", "id": "tab-profile-2-lbl", "content": "Profile" } ] }
                  ] }
              ] }
        ])
    }

    #[test]
    fn fully_wired_multi_screen_doc_scores_app_mode_ready() {
        let state = state_from(app_mode_ready_doc());
        let rubric = rubric_report(&state, None);
        let interactivity = &rubric["interactivity"];
        assert_eq!(
            interactivity["screenCount"],
            serde_json::json!(2),
            "{rubric}"
        );
        assert_eq!(interactivity["hasEntryScreen"], serde_json::json!(true));
        assert_eq!(interactivity["navTotalTabs"], serde_json::json!(4));
        assert_eq!(
            interactivity["navBoundTabs"],
            serde_json::json!(3),
            "one tab (tab-home-2) is deliberately left unbound; {rubric}"
        );
        assert_eq!(interactivity["popBound"], serde_json::json!(1));
        assert_eq!(interactivity["appModeReady"], serde_json::json!(true));
    }

    #[test]
    fn single_screen_doc_is_never_app_mode_ready() {
        // One top-level frame, no `screen` marker, no nav — the classic
        // scrolling-page shape `wire_screen_navigation` itself never
        // touches (its own <2-screen gate). Every interactivity field must
        // read as "not wired", not merely `appModeReady: false`.
        let state = state_from(serde_json::json!([{
            "type": "frame", "id": "home", "name": "Home", "width": 390, "height": 844,
            "children": [
                { "type": "text", "id": "t", "name": "T", "content": "hi", "width": 100, "height": 20 }
            ]
        }]));
        let rubric = rubric_report(&state, None);
        let interactivity = &rubric["interactivity"];
        assert_eq!(
            interactivity["screenCount"],
            serde_json::json!(0),
            "{rubric}"
        );
        assert_eq!(interactivity["hasEntryScreen"], serde_json::json!(false));
        assert_eq!(interactivity["navBoundTabs"], serde_json::json!(0));
        assert_eq!(interactivity["popBound"], serde_json::json!(0));
        assert_eq!(interactivity["appModeReady"], serde_json::json!(false));
    }

    #[test]
    fn mobile_screen_missing_chrome_scores_incomplete_and_desktop_is_exempt() {
        let state = state_from(serde_json::json!([
            { "type": "frame", "id": "m", "name": "Bare Mobile", "width": 390, "height": 844,
              "children": [ { "type": "text", "id": "t", "name": "T", "content": "hi",
                              "width": 100, "height": 20 } ] },
            { "type": "frame", "id": "d", "name": "Dashboard", "width": 1440, "height": 900,
              "children": [ { "type": "text", "id": "t2", "name": "T2", "content": "hi",
                              "width": 100, "height": 20 } ] }
        ]));
        let rubric = rubric_report(&state, None);
        assert_eq!(
            rubric["screens"][0]["chromeComplete"],
            serde_json::json!(false)
        );
        assert_eq!(
            rubric["screens"][1]["chromeComplete"],
            serde_json::Value::Null
        );
    }

    fn subtask_ok(id: &str, node_count: usize) -> op_orchestrator::SubtaskOutcome {
        op_orchestrator::SubtaskOutcome {
            id: id.to_string(),
            node_count,
            error: None,
            inserted_root_ids: Vec::new(),
            headline: None,
            subtask: None,
        }
    }

    fn subtask_failed(id: &str) -> op_orchestrator::SubtaskOutcome {
        op_orchestrator::SubtaskOutcome {
            id: id.to_string(),
            node_count: 0,
            error: Some("all 3 retry-ladder attempts exhausted".to_string()),
            inserted_root_ids: Vec::new(),
            headline: None,
            subtask: None,
        }
    }

    fn run_summary(subtasks: Vec<op_orchestrator::SubtaskOutcome>) -> op_orchestrator::RunSummary {
        op_orchestrator::RunSummary {
            root_frame_id: "root".to_string(),
            total_nodes: subtasks.iter().map(|s| s.node_count).sum(),
            subtasks,
            unfilled_screens: Vec::new(),
        }
    }

    /// Completeness is independent of document content — this fixture is
    /// deliberately minimal, the tests below drive it entirely off the
    /// `RunSummary` argument.
    fn trivial_doc() -> serde_json::Value {
        serde_json::json!([{ "type": "frame", "id": "r", "name": "R",
                             "width": 390, "height": 844, "children": [] }])
    }

    #[test]
    fn completeness_all_delivered_scores_complete() {
        let state = state_from(trivial_doc());
        let summary = run_summary(vec![
            subtask_ok("header", 4),
            subtask_ok("hero", 6),
            subtask_ok("footer", 3),
        ]);
        let rubric = rubric_report(&state, Some(&summary));
        let completeness = &rubric["completeness"];
        assert_eq!(completeness["plannedSubtasks"], serde_json::json!(3));
        assert_eq!(completeness["deliveredSubtasks"], serde_json::json!(3));
        assert_eq!(completeness["permanentFailures"], serde_json::json!([]));
        assert_eq!(
            completeness["complete"],
            serde_json::json!(true),
            "{rubric}"
        );
    }

    #[test]
    fn completeness_names_permanent_failures_and_scores_incomplete() {
        let state = state_from(trivial_doc());
        let summary = run_summary(vec![
            subtask_ok("header", 4),
            subtask_failed("revenue-chart"),
            subtask_failed("activity-table"),
            subtask_ok("footer", 3),
        ]);
        let rubric = rubric_report(&state, Some(&summary));
        let completeness = &rubric["completeness"];
        assert_eq!(completeness["plannedSubtasks"], serde_json::json!(4));
        assert_eq!(completeness["deliveredSubtasks"], serde_json::json!(2));
        assert_eq!(
            completeness["permanentFailures"],
            serde_json::json!(["revenue-chart", "activity-table"]),
            "{rubric}"
        );
        assert_eq!(completeness["complete"], serde_json::json!(false));
    }

    #[test]
    fn completeness_section_is_absent_without_a_run_summary() {
        // A pure-render audit (`OPENPENCIL_SMOKE_AUDIT` loading an existing
        // `.op` off disk) never drove the orchestrator, so it has no
        // `RunSummary` — the section must be missing entirely, not a
        // fabricated `{"plannedSubtasks":0,"deliveredSubtasks":0,...}` that
        // would misreport a document nobody ever planned subtasks for.
        let state = state_from(trivial_doc());
        let rubric = rubric_report(&state, None);
        assert!(
            rubric.get("completeness").is_none(),
            "completeness must be absent, not null or zeroed: {rubric}"
        );
    }
}
