//! Opt-in acceptance replay of saved, unedited generated documents through Play.
//! Coordinates come from the live scene; this exercises hit testing + bindings.

use super::{test_measure, PreviewSession};
use op_editor_ui::layout_scene::{LayoutScene, SceneNode};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf};

fn scene_snapshot(scene: &LayoutScene) -> BTreeMap<String, Value> {
    fn collect(nodes: &[SceneNode], out: &mut BTreeMap<String, Value>) {
        for node in nodes {
            out.insert(node.id.clone(), json!({
                "text":node.text, "fill":format!("{:?}",node.fill),
                "opacity":node.opacity, "hidden":node.hidden, "rotation":node.rotation,
                "rect":[node.bounds.origin.x,node.bounds.origin.y,node.bounds.size.x,node.bounds.size.y]
            }));
            collect(&node.children, out);
        }
    }
    let mut out = BTreeMap::new();
    if let Some(page) = scene.active_page() {
        collect(&page.children, &mut out);
    }
    out
}

#[test]
#[ignore = "requires OPENPENCIL_PREVIEW_AUDIT_INPUT, OPENPENCIL_PREVIEW_TAP_IDS and OPENPENCIL_PREVIEW_AUDIT_OUT"]
fn saved_generated_controls_change_the_live_preview_scene() {
    let input = PathBuf::from(std::env::var("OPENPENCIL_PREVIEW_AUDIT_INPUT").unwrap());
    let output = PathBuf::from(std::env::var("OPENPENCIL_PREVIEW_AUDIT_OUT").unwrap());
    let ids: Vec<String> =
        serde_json::from_str(&std::env::var("OPENPENCIL_PREVIEW_TAP_IDS").unwrap()).unwrap();
    assert!(!ids.is_empty());
    let source = std::fs::read_to_string(&input).unwrap();
    let doc = jian_ops_schema::load_str(&source).unwrap().value;
    let mut results = Vec::new();
    for id in ids {
        // Reset each scenario so another button cannot satisfy this assertion.
        let mut session = PreviewSession::enter(
            &doc,
            (1440.0, 1000.0),
            &BTreeMap::new(),
            0,
            false,
            false,
            test_measure(),
            0,
        )
        .unwrap();
        let initial = scene_snapshot(&session.preview_scene_for_test());
        let before_state = session.debug_snapshot();
        let mut taps = Vec::new();
        for attempt in 0..2 {
            let scene = session.preview_scene_for_test();
            let before = scene_snapshot(&scene);
            let Some(node) = scene.active_page().and_then(|page| page.find(&id)) else {
                taps.push(json!({"attempt":attempt,"missing":true}));
                break;
            };
            let rect = node.bounds;
            let hit = session
                .deepest_mapped_hit(
                    rect.origin.x + rect.size.x / 2.0,
                    rect.origin.y + rect.size.y / 2.0,
                )
                .map(|(_, _, id)| id);
            session.set_now_ms(100 + attempt * 600);
            let consumed = session.dispatch_tap(
                rect.origin.x + rect.size.x / 2.0,
                rect.origin.y + rect.size.y / 2.0,
            );
            session.pump(600 + attempt * 600);
            let after = scene_snapshot(&session.preview_scene_for_test());
            let keys: std::collections::BTreeSet<_> = before.keys().chain(after.keys()).collect();
            let changes: Vec<_> = keys
                .into_iter()
                .filter(|key| before.get(*key) != after.get(*key))
                .map(|key| json!({"id":key,"before":before.get(key),"after":after.get(key)}))
                .collect();
            taps.push(json!({"attempt":attempt,"hit_node":hit,"consumed":consumed,"changes":changes,"trace":session.trace_entries()}));
        }
        results.push(
            json!({"node_id":id,"warnings":session.warnings(),"before_state":before_state,
            "after_state":session.debug_snapshot(),"initial_nodes":initial.len(),"taps":taps}),
        );
    }
    std::fs::write(&output,serde_json::to_string_pretty(&json!({"input":input,"scope":"OP Play runtime, scene-space taps; no native-window input", "results":results})).unwrap()).unwrap();
    assert_eq!(
        source,
        std::fs::read_to_string(&input).unwrap(),
        "audit must not edit the saved artwork"
    );
    for result in results {
        assert_eq!(result["taps"][0]["consumed"], true, "{result}");
        assert!(
            !result["taps"][0]["changes"].as_array().unwrap().is_empty(),
            "No visible scene change: {result}"
        );
    }
}
