use std::collections::HashSet;

use op_editor_core::{EditorState, NodeId};
use op_image_enrich::{apply_result, collect_targets};
use serde_json::json;

fn state(kind: &str, name: &str, explicit: bool) -> EditorState {
    let fill = if explicit {
        json!([{"type":"image","url":"","mode":"crop"}])
    } else {
        json!([{"type":"solid","color":"#0A0A0A"}])
    };
    EditorState::from_document(
        serde_json::from_value(json!({
            "version":"1.0", "children":[{
                "type":kind, "id":"cover", "name":name, "width":327, "height":150,
                "fill":fill,
                "animations":[{"trigger":"inView","durationMs":900,"keyframes":[
                    {"offset":0,"values":{"translateX":0}},
                    {"offset":1,"values":{"translateX":340}}
                ]}]
            }]
        }))
        .unwrap(),
    )
}

#[test]
fn automatic_search_and_stale_writeback_preserve_animation_geometry() {
    for kind in ["frame", "rectangle"] {
        for name in [
            "Reveal Cover",
            "WipeCover",
            "Hero Overlay",
            "Image Mask",
            "图片遮罩",
        ] {
            let mut state = state(kind, name, false);
            assert!(
                collect_targets(&state, &HashSet::new()).is_empty(),
                "{kind}: {name}"
            );
            assert!(!apply_result(
                &mut state,
                &NodeId::new("cover"),
                "https://example.com/photo.jpg"
            ));
        }
    }
}

#[test]
fn real_media_and_explicit_animated_image_slots_remain_eligible() {
    for kind in ["frame", "rectangle"] {
        for (name, explicit) in [("Album Cover", false), ("Reveal Cover", true)] {
            let state = state(kind, name, explicit);
            assert_eq!(
                collect_targets(&state, &HashSet::new()).len(),
                1,
                "{kind}: {name}"
            );
        }
    }
}
