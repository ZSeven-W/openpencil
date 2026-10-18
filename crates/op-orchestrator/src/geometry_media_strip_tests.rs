//! A proven media lane must not disable ordinary overflow repairs nearby.

use super::*;
use serde_json::json;

fn viewport() -> Value {
    json!({"type":"frame","id":"view","layout":"none","width":375,
        "height":340,"clipContent":true,"children":[
        {"type":"frame","id":"lane","x":0,"y":0,"width":750,"height":340,
         "layout":"horizontal","gap":0,"clipContent":false,"children":[
            {"type":"image","id":"first","width":375,"height":340,"src":""},
            {"type":"image","id":"second","width":375,"height":340,"src":""}
        ]},
        {"type":"frame","id":"controls","x":16,"y":16,"width":400,"height":44,
         "layout":"horizontal","children":[
            {"type":"icon_font","id":"back","width":20,"height":20,
             "iconFontName":"chevron-left","iconFontFamily":"lucide"}
        ]}
    ]})
}

fn rects() -> HashMap<String, Rect> {
    HashMap::from([
        (
            "view".into(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 375.0,
                h: 340.0,
            },
        ),
        (
            "lane".into(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 750.0,
                h: 340.0,
            },
        ),
        (
            "controls".into(),
            Rect {
                x: 16.0,
                y: 16.0,
                w: 400.0,
                h: 44.0,
            },
        ),
    ])
}

#[test]
fn only_the_proven_lane_avoids_shrink_while_overlay_controls_are_repaired() {
    let mut commands = Vec::new();
    absolute_child_clamp::collect_absolute_child_shrink_fixes(&viewport(), &rects(), &mut commands);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert!(
        matches!(&commands[0], EditorCommand::UpdateNode { node_id, width:Some(343), .. }
        if node_id.as_str() == "controls")
    );
}

#[test]
fn ordinary_wide_rows_and_unproven_media_keep_the_original_shrink() {
    for mutate in ["unclipped", "short-page", "incomplete-lane", "thumbnail"] {
        let mut value = viewport();
        match mutate {
            "unclipped" => value["clipContent"] = json!(false),
            "short-page" => value["children"][0]["children"][0]["height"] = json!(200),
            "incomplete-lane" => value["children"][0]["width"] = json!(700),
            "thumbnail" => {
                value["children"][0]["width"] = json!(480);
                for image in value["children"][0]["children"].as_array_mut().unwrap() {
                    image["width"] = json!(240);
                }
            }
            _ => unreachable!(),
        }
        let mut commands = Vec::new();
        absolute_child_clamp::collect_absolute_child_shrink_fixes(&value, &rects(), &mut commands);
        assert!(
            commands.iter().any(|command| matches!(command,
            EditorCommand::UpdateNode { node_id, width:Some(375), .. }
                if node_id.as_str() == "lane")),
            "{mutate}: {commands:?}"
        );
    }
}

#[test]
fn unproven_oversized_image_slots_still_fit_their_declared_parent() {
    let value = json!({"type":"frame","id":"slot","width":100,"height":80,
    "layout":"horizontal","children":[
        {"type":"image","id":"oversized","src":"","width":400,"height":300}
    ]});
    let mut commands = Vec::new();
    collect_oversized_image_fixes(&value, &HashMap::new(), &mut commands);
    assert_eq!(commands.len(), 2);
    for property in ["width", "height"] {
        assert!(commands.iter().any(|command| matches!(command,
            EditorCommand::SetNodeLayoutProp { node_id, property: key,
                value: LayoutPropValue::Keyword(value) }
                if node_id.as_str() == "oversized" && key == property && value == "fill_container")));
    }
}
