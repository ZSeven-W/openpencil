//! Tests for `fix_deck_front_card_transparency` — the `layout:none`
//! card/deck-stack front-card transparency contract (0724-1-gm-3.op's
//! Flashcard Deck Stack, where the front card's empty fill let the painted
//! "Back Layer 1" bleed through the text).

use super::*;

fn run(mut nodes: Vec<PenNode>) -> serde_json::Value {
    post_pass_forest(&mut nodes, 375.0);
    serde_json::to_value(&nodes[0]).unwrap()
}

#[test]
fn deck_front_card_gets_a_surface_fill_when_a_painted_layer_bleeds_through() {
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"deck","name":"Flashcard Deck Stack","layout":"none",
        "x":0,"y":0,"width":354,"height":132,
        "children":[
            {"type":"frame","id":"front","name":"Front Flashcard",
             "x":0,"y":0,"width":338,"height":124,
             "children":[{"type":"text","id":"t1","content":"Hello"}]},
            {"type":"frame","id":"back1","name":"Back Layer 1",
             "x":8,"y":4,"width":338,"height":124,
             "fill":[{"type":"solid","color":"$--accent"}],
             "children":[]},
            {"type":"frame","id":"back2","name":"Back Layer 2",
             "x":16,"y":8,"width":338,"height":124,
             "children":[]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    assert_eq!(
        v["children"][0]["fill"],
        json!([{"type":"solid","color":"$--card"}]),
        "front card must get a semantic surface fill so the painted back layer stops bleeding through: {v}"
    );
}

#[test]
fn front_card_with_its_own_fill_is_left_alone() {
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"deck","name":"Deck","layout":"none",
        "x":0,"y":0,"width":354,"height":132,
        "children":[
            {"type":"frame","id":"front","name":"Front",
             "x":0,"y":0,"width":338,"height":124,
             "fill":[{"type":"solid","color":"#FFFFFF"}],
             "children":[{"type":"text","id":"t1","content":"Hello"}]},
            {"type":"frame","id":"back1","name":"Back",
             "x":8,"y":4,"width":338,"height":124,
             "fill":[{"type":"solid","color":"$--accent"}],
             "children":[]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    assert_eq!(
        v["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "front card already opaque — must not be touched: {v}"
    );
}

#[test]
fn unpainted_back_layer_is_not_a_leak_source() {
    // Neither layer paints anything — no fact of a visible leak, so the
    // front card (still unfilled) must not be touched.
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"deck","name":"Deck","layout":"none",
        "x":0,"y":0,"width":354,"height":132,
        "children":[
            {"type":"frame","id":"front","name":"Front",
             "x":0,"y":0,"width":338,"height":124,
             "children":[{"type":"text","id":"t1","content":"Hello"}]},
            {"type":"frame","id":"back1","name":"Back",
             "x":8,"y":4,"width":338,"height":124,
             "children":[]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    assert_eq!(
        v["children"][0].get("fill"),
        None,
        "no painted sibling behind the front card — nothing to leak, must stay untouched: {v}"
    );
}

#[test]
fn donut_ring_ellipse_sibling_behind_a_label_is_not_misfired_on() {
    // A ring/donut composition: an unfilled label frame sits over a PAINTED
    // ellipse ring by deliberate design (the ring must show through) — the
    // sibling is an ellipse, never eligible as a "leak source" painted card.
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"donut","name":"Donut Wrapper","layout":"none",
        "x":0,"y":0,"width":120,"height":120,
        "children":[
            {"type":"frame","id":"label","name":"Center Label",
             "x":30,"y":45,"width":60,"height":30,
             "children":[{"type":"text","id":"t1","content":"72%"}]},
            {"type":"ellipse","id":"ring","name":"Ring",
             "x":0,"y":0,"width":120,"height":120,
             "fill":[{"type":"solid","color":"$--primary"}]}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    assert_eq!(
        v["children"][0].get("fill"),
        None,
        "ellipse ring siblings must never trigger the deck front-card fix: {v}"
    );
}

#[test]
fn image_backed_card_keeps_its_transparent_content_layer() {
    // 0728-gm.op theme-list card: photo at the bottom, alpha gradient scrim
    // in the middle, transparent text-bearing content stack on top. The
    // see-through front IS the composition — filling it hides the photo.
    let nodes: Vec<PenNode> = vec![serde_json::from_value(json!({
        "type":"frame","id":"card","name":"Theme Card","layout":"none",
        "x":0,"y":0,"width":240,"height":140,
        "children":[
            {"type":"frame","id":"content","name":"Content Stack",
             "x":0,"y":0,"width":240,"height":140,
             "children":[{"type":"text","id":"t1","content":"高分赛博朋克佳作"}]},
            {"type":"rectangle","id":"scrim","name":"Gradient Scrim",
             "x":0,"y":0,"width":240,"height":140,
             "fill":[{"type":"linear_gradient","angle":180.0,"stops":[
                 {"offset":0.0,"color":"#0F172A33"},
                 {"offset":1.0,"color":"#0F172AF2"}]}]},
            {"type":"image","id":"photo","name":"Background Image",
             "x":0,"y":0,"width":240,"height":140,
             "src":"op-image:207917279cfb430f",
             "imageSearchQuery":"cyberpunk city neon"}
        ]
    }))
    .unwrap()];
    let v = run(nodes);
    assert_eq!(
        v["children"][0].get("fill"),
        None,
        "image-backed card's transparent content layer must stay transparent: {v}"
    );
}
