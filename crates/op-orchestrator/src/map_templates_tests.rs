use super::*;
use crate::map_placeholder::{build_map_placeholder, map_placeholder};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use std::collections::{HashMap, HashSet};

fn slot(kind: MapTemplate, w: f64, h: f64) -> PenNode {
    serde_json::from_value(json!({"type":"image","id":"map","name":"Map","src":"",
        "width":w,"height":h,"imageSearchQuery":format!("map-template:{}",kind.key())}))
    .unwrap()
}

#[test]
fn map_template_selection_prefers_explicit_intent() {
    assert_eq!(
        MapTemplate::from_hint("running park map"),
        MapTemplate::Running
    );
    assert_eq!(
        MapTemplate::from_hint("外卖配送地图"),
        MapTemplate::Delivery
    );
    assert_eq!(
        MapTemplate::from_hint("store location map"),
        MapTemplate::Location
    );
    assert_eq!(
        MapTemplate::from_hint("running map-template:location"),
        MapTemplate::Location
    );
}

#[test]
fn map_templates_have_stable_bounds_endpoints_and_topmost_controls() {
    for kind in [
        MapTemplate::Running,
        MapTemplate::Delivery,
        MapTemplate::Location,
    ] {
        for theme in [Theme::Light, Theme::Dark] {
            for (w, h) in [(327.0, 180.0), (375.0, 260.0), (720.0, 340.0), (80.0, 60.0)] {
                let nodes = children("map", w, h, theme, kind);
                assert_eq!(nodes, children("map", w, h, theme, kind));
                for n in &nodes {
                    let (x, y, nw, nh) = (
                        n["x"].as_f64().unwrap(),
                        n["y"].as_f64().unwrap(),
                        n["width"].as_f64().unwrap(),
                        n["height"].as_f64().unwrap(),
                    );
                    assert!(
                        x >= 0.0 && y >= 0.0 && x + nw <= w + 0.01 && y + nh <= h + 0.01,
                        "out of bounds: {n}"
                    );
                }
                let route = nodes.iter().find(|n| n["name"] == "map-route");
                if let Some(route) = route {
                    let coords: Vec<f64> = route["d"]
                        .as_str()
                        .unwrap()
                        .split_whitespace()
                        .filter_map(|t| t.parse().ok())
                        .collect();
                    let pin = nodes.iter().find(|n| n["name"] == "map-pin").unwrap();
                    assert!(
                        (route["x"].as_f64().unwrap() + coords[coords.len() - 2]
                            - pin["x"].as_f64().unwrap()
                            - pin["width"].as_f64().unwrap() / 2.0)
                            .abs()
                            < 0.01
                    );
                    assert!(
                        (route["y"].as_f64().unwrap() + coords[coords.len() - 1]
                            - pin["y"].as_f64().unwrap()
                            - pin["height"].as_f64().unwrap() / 2.0)
                            .abs()
                            < 0.01
                    );
                    let xs: Vec<f64> = coords.iter().step_by(2).copied().collect();
                    let ys: Vec<f64> = coords.iter().skip(1).step_by(2).copied().collect();
                    assert!(xs.iter().copied().fold(f64::INFINITY, f64::min).abs() < 0.01);
                    assert!(ys.iter().copied().fold(f64::INFINITY, f64::min).abs() < 0.01);
                    assert!(
                        (xs.iter().copied().fold(0.0, f64::max) - route["width"].as_f64().unwrap())
                            .abs()
                            < 0.01
                    );
                    assert!(
                        (ys.iter().copied().fold(0.0, f64::max)
                            - route["height"].as_f64().unwrap())
                        .abs()
                            < 0.01
                    );
                } else {
                    assert_eq!(kind, MapTemplate::Location);
                }
                if w >= 160.0 && h >= 120.0 {
                    assert_eq!(nodes[0]["name"], "Map layer control");
                    assert_eq!(nodes[0]["width"], 40);
                    assert_eq!(nodes[0]["height"], 40);
                }
                assert_eq!(nodes.last().unwrap()["name"], "map-ground");
            }
        }
    }
}

#[test]
fn map_templates_save_reload_without_image_requests_or_repeat_rewrites() {
    for kind in [
        MapTemplate::Running,
        MapTemplate::Delivery,
        MapTemplate::Location,
    ] {
        let mut node = serde_json::to_value(slot(kind, 375.0, 260.0)).unwrap();
        let patch = build_map_placeholder(&slot(kind, 375.0, 260.0), 375.0, 260.0, Theme::Light);
        node.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let node: PenNode = serde_json::from_value(node).unwrap();
        let mut state = EditorState::new();
        state.apply(EditorCommand::InsertAuthoredSubtree {
            nodes: vec![node.clone()],
            parent_id: NodeId::NONE,
            page_id: None,
        });
        assert!(op_image_enrich::collect_targets(&state, &HashSet::new()).is_empty());
        assert!(map_placeholder(&node, &HashMap::new(), Theme::Light).is_empty());
        let saved = serde_json::to_string(&state.doc).unwrap();
        let _: op_editor_core::PenDocument = serde_json::from_str(&saved).unwrap();
    }
}

/// Optional native preview fixtures; output only when explicitly requested.
#[test]
fn map_templates_preview_fixtures() {
    let Ok(dir) = std::env::var("OPENPENCIL_MAP_PREVIEW_DIR") else {
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    for kind in [
        MapTemplate::Running,
        MapTemplate::Delivery,
        MapTemplate::Location,
    ] {
        for (theme, label) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
            for (w, h, size) in [(375.0, 260.0, "mobile"), (720.0, 340.0, "wide")] {
                let mut node = serde_json::to_value(slot(kind, w, h)).unwrap();
                node.as_object_mut().unwrap().extend(
                    build_map_placeholder(&slot(kind, w, h), w, h, theme)
                        .as_object()
                        .unwrap()
                        .clone(),
                );
                let doc = json!({"version":"1.0","children":[node],"variables":{
                    "--primary":{"type":"color","value":if theme==Theme::Dark {"#6EDBA6"} else {"#18966B"}},
                    "--muted":{"type":"color","value":"#F5F7F4"},"--secondary":{"type":"color","value":"#18232B"}}});
                std::fs::write(
                    std::path::Path::new(&dir).join(format!("{}-{label}-{size}.op", kind.key())),
                    serde_json::to_string_pretty(&doc).unwrap(),
                )
                .unwrap();
            }
        }
    }
}
