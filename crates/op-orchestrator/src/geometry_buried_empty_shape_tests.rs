//! Glyphs buried under empty shapes authored before them.

use super::*;
use serde_json::json;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn moves(cmds: &[EditorCommand]) -> Vec<(String, usize)> {
    cmds.iter()
        .filter_map(|cmd| match cmd {
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                index,
                ..
            } => {
                assert_eq!(target_parent.as_str(), "stack");
                Some((node_id.as_str().to_string(), index.unwrap()))
            }
            _ => None,
        })
        .collect()
}

fn fixes(stack: &Value, rects: &HashMap<String, Rect>) -> Vec<(String, usize)> {
    let mut cmds = Vec::new();
    collect_buried_overlay_fixes(stack, rects, &mut cmds);
    moves(&cmds)
}

/// A day cell from the DeepSeek travel booking page (ds-night-post-1021):
/// the range band, the endpoint circle and the day number, authored
/// back-to-front, so the band and circle painted over the number.
fn day_cell(band: Rect, marker_opacity: f64) -> (Value, HashMap<String, Rect>) {
    let stack = json!({"type":"frame","id":"stack","name":"Day cell 14","layout":"none","children":[
        {"type":"rectangle","id":"band","name":"Range band","opacity":1.0,
         "fill":[{"type":"solid","color":"$--input"}]},
        {"type":"ellipse","id":"marker","name":"Day marker","opacity":marker_opacity,
         "fill":[{"type":"solid","color":"$--primary"}]},
        {"type":"text","id":"number","name":"Day number","content":"14","textAlign":"center"},
        {"type":"ellipse","id":"dot","name":"Today dot","opacity":0.0,
         "fill":[{"type":"solid","color":"$--primary"}]}
    ]});
    let rects = HashMap::from([
        ("stack".to_string(), rect(0.0, 0.0, 42.0, 24.0)),
        ("band".to_string(), band),
        ("marker".to_string(), rect(10.0, 1.0, 22.0, 22.0)),
        ("number".to_string(), rect(0.0, 3.0, 42.0, 18.0)),
        ("dot".to_string(), rect(19.0, 20.0, 4.0, 4.0)),
    ]);
    (stack, rects)
}

#[test]
fn calendar_day_numbers_rise_above_the_range_band_and_endpoint_circle() {
    // Endpoint day: half band from the centre plus the selected circle.
    let (stack, rects) = day_cell(rect(21.0, 3.0, 21.0, 18.0), 1.0);
    assert_eq!(fixes(&stack, &rects), vec![("number".to_string(), 0)]);
    // In-range day: a full band the number's own size.
    let (stack, rects) = day_cell(rect(0.0, 3.0, 42.0, 18.0), 0.0);
    assert_eq!(fixes(&stack, &rects), vec![("number".to_string(), 0)]);
}

#[test]
fn hidden_shapes_and_correctly_ordered_cells_are_left_alone() {
    // Neither band nor circle shows on an ordinary day.
    let (mut stack, rects) = day_cell(rect(0.0, 3.0, 42.0, 18.0), 0.0);
    stack["children"][0]["opacity"] = json!(0.0);
    assert!(fixes(&stack, &rects).is_empty());
    // Number authored first is already on top.
    let (mut stack, rects) = day_cell(rect(0.0, 3.0, 42.0, 18.0), 1.0);
    let kids = stack["children"].as_array_mut().unwrap();
    let number = kids.remove(2);
    kids.insert(0, number);
    assert!(fixes(&stack, &rects).is_empty());
}

#[test]
fn a_caption_and_a_map_pin_rise_just_above_the_first_shape_hiding_them() {
    // Knowledge card composition: decorative shapes first, caption last.
    let card = json!({"type":"frame","id":"stack","name":"序号构图","layout":"none","children":[
        {"type":"ellipse","id":"ring","name":"大圆环","fill":[{"type":"solid","color":"$--border"}]},
        {"type":"rectangle","id":"ink","name":"墨色条","fill":[{"type":"solid","color":"#1E1A16"}]},
        {"type":"text","id":"caption","name":"序号注脚","content":"第一步 / STEP ONE"}
    ]});
    let rects = HashMap::from([
        ("ring".to_string(), rect(400.0, 20.0, 520.0, 520.0)),
        ("ink".to_string(), rect(0.0, 420.0, 340.0, 56.0)),
        ("caption".to_string(), rect(64.0, 410.0, 260.0, 40.0)),
    ]);
    assert_eq!(fixes(&card, &rects), vec![("caption".to_string(), 1)]);

    // Map placeholder: a road band drawn over the destination pin.
    let map = json!({"type":"frame","id":"stack","name":"map","layout":"none","children":[
        {"type":"rectangle","id":"block","name":"map-block-4","fill":[{"type":"solid","color":"$--accent"}]},
        {"type":"rectangle","id":"road","name":"map-road-h","fill":[{"type":"solid","color":"$--background"}]},
        {"type":"icon_font","id":"pin","name":"map-pin","iconFontName":"map-pin"}
    ]});
    let rects = HashMap::from([
        ("block".to_string(), rect(248.25, 12.0, 66.75, 26.7)),
        ("road".to_string(), rect(0.0, 44.7, 327.0, 10.0)),
        ("pin".to_string(), rect(267.6, 36.0, 28.0, 28.0)),
    ]);
    assert_eq!(fixes(&map, &rects), vec![("pin".to_string(), 1)]);
}

#[test]
fn strike_throughs_translucent_washes_and_content_cards_keep_their_order() {
    let base = |cover: Value| {
        json!({"type":"frame","id":"stack","layout":"none","children":[cover,
            {"type":"text","id":"label","content":"¥299"}]})
    };
    let rects = HashMap::from([
        ("cover".to_string(), rect(0.0, 9.0, 80.0, 2.0)),
        ("label".to_string(), rect(0.0, 0.0, 80.0, 20.0)),
    ]);
    // An explicit strike-through over an old price is the point.
    let strike = base(
        json!({"type":"rectangle","id":"cover","name":"price-strike",
        "fill":[{"type":"solid","color":"#111111"}]}),
    );
    assert!(fixes(&strike, &rects).is_empty());
    // A translucent wash shows the glyph through it.
    let wash = base(json!({"type":"rectangle","id":"cover","name":"wash",
        "fill":[{"type":"solid","color":"#11111140"}]}));
    assert!(fixes(&wash, &rects).is_empty());
    // A shape with content of its own is not an empty decoration.
    let card = base(json!({"type":"rectangle","id":"cover","name":"chip",
        "fill":[{"type":"solid","color":"#111111"}],
        "children":[{"type":"text","id":"inner","content":"新"}]}));
    assert!(fixes(&card, &rects).is_empty());
}
