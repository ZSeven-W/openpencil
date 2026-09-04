//! Tests for `cleanup_slide_padding::enforce_slide_padding_floor` (DS P1-a,
//! pass 3): a deck whose content sits flush against the root edge gets the
//! 64px horizontal safe-margin floor; a card gets the 48px floor — and its
//! VERTICAL edges are lifted PER EDGE on their own flush evidence (DS P2-b A
//! top, DS P2-c A bottom). Everything that is not a provably margin-less
//! deck/card is left alone.

use super::*;
use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
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

fn run_pass(sink: &mut VecDocSink, root_id: &str) -> bool {
    enforce_slide_padding_floor(sink, root_id)
}

fn run_driver(sink: &mut VecDocSink, root_id: &str) -> RepairSummary {
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: root_id.to_string(),
            name: "Deck".into(),
            width: 1920.0,
            height: 1080.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(sink, &plan, &[root_id], &mut summary);
    summary
}

/// The measured 0814 shape: a deck page whose title is flush against the
/// canvas left edge with zero root padding.
fn flush_title_deck() -> serde_json::Value {
    json!({
        "type": "frame",
        "id": "board",
        "name": "Slide 3",
        "width": 1920,
        "height": 1080,
        "layout": "vertical",
        "children": [
            { "type": "text", "id": "title", "content": "标题", "fontSize": 64 },
            { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
              "width": 800, "height": 400,
              "children": [{ "type": "text", "id": "body-text", "content": "正文", "fontSize": 28 }] }
        ]
    })
}

#[test]
fn a_flush_title_on_a_zero_margin_deck_gets_the_floor() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &flush_title_deck());

    assert!(run_pass(&mut sink, "board"), "the flush title must trigger");
    assert_eq!(
        root_json(&sink)["padding"],
        json!([0.0, 64.0]),
        "horizontal padding must rise to the floor, vertical stays untouched"
    );
}

#[test]
fn a_full_bleed_hero_page_does_not_trigger() {
    // The only child is a root-sized background image: decoration, not
    // content, so there is no content edge violation to prove.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "board", "name": "Hero",
            "width": 1920, "height": 1080, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "bg", "name": "Background", "layout": "none",
                  "width": 1920, "height": 1080,
                  "fill": [{ "type": "image", "url": "https://example.com/hero.jpg" }] }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "board"),
        "a hero image is not a margin defect"
    );
    assert!(root_json(&sink).get("padding").is_none());
}

#[test]
fn a_full_bleed_background_does_not_block_repairing_flush_content() {
    // The background layer spans the root; the flush title still proves the
    // margin is missing, and the layer must not stop the repair.
    let mut sink = VecDocSink::new();
    let mut tree = flush_title_deck();
    tree["children"]
        .as_array_mut()
        .expect("children")
        .push(json!({
            "type": "frame", "id": "bg", "name": "Background", "layout": "none",
            "x": 0, "y": 0, "width": 1920, "height": 1080,
            "fill": [{ "type": "image", "url": "https://example.com/hero.jpg" }]
        }));
    insert_tree(&mut sink, &tree);

    assert!(run_pass(&mut sink, "board"));
    assert_eq!(root_json(&sink)["padding"], json!([0.0, 64.0]));
}

#[test]
fn an_adequate_margin_is_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "board", "name": "Slide",
            "width": 1920, "height": 1080, "layout": "vertical",
            "padding": 80,
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "board"),
        "80px already clears the floor"
    );
    assert_eq!(root_json(&sink)["padding"], json!(80.0));
}

#[test]
fn in_flow_content_respecting_a_small_padding_is_left_alone() {
    // 32px padding keeps the title 32px from the edge — above the 24px
    // violation band — so there is no geometry proof of a defect even though
    // the padding is below the floor. Narrow predicate: only flush content
    // proves a missing margin.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "board", "name": "Slide",
            "width": 1920, "height": 1080, "layout": "vertical",
            "padding": 32,
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
            ]
        }),
    );

    assert!(!run_pass(&mut sink, "board"));
    assert_eq!(root_json(&sink)["padding"], json!(32.0));
}

#[test]
fn a_mobile_screen_is_not_a_deck() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "phone", "name": "Home",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 32 }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "phone"),
        "the deck gate must exclude a phone"
    );
    assert!(root_json(&sink).get("padding").is_none());
}

// ── Card gate (DS P1.5) ─────────────────────────────────────────────────────

#[test]
fn a_flush_title_on_a_zero_margin_card_gets_the_48_floor() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 },
                { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
                  "width": 800, "height": 400,
                  "children": [{ "type": "text", "id": "body-text", "content": "正文", "fontSize": 28 }] }
            ]
        }),
    );

    assert!(run_pass(&mut sink, "card"), "the flush title must trigger");
    assert_eq!(
        root_json(&sink)["padding"],
        json!([48.0, 48.0, 0.0, 48.0]),
        "the card floor is 48 per proven edge: the flush title proves the \
         top and horizontal margins missing (top + left/right rise), but the \
         content ends 464px short of the bottom, so the bottom stays 0"
    );
}

#[test]
fn a_card_margin_above_the_48_floor_is_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": 60,
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "card"),
        "60px already clears the card floor"
    );
    assert_eq!(root_json(&sink)["padding"], json!(60.0));
}

// ── Card vertical floor (DS P2-b A) ─────────────────────────────────────────

#[test]
fn a_flush_masthead_on_a_card_gets_the_vertical_floor_without_touching_horizontal() {
    // The 0815 lesion: horizontal margins are fine (60 >= 48) but the
    // masthead sits against the board top — only TOP rises (per-edge
    // semantics, DS P2-c A), the bottom has no flush evidence and stays 0.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [0, 60],
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
            ]
        }),
    );

    assert!(
        run_pass(&mut sink, "card"),
        "the flush masthead must trigger"
    );
    assert_eq!(
        root_json(&sink)["padding"],
        json!([48.0, 60.0, 0.0, 60.0]),
        "only the top edge rises to 48 — horizontal stays at its authored 60 \
         and the unproven bottom stays 0"
    );
    assert!(
        !run_pass(&mut sink, "card"),
        "the vertical floor is idempotent"
    );
}

#[test]
fn a_card_masthead_respecting_the_24px_band_keeps_small_vertical_padding() {
    // 32px of vertical padding keeps the masthead 32px from the top — above
    // the 24px violation band — so no geometry proves the margin missing,
    // even though the padding is below the floor. Same narrow predicate as
    // the horizontal side.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [32, 60],
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
            ]
        }),
    );

    assert!(!run_pass(&mut sink, "card"));
    assert_eq!(root_json(&sink)["padding"], json!([32.0, 60.0]));
}

#[test]
fn a_flush_masthead_on_a_deck_gets_no_vertical_floor() {
    // The deck's vertical composition belongs to the centre pass — the
    // vertical floor is card-only, so a deck with flush content and no
    // vertical padding must stay untouched on the vertical axis.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "board", "name": "Slide",
            "width": 1920, "height": 1080, "layout": "vertical",
            "padding": [0, 64],
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "board"),
        "no horizontal violation and no deck vertical floor — nothing to do"
    );
    assert_eq!(root_json(&sink)["padding"], json!([0.0, 64.0]));
}

#[test]
fn the_vertical_floor_drills_through_a_full_width_section_to_its_masthead() {
    // A full-width section spans the board by construction; its own flush
    // top is structural. The text INSIDE it is what proves the missing
    // vertical margin (the P1.5 drill-down semantics, on the y axis).
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "padding": [0, 60],
            "children": [
                { "type": "frame", "id": "s1", "name": "Cover", "layout": "vertical",
                  "width": "fill_container", "height": 120,
                  "children": [{ "type": "text", "id": "s1-t", "content": "标题", "fontSize": 28 }] }
            ]
        }),
    );

    assert!(
        run_pass(&mut sink, "card"),
        "the masthead inside the fill_container section is flush against the top"
    );
    assert_eq!(root_json(&sink)["padding"], json!([48.0, 60.0, 0.0, 60.0]));
}

#[test]
fn a_square_board_is_not_a_card() {
    // 1080x1080 reads as Page, not Card — the fixed-board floor does not
    // apply to a scroll page.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "square", "name": "方版",
            "width": 1080, "height": 1080, "layout": "vertical",
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "square"),
        "a square is not a card board"
    );
    assert!(root_json(&sink).get("padding").is_none());
}

#[test]
fn a_mobile_screen_is_not_a_card() {
    // The phone edge-to-edge contract is legal — flush content proves
    // nothing on a 390 board.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "phone", "name": "Home",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 32 }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "phone"),
        "the card gate must exclude a phone"
    );
    assert!(root_json(&sink).get("padding").is_none());
}

/// The 0815 lesion through the FULL driver: sections whose group max margin
/// is only 20px, with no 2/3 majority on any edge (so the equalize pass has
/// no vote to swing). `unify-section-margins` raises every section to [0,20]
/// first, and content still sits < 24px from the canvas edge — the card
/// floor (48) then catches the rest at the ROOT. This is the exact
/// "normalize first, floor as the backstop" order.
#[test]
fn a_card_with_small_unified_section_margins_gets_the_floor_backstop() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "s1", "name": "法则 01", "width": "fill_container",
                  "height": 120, "layout": "vertical", "padding": [0, 20],
                  "children": [{ "type": "text", "id": "s1-t", "content": "一", "fontSize": 28 }] },
                { "type": "frame", "id": "s2", "name": "法则 02", "width": "fill_container",
                  "height": 120, "layout": "vertical", "padding": [0, 8],
                  "children": [{ "type": "text", "id": "s2-t", "content": "二", "fontSize": 28 }] },
                { "type": "frame", "id": "s3", "name": "法则 03", "width": "fill_container",
                  "height": 120, "layout": "vertical",
                  "children": [{ "type": "text", "id": "s3-t", "content": "三", "fontSize": 28 }] }
            ]
        }),
    );
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

    assert_eq!(
        root_json(&sink)["padding"],
        json!([48.0, 48.0, 0.0, 48.0]),
        "the card floor must backstop the 20px section norm at the ROOT — \
         horizontally (the section texts sit flush against the left edge too) \
         and on TOP (the first masthead is flush against the board top). The \
         bottom has no flush evidence and stays 0"
    );
    for pass in ["unify-section-margins", "slide-padding-floor"] {
        assert!(
            summary.records().iter().any(|record| record.pass == pass),
            "pass {pass:?} must fire in order: {:?}",
            summary.records()
        );
    }
}

#[test]
fn the_pass_is_idempotent() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &flush_title_deck());
    assert!(run_pass(&mut sink, "board"));
    assert!(
        !run_pass(&mut sink, "board"),
        "the second run must find the floor already in place"
    );
}

/// Defect ① regression + P2-b A: a card whose sections ALREADY own a
/// uniform [0,80] HORIZONTAL margin (the state `unify-section-margins`
/// produces one pass earlier) keeps that delegation — the fill_container
/// section frames span the board by construction, so their own flush frame
/// edges must not prove anything horizontally. But those sections carry NO
/// vertical padding, and their masthead texts sit flush against the board
/// top: the P2-b vertical floor reads that as the 0815 lesion and raises
/// the ROOT's vertical pair to 48 — the root owns the vertical margin, the
/// sections keep the horizontal one.
#[test]
fn unified_section_margins_leave_no_flush_for_the_floor_to_see() {
    let section = |id: &str| {
        json!({
            "type": "frame", "id": id, "name": id,
            "width": "fill_container", "height": 120, "layout": "vertical",
            "padding": [0, 80],
            "children": [{ "type": "text", "id": format!("{id}-t"), "content": "标题", "fontSize": 28 }]
        })
    };
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical", "gap": 20,
            "children": [section("s1"), section("s2"), section("s3")]
        }),
    );

    assert!(
        run_pass(&mut sink, "card"),
        "the masthead flush against the board top is a missing VERTICAL margin"
    );
    assert_eq!(
        root_json(&sink)["padding"],
        json!([48.0, 0.0, 0.0, 0.0]),
        "only TOP rises to the floor (per-edge semantics): the delegated \
         horizontal margin ownership is untouched and the bottom has no \
         flush evidence"
    );
}

// ── Card bottom floor (DS P2-c A) ───────────────────────────────────────────

/// The P2-c footer shape: a card whose header owns the 96px head gap
/// (`[96, 80]` padding) and whose tail section owns the horizontal margin
/// (`[0, 80]`) but no bottom padding — the footer is flush against the
/// board's bottom edge. The tail fills the remaining board height so the
/// flush is deterministic regardless of the measure backend.
fn footer_card(
    root_padding: serde_json::Value,
    tail_padding: serde_json::Value,
) -> serde_json::Value {
    json!({
        "type": "frame", "id": "card", "name": "知识卡片",
        "width": 1080, "height": 1440, "layout": "vertical",
        "padding": root_padding,
        "children": [
            { "type": "frame", "id": "header", "name": "卡片头部", "layout": "vertical",
              "width": "fill_container", "height": "fit_content", "padding": [96, 80],
              "children": [
                { "type": "text", "id": "title", "content": "标题", "fontSize": 64 }
              ]
            },
            { "type": "frame", "id": "tail", "name": "卡片尾部", "layout": "vertical",
              "width": "fill_container", "height": "fill_container", "padding": tail_padding,
              "justifyContent": "end",
              "children": [
                { "type": "text", "id": "footer", "content": "关注我，每天一个设计小技巧",
                  "fontSize": 28 }
              ]
            }
        ]
    })
}

#[test]
fn a_flush_footer_on_a_card_lifts_only_the_bottom_edge() {
    // The P2-c lesion: no root padding at all; the header section owns the
    // 96px head gap ([96, 80]) and the tail section owns the horizontal
    // margin ([0, 80]) but no bottom padding — the footer is flush against
    // the board's bottom edge. Per-edge semantics: only the bottom edge
    // rises — symmetric lifting would blow the composed 96px head gap to
    // 144 and destroy it, and the horizontal margin is delegated to the
    // sections on purpose.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &footer_card(serde_json::Value::Null, json!([0, 80])),
    );

    assert!(run_pass(&mut sink, "card"), "the flush footer must trigger");
    assert_eq!(
        root_json(&sink)["padding"],
        json!([0.0, 0.0, 48.0, 0.0]),
        "only the bottom edge rises to 48: the 96px head gap is a \
         composition and the 80px horizontal margin stays owned by the \
         sections"
    );
    assert!(
        !run_pass(&mut sink, "card"),
        "the bottom floor is idempotent"
    );
}

#[test]
fn a_flush_footer_on_a_deck_gets_no_vertical_floor() {
    // The deck's vertical composition belongs to the centre pass — the
    // vertical floor is card-only, so a deck footer flush against the
    // bottom edge must stay untouched on the vertical axis.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "board", "name": "Slide",
            "width": 1920, "height": 1080, "layout": "vertical",
            "padding": [0, 64],
            "children": [
                { "type": "frame", "id": "header", "name": "Head", "layout": "vertical",
                  "width": "fill_container", "height": "fit_content", "padding": [96, 80],
                  "children": [{ "type": "text", "id": "title", "content": "标题", "fontSize": 64 }] },
                { "type": "frame", "id": "tail", "name": "Tail", "layout": "vertical",
                  "width": "fill_container", "height": "fill_container", "padding": [0, 80],
                  "justifyContent": "end",
                  "children": [{ "type": "text", "id": "footer", "content": "页脚", "fontSize": 28 }] }
            ]
        }),
    );

    assert!(
        !run_pass(&mut sink, "board"),
        "no deck vertical floor — nothing to do"
    );
    assert_eq!(root_json(&sink)["padding"], json!([0.0, 64.0]));
}

#[test]
fn a_card_footer_with_a_48_px_margin_owned_by_the_tail_section_is_left_alone() {
    // The tail section's own [0, 80, 48, 80] padding keeps the footer 48px
    // off the board bottom — outside the 24px evidence band, so no geometry
    // proves a missing margin and the root's padding stays untouched.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &footer_card(serde_json::Value::Null, json!([0, 80, 48, 80])),
    );

    assert!(!run_pass(&mut sink, "card"));
    assert!(root_json(&sink).get("padding").is_none());
}

#[test]
fn a_card_bottom_padding_already_at_the_floor_never_rises_even_with_flush_content() {
    // Content overflows the fixed board, so the footer's bottom edge is
    // provably past the board bottom — yet the root already carries the 48px
    // bottom floor: `max(current, 48)` only lifts, and the at-floor edge is
    // not gated in at all. The pass must stand down, not re-patch.
    let mut tree = footer_card(json!([0, 0, 48, 0]), json!([0, 80]));
    tree["children"][0]["height"] = json!(1450); // header taller than the board
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &tree);

    assert!(!run_pass(&mut sink, "card"));
    assert_eq!(root_json(&sink)["padding"], json!([0.0, 0.0, 48.0, 0.0]));
}

#[test]
fn driver_attributes_the_repair_to_the_slide_padding_checkpoint() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &flush_title_deck());
    let summary = run_driver(&mut sink, "board");

    assert!(
        summary.records().iter().any(|record| {
            record.pass == "slide-padding-floor" && record.category == CheckCategory::Layout
        }),
        "the pass must be mounted and checkpointed in the driver: {:?}",
        summary.records()
    );
}
