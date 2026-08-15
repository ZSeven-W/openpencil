//! Tests for `cleanup_slide_padding::wrap_board_overflowing_text` (DS P2-c B,
//! pass `board-text-wrap`): on a Card/Deck board, a text descendant whose real
//! layout puts its right edge past the board's right inner edge by more than
//! 2px while its `textGrowth` is not fixed-width is provably clipped, and the
//! pass converts it — together with every duplicate copy sharing its parent,
//! content and `fontSize` — to `width: fill_container` + `textGrowth:
//! fixed-width`. Everything else (already-wrapped text, floating overlays,
//! short titles, non-board roots) is left alone, and a second round is a
//! no-op. The file also carries the combined lesion acceptance test: the
//! P2-c card's flush footer lifts ONLY the root's bottom padding while the
//! 96px head gap keeps the top untouched.

use super::*;
use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::EditorCommand;
use serde_json::json;

fn insert_tree(sink: &mut VecDocSink, tree: &serde_json::Value) {
    let tree: PenNode = serde_json::from_value(tree.clone()).expect("test tree json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn root_json(sink: &VecDocSink) -> serde_json::Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialize")
}

fn find_json<'a>(value: &'a serde_json::Value, id: &str) -> Option<&'a serde_json::Value> {
    if value.get("id").and_then(serde_json::Value::as_str) == Some(id) {
        return Some(value);
    }
    for child in value
        .get("children")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(found) = find_json(child, id) {
            return Some(found);
        }
    }
    None
}

fn node_json(sink: &VecDocSink, id: &str) -> serde_json::Value {
    let root = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize");
    find_json(&root, id).expect("node exists").clone()
}

fn run_wrap(sink: &mut VecDocSink, root_id: &str) -> bool {
    wrap_board_overflowing_text(sink, root_id)
}

fn run_floor(sink: &mut VecDocSink, root_id: &str) -> bool {
    enforce_slide_padding_floor(sink, root_id)
}

/// The P2-c lesion card, mirrored from the measured
/// `p2b-v4-pro-card.op`: a 1080×1440 board with no root padding, sitting at
/// canvas position (80, 40) exactly like the real file (the root's own x/y
/// must never read as a floating-layer pin); a header section owning the
/// 96px head gap (`[96, 80]` padding); an in-flow full-width `layout:none`
/// headline band holding the 88px double-copy title (`fit_content` + `auto`
/// — the shadow at x:3,y:3, the main at x:0,y:0); the correctly wrapped
/// subtitle; and a tail section carrying `[0, 80]` (no bottom padding) whose
/// footer is flush against the board's bottom edge. The tail fills the
/// remaining height so the flush is deterministic regardless of the measure
/// backend.
fn p2c_lesion_card() -> serde_json::Value {
    json!({
        "type": "frame",
        "id": "card",
        "name": "知识卡片",
        "width": 1080,
        "height": 1440,
        "layout": "vertical",
        "x": 80,
        "y": 40,
        "children": [
            { "type": "frame", "id": "header", "name": "卡片头部", "layout": "vertical",
              "width": "fill_container", "height": "fit_content", "padding": [96, 80],
              "children": [
                { "type": "frame", "id": "headline-wrap", "name": "headline-wrap",
                  "width": "fill_container", "height": 120, "layout": "none",
                  "children": [
                    { "type": "text", "id": "headline-shadow",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 88, "width": "fit_content", "height": "fit_content",
                      "textGrowth": "auto", "x": 3, "y": 3 },
                    { "type": "text", "id": "headline-main",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 88, "width": "fit_content", "height": "fit_content",
                      "textGrowth": "auto", "x": 0, "y": 0 }
                  ]
                },
                { "type": "text", "id": "subtitle", "content": "一键提升质感，告别廉价感",
                  "fontSize": 40, "width": "fill_container", "height": "fit_content",
                  "textGrowth": "fixed-width" }
              ]
            },
            { "type": "frame", "id": "tail", "name": "卡片尾部", "layout": "vertical",
              "width": "fill_container", "height": "fill_container", "padding": [0, 80],
              "justifyContent": "end",
              "children": [
                { "type": "text", "id": "footer", "content": "关注我，每天一个设计小技巧",
                  "fontSize": 28, "width": "fit_content", "height": "fit_content",
                  "textGrowth": "auto" }
              ]
            }
        ]
    })
}

// ── The combined lesion acceptance test (DS P2-c A + B) ─────────────────────

#[test]
fn the_p2c_lesion_card_lifts_only_the_bottom_floor_and_wraps_both_title_copies() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &p2c_lesion_card());

    // A: the flush footer proves the bottom margin missing — only BOTTOM
    // rises to 48. The 96px head gap is a composition, not a defect, so the
    // top stays 0; the 80px horizontal margin is delegated to the sections,
    // so left/right stay 0.
    assert!(
        run_floor(&mut sink, "card"),
        "the flush footer must trigger the floor"
    );
    assert_eq!(
        root_json(&sink)["padding"],
        json!([0.0, 0.0, 48.0, 0.0]),
        "only the bottom edge rises (96px head gap + section-owned \
         horizontal margin are compositions, not defects)"
    );

    // B: the 88px single-line headline copies are provably clipped — both
    // copies convert to the wrap posture the subtitle already uses.
    assert!(
        run_wrap(&mut sink, "card"),
        "the clipped headline must trigger the wrap"
    );
    for id in ["headline-main", "headline-shadow"] {
        assert_eq!(
            node_json(&sink, id)["width"],
            json!("fill_container"),
            "{id}"
        );
        assert_eq!(
            node_json(&sink, id)["textGrowth"],
            json!("fixed-width"),
            "{id}"
        );
    }
    // The already-correct subtitle is untouched; so are the short footer
    // texts that never overflowed.
    assert_eq!(
        node_json(&sink, "subtitle")["textGrowth"],
        json!("fixed-width")
    );
    assert_eq!(
        node_json(&sink, "subtitle")["width"],
        json!("fill_container")
    );
    assert_eq!(node_json(&sink, "footer")["width"], json!("fit_content"));
    assert_eq!(node_json(&sink, "footer")["textGrowth"], json!("auto"));

    // Round two: both passes find their repairs already in place.
    assert!(
        !run_floor(&mut sink, "card"),
        "the bottom floor is idempotent"
    );
    assert!(
        !run_wrap(&mut sink, "card"),
        "the wrap posture is idempotent"
    );
}

// ── The duplicate-copy group rule ────────────────────────────────────────────

#[test]
fn a_pinned_shadow_copy_is_wrapped_along_with_its_triggering_twin() {
    // The double-copy hack with the shadow authored as a pinned overlay on
    // the flex flow: only the in-flow copy proves the clip (a pinned text
    // under a flex parent is a floating overlay and cannot trigger), but the
    // group rule — same parent, same content, same fontSize — must wrap the
    // shadow with it: two copies that wrap differently destroy the effect.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [0, 60],
            "children": [
                { "type": "frame", "id": "header", "name": "卡片头部", "layout": "vertical",
                  "width": "fill_container", "height": "fit_content", "padding": [96, 80],
                  "children": [
                    { "type": "text", "id": "headline-shadow",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 88, "width": "fit_content", "height": "fit_content",
                      "textGrowth": "auto", "x": 3, "y": 3 },
                    { "type": "text", "id": "headline-main",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 88, "width": "fit_content", "height": "fit_content",
                      "textGrowth": "auto" }
                  ]
                }
            ]
        }),
    );

    assert!(run_wrap(&mut sink, "card"), "the in-flow copy must trigger");
    for id in ["headline-main", "headline-shadow"] {
        assert_eq!(
            node_json(&sink, id)["width"],
            json!("fill_container"),
            "{id}"
        );
        assert_eq!(
            node_json(&sink, id)["textGrowth"],
            json!("fixed-width"),
            "{id}"
        );
    }
}

#[test]
fn a_same_content_sibling_with_a_different_font_size_stays_out_of_the_group() {
    // The group key is content + fontSize: a same-string neighbour at a
    // different size is a different shape and must not be dragged along.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [0, 60],
            "children": [
                { "type": "frame", "id": "header", "name": "卡片头部", "layout": "vertical",
                  "width": "fill_container", "height": "fit_content", "padding": [96, 80],
                  "children": [
                    { "type": "text", "id": "headline",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 88, "width": "fit_content", "height": "fit_content",
                      "textGrowth": "auto" },
                    { "type": "text", "id": "caption",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 32, "width": "fit_content", "height": "fit_content",
                      "textGrowth": "auto" }
                  ]
                }
            ]
        }),
    );

    assert!(run_wrap(&mut sink, "card"));
    assert_eq!(
        node_json(&sink, "headline")["textGrowth"],
        json!("fixed-width")
    );
    assert_eq!(
        node_json(&sink, "caption")["textGrowth"],
        json!("auto"),
        "a different fontSize is a different shape, not a copy"
    );
    assert_eq!(node_json(&sink, "caption")["width"], json!("fit_content"));
}

// ── Negative cases ───────────────────────────────────────────────────────────

#[test]
fn a_fixed_width_copy_whose_shadow_offset_still_pokes_past_the_edge_is_left_alone() {
    // After the repair the shadow copy (x:3) resolves 3px past the right
    // inner edge — the growth keyword is the proof the repair already ran,
    // so the residual offset must not re-trigger the pass (idempotence).
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [0, 60],
            "children": [
                { "type": "frame", "id": "header", "name": "卡片头部", "layout": "vertical",
                  "width": "fill_container", "height": "fit_content", "padding": [96, 80],
                  "children": [
                    { "type": "text", "id": "headline",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 88, "width": "fill_container", "height": "fit_content",
                      "textGrowth": "fixed-width", "x": 3, "y": 3 }
                  ]
                }
            ]
        }),
    );

    assert!(!run_wrap(&mut sink, "card"));
    assert_eq!(
        node_json(&sink, "headline")["width"],
        json!("fill_container")
    );
    assert_eq!(
        node_json(&sink, "headline")["textGrowth"],
        json!("fixed-width")
    );
}

#[test]
fn floating_overlay_text_is_left_alone_even_when_it_bleeds_past_the_edge() {
    // A text pinned directly on the board flow and a text inside a pinned
    // `layout:none` sticker layer both bleed past the right edge BY POSITION
    // — a floating layer's bleed can be authored intent (the badge
    // half-off-canvas look), so neither may wrap.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [0, 60],
            "children": [
                { "type": "text", "id": "pinned",
                  "content": "让文字立刻高级的 5 个排版法则",
                  "fontSize": 88, "width": "fit_content", "height": "fit_content",
                  "textGrowth": "auto", "x": 1000, "y": 100 },
                { "type": "frame", "id": "sticker", "name": "贴纸", "layout": "none",
                  "width": "fit_content", "height": "fit_content", "x": 1000, "y": 200,
                  "children": [
                    { "type": "text", "id": "sticker-text",
                      "content": "让文字立刻高级的 5 个排版法则",
                      "fontSize": 88, "width": "fit_content", "height": "fit_content",
                      "textGrowth": "auto", "x": 0, "y": 0 }
                  ]
                }
            ]
        }),
    );

    assert!(
        !run_wrap(&mut sink, "card"),
        "a floating overlay's bleed is authored intent"
    );
    for id in ["pinned", "sticker-text"] {
        assert_eq!(node_json(&sink, id)["width"], json!("fit_content"), "{id}");
        assert_eq!(node_json(&sink, id)["textGrowth"], json!("auto"), "{id}");
    }
}

#[test]
fn a_short_fit_content_title_is_left_alone() {
    // A title that fits inside the board never had clipped glyphs — the
    // predicate is the resolved right edge, not the fit_content keyword.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [0, 60],
            "children": [
                { "type": "text", "id": "title", "content": "排版法则",
                  "fontSize": 88, "width": "fit_content", "height": "fit_content",
                  "textGrowth": "auto" }
            ]
        }),
    );

    assert!(!run_wrap(&mut sink, "card"));
    assert_eq!(node_json(&sink, "title")["width"], json!("fit_content"));
    assert_eq!(node_json(&sink, "title")["textGrowth"], json!("auto"));
}

#[test]
fn a_mobile_screen_is_not_a_board() {
    // The phone edge-to-edge contract is legal — the wrap gate is Card/Deck.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "phone", "name": "Home",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "text", "id": "title",
                  "content": "让文字立刻高级的 5 个排版法则",
                  "fontSize": 88, "width": "fit_content", "height": "fit_content",
                  "textGrowth": "auto" }
            ]
        }),
    );

    assert!(!run_wrap(&mut sink, "phone"));
    assert_eq!(node_json(&sink, "title")["width"], json!("fit_content"));
    assert_eq!(node_json(&sink, "title")["textGrowth"], json!("auto"));
}

#[test]
fn a_square_page_is_not_a_board() {
    // 1080×1080 reads as Page, not Card — the board gate excludes it.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "square", "name": "方版",
            "width": 1080, "height": 1080, "layout": "vertical",
            "children": [
                { "type": "text", "id": "title",
                  "content": "让文字立刻高级的 5 个排版法则",
                  "fontSize": 88, "width": "fit_content", "height": "fit_content",
                  "textGrowth": "auto" }
            ]
        }),
    );

    assert!(!run_wrap(&mut sink, "square"));
    assert_eq!(node_json(&sink, "title")["width"], json!("fit_content"));
}

#[test]
fn a_deck_board_gets_the_wrap_too() {
    // The gate is Card AND Deck: a 1920×1080 slide whose headline band holds
    // the same clipped-glyph proof. The band shape is deliberate — an
    // in-flow fit_content text makes the fixed root GROW to fit it (the
    // engine's grow-to-content behaviour), while the layout:none band's
    // pinned texts bleed past the inner edge exactly like the measured card.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "deck", "name": "Slide",
            "width": 1920, "height": 1080, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "header", "name": "Head", "layout": "vertical",
                  "width": "fill_container", "height": "fit_content", "padding": [96, 80],
                  "children": [
                    { "type": "frame", "id": "headline-wrap", "name": "headline-wrap",
                      "width": "fill_container", "height": 120, "layout": "none",
                      "children": [
                        { "type": "text", "id": "headline-main",
                          "content": "让文字立刻高级的 5 个排版法则让文字立刻高级的 5 个排版法则让文字立刻高级的 5 个排版法则让文字立刻高级",
                          "fontSize": 88, "width": "fit_content", "height": "fit_content",
                          "textGrowth": "auto", "x": 0, "y": 0 }
                      ]
                    }
                  ]
                }
            ]
        }),
    );

    assert!(run_wrap(&mut sink, "deck"), "the deck headline must wrap");
    assert_eq!(
        node_json(&sink, "headline-main")["width"],
        json!("fill_container")
    );
    assert_eq!(
        node_json(&sink, "headline-main")["textGrowth"],
        json!("fixed-width")
    );
}

// ── Driver mounting ──────────────────────────────────────────────────────────

#[test]
fn the_driver_attributes_the_wrap_to_the_board_text_wrap_checkpoint() {
    // The full driver: the geometry loop skips the layout:none headline band
    // (its parent is not a flex container, the measured reason the loop
    // never caught this lesion), so the wrap pass is the pass that fires —
    // and it must be checkpointed as `board-text-wrap` under Overflow.
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &p2c_lesion_card());
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "card".to_string(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(&mut sink, &plan, &["card"], &mut summary);

    for id in ["headline-main", "headline-shadow"] {
        assert_eq!(
            node_json(&sink, id)["width"],
            json!("fill_container"),
            "{id}"
        );
        assert_eq!(
            node_json(&sink, id)["textGrowth"],
            json!("fixed-width"),
            "{id}"
        );
    }
    assert!(
        summary.records().iter().any(|record| {
            record.pass == "board-text-wrap" && record.category == CheckCategory::Overflow
        }),
        "the wrap must be mounted and checkpointed as board-text-wrap under \
         Overflow: {:?}",
        summary.records()
    );
}
