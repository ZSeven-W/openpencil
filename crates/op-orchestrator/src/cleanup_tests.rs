use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use op_editor_core::EffectField;
use serde_json::json;

/// Walk the active-page tree by name and return the first matching node's id.
fn find_node_id_by_name(state: &EditorState, name: &str) -> String {
    fn walk(nodes: &[PenNode], name: &str) -> Option<String> {
        for n in nodes {
            if n.base().name.as_deref().unwrap_or("") == name {
                return Some(n.id_str().to_string());
            }
            if let Some(c) = n.children() {
                if let Some(hit) = walk(c, name) {
                    return Some(hit);
                }
            }
        }
        None
    }
    walk(state.active_children(), name).expect("named node exists")
}

/// 同 `frame_json` 但返回 `serde_json::Value`(供嵌套构造)。
fn frame_json_value(id: &str, children: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "frame", "id": id, "name": id,
        "x": 0, "y": 0, "width": 100, "height": 100,
        "children": children,
    })
}

fn frame_json(id: &str, children: serde_json::Value) -> PenNode {
    serde_json::from_value(frame_json_value(id, children)).expect("frame json")
}

#[test]
fn explicit_mobile_viewport_contract_accepts_all_container_variants() {
    for root_type in ["frame", "group", "rectangle"] {
        for viewport_type in ["frame", "group", "rectangle"] {
            let root: PenNode = serde_json::from_value(json!({
                "type": root_type,
                "id": format!("{root_type}-root"),
                "width": 390,
                "height": 844,
                "layout": "vertical",
                "children": [{
                    "type": viewport_type,
                    "id": format!("{viewport_type}-viewport"),
                    "role": "viewport",
                    "width": "fill_container",
                    "height": "fill_container",
                    "layout": "vertical",
                    "clipContent": true,
                    "children": [{
                        "type": "frame",
                        "id": "content",
                        "width": "fill_container",
                        "height": 1200
                    }]
                }]
            }))
            .expect("valid container contract");

            assert!(
                has_explicit_mobile_viewport_contract(&root),
                "{root_type} root with {viewport_type} viewport must preserve its height"
            );
        }
    }
}

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

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

#[test]
fn cleanup_injects_missing_bottom_nav_surface_on_light_mobile_root() {
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
        "cleanup should inject the root light surface on missing-fill bottom navs"
    );
    assert!(
        sink.applied
            .iter()
            .all(|c| !matches!(c, EditorCommand::AddNodeEffect { .. })),
        "bottom nav cleanup should not add a shadow band"
    );
}

#[test]
fn cleanup_leaves_top_navbar_transparent_on_light_mobile_root() {
    // The top header is transparent on mobile (TS references). A previous
    // version of `is_nav_surface` matched `role:"navbar"`, so this pass re-filled
    // the header with the root surface hex + a downward shadow — the "mysterious
    // background + rounded border" the user flagged. The bottom nav still gets a
    // surface; the top header must be left untouched.
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Brooklyn Food Delivery",
        "x": 80, "y": 40, "width": 390, "height": 844,
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#FFF8F0" }],
        "children": [
            {
                "type": "frame", "id": "header", "name": "Header", "role": "navbar",
                "width": "fill_container", "height": 56, "children": []
            },
            {
                "type": "frame", "id": "content", "name": "Content",
                "width": "fill_container", "height": 704, "children": []
            },
            {
                "type": "frame", "id": "bottom-nav", "name": "Bottom Navigation",
                "role": "bottom-tab-bar", "width": "fill_container", "height": 84, "children": []
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

    // TODO(reconcile w/ Kayshen e3ed2f1e "normalize mobile bottom tabs"): the
    // bottom-nav upward shadow (offsetY = -4) assertion is deferred — his cleanup
    // change to mobile bottom-tab handling supersedes our inject_nav_surface
    // shadow path. Re-enable once we align on whether the bottom nav keeps a
    // surface shadow under his normalization.
    // The top header is NOT repaired → no downward (offsetY = +4) header shadow.
    assert!(
        !sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetEffectParam { field: EffectField::OffsetY, value, .. }
                if (*value - 4.0).abs() < f32::EPSILON
        )),
        "top navbar must stay transparent — no surface shadow re-boxing it"
    );
}

#[test]
fn cleanup_recolors_white_bottom_nav_to_tinted_mobile_root_surface() {
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
                "fill": [{ "type": "solid", "color": "#FFFFFF" }],
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
        "cream mobile roots should not keep a pure-white bottom nav band"
    );
}

#[test]
fn cleanup_preserves_authored_mobile_section_padding_and_width() {
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
                "id": "popular-section",
                "name": "Popular Restaurants",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "children": [
                    {
                        "type": "frame",
                        "id": "restaurant-card",
                        "name": "Restaurant Card",
                        "width": 390,
                        "height": 120,
                        "children": []
                    }
                ]
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
        !sink.applied.iter().any(|command| matches!(
            command,
            EditorCommand::SetNodeLayoutProp { node_id, property, .. }
                if (node_id.as_str() == "popular-section" && property == "padding")
                    || (node_id.as_str() == "restaurant-card" && property == "width")
        )),
        "mobile padding and full-bleed width are design intent"
    );
}

#[test]
fn cleanup_preserves_authored_absolute_mobile_position() {
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
                "id": "promo-section",
                "name": "Promo Banner",
                "width": "fill_container",
                "height": 140,
                "layout": "none",
                "padding": [0, 24, 0, 24],
                "children": [
                    {
                        "type": "frame",
                        "id": "promo-icon-tile",
                        "name": "Promo Icon Tile",
                        "x": 330,
                        "y": 40,
                        "width": 56,
                        "height": 56,
                        "fill": [{ "type": "solid", "color": "#FF6B00" }],
                        "children": []
                    }
                ]
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
        !sink.applied.iter().any(|command| matches!(
            command,
            EditorCommand::UpdateNode { node_id, x: Some(_), .. }
                if node_id.as_str() == "promo-icon-tile"
        )),
        "an absolute decoration is not repositioned from a guessed content inset"
    );
}

#[test]
fn cleanup_preserves_blank_gray_mobile_surface_color() {
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
                "id": "restaurant-section",
                "name": "Restaurant Cards",
                "width": "fill_container",
                "height": 240,
                "padding": [0, 24, 0, 24],
                "children": [
                    {
                        "type": "rectangle",
                        "id": "tile",
                        "name": "Tile",
                        "width": 72,
                        "height": 72,
                        "fill": [{ "type": "solid", "color": "#E5E7EB" }]
                    }
                ]
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
        !sink.applied.iter().any(|command| matches!(
            command,
            EditorCommand::SetNodeFillHex { node_id, .. } if node_id.as_str() == "tile"
        )),
        "a neutral surface is not recolored from its dimensions"
    );
}

#[test]
fn cleanup_preserves_authored_mobile_tile_shapes() {
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
                "id": "content-section",
                "name": "Content",
                "width": "fill_container",
                "height": 260,
                "padding": [0, 24, 0, 24],
                "children": [
                    {
                        "type": "frame",
                        "id": "filter-button",
                        "name": "Filter Button",
                        "width": 52,
                        "height": 72,
                        "fill": [{ "type": "solid", "color": "#FF6B00" }],
                        "children": [
                            {
                                "type": "icon_font",
                                "id": "filter-icon",
                                "name": "Sliders",
                                "iconFontName": "sliders-horizontal",
                                "width": 24,
                                "height": 24
                            }
                        ]
                    },
                    {
                        "type": "rectangle",
                        "id": "restaurant-media",
                        "name": "Tile",
                        "width": 82,
                        "height": 112,
                        "fill": [{ "type": "solid", "color": "#E5E7EB" }]
                    }
                ]
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
        !sink.applied.iter().any(|command| matches!(
            command,
            EditorCommand::UpdateNode { node_id, width: Some(_), height: Some(_), .. }
                if matches!(node_id.as_str(), "filter-button" | "restaurant-media")
        )),
        "generic mobile geometry does not imply square controls or media"
    );
}

#[test]
fn cleanup_preserves_dark_bottom_nav_on_dark_mobile_root() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Dark Delivery App",
        "x": 80,
        "y": 40,
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": "#111827" }],
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
            .all(|c| !matches!(c, EditorCommand::SetNodeFillHex { .. })),
        "cleanup should not force a light nav surface when the root itself is dark"
    );
}

#[test]
fn run_cleanup_passes_repairs_overbold_text_hierarchy() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "width": 390,
        "height": 844,
        "children": [
            {
                "type": "text",
                "id": "title",
                "role": "heading",
                "content": "Popular Restaurants",
                "width": 320,
                "height": 40,
                "fontSize": 30,
                "fontWeight": 800
            },
            {
                "type": "text",
                "id": "subtitle",
                "role": "body-text",
                "content": "Fresh Brooklyn favorites, delivered fast.",
                "width": 320,
                "height": 22,
                "fontSize": 16,
                "fontWeight": 800
            },
            {
                "type": "text",
                "id": "placeholder",
                "name": "Placeholder",
                "content": "Search restaurants or dishes",
                "width": 280,
                "height": 24,
                "fontSize": 17,
                "fontWeight": 800
            },
            {
                "type": "text",
                "id": "metadata",
                "role": "caption",
                "content": "20-30 min",
                "width": 100,
                "height": 18,
                "fontSize": 14,
                "fontWeight": 800
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
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeFontWeight {
                font_weight: 400,
                ..
            }
        )),
        "cleanup should downgrade non-heading text when the whole screen was emitted as bold"
    );
}

#[test]
fn cleanup_finds_nested_appended_root() {
    let mut sink = VecDocSink::new();
    // `target` is a pre-existing container frame.  `appended` is a new section
    // nested under it — matching how append-mode subagent.rs inserts roots
    // under an existing target frame rather than at top level.
    let tree = frame_json(
        "target",
        json!([
            frame_json_value("old-section", json!([])),
            frame_json_value(
                "appended",
                json!([
                    frame_json_value("status-bar-1", json!([])),
                    frame_json_value("hero", json!([])),
                    frame_json_value("status-bar-2", json!([])),
                ]),
            ),
        ]),
    );
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    // Resolve the real (remapped) id of the nested `appended` frame by name.
    let appended_id = find_node_id_by_name(&sink.state, "appended");
    sink.applied.clear();
    run_cleanup_passes(&mut sink, &plan(), &[&appended_id]);
    // The dup-status-bar pass ran against the NESTED root → a DeleteNode fired.
    assert!(
        sink.applied
            .iter()
            .any(|c| matches!(c, EditorCommand::DeleteNode { .. })),
        "find_root must locate a nested appended root so cleanup passes fire"
    );
}
