//! `collect_card_inner_padding_fixes` — the flush-against-the-card-edge
//! padding repair.
//!
//! Fixtures mirror the measured fitness screen: a painted, rounded
//! horizontal "Exercise Row" (children = thumbnail image + info frame +
//! duration text) with no authored padding. Resolved rects are hand-built;
//! the chip exclusion is the only predicate that reads them.

use super::*;
use serde_json::json;
use std::collections::HashSet;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn no_chrome() -> HashSet<String> {
    HashSet::new()
}

fn card(extra_children: Vec<Value>, padding: Option<Value>) -> Value {
    let mut card = json!({
        "type":"frame","id":"card","name":"Exercise Row",
        "layout":"horizontal","width":"fill_container","height":"fit_content",
        "cornerRadius":16,"fill":[{"type":"solid","color":"$--card"}],
        "children":[]
    });
    if let Some(padding) = padding {
        card["padding"] = padding;
    }
    card["children"] = Value::Array(extra_children);
    card
}

fn padding_of(cmds: &[EditorCommand], id: &str) -> Option<LayoutPropValue> {
    cmds.iter().find_map(|cmd| match cmd {
        EditorCommand::SetNodeLayoutProp {
            node_id,
            property,
            value,
        } if node_id.as_str() == id && property == "padding" => Some(value.clone()),
        _ => None,
    })
}

#[test]
fn painted_card_with_edge_content_gets_the_standard_inset() {
    // The fitness row: painted, radius 16, children = image + frame(texts) +
    // text, padding absent → [12, 16] (vertical 12, horizontal 16).
    let card = card(
        vec![
            json!({"type":"image","id":"thumb","width":56,"height":56}),
            json!({"type":"frame","id":"info","layout":"vertical","children":[
                {"type":"text","id":"t1","text":"Bench Press"}
            ]}),
            json!({"type":"text","id":"duration","text":"12 min"}),
        ],
        None,
    );
    let rects = HashMap::from([("card".to_string(), rect(24.0, 0.0, 327.0, 80.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&card, &rects, &no_chrome(), &mut cmds);
    assert_eq!(
        padding_of(&cmds, "card"),
        Some(LayoutPropValue::NumberArray(vec![12.0, 16.0]))
    );
    assert_eq!(cmds.len(), 1, "only the card inset: {cmds:?}");
}

#[test]
fn wrapper_with_only_frame_children_is_untouched() {
    // A structural wrapper is not a card: no text/image/icon leaf touches
    // its edge.
    let wrapper = card(
        vec![
            json!({"type":"frame","id":"a","layout":"vertical","children":[
                {"type":"text","id":"t1","text":"Title"}
            ]}),
            json!({"type":"frame","id":"b","layout":"vertical","children":[
                {"type":"text","id":"t2","text":"Body"}
            ]}),
        ],
        None,
    );
    let rects = HashMap::from([("card".to_string(), rect(0.0, 0.0, 327.0, 120.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&wrapper, &rects, &no_chrome(), &mut cmds);
    assert!(
        padding_of(&cmds, "card").is_none(),
        "wrapper untouched: {cmds:?}"
    );
}

#[test]
fn image_only_card_is_untouched() {
    // A single full-bleed thumbnail is meant to reach the painted edge.
    let card = card(
        vec![json!({"type":"image","id":"cover","width":"fill_container","height":160})],
        None,
    );
    let rects = HashMap::from([("card".to_string(), rect(0.0, 0.0, 327.0, 160.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&card, &rects, &no_chrome(), &mut cmds);
    assert!(cmds.is_empty(), "image-only card untouched: {cmds:?}");
}

#[test]
fn chip_is_untouched() {
    // Resolved 100x32: narrower than 120 AND shorter than the 44px touch
    // floor — a chip, owned by its own rules.
    let chip = card(
        vec![json!({"type":"text","id":"label","text":"Yoga"})],
        None,
    );
    let rects = HashMap::from([("card".to_string(), rect(0.0, 0.0, 100.0, 32.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&chip, &rects, &no_chrome(), &mut cmds);
    assert!(cmds.is_empty(), "chip untouched: {cmds:?}");
}

#[test]
fn card_with_existing_padding_is_untouched() {
    // Any non-zero side is authored intent — [0, 20] stays.
    let card = card(
        vec![json!({"type":"text","id":"label","text":"Bench Press"})],
        Some(json!([0, 20])),
    );
    let rects = HashMap::from([("card".to_string(), rect(0.0, 0.0, 327.0, 80.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&card, &rects, &no_chrome(), &mut cmds);
    assert!(cmds.is_empty(), "padded card untouched: {cmds:?}");
}

#[test]
fn unpadded_wrapper_chain_gets_padding_on_the_card() {
    let card = card(
        vec![json!({
            "type":"frame", "id":"wrapper", "layout":"vertical",
            "children":[{
                "type":"frame", "id":"row", "layout":"horizontal",
                "children":[{"type":"text", "id":"hero-number", "text":"1,286,430.52"}]
            }]
        })],
        None,
    );
    let rects = HashMap::from([("card".to_string(), rect(0.0, 0.0, 327.0, 80.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&card, &rects, &no_chrome(), &mut cmds);
    assert_eq!(
        padding_of(&cmds, "card"),
        Some(LayoutPropValue::NumberArray(vec![12.0, 16.0]))
    );
    assert_eq!(cmds.len(), 1, "only the card gets the inset: {cmds:?}");
}

#[test]
fn padded_wrapper_chain_is_untouched() {
    let card = card(
        vec![json!({
            "type":"frame", "id":"wrapper", "layout":"vertical", "padding":[0,16],
            "children":[{"type":"text", "id":"label", "text":"Balance"}]
        })],
        None,
    );
    let rects = HashMap::from([("card".to_string(), rect(0.0, 0.0, 327.0, 80.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&card, &rects, &no_chrome(), &mut cmds);
    assert!(cmds.is_empty(), "padded wrapper owns the inset: {cmds:?}");
}

#[test]
fn none_layout_wrapper_chain_is_untouched() {
    let card = card(
        vec![json!({
            "type":"frame", "id":"wrapper", "layout":"none",
            "children":[{"type":"text", "id":"label", "text":"Balance"}]
        })],
        None,
    );
    let rects = HashMap::from([("card".to_string(), rect(0.0, 0.0, 327.0, 80.0))]);
    let mut cmds = Vec::new();
    collect_card_inner_padding_fixes(&card, &rects, &no_chrome(), &mut cmds);
    assert!(cmds.is_empty(), "absolute wrapper owns placement: {cmds:?}");
}
