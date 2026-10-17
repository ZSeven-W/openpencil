use super::*;
use crate::test_support::VecDocSink;
use serde_json::json;

fn screen_root(id: &str, x: Option<f64>, y: Option<f64>, width: f64, height: f64) -> PenNode {
    let mut root = json!({
        "type": "frame",
        "id": id,
        "name": id,
        "width": width,
        "height": height,
        "layout": "vertical",
        "children": [
            { "type": "frame", "id": format!("{id}-nav"), "name": "Tab Bar",
              "role": "bottom-tab-bar", "width": "fill_container", "height": 72,
              "layout": "horizontal", "children": [] }
        ]
    });
    if let Some(x) = x {
        root["x"] = json!(x);
    }
    if let Some(y) = y {
        root["y"] = json!(y);
    }
    serde_json::from_value(root).expect("valid screen root")
}

fn sink_with_roots(roots: Vec<PenNode>) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: roots,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

/// `InsertSubtree` remaps every node's id on insert, so tests resolve roots
/// by NAME (set equal to the pre-insert id by `screen_root`/`screen_with_nav`
/// below) rather than by the id that no longer exists post-insert — the
/// same idiom `cleanup_abandoned_duplicate_roots_tests.rs` uses.
fn rect_of(sink: &VecDocSink, name: &str) -> (f64, f64, f64, f64) {
    let root = sink
        .state
        .active_children()
        .iter()
        .find(|n| n.base().name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("{name} exists"));
    (
        root.base().x.unwrap_or(0.0),
        root.base().y.unwrap_or(0.0),
        root.width_px().unwrap(),
        root.height_px().unwrap(),
    )
}

fn overlaps(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

#[test]
fn two_unpositioned_overlapping_screens_are_spread() {
    let mut sink = sink_with_roots(vec![
        screen_root("a", None, None, 402.0, 864.0),
        screen_root("b", None, None, 402.0, 844.0),
    ]);

    let moved = spread_overlapping_screen_roots(&mut sink);

    assert_eq!(
        moved, 1,
        "the first root stays put, only the collider moves"
    );
    let a = rect_of(&sink, "a");
    let b = rect_of(&sink, "b");
    assert_eq!(
        (a.0, a.1),
        (0.0, 0.0),
        "the first root is left exactly alone"
    );
    assert!(!overlaps(a, b), "a={a:?} b={b:?}");
    assert_eq!(
        b.0,
        a.2 + SCREEN_ROOT_GAP,
        "b lands exactly one gap past a's right edge"
    );
}

/// De-identified reproduction of `0718-1-glm.op`: three cleanly-separated,
/// single-nav screens ("Trips Overview" 402×864 / "Destination Detail"
/// 402×844 / "Saved Places" 402×1155), all missing `x`/`y`, all resolving
/// to the same (0, 0) origin — a canvas screenshot of this reads as
/// duplicate bottom navs stacked inside one giant frame, but there is no
/// single frame to split; the three roots just need to stop overlapping.
#[test]
fn three_stacked_screens_reproduce_0718_1_glm_and_get_spread() {
    let mut sink = sink_with_roots(vec![
        screen_root("trips", None, None, 402.0, 864.0),
        screen_root("destination", None, None, 402.0, 844.0),
        screen_root("saved", None, None, 402.0, 1155.0),
    ]);

    let moved = spread_overlapping_screen_roots(&mut sink);

    assert_eq!(
        moved, 2,
        "trips stays anchored, destination and saved both collided"
    );
    let rects = [
        rect_of(&sink, "trips"),
        rect_of(&sink, "destination"),
        rect_of(&sink, "saved"),
    ];
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            assert!(
                !overlaps(rects[i], rects[j]),
                "roots {i} and {j} still overlap: {rects:?}"
            );
        }
    }
    // Left-to-right in document order, each exactly one gap past the
    // previous root's right edge.
    assert_eq!(rects[0].0, 0.0);
    assert_eq!(rects[1].0, rects[0].0 + rects[0].2 + SCREEN_ROOT_GAP);
    assert_eq!(rects[2].0, rects[1].0 + rects[1].2 + SCREEN_ROOT_GAP);
}

#[test]
fn single_screen_root_is_left_untouched() {
    // A long single-column scrolling page is a legitimate shape, not a bug
    // — nothing to compare it against, so nothing should move.
    let mut sink = sink_with_roots(vec![screen_root("only", None, None, 390.0, 3000.0)]);

    let moved = spread_overlapping_screen_roots(&mut sink);

    assert_eq!(moved, 0);
    assert_eq!(rect_of(&sink, "only"), (0.0, 0.0, 390.0, 3000.0));
}

#[test]
fn already_separated_authored_screens_are_left_untouched() {
    let mut sink = sink_with_roots(vec![
        screen_root("a", Some(0.0), Some(0.0), 390.0, 844.0),
        screen_root("b", Some(470.0), Some(0.0), 390.0, 844.0),
    ]);

    let moved = spread_overlapping_screen_roots(&mut sink);

    assert_eq!(
        moved, 0,
        "authored, non-overlapping screens must not be touched"
    );
    assert_eq!(rect_of(&sink, "a"), (0.0, 0.0, 390.0, 844.0));
    assert_eq!(rect_of(&sink, "b"), (470.0, 0.0, 390.0, 844.0));
}

#[test]
fn spread_is_idempotent() {
    let mut sink = sink_with_roots(vec![
        screen_root("trips", None, None, 402.0, 864.0),
        screen_root("destination", None, None, 402.0, 844.0),
        screen_root("saved", None, None, 402.0, 1155.0),
    ]);

    let first_pass = spread_overlapping_screen_roots(&mut sink);
    assert_eq!(first_pass, 2);
    let rects_after_first = [
        rect_of(&sink, "trips"),
        rect_of(&sink, "destination"),
        rect_of(&sink, "saved"),
    ];

    let second_pass = spread_overlapping_screen_roots(&mut sink);
    assert_eq!(
        second_pass, 0,
        "nothing left to fix — the doc is already clean"
    );
    let rects_after_second = [
        rect_of(&sink, "trips"),
        rect_of(&sink, "destination"),
        rect_of(&sink, "saved"),
    ];
    assert_eq!(
        rects_after_first, rects_after_second,
        "a second run must not move anything further"
    );
}

/// End-to-end: `run_cleanup_passes` chains
/// `spread_overlapping_screen_roots` → `unify_shared_nav` →
/// `wire_screen_navigation`. Two screens, both starting at the origin
/// (the actual failure mode) with a bottom nav each (deliberately DIFFERENT
/// tab labels on each, so a real unify has something to reconcile) —
/// after the full pipeline the roots must no longer overlap AND the
/// document must have actually entered App Mode (every screen tagged,
/// nav tabs bound), proving the spread doesn't just fix geometry but
/// leaves the rest of the chain able to do its job on the result.
#[test]
fn spread_chains_into_unify_and_wire_end_to_end() {
    fn screen_with_nav(id: &str, tabs: &[(&str, &str)]) -> PenNode {
        let tab_nodes: Vec<serde_json::Value> = tabs
            .iter()
            .map(|(icon, label)| {
                json!({
                    "type": "frame", "id": format!("{id}-tab-{label}"), "width": 80, "height": 40,
                    "layout": "vertical",
                    "children": [
                        { "type": "icon_font", "id": format!("{id}-tab-{label}-icon"),
                          "iconFontName": icon, "width": 20, "height": 20 },
                        { "type": "text", "id": format!("{id}-tab-{label}-lbl"), "content": label,
                          "width": "fit_content", "height": "fit_content" }
                    ]
                })
            })
            .collect();
        let root = json!({
            "type": "frame", "id": id, "name": id, "width": 390, "height": 844,
            "layout": "vertical",
            "children": [
                { "type": "frame", "id": format!("{id}-nav"), "name": "Tab Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72,
                  "layout": "horizontal", "children": tab_nodes }
            ]
        });
        serde_json::from_value(root).expect("valid screen root")
    }

    let mut sink = sink_with_roots(vec![
        screen_with_nav("home", &[("home", "Home"), ("bookmark", "Saved")]),
        screen_with_nav("saved", &[("home", "Home"), ("bookmark", "Saved")]),
    ]);

    let root_ids: Vec<String> = sink
        .state
        .active_children()
        .iter()
        .map(|n| n.id_str().to_string())
        .collect();
    let root_id_refs: Vec<&str> = root_ids.iter().map(String::as_str).collect();
    let plan = crate::plan::OrchestratorPlan {
        shared_context: None,
        root_frame: crate::plan::RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
        palette: None,
    };
    crate::cleanup::run_cleanup_passes(&mut sink, &plan, &root_id_refs);

    let roots = sink.state.active_children();
    assert_eq!(roots.len(), 2, "both screens survive cleanup: {roots:?}");
    let home = rect_of(&sink, "home");
    let saved = rect_of(&sink, "saved");
    assert!(
        !overlaps(home, saved),
        "spread must have separated them before unify/wire ran: home={home:?} saved={saved:?}"
    );

    let value = serde_json::to_value(roots).expect("serialize");
    let screens = value.as_array().expect("array");
    assert!(
        screens
            .iter()
            .all(|s| s.get("screen").and_then(|v| v.as_str()).is_some()),
        "wire_screen_navigation must have tagged every screen: {screens:?}"
    );
    let any_tab_bound = screens.iter().any(|s| {
        s.get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .any(|nav| {
                nav.get("children")
                    .and_then(|c| c.as_array())
                    .into_iter()
                    .flatten()
                    .any(|tab| tab.get("events").is_some())
            })
    });
    assert!(
        any_tab_bound,
        "at least one nav tab must be bound: {screens:?}"
    );
}
