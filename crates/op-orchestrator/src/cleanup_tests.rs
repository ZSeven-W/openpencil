use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use serde_json::json;

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
fn cleanup_does_not_shrink_fixed_mobile_root_to_partial_child_sum() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "x": 80,
        "y": 40,
        "width": 390,
        "height": 844,
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": "status", "name": "Status Bar", "width": "fill_container", "height": 32},
            {"type": "frame", "id": "section", "name": "Fit Content Section", "width": "fill_container", "height": "fit_content", "children": [
                {"type": "frame", "id": "card", "name": "Card", "width": "fill_container", "height": 120}
            ]}
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
            .all(|c| !matches!(c, EditorCommand::UpdateNode { .. })),
        "cleanup must not shrink a fixed mobile root from 844 to the visible status-bar-only sum"
    );
    let root = sink
        .state
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
        .expect("root survives cleanup");
    assert_eq!(root.height_px(), Some(844.0));
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

    // The bottom nav is still repaired → a downward (offsetY = -4) shadow exists.
    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetEffectParam { field: EffectField::OffsetY, value, .. }
                if (*value - -4.0).abs() < f32::EPSILON
        )),
        "bottom nav should still receive its upward-pointing shadow"
    );
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
fn cleanup_repairs_mobile_section_padding_and_overwide_children() {
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
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeLayoutProp {
                property,
                value: op_editor_core::LayoutPropValue::NumberArray(values),
                ..
            } if property == "padding" && values == &vec![0.0, 24.0, 0.0, 24.0]
        )),
        "mobile content sections need horizontal padding so headings/cards do not hug the screen edge"
    );
    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeLayoutProp {
                property,
                value: op_editor_core::LayoutPropValue::Keyword(value),
                ..
            } if property == "width" && value == "fill_container"
        )),
        "overwide children inside padded mobile sections should be converted to fill_container"
    );
}

#[test]
fn cleanup_clamps_mobile_absolute_overflow_inside_sections() {
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
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::UpdateNode {
                x: Some(x),
                ..
            } if *x <= 286
        )),
        "absolute-positioned mobile children that exceed section width should be clamped back inside"
    );
}

#[test]
fn cleanup_recolors_blank_gray_mobile_placeholders() {
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
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::SetNodeFillHex { hex, .. } if hex == "#FF6B00"
        )),
        "blank gray mobile food placeholders should be turned into colored icon/media tiles"
    );
}

#[test]
fn cleanup_squares_mobile_icon_and_placeholder_tiles() {
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
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::UpdateNode {
                width: Some(52),
                height: Some(52),
                ..
            }
        )),
        "icon-only filter buttons should be normalized back to square controls"
    );
    assert!(
        sink.applied.iter().any(|c| matches!(
            c,
            EditorCommand::UpdateNode {
                width: Some(82),
                height: Some(82),
                ..
            }
        )),
        "blank mobile media placeholders should be normalized back to square tiles"
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
                "fontSize": 30,
                "fontWeight": 800
            },
            {
                "type": "text",
                "id": "subtitle",
                "role": "body-text",
                "content": "Fresh Brooklyn favorites, delivered fast.",
                "fontSize": 16,
                "fontWeight": 800
            },
            {
                "type": "text",
                "id": "placeholder",
                "name": "Placeholder",
                "content": "Search restaurants or dishes",
                "fontSize": 17,
                "fontWeight": 800
            },
            {
                "type": "text",
                "id": "metadata",
                "role": "caption",
                "content": "20-30 min",
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
