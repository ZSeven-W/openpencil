//! Descendant counting, duplicate status bars, split-shell flipping,
//! decorative strokes and root-height preservation / growth.

use super::*;

#[test]
fn descendant_count_counts_nested() {
    let mut sink = VecDocSink::new();
    // root 套 child 套 grandchild
    let tree = frame_json(
        "root",
        json!([frame_json_value(
            "c",
            json!([frame_json_value("gc", json!([]))])
        )]),
    );
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    assert_eq!(descendant_count(&sink.state, &root_id), 2);
    assert_eq!(descendant_count(&sink.state, "missing"), 0);
}

#[test]
fn remove_duplicate_status_bars_keeps_one() {
    let mut sink = VecDocSink::new();
    // root 下有两个状态栏 + 一个普通区块
    let tree = frame_json(
        "root",
        json!([
            frame_json_value("status-bar-1", json!([])),
            frame_json_value("hero", json!([])),
            frame_json_value("status-bar-2", json!([])),
        ]),
    );
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();
    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);
    // 至少发了一条 DeleteNode(去掉多出来的状态栏)。
    assert!(sink
        .applied
        .iter()
        .any(|c| matches!(c, EditorCommand::DeleteNode { .. })));
}

#[test]
fn cleanup_flips_flat_split_shell_to_horizontal_row() {
    // minimax2 reproduction: the agentic loop's model already split the root into
    // [Sidebar, Main] but left it WITHOUT a horizontal layout, so the two columns
    // stack/overlap (render showed only the sidebar). The whole-root finalize
    // pipeline must flip it to a horizontal row.
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "root", "name": "Barbershop Dashboard",
        "children": [
            {"type":"frame","id":"sb","name":"Sidebar","layout":"vertical","height":"fill_container",
             "children":[{"type":"frame","id":"nav","name":"Nav","layout":"vertical","children":[]}]},
            {"type":"frame","id":"main","name":"Main","layout":"vertical",
             "children":[{"type":"frame","id":"stats","name":"Stats","layout":"horizontal","children":[]}]}
        ]
    }))
    .expect("valid split-shell fixture");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);
    // The current root (a fresh id if a transform swapped it) must be a row now.
    let root = &sink.state.active_children()[0];
    let v = serde_json::to_value(root).expect("serialize root");
    assert_eq!(
        v["layout"],
        json!("horizontal"),
        "flat [sidebar | main] split shell must become a horizontal row; got {:?}",
        v["layout"]
    );
}

#[test]
fn strip_decorative_filled_strokes_clears_only_shadowed_card_borders() {
    let mut sink = VecDocSink::new();
    let shadow = json!([{"type": "shadow", "offsetX": 0, "offsetY": 2, "blur": 8, "spread": 0, "color": "#0000001A"}]);
    let tree = serde_json::from_value::<PenNode>(json!({
        "type": "frame", "id": "root", "name": "root", "width": 375, "height": 600,
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "children": [
            // Filled + SHADOWED card with a border → redundant stroke, cleared.
            {"type": "frame", "id": "card", "name": "Card", "width": 100, "height": 80,
             "fill": [{"type": "solid", "color": "#FFFFFF"}],
             "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]},
             "effects": shadow,
             "children": []},
            // Filled card WITHOUT a shadow → border is the intentional boundary, KEPT.
            {"type": "frame", "id": "plain", "name": "PlainCard", "width": 100, "height": 80,
             "fill": [{"type": "solid", "color": "#FFFFFF"}],
             "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]},
             "children": []},
            // Unfilled divider rectangle → an intentional outline, KEPT.
            {"type": "rectangle", "id": "divider", "name": "Border", "width": 100, "height": 1,
             "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]}},
            // Input border → legitimate, KEPT (text_input is not a container).
            {"type": "text_input", "id": "search", "name": "Search", "width": 200, "height": 44,
             "fill": [{"type": "solid", "color": "#FFFFFF"}],
             "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]}}
        ]
    }))
    .expect("tree json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();
    strip_decorative_filled_strokes(&mut sink, &root_id);

    // InsertSubtree re-IDs nodes, so resolve the live ids by name.
    let card_id = find_node_id_by_name(&sink.state, "Card");
    let plain_id = find_node_id_by_name(&sink.state, "PlainCard");
    let divider_id = find_node_id_by_name(&sink.state, "Border");
    let search_id = find_node_id_by_name(&sink.state, "Search");
    let cleared: Vec<String> = sink
        .applied
        .iter()
        .filter_map(|c| match c {
            EditorCommand::SetNodeStrokeWidth { node_id, width } if *width == 0.0 => {
                Some(node_id.as_str().to_string())
            }
            _ => None,
        })
        .collect();
    assert!(cleared.contains(&card_id), "shadowed card border cleared");
    assert!(
        !cleared.contains(&plain_id),
        "border-only card KEPT (intentional)"
    );
    assert!(!cleared.contains(&divider_id), "unfilled divider kept");
    assert!(!cleared.contains(&search_id), "text_input border kept");
}

#[test]
fn run_cleanup_passes_callable_on_empty_root() {
    let mut sink = VecDocSink::new();
    let tree = frame_json("root", json!([]));
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();
    // 空 root —— 不 panic,不发命令。
    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);
    assert!(sink.applied.is_empty());
}

#[test]
fn cleanup_preserves_authored_fit_content_mobile_root() {
    // A fit-content mobile root is an explicit long-form/scroll-surface choice.
    // Even when its current content is shorter than a device viewport, cleanup
    // must not infer that the model meant a numeric artboard instead.
    let mut sink = VecDocSink::new();
    let mut children = vec![json!(
        {"type": "frame", "id": "status", "name": "Status Bar", "width": "fill_container", "height": 62}
    )];
    for i in 0..2 {
        children.push(json!({
            "type": "frame",
            "id": format!("section{i}"),
            "name": format!("Section {i}"),
            "width": "fill_container",
            "height": 170
        }));
    }
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "x": 80,
        "y": 40,
        "width": 375,
        "height": "fit_content",
        "layout": "vertical",
        "gap": 16,
        "children": children
    }))
    .expect("mobile root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .expect("root survives cleanup");
    assert_eq!(
        root.height_px(),
        None,
        "the authored fit_content sizing mode must survive cleanup"
    );
}

#[test]
fn cleanup_preserves_fixed_844_mobile_root_with_tall_content() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Result",
        "x": 80,
        "y": 40,
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "children": [
            {"type":"frame", "id":"status", "name":"Status Bar", "width":"fill_container", "height":62},
            {"type":"frame", "id":"viewport", "name":"Scroll Viewport", "role":"viewport",
             "width":"fill_container", "height":"fill_container", "layout":"vertical", "clipContent":true,
             "children":[{"type":"frame", "id":"long", "name":"Long Content", "width":"fill_container", "height":1000}]},
            {"type":"frame", "id":"nav", "name":"Bottom Tab Bar", "role":"bottom-tab-bar",
             "width":390, "height":72, "layout":"horizontal"}
        ]
    }))
    .expect("mobile root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .expect("root survives cleanup");
    assert_eq!(
        root.height_px(),
        Some(844.0),
        "an explicit clipped fill-height viewport must preserve its numeric root"
    );
}

#[test]
fn cleanup_keeps_an_overlay_root_at_its_tallest_layer_not_the_sum() {
    // `layout: none` compiles to a single-cell grid (jian 2026-07-28): the
    // children are stacked ON TOP of each other, so the root is exactly as
    // tall as its tallest layer. The content-height estimator used to sum
    // them like a vertical stack, and the root-height repair then inflated
    // every overlay root to N x its authored height.
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Layered Hero",
        "width": 390,
        "height": 844,
        "layout": "none",
        "children": [
            {"type":"rectangle", "id":"base", "name":"Base", "width":390, "height":844},
            {"type":"rectangle", "id":"noise", "name":"Noise", "width":390, "height":844},
            {"type":"rectangle", "id":"vignette", "name":"Vignette", "width":390, "height":844}
        ]
    }))
    .expect("overlay root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .expect("root survives cleanup");
    assert_eq!(
        root.height_px(),
        Some(844.0),
        "three stacked 844px overlay layers must not triple the root height"
    );
}

#[test]
fn cleanup_grows_390x844_poster_despite_mobile_geometry_and_status_bar() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Summer Festival Poster",
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "gap": 16,
        "children": [
            // A status bar can be inherited from an earlier width-only mobile
            // classification, so it is not sufficient proof of viewport intent.
            {"type":"frame", "id":"status", "name":"Status Bar", "width":"fill_container", "height":62},
            {"type":"frame", "id":"hero", "name":"Poster Hero", "width":"fill_container", "height":420},
            {"type":"frame", "id":"lineup", "name":"Lineup", "width":"fill_container", "height":420}
        ]
    }))
    .expect("poster root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Summer Festival Poster"))
        .expect("poster survives cleanup");
    assert!(
        root.height_px().is_some_and(|height| height > 844.0),
        "390x844 alone, even with an inherited status bar, must not freeze a poster root"
    );
}

#[test]
fn cleanup_grows_narrow_non_mobile_artboards_instead_of_freezing_them() {
    for name in ["Narrow Artboard", "Component Board"] {
        let mut sink = VecDocSink::new();
        let tree: PenNode = serde_json::from_value(json!({
            "type": "frame",
            "id": "root",
            "name": name,
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "gap": 20,
            "children": [
                {"type":"frame", "id":"states-a", "name":"State Set A", "width":"fill_container", "height":430},
                {"type":"frame", "id":"states-b", "name":"State Set B", "width":"fill_container", "height":430}
            ]
        }))
        .expect("narrow artboard json");
        sink.state.apply(EditorCommand::InsertSubtree {
            nodes: vec![tree],
            parent_id: NodeId::NONE,
            page_id: None,
        });
        let root_id = sink.state.active_children()[0].id_str().to_string();
        sink.applied.clear();

        run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

        let root = sink
            .state
            .active_children()
            .iter()
            .find(|node| node.base().name.as_deref() == Some(name))
            .expect("narrow artboard survives cleanup");
        assert!(
            root.height_px().is_some_and(|height| height > 844.0),
            "{name} is not a mobile viewport without explicit mobile semantics"
        );
    }
}

#[test]
fn cleanup_expands_zero_height_desktop_root_from_fit_content_children() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Dashboard",
        "width": 1200,
        "height": 0,
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        "children": [
            {
                "type": "frame",
                "id": "section",
                "name": "Fit Content Section",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 12,
                "children": [
                    {"type": "frame", "id": "header", "width": "fill_container", "height": 64},
                    {"type": "frame", "id": "chart", "width": "fill_container", "height": 240}
                ]
            }
        ]
    }))
    .expect("desktop root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .expect("root survives cleanup");
    assert_eq!(root.height_px(), Some(316.0));
}

#[test]
fn cleanup_reconciles_root_height_with_resolved_wrapped_text() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Long Landing Page",
        "width": 320,
        "height": 40,
        "layout": "vertical",
        "children": [{
            "type": "text",
            "id": "copy",
            "name": "Wrapped Copy",
            "content": "A deliberately long fixed-width sentence that wraps across several lines in the real layout engine.",
            "width": 88,
            "textGrowth": "fixed-width",
            "fontSize": 18,
            "lineHeight": 1.5
        }]
    }))
    .expect("wrapped text root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    let declared = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == root_id)
        .and_then(PenNodeExt::height_px)
        .expect("numeric root height");
    let resolved = crate::geometry_validation::resolved_node_height(&sink.state, &root_id)
        .expect("resolved root height");
    assert!(
        declared + 0.5 >= resolved,
        "declared {declared}px must contain resolved {resolved}px"
    );
}

fn overfull_desktop_artboard() -> (VecDocSink, String) {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Explicit Desktop Artboard",
        "width": 1440,
        "height": 900,
        "layout": "vertical",
        "gap": 24,
        "children": [
            {"type":"frame", "id":"upper", "name":"Upper Section",
             "width":"fill_container", "height":500, "children":[]},
            {"type":"frame", "id":"lower", "name":"Lower Section",
             "width":"fill_container", "height":500, "children":[]}
        ]
    }))
    .expect("desktop artboard json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();
    (sink, root_id)
}

#[test]
fn cleanup_policy_preserves_only_requested_fixed_root_height() {
    let (mut growing_sink, growing_root_id) = overfull_desktop_artboard();
    run_cleanup_passes(&mut growing_sink, &plan(), &[&growing_root_id]);
    let grown_height = growing_sink
        .state
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Explicit Desktop Artboard"))
        .and_then(PenNodeExt::height_px)
        .expect("grown root height");
    assert!(
        grown_height > 900.0,
        "default cleanup must keep growing ordinary overfull roots"
    );

    let (mut preserved_sink, preserved_root_id) = overfull_desktop_artboard();
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary_and_policy(
        &mut preserved_sink,
        &plan(),
        &[&preserved_root_id],
        &mut summary,
        CleanupPolicy {
            is_deck: false,
            preserve_requested_root_height: true,
            // These fixtures stand in for an orchestrator run, whose
            // root_ids are its own freshly inserted roots.
            roots_are_run_output: true,
        },
    );
    let preserved_height = preserved_sink
        .state
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Explicit Desktop Artboard"))
        .and_then(PenNodeExt::height_px)
        .expect("preserved root height");
    assert_eq!(
        preserved_height, 900.0,
        "request-derived cleanup policy must freeze the explicit 1440x900 root"
    );
}

#[test]
fn cleanup_recolors_safe_dark_bottom_nav_on_light_mobile_root() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Brooklyn Food Delivery",
        "x": 80,
        "y": 40,
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#FFF8F0" }],
        "children": [
            {
                "type": "frame",
                "id": "content",
                "name": "Content",
                "width": "fill_container",
                "height": 760,
                "children": []
            },
            {
                "type": "frame",
                "id": "bottom-nav",
                "name": "Bottom Navigation",
                "role": "bottom-tab-bar",
                "width": "fill_container",
                "height": 84,
                "fill": [{ "type": "solid", "color": "#0F0F0F" }],
                "children": []
            }
        ]
    }))
    .expect("mobile root json");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &[&root_id]);

    assert!(
        sink.applied
            .iter()
            .any(|c| matches!(c, EditorCommand::SetNodeFillHex { hex, .. } if hex == "#FFF8F0")),
        "cleanup should replace safe-dark bottom nav fill with the light mobile root surface"
    );
}

/// A deck board centres its content instead of stacking it from the top.
///
/// The board is a fixed 1080 tall while its sections hug their own height, so
/// the default leaves the lower half of every slide blank.
#[test]
fn a_deck_board_centres_its_content() {
    use op_editor_core::{EditorCommand, NodeId};

    let mut sink = VecDocSink::new();
    let tree: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "board",
        "name": "Cover",
        "width": 1920,
        "height": 1080,
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": "s1", "name": "Title", "width": "fill_container", "height": "fit_content"},
            {"type": "frame", "id": "s2", "name": "Body", "width": "fill_container", "height": "fit_content"},
            {"type": "frame", "id": "s3", "name": "Meta", "width": "fill_container", "height": "fit_content"}
        ]
    }))
    .expect("board");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();

    let mut summary = crate::repair_summary::RepairSummary::default();
    crate::cleanup::run_cleanup_passes_with_summary_and_policy_for_tests(
        &mut sink,
        &deck_plan(&root_id),
        &[&root_id],
        &mut summary,
        CleanupPolicy {
            is_deck: true,
            preserve_requested_root_height: true,
            // These fixtures stand in for an orchestrator run, whose
            // root_ids are its own freshly inserted roots.
            roots_are_run_output: true,
        },
    );

    let root = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize");
    assert_eq!(
        root["justifyContent"].as_str(),
        Some("center"),
        "a deck board must centre its content"
    );
    assert_eq!(
        root["height"].as_f64(),
        Some(1080.0),
        "centring must not disturb the pinned board height"
    );
}

/// An authored distribution is a composition, not the default top-stack.
#[test]
fn a_deck_board_with_an_explicit_distribution_is_left_alone() {
    use op_editor_core::{EditorCommand, NodeId};

    let mut sink = VecDocSink::new();
    let tree: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "board",
        "name": "Cover",
        "width": 1920,
        "height": 1080,
        "layout": "vertical",
        "justifyContent": "space_between",
        "children": [
            {"type": "frame", "id": "s1", "name": "Title", "width": "fill_container", "height": "fit_content"},
            {"type": "frame", "id": "s2", "name": "Meta", "width": "fill_container", "height": "fit_content"}
        ]
    }))
    .expect("board");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();

    let mut summary = crate::repair_summary::RepairSummary::default();
    crate::cleanup::run_cleanup_passes_with_summary_and_policy_for_tests(
        &mut sink,
        &deck_plan(&root_id),
        &[&root_id],
        &mut summary,
        CleanupPolicy {
            is_deck: true,
            preserve_requested_root_height: true,
            // These fixtures stand in for an orchestrator run, whose
            // root_ids are its own freshly inserted roots.
            roots_are_run_output: true,
        },
    );

    let root = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize");
    assert_eq!(root["justifyContent"].as_str(), Some("space_between"));
}

fn deck_plan(root_id: &str) -> crate::plan::OrchestratorPlan {
    crate::plan::OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: root_id.to_string(),
            name: "Deck".into(),
            width: 1920.0,
            height: 1080.0,
            layout: Some("vertical".into()),
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
    }
}
