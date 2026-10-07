use super::*;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn identical_headline_with_a_small_authored_offset_is_a_text_effect() {
    let doc = jian_ops_schema::load_str(&json!({"version":"1.0.0","children":[
        {"type":"frame","id":"board","width":800,"height":600,"children":[
            {"type":"frame","id":"effect","layout":"none","width":600,"height":180,"children":[
                {"type":"text","id":"ink","x":0,"y":0,"content":"咖啡小聚","fontSize":100,"fontFamily":"Noto Serif SC","fontWeight":700,"width":"fit_content","height":"fit_content","fill":[{"type":"solid","color":"#1E1A16"}]},
                {"type":"text","id":"red","x":3,"y":3,"content":"咖啡小聚","fontSize":100,"fontFamily":"Noto Serif SC","fontWeight":700,"width":"fit_content","height":"fit_content","fill":[{"type":"solid","color":"#B74A37"}]}
            ]}
        ]}
    ]}).to_string()).unwrap().value;
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = crate::geometry_validation::geometry_diagnostics(&state);
    assert!(
        !issues.iter().any(|i| i.contains("TEXT leaves")),
        "{issues:?}"
    );
}

fn text_node(id: &str, name: &str, content: &str) -> serde_json::Value {
    json!({ "type": "text", "id": id, "name": name, "content": content })
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

// ── positive: reproduces the measured 0724-1-gm-2.op "今日词卡" shape ──
//
// A `layout:none` deck stacks a front card over a back card. Both anchor
// their own text near the bottom of their own card, and because the two
// cards are offset by only 12px (front on top, back peeking out below —
// the deliberate "stacked deck" look), the front card's meaning line and
// the back card's example line land on the same pixels. The two text
// nodes are cousins (different parents, both `layout:none` descendants),
// so `collect_sibling_jam_diagnostics` never sees this pair at all.

#[test]
fn stacked_deck_cousin_text_collision_is_detected() {
    let deck = json!({
        "type": "frame", "id": "deck", "name": "Stacked Card Container", "layout": "none",
        "children": [
            { "type": "frame", "id": "front", "name": "Front Vocabulary Card", "children": [
                text_node("front-meaning", "Chinese Meaning", "有适应力的；能迅速恢复的")
            ] },
            { "type": "frame", "id": "back", "name": "Back Example Card", "children": [
                text_node("back-example", "Example English", "She is resilient in facing hard times")
            ] }
        ]
    });
    let mut rects = HashMap::new();
    // Mirrors the measured resolved geometry: both text blocks land in the
    // same 300-ish-wide column, ~14-20px tall, offset by only a few px.
    rects.insert("front-meaning".to_string(), rect(118.0, 708.0, 244.0, 20.0));
    rects.insert("back-example".to_string(), rect(126.0, 705.0, 283.0, 17.0));

    let collisions = collect_text_collisions(&deck, &rects);
    assert_eq!(
        collisions.len(),
        1,
        "exactly one colliding pair: {collisions:?}"
    );
    let c = &collisions[0];
    assert_eq!(c.a_id, "front-meaning");
    assert_eq!(c.b_id, "back-example");
    assert!(c.overlap_x > MIN_AXIS_OVERLAP_PX && c.overlap_y > MIN_AXIS_OVERLAP_PX);
    assert!(c.overlap_area_ratio > MIN_OVERLAP_AREA_RATIO);

    let mut out = Vec::new();
    push_text_collision_diagnostics(&deck, &rects, &mut out);
    assert!(
        out.iter().any(|line| line.contains("Chinese Meaning")
            && line.contains("Example English")
            && line.contains("OVERLAP")),
        "diagnostic line must name both text nodes: {out:?}"
    );
}

/// Real jian layout end-to-end (not a manually-set rects map): a
/// `layout:none` deck of two cards, each bottom-anchoring its own text via
/// `justifyContent: end`, with the cards offset by only 4px — the
/// canonical "stacked deck" authoring shape. Proves the detector is
/// actually wired into `geometry_diagnostics`, not just unit-testable in
/// isolation.
#[test]
fn wired_into_geometry_diagnostics_under_real_layout() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Screen", "width": 375, "height": 200,
            "children": [{
                "type": "frame", "id": "deck", "name": "Stacked Card Container", "layout": "none",
                "width": "fill_container", "height": "fill_container",
                "children": [
                    { "type": "frame", "id": "front", "name": "Front Card",
                      "x": 0, "y": 0, "width": 300, "height": 130, "layout": "vertical",
                      "justifyContent": "end",
                      "children": [ text_node("front-meaning", "Chinese Meaning", "有适应力的；能迅速恢复的") ] },
                    { "type": "frame", "id": "back", "name": "Back Card",
                      "x": 8, "y": 4, "width": 284, "height": 130, "layout": "vertical",
                      "justifyContent": "end",
                      "children": [ text_node("back-example", "Example English", "She is resilient in facing hard times") ] }
                ]
            }]
        }]
    }))
    .expect("doc");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = crate::geometry_validation::geometry_diagnostics(&state);
    assert!(
        issues.iter().any(|i| i.contains("Chinese Meaning")
            && i.contains("Example English")
            && i.contains("OVERLAP")),
        "real-layout deck must surface the text collision: {issues:?}"
    );
}

// ── false-positive defenses ──

/// Normal vertical stack (title above subtitle) that merely touches by a
/// rounding-scale ~1px gap must not be reported — no real overlap.
#[test]
fn adjacent_flow_text_with_a_hairline_gap_is_not_reported() {
    let col = json!({
        "type": "frame", "id": "col", "name": "Text Column", "layout": "vertical",
        "children": [
            text_node("title", "Task Title", "听力训练"),
            text_node("subtitle", "Task Subtitle", "听短文回答 5 个问题 · +20 XP")
        ]
    });
    let mut rects = HashMap::new();
    rects.insert("title".to_string(), rect(0.0, 0.0, 120.0, 20.0));
    rects.insert("subtitle".to_string(), rect(0.0, 21.0, 200.0, 16.0));
    let collisions = collect_text_collisions(&col, &rects);
    assert!(
        collisions.is_empty(),
        "no overlap, must not report: {collisions:?}"
    );
}

/// A ring/badge number centered on a donut (ellipse) is not a text-vs-text
/// pair at all — the ellipse never enters the text-leaf list, so there is
/// nothing to compare it against.
#[test]
fn ring_center_text_over_a_non_text_ring_is_not_reported() {
    let stack = json!({
        "type": "frame", "id": "ring-stack", "name": "Progress Ring Stack", "layout": "none",
        "children": [
            { "type": "frame", "id": "ring-center", "name": "Ring Center", "children": [
                text_node("percent", "Percent Text", "2/3")
            ] },
            { "type": "ellipse", "id": "arc", "name": "Progress Arc" },
            { "type": "ellipse", "id": "track", "name": "Track Ring" }
        ]
    });
    let mut rects = HashMap::new();
    rects.insert("percent".to_string(), rect(20.0, 20.0, 20.0, 14.0));
    rects.insert("arc".to_string(), rect(0.0, 0.0, 60.0, 60.0));
    rects.insert("track".to_string(), rect(0.0, 0.0, 60.0, 60.0));
    let collisions = collect_text_collisions(&stack, &rects);
    assert!(
        collisions.is_empty(),
        "ellipse is never a text leaf: {collisions:?}"
    );
}

/// A correctly-authored opaque deck where only the FRONT card carries text
/// (the back card is pure chrome, no text child) — nothing to collide with.
#[test]
fn opaque_deck_with_text_only_on_the_front_card_is_not_reported() {
    let deck = json!({
        "type": "frame", "id": "deck", "name": "Stacked Card Container", "layout": "none",
        "children": [
            { "type": "frame", "id": "front", "name": "Front Card", "children": [
                text_node("only-text", "Word", "resilient")
            ] },
            { "type": "frame", "id": "back", "name": "Back Card", "children": [] }
        ]
    });
    let mut rects = HashMap::new();
    rects.insert("only-text".to_string(), rect(0.0, 0.0, 80.0, 20.0));
    let collisions = collect_text_collisions(&deck, &rects);
    assert!(
        collisions.is_empty(),
        "no second text leaf to collide with: {collisions:?}"
    );
}

/// Ordinary horizontal flex row of short text/value pairs sitting flush
/// side-by-side (not overlapping) must not be reported — that shape is
/// `collect_sibling_jam_diagnostics`'s territory (and even there, flush
/// but non-overlapping is a JAM warning, not an OVERLAP).
#[test]
fn normal_flex_row_text_siblings_are_not_reported() {
    let row = json!({
        "type": "frame", "id": "row", "name": "Price Row", "layout": "horizontal",
        "children": [
            text_node("price", "Price", "$29"),
            text_node("unit", "Unit", "/mo")
        ]
    });
    let mut rects = HashMap::new();
    rects.insert("price".to_string(), rect(0.0, 0.0, 30.0, 20.0));
    rects.insert("unit".to_string(), rect(30.0, 4.0, 20.0, 14.0));
    let collisions = collect_text_collisions(&row, &rects);
    assert!(
        collisions.is_empty(),
        "flush but non-overlapping: {collisions:?}"
    );
}

/// Empty / whitespace-only text and explicitly hidden text never enter
/// the leaf list, regardless of their resolved rect.
#[test]
fn empty_and_hidden_text_are_excluded() {
    let group = json!({
        "type": "frame", "id": "group", "name": "Group", "layout": "none",
        "children": [
            text_node("empty", "Empty", "   "),
            { "type": "text", "id": "hidden", "name": "Hidden", "content": "resilient", "visible": false },
            text_node("real", "Real", "resilient")
        ]
    });
    let mut rects = HashMap::new();
    // Deliberately overlapping rects — if either exclusion were missing
    // this would false-positive against "real".
    for id in ["empty", "hidden", "real"] {
        rects.insert(id.to_string(), rect(0.0, 0.0, 80.0, 20.0));
    }
    let collisions = collect_text_collisions(&group, &rects);
    assert!(
        collisions.is_empty(),
        "empty/hidden text must never participate: {collisions:?}"
    );
}

// ── real-sample harness (manual, not part of the default suite) ──
//
// `OP_TEXT_COLLISION_SAMPLE=/path/to/0724-1-gm-2.op cargo test -p
// op-orchestrator text_collision -- --ignored --nocapture`
#[test]
#[ignore]
fn detects_the_measured_stacked_word_card_sample() {
    let path = std::env::var("OP_TEXT_COLLISION_SAMPLE").expect("OP_TEXT_COLLISION_SAMPLE");
    let text = std::fs::read_to_string(&path).expect("read sample .op");
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(&text).expect("parse .op");
    let state = op_editor_core::EditorState::from_document(doc);
    let issues = crate::geometry_validation::geometry_diagnostics(&state);
    assert!(
        issues.iter().any(|i| i.contains("Chinese Meaning")
            && i.contains("Example English")
            && i.contains("OVERLAP")),
        "front card meaning vs back card example must be reported: {issues:?}"
    );
}

/// `lecture-deck-light` slide 02, verbatim: the 460px page numeral at 6%
/// opacity sits behind the title block, which crosses it by 450x85px.
fn deck_watermark_slide(numeral_opacity: f64) -> (Value, HashMap<String, Rect>) {
    let board = json!({
        "type":"frame","id":"board","name":"02 学习目标","layout":"none",
        "x":2040,"y":0,"width":1920,"height":1080,
        "children":[
            {"type":"text","id":"ghost","name":"ghost 页码","content":"02",
             "fontSize":460,"fontWeight":700,"opacity":numeral_opacity},
            {"type":"text","id":"title","name":"页标题",
             "content":"这节课下课时，你应该能——","fontSize":76,"fontWeight":700}
        ]
    });
    let rects = HashMap::from([
        (
            "board".to_string(),
            Rect {
                x: 2040.0,
                y: 0.0,
                w: 1920.0,
                h: 1080.0,
            },
        ),
        (
            "ghost".to_string(),
            Rect {
                x: 3390.0,
                y: 40.0,
                w: 554.0,
                h: 460.0,
            },
        ),
        (
            "title".to_string(),
            Rect {
                x: 2160.0,
                y: 120.0,
                w: 1680.0,
                h: 85.0,
            },
        ),
    ]);
    (board, rects)
}

#[test]
fn a_watermark_numeral_behind_a_title_is_not_a_collision() {
    // The shipped deck templates' idiom. 6% ink cannot obscure anything, so
    // the detector's own premise ("neither block is readable") never holds.
    let (board, rects) = deck_watermark_slide(0.06);
    assert!(
        collect_text_collisions(&board, &rects).is_empty(),
        "watermark page numeral is a graphic, not a competing text block"
    );
}

#[test]
fn the_same_pair_at_full_opacity_is_still_a_collision() {
    // Proves the gate is doing the work — identical geometry, opaque ink.
    let (board, rects) = deck_watermark_slide(1.0);
    let hits = collect_text_collisions(&board, &rects);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].a_id, "ghost");
    assert_eq!(hits[0].b_id, "title");
}

#[test]
fn a_translucent_but_readable_label_still_collides() {
    // 0.4 is a design choice a reader can still read through — well above
    // the watermark line, so it must NOT be swallowed.
    let (board, rects) = deck_watermark_slide(0.4);
    assert_eq!(collect_text_collisions(&board, &rects).len(), 1);
}

#[test]
fn the_measured_footer_column_overlap_still_reports() {
    // `0808-gm-2.op`'s footer, real resolved rects: the legal-links column
    // spilled across the newsletter block. Same artboard, ordinary opacity —
    // the watermark gate must not touch it.
    let footer = json!({
        "type":"frame","id":"footer","name":"Footer Content","layout":"horizontal",
        "children":[
            {"type":"text","id":"n729","name":"text","content":"条款与隐私"},
            {"type":"text","id":"n735","name":"text","content":"订阅技术资讯"}
        ]
    });
    let rects = HashMap::from([
        (
            "footer".to_string(),
            Rect {
                x: 160.0,
                y: 2975.0,
                w: 1040.0,
                h: 151.0,
            },
        ),
        (
            "n729".to_string(),
            Rect {
                x: 912.0,
                y: 2975.0,
                w: 96.0,
                h: 20.0,
            },
        ),
        (
            "n735".to_string(),
            Rect {
                x: 860.0,
                y: 2975.0,
                w: 84.0,
                h: 20.0,
            },
        ),
    ]);
    assert_eq!(collect_text_collisions(&footer, &rects).len(), 1);
}
