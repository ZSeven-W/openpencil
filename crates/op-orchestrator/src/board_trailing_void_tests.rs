//! Tests for `board_trailing_void::collect_board_trailing_void` (DS P2-b C):
//! fixed Card/Deck boards whose trailing void is still >= 25% of the board
//! height after the cleanup passes report one read-only advisory naming the
//! void percentage; full boards and non-board roots stay silent.

use super::*;
use crate::test_support::VecDocSink;
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

#[test]
fn a_sparse_card_board_reports_a_40_percent_void_advisory() {
    // 864px of content on a 1440px board: exactly 40% trailing void — the
    // advisory must name the percentage and point at the density fix.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
                  "width": 900, "height": 864,
                  "children": [{ "type": "text", "id": "t", "content": "正文", "fontSize": 28 }] }
            ]
        }),
    );

    let advisories = collect_board_trailing_void(&sink.state);
    assert_eq!(
        advisories.len(),
        1,
        "exactly one void advisory: {advisories:?}"
    );
    let advisory = &advisories[0];
    assert_eq!(advisory.code, "board-trailing-void");
    assert_eq!(advisory.node_ids, vec!["card".to_string()]);
    assert!(
        advisory.message.contains("40%")
            && advisory
                .message
                .contains("add content or scale up type/spacing"),
        "the message names the void and the fix direction: {}",
        advisory.message
    );
}

#[test]
fn a_full_card_board_reports_no_void_advisory() {
    // 1400px of content on a 1440px board: ~3% void — far below the 25%
    // advisory floor.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
                  "width": 1000, "height": 1400,
                  "children": [{ "type": "text", "id": "t", "content": "正文", "fontSize": 28 }] }
            ]
        }),
    );

    assert!(
        collect_board_trailing_void(&sink.state).is_empty(),
        "a full board must stay silent"
    );
}

#[test]
fn a_sparse_deck_board_reports_the_void_advisory_too() {
    // The advisory gate accepts both fixed-board forms: a 16:9 deck with a
    // 63% void is the same "content too sparse" finding.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "deck", "name": "Cover",
            "width": 1920, "height": 1080, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
                  "width": 1600, "height": 400,
                  "children": [{ "type": "text", "id": "t", "content": "x", "fontSize": 28 }] }
            ]
        }),
    );

    let advisories = collect_board_trailing_void(&sink.state);
    assert_eq!(advisories.len(), 1);
    assert_eq!(advisories[0].code, "board-trailing-void");
    assert_eq!(advisories[0].node_ids, vec!["deck".to_string()]);
}

#[test]
fn a_non_board_root_reports_no_void_advisory() {
    // A scrolling page has no fixed surface — sparse content there is a
    // different problem, not this one. 1200x3000 is taller than the card
    // band's 2:1 cap, so it reads as Page, never Card.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "page", "name": "Landing",
            "width": 1200, "height": 3000, "layout": "vertical",
            "children": [
                { "type": "text", "id": "t", "content": "正文", "fontSize": 28 }
            ]
        }),
    );

    assert!(collect_board_trailing_void(&sink.state).is_empty());
}

#[test]
fn an_authored_overlay_does_not_mask_the_void() {
    // A badge pinned near the board bottom is decoration, not content — the
    // void is measured against the real content bottom (P1.5 discipline).
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
                  "width": 900, "height": 600,
                  "children": [{ "type": "text", "id": "t", "content": "正文", "fontSize": 28 }] },
                { "type": "frame", "id": "badge", "name": "Badge", "layout": "none",
                  "x": 40, "y": 1400, "width": 200, "height": 40 }
            ]
        }),
    );

    let advisories = collect_board_trailing_void(&sink.state);
    assert_eq!(advisories.len(), 1, "the overlay must not mask the void");
    assert!(
        advisories[0].message.contains("58%"),
        "void stays measured against the content bottom: {}",
        advisories[0].message
    );
}

#[test]
fn a_full_bleed_background_neither_fills_nor_masks_the_void() {
    // A board-sized background layer is decoration: it must not count as
    // content (which would zero the void) — the sparse board still reports.
    // (The bg carries an authored x/y, so the walk skips it as an overlay
    // even though the test layout engine keeps it in the flow after the
    // body.)
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
                  "width": 900, "height": 600,
                  "children": [{ "type": "text", "id": "t", "content": "正文", "fontSize": 28 }] },
                { "type": "frame", "id": "bg", "name": "Background", "layout": "none",
                  "x": 0, "y": 0, "width": 1080, "height": 1440,
                  "fill": [{ "type": "solid", "color": "#000000" }] }
            ]
        }),
    );

    let advisories = collect_board_trailing_void(&sink.state);
    assert_eq!(advisories.len(), 1);
    assert!(
        advisories[0].message.contains("58%"),
        "the background must not zero the void: {}",
        advisories[0].message
    );
}

#[test]
fn a_board_sized_in_flow_child_alone_reports_nothing() {
    // A child that resolves to the full board size is whole-board decoration
    // even without an authored x/y — skipping it leaves no content evidence,
    // and an empty board is a "no content" problem, not a void one.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": 1440, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "bg", "name": "Background", "layout": "vertical",
                  "width": 1080, "height": 1440,
                  "fill": [{ "type": "solid", "color": "#000000" }] }
            ]
        }),
    );

    assert!(collect_board_trailing_void(&sink.state).is_empty());
}
