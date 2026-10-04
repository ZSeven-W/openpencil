use super::*;
use serde_json::json;

fn fixture() -> (Value, HashMap<String, Rect>) {
    let tree = json!({"id":"page","type":"frame","width":1080,"height":1440,"children":[
        {"id":"image","type":"image","name":"screenshot-3"},
        {"id":"base","type":"ellipse","name":"步骤编号底","fill":[{"type":"solid","color":"#1A1A1A"}]},
        {"id":"number","type":"text","name":"步骤编号","content":"3","fontSize":44},
        {"id":"caption","type":"text","name":"标注3","content":"左右箭头翻页 · 改这一页只改当前页","fontSize":22},
        {"id":"footer","type":"text","name":"页脚页码","content":"第 3 页 / 共 3 页"}
    ]});
    let rects = HashMap::from([
        (
            "page".into(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1080.0,
                h: 1440.0,
            },
        ),
        (
            "image".into(),
            Rect {
                x: 100.0,
                y: 340.0,
                w: 880.0,
                h: 960.0,
            },
        ),
        (
            "base".into(),
            Rect {
                x: 100.0,
                y: 180.0,
                w: 88.0,
                h: 88.0,
            },
        ),
        (
            "number".into(),
            Rect {
                x: 100.0,
                y: 202.0,
                w: 88.0,
                h: 66.0,
            },
        ),
        (
            "caption".into(),
            Rect {
                x: 130.0,
                y: 1250.0,
                w: 560.0,
                h: 66.0,
            },
        ),
        (
            "footer".into(),
            Rect {
                x: 100.0,
                y: 1370.0,
                w: 880.0,
                h: 36.0,
            },
        ),
    ]);
    (tree, rects)
}

#[test]
fn retained_large_step_number_is_lifted_above_its_empty_backing() {
    let (tree, rects) = fixture();
    let mut cmds = Vec::new();
    collect_badges(&tree, &rects, &mut cmds);
    assert!(
        matches!(&cmds[..], [EditorCommand::MoveNode { node_id, index:Some(1), .. }] if node_id.as_str()=="number")
    );
    assert_eq!(tree["children"][2]["content"], "3");
    assert_eq!(tree["children"][2]["fontSize"], 44);
}

#[test]
fn caption_is_measured_at_full_width_before_moving_below_screenshot() {
    let (tree, mut rects) = fixture();
    let mut cmds = Vec::new();
    collect_captions(&tree, &rects, &mut cmds);
    assert!(matches!(
        &cmds[..],
        [EditorCommand::UpdateNode {
            y: None,
            width: Some(880),
            ..
        }]
    ));
    rects.get_mut("caption").unwrap().w = 880.0;
    rects.get_mut("caption").unwrap().h = 33.0;
    cmds.clear();
    collect_captions(&tree, &rects, &mut cmds);
    assert!(matches!(
        &cmds[..],
        [EditorCommand::UpdateNode {
            x: Some(100),
            y: Some(1312),
            width: None,
            ..
        }]
    ));
    rects.get_mut("caption").unwrap().y = 1312.0;
    cmds.clear();
    collect_captions(&tree, &rects, &mut cmds);
    assert!(cmds.is_empty(), "settled caption is stable");
}

#[test]
fn no_room_does_not_move_caption_into_footer_or_shrink_fonts() {
    let (tree, mut rects) = fixture();
    rects.get_mut("caption").unwrap().w = 880.0;
    let mut cmds = Vec::new();
    collect_captions(&tree, &rects, &mut cmds);
    assert!(cmds.is_empty());
}

#[test]
fn hero_copy_and_decorative_peer_layers_keep_authored_positions() {
    let (mut tree, rects) = fixture();
    tree["children"][1]["name"] = json!("decorative foreground");
    tree["children"][3]["name"] = json!("hero headline");
    let mut cmds = Vec::new();
    collect_fixes(&tree, &rects, &mut cmds);
    assert!(cmds.is_empty());
    tree["layout"] = json!("vertical");
    tree["children"][1]["name"] = json!("步骤编号底");
    collect_fixes(&tree, &rects, &mut cmds);
    assert!(cmds.is_empty(), "flex flow has no absolute repair");
}

#[test]
fn real_layout_repair_preserves_screenshot_annotation_and_unselected_board() {
    let (mut tree, rects) = fixture();
    tree["fill"] = json!([{"type":"solid","color":"#F5F4F0"}]);
    for child in tree["children"].as_array_mut().unwrap() {
        let r = rects.get(child["id"].as_str().unwrap()).unwrap();
        child["x"] = json!(r.x);
        child["y"] = json!(r.y);
        child["width"] = json!(r.w);
        child["height"] = if child["type"] == "text" {
            json!("fit_content")
        } else {
            json!(r.h)
        };
    }
    tree["children"][0]["src"] = json!("data:image/png;base64,PRESERVED");
    tree["children"].as_array_mut().unwrap().insert(
        0,
        json!({
            "id":"annotation","type":"ellipse","name":"标注 · 改这一页",
            "x":594,"y":1201,"width":89,"height":35,
            "fill":[],"stroke":{"thickness":4,"fill":[{"type":"solid","color":"#F97316"}]}
        }),
    );
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state
        .active_children_mut()
        .push(serde_json::from_value(tree).unwrap());
    let untouched = json!({"id":"other","type":"frame","width":390,"height":844,"children":[
        {"id":"other-base","type":"ellipse","name":"步骤编号底","x":0,"y":0,"width":88,"height":88,"fill":[{"type":"solid","color":"#000000"}]},
        {"id":"other-number","type":"text","name":"步骤编号","content":"4","x":0,"y":0,"width":88,"fontSize":44}
    ]});
    state
        .active_children_mut()
        .push(serde_json::from_value(untouched).unwrap());
    let other_before = serde_json::to_value(&state.active_children()[1]).unwrap();
    let before = serde_json::to_value(&state.active_children()[0]).unwrap();
    assert!(repair_scoped_tutorial_labels(&mut state, &["page".into()]) > 0);
    let after = serde_json::to_value(&state.active_children()[0]).unwrap();
    let find = |v: &Value, id: &str| children(v).iter().find(|n| n["id"] == id).unwrap().clone();
    for id in ["image", "annotation"] {
        assert_eq!(find(&before, id), find(&after, id), "{id} data retained");
    }
    for field in [
        "id",
        "content",
        "fontSize",
        "fontWeight",
        "x",
        "y",
        "width",
        "height",
    ] {
        assert_eq!(
            find(&before, "number")[field],
            find(&after, "number")[field]
        );
    }
    assert_eq!(find(&after, "number")["fill"][0]["color"], "#FFFFFF");
    assert_eq!(after["width"].as_f64(), Some(1080.0));
    assert_eq!(after["height"].as_f64(), Some(1440.0));
    let kids = children(&after);
    assert!(
        kids.iter().position(|n| n["id"] == "number") < kids.iter().position(|n| n["id"] == "base")
    );
    let positions = resolved_rects(&state);
    assert!(!intersects(&positions["caption"], &positions["image"]));
    assert!(!intersects(&positions["caption"], &positions["footer"]));
    assert_eq!(
        serde_json::to_value(&state.active_children()[1]).unwrap(),
        other_before
    );
    assert_eq!(
        repair_scoped_tutorial_labels(&mut state, &["page".into()]),
        0
    );
}

#[test]
#[ignore = "requires retained QA documents and a separate output directory"]
fn replay_retained_tutorial_documents_through_scoped_production_repair() {
    let input = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_TUTORIAL_INPUT").unwrap());
    let output = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_TUTORIAL_OUTPUT").unwrap());
    assert_ne!(input, output);
    std::fs::create_dir_all(&output).unwrap();
    for repeat in 1..=3 {
        let folder = format!("screenshot-tutorial-r{repeat}");
        let source = input.join(&folder).join("first-draft.op");
        if !source.exists() {
            continue;
        }
        let original = std::fs::read(&source).unwrap();
        let original_json: Value = serde_json::from_slice(&original).unwrap();
        let mut state = EditorState::new();
        state.doc = serde_json::from_slice(&original).unwrap();
        let roots: Vec<String> = state
            .active_children()
            .iter()
            .map(|n| {
                serde_json::to_value(n).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        let changed = repair_scoped_tutorial_labels(&mut state, &roots);
        assert_eq!(repair_scoped_tutorial_labels(&mut state, &roots), 0);
        let destination = output.join(&folder);
        std::fs::create_dir_all(&destination).unwrap();
        let mut saved = serde_json::to_value(&state.doc).unwrap();
        for field in ["images", "editorMeta"] {
            if let Some(value) = original_json.get(field) {
                saved[field] = value.clone();
            }
        }
        std::fs::write(
            destination.join("first-draft.op"),
            serde_json::to_vec_pretty(&saved).unwrap(),
        )
        .unwrap();
        std::fs::write(destination.join("result.json"), json!({"status":"completed","phase":"retained scoped production repairs","repeat":repeat,"repair_count":changed}).to_string()).unwrap();
        assert_eq!(
            std::fs::read(input.join(&folder).join("first-draft.op")).unwrap(),
            original
        );
        println!("{folder}: {changed} targeted repairs");
    }
}
