//! Scrims authored above the copy they were meant to sit under.

use super::super::collect_buried_overlay_fixes;
use super::*;
use op_editor_core::EditorCommand;
use serde_json::json;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn moves(stack: &Value, rects: &HashMap<String, Rect>) -> Vec<(String, Option<usize>)> {
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(stack, rects, &mut cmds);
    cmds.into_iter()
        .filter_map(|cmd| match cmd {
            EditorCommand::MoveNode { node_id, index, .. } => {
                Some((node_id.as_str().to_string(), index))
            }
            _ => None,
        })
        .collect()
}

fn fade_scrim() -> Value {
    json!({"type":"rectangle","id":"scrim","name":"hero-scrim","x":0,"y":280,
           "width":375,"height":285,
           "fill":[{"type":"linear_gradient","angle":180,"stops":[
               {"offset":0.0,"color":"#FFFFFF00"},{"offset":1.0,"color":"$--background"}]}]})
}

fn photo() -> Value {
    json!({"type":"image","id":"photo","name":"hero-illustration","x":0,"y":0,
           "width":375,"height":565,"imageSearchQuery":"mobile app illustration"})
}

/// arena-m04 0929a: `[hero-scrim, hero-copy, hero-illustration]`.
#[test]
fn a_scrim_listed_before_the_copy_drops_to_just_above_the_photo() {
    let stack = json!({"type":"frame","id":"stack","layout":"none","width":375,"height":565,
        "children":[fade_scrim(),
            {"type":"frame","id":"copy","layout":"vertical","x":0,"y":344,"width":375,"height":221,
             "children":[{"type":"text","id":"title","content":"Everything you need"}]},
            photo()]});
    let rects = HashMap::from([
        ("stack".to_string(), rect(0.0, 0.0, 375.0, 565.0)),
        ("scrim".to_string(), rect(0.0, 280.0, 375.0, 285.0)),
        ("copy".to_string(), rect(0.0, 344.0, 375.0, 221.0)),
        ("photo".to_string(), rect(0.0, 0.0, 375.0, 565.0)),
    ]);
    assert_eq!(moves(&stack, &rects), vec![("scrim".to_string(), Some(1))]);
}

/// arena-m05 0929a: the scrim sat over the back / bookmark controls and the
/// title; it lands between the title and the photo.
#[test]
fn every_covered_layer_ends_up_above_the_scrim() {
    let stack = json!({"type":"frame","id":"stack","layout":"none","width":375,"height":420,
    "children":[
        {"type":"rectangle","id":"scrim","x":0,"y":0,"width":375,"height":420,
         "fill":[{"type":"linear_gradient","angle":180,"stops":[
            {"offset":0.0,"color":"#00000055"},{"offset":1.0,"color":"#1E1B2ECC"}]}]},
        {"type":"frame","id":"back","x":16,"y":16,"width":40,"height":40,
         "children":[{"type":"icon_font","id":"backi","iconFontName":"chevron-left"}]},
        {"type":"text","id":"title","x":16,"y":340,"width":343,"height":40,"content":"夜色里的城市漫游"},
        {"type":"image","id":"photo","x":0,"y":0,"width":375,"height":420,"src":"op-image:1"}
    ]});
    let rects = HashMap::from([
        ("stack".to_string(), rect(0.0, 0.0, 375.0, 420.0)),
        ("scrim".to_string(), rect(0.0, 0.0, 375.0, 420.0)),
        ("back".to_string(), rect(16.0, 16.0, 40.0, 40.0)),
        ("title".to_string(), rect(16.0, 340.0, 343.0, 40.0)),
        ("photo".to_string(), rect(0.0, 0.0, 375.0, 420.0)),
    ]);
    assert_eq!(moves(&stack, &rects), vec![("scrim".to_string(), Some(2))]);
}

#[test]
fn a_scrim_already_under_the_copy_is_left_alone() {
    let stack = json!({"type":"frame","id":"stack","layout":"none","width":375,"height":565,
        "children":[
            {"type":"text","id":"title","x":16,"y":400,"width":343,"height":40,"content":"Hello"},
            fade_scrim(), photo()]});
    let rects = HashMap::from([
        ("stack".to_string(), rect(0.0, 0.0, 375.0, 565.0)),
        ("title".to_string(), rect(16.0, 400.0, 343.0, 40.0)),
        ("scrim".to_string(), rect(0.0, 280.0, 375.0, 285.0)),
        ("photo".to_string(), rect(0.0, 0.0, 375.0, 565.0)),
    ]);
    assert!(moves(&stack, &rects).is_empty());
}

/// A modal backdrop dims the page on purpose: no photo sits as a direct
/// sibling below the content it covers.
#[test]
fn a_modal_backdrop_over_page_content_is_left_alone() {
    let stack = json!({"type":"frame","id":"stack","layout":"none","width":375,"height":812,
    "children":[
        {"type":"frame","id":"dialog","x":24,"y":300,"width":327,"height":200,
         "children":[{"type":"text","id":"q","content":"Delete?"}]},
        {"type":"rectangle","id":"backdrop","x":0,"y":0,"width":375,"height":812,
         "fill":[{"type":"solid","color":"#00000080"}]},
        {"type":"frame","id":"page","x":0,"y":0,"width":375,"height":812,
         "children":[{"type":"text","id":"t","content":"Feed"},
                     {"type":"image","id":"img","width":375,"height":200,"src":"op-image:2"}]}
    ]});
    let rects = HashMap::from([
        ("stack".to_string(), rect(0.0, 0.0, 375.0, 812.0)),
        ("dialog".to_string(), rect(24.0, 300.0, 327.0, 200.0)),
        ("backdrop".to_string(), rect(0.0, 0.0, 375.0, 812.0)),
        ("page".to_string(), rect(0.0, 0.0, 375.0, 812.0)),
    ]);
    assert!(moves(&stack, &rects).is_empty());
}
