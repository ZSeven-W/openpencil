use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::container::CornerRadius;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

#[test]
fn cleanup_strips_mobile_search_categories_section_shell() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFF8F0" }],
            "children": [
                {
                    "type": "frame",
                    "id": "search-categories-shell",
                    "name": "Search And Categories",
                    "role": "section",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "padding": [16, 24],
                    "gap": 12,
                    "cornerRadius": 28,
                    "fill": [{ "type": "solid", "color": "#D7EBFF" }],
                    "stroke": {
                        "thickness": 1,
                        "fill": [{ "type": "solid", "color": "#C7DDF5" }]
                    },
                    "effects": [{
                        "type": "shadow",
                        "offsetX": 0,
                        "offsetY": 10,
                        "blur": 22,
                        "spread": 0,
                        "color": "#0000001F"
                    }],
                    "children": [
                        {
                            "type": "frame",
                            "id": "search-bar",
                            "name": "Search Bar",
                            "role": "search-bar",
                            "width": "fill_container",
                            "height": 50,
                            "cornerRadius": 16,
                            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                            "stroke": {
                                "thickness": 1,
                                "fill": [{ "type": "solid", "color": "#E8D4C4" }]
                            },
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "category-row",
                            "name": "Category Chips",
                            "width": "fill_container",
                            "height": 78,
                            "children": []
                        }
                    ]
                },
                {
                    "type": "frame",
                    "id": "promo-card",
                    "name": "Limited Offer Promo Card",
                    "role": "card",
                    "width": "fill_container",
                    "height": 168,
                    "cornerRadius": 24,
                    "fill": [{ "type": "solid", "color": "#FF6B00" }],
                    "children": []
                }
            ]
        }"##,
    )
    .expect("mobile shell json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == "root")
        .expect("root survives");
    let shell = find_node(root, "search-categories-shell").expect("shell survives");
    let promo = find_node(root, "promo-card").expect("promo survives");
    match shell {
        PenNode::Frame(frame) => {
            assert!(
                frame.container.fill.is_none(),
                "structural shell fill is removed"
            );
            assert!(
                frame.container.stroke.is_none(),
                "structural shell stroke is removed"
            );
            assert!(
                frame.container.effects.is_none(),
                "structural shell shadow is removed"
            );
            assert_eq!(
                frame.container.corner_radius,
                Some(CornerRadius::Uniform(0.0)),
                "structural shell radius is neutralized"
            );
        }
        _ => panic!("shell should be a frame"),
    }
    assert_eq!(
        first_solid_fill_hex(promo),
        Some("#FF6B00"),
        "real promo/card surfaces should keep their visual treatment"
    );
}

#[test]
fn cleanup_strips_misrolled_mobile_search_bar_wrapper() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFF8F0" }],
            "children": [
                {
                    "type": "frame",
                    "id": "outer-search",
                    "name": "Search Bar",
                    "role": "search-bar",
                    "width": "fill_container",
                    "height": 84,
                    "layout": "horizontal",
                    "padding": [10, 16],
                    "cornerRadius": 28,
                    "fill": [{ "type": "solid", "color": "#D7EBFF" }],
                    "stroke": {
                        "thickness": 1,
                        "fill": [{ "type": "solid", "color": "#D0E2F4" }]
                    },
                    "children": [
                        {
                            "type": "frame",
                            "id": "real-input",
                            "name": "Search Input",
                            "role": "form-input",
                            "width": "fill_container",
                            "height": 48,
                            "cornerRadius": 14,
                            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                            "stroke": {
                                "thickness": 1,
                                "fill": [{ "type": "solid", "color": "#E2E8F0" }]
                            },
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "filter-button",
                            "name": "Filter Button",
                            "role": "icon-button",
                            "width": 48,
                            "height": 48,
                            "cornerRadius": 14,
                            "fill": [{ "type": "solid", "color": "#FF6B00" }],
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    )
    .expect("mobile search json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == "root")
        .expect("root survives");
    let outer = find_node(root, "outer-search").expect("outer search survives");
    let input = find_node(root, "real-input").expect("input survives");
    let filter = find_node(root, "filter-button").expect("filter survives");
    match outer {
        PenNode::Frame(frame) => {
            assert!(
                frame.container.fill.is_none(),
                "outer search fill is removed"
            );
            assert!(
                frame.container.stroke.is_none(),
                "outer search stroke is removed"
            );
            assert_eq!(
                frame.container.corner_radius,
                Some(CornerRadius::Uniform(0.0)),
                "outer search radius is neutralized"
            );
        }
        _ => panic!("outer search should be a frame"),
    }
    assert_eq!(first_solid_fill_hex(input), Some("#FFFFFF"));
    assert_eq!(first_solid_fill_hex(filter), Some("#FF6B00"));
}
