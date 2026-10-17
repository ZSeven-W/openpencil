use super::*;
use serde_json::json;

fn stack() -> Value {
    json!({"type":"frame","layout":"none","children":[
        {"type":"frame","id":"disc","name":"Round background","width":48,"height":48,
            "cornerRadius":24,"fill":[{"type":"solid","color":"#FF5A1F"}]},
        {"type":"icon_font","id":"glyph","iconFontName":"shopping-cart","width":22,"height":22,
            "fill":[{"type":"solid","color":"#FFFFFF"}]}
    ]})
}

fn stub_ids(parent: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    collect_stub_ids(parent, None, &HashMap::new(), &mut ids);
    ids
}

#[test]
fn absolute_circle_icon_background_is_not_a_stub_in_repair_or_diagnostics() {
    for positioned_icon in [false, true] {
        let mut parent = stack();
        if positioned_icon {
            parent["children"][1]["x"] = json!(13);
            parent["children"][1]["y"] = json!(13);
        }
        assert!(stub_ids(&parent).is_empty());
        assert!(
            empty_decorated_stub_diagnostic(&parent["children"][0], Some(&parent), None).is_none()
        );
    }
}

#[test]
fn ordinary_empty_shells_and_unrelated_icons_still_get_removed_and_reported() {
    for case in [
        "flow",
        "omitted layout",
        "no icon",
        "large icon",
        "outside icon",
        "hidden icon",
        "opacity zero",
        "transparent icon",
        "empty icon",
        "text sibling",
        "rectangle",
        "partial radius",
    ] {
        let mut parent = stack();
        match case {
            "flow" => parent["layout"] = json!("horizontal"),
            "omitted layout" => {
                parent.as_object_mut().unwrap().remove("layout");
            }
            "no icon" => {
                parent["children"].as_array_mut().unwrap().pop();
            }
            "large icon" => parent["children"][1]["width"] = json!(48),
            "outside icon" => parent["children"][1]["x"] = json!(40),
            "hidden icon" => parent["children"][1]["visible"] = json!(false),
            "opacity zero" => parent["children"][1]["opacity"] = json!(0),
            "transparent icon" => parent["children"][1]["fill"][0]["color"] = json!("#FFFFFF00"),
            "empty icon" => parent["children"][1]["iconFontName"] = json!(""),
            "text sibling" => parent["children"][1]["type"] = json!("text"),
            "rectangle" => parent["children"][0]["width"] = json!(64),
            "partial radius" => parent["children"][0]["cornerRadius"] = json!(12),
            _ => unreachable!(),
        }
        assert_eq!(stub_ids(&parent), vec!["disc"], "{case}");
        assert!(
            empty_decorated_stub_diagnostic(&parent["children"][0], Some(&parent), None).is_some(),
            "{case}"
        );
    }
    let parent = stack();
    assert!(empty_decorated_stub_diagnostic(&parent["children"][0], None, None).is_some());
}
