//! Resolved-rect coverage for the single-line text-fit cleanup repair.

use super::*;
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::{json, Value};

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn update_font_size(cmds: &[EditorCommand], id: &str) -> Option<f32> {
    cmds.iter().find_map(|cmd| match cmd {
        EditorCommand::SetNodeFontSize { node_id, font_size } if node_id.as_str() == id => {
            Some(*font_size)
        }
        _ => None,
    })
}

fn collect(card: Value, rects: HashMap<String, Rect>) -> Vec<EditorCommand> {
    let mut cmds = Vec::new();
    collect_text_fit_fixes(&card, &rects, &mut cmds);
    cmds
}

fn amount_card(text: Value) -> (Value, HashMap<String, Rect>) {
    (
        json!({
            "type": "frame",
            "id": "card",
            "width": 327,
            "height": 100,
            "clipContent": true,
            "children": [text]
        }),
        HashMap::from([
            ("card".to_string(), rect(0.0, 0.0, 327.0, 100.0)),
            ("text".to_string(), rect(0.0, 0.0, 370.0, 52.0)),
        ]),
    )
}

fn estimated_card(text: Value) -> (Value, HashMap<String, Rect>) {
    (
        json!({
            "type": "frame",
            "id": "card",
            "width": 327,
            "height": 100,
            "clipContent": true,
            "padding": [0, 28.5],
            "children": [text]
        }),
        HashMap::from([("card".to_string(), rect(0.0, 0.0, 327.0, 100.0))]),
    )
}

#[test]
fn measured_52px_amount_shrinks_to_the_proportional_floor() {
    let (card, rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "$52,480.16",
        "fontSize": 52,
        "fontWeight": 600,
        "textGrowth": "auto"
    }));

    let cmds = collect(card, rects);
    assert_eq!(update_font_size(&cmds, "text"), Some(45.0));
}

#[test]
fn a_17px_label_that_fits_is_untouched() {
    let (card, mut rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "Paid",
        "fontSize": 17,
        "textGrowth": "auto"
    }));
    rects.insert("text".to_string(), rect(0.0, 0.0, 100.0, 20.0));

    assert!(collect(card, rects).is_empty());
}

#[test]
fn wrapped_paragraph_is_untouched() {
    let (card, rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "A paragraph that intentionally wraps.",
        "fontSize": 40,
        "textGrowth": "fixed-width"
    }));

    assert!(collect(card, rects).is_empty());
}

#[test]
fn app19_unbreakable_amount_uses_the_estimate_and_fits_at_41px() {
    let (card, rects) = estimated_card(json!({
        "type": "text",
        "id": "text",
        "content": "1,286,430.52",
        "fontFamily": "DM Mono",
        "fontSize": 44,
        "fontWeight": 600,
        "letterSpacing": -0.5,
        "textGrowth": "fixed-width"
    }));

    let cmds = collect(card, rects);
    assert_eq!(update_font_size(&cmds, "text"), Some(41.0));
    assert!(estimate_unbreakable_text_width(&json!({"content": "1,286,430.52"}), 41.0) <= 270.0);
}

#[test]
fn fixed_width_sentence_with_spaces_is_untouched() {
    let (card, mut rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "A fixed width sentence",
        "fontSize": 44,
        "textGrowth": "fixed-width"
    }));
    rects.remove("text");

    assert!(collect(card, rects).is_empty());
}

#[test]
fn cjk_fixed_width_text_is_untouched() {
    let (card, mut rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "总资产数字",
        "fontSize": 44,
        "textGrowth": "fixed-width-height"
    }));
    rects.remove("text");

    assert!(collect(card, rects).is_empty());
}

#[test]
fn an_unbreakable_token_that_fits_is_untouched() {
    let (card, mut rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "1,286,430.52",
        "fontSize": 44,
        "textGrowth": "fixed-width-height"
    }));
    rects.remove("text");

    assert!(collect(card, rects).is_empty());
}

#[test]
fn text_inside_a_horizontal_scroller_is_untouched() {
    let card = json!({
        "type": "frame",
        "id": "viewport",
        "width": 327,
        "height": 100,
        "layout": "horizontal",
        "clipContent": true,
        "children": [{
            "type": "text",
            "id": "text",
            "content": "Scrollable amount",
            "fontSize": 52,
            "textGrowth": "auto"
        }]
    });
    let rects = HashMap::from([
        ("viewport".to_string(), rect(0.0, 0.0, 327.0, 100.0)),
        ("text".to_string(), rect(0.0, 0.0, 370.0, 52.0)),
    ]);

    assert!(collect(card, rects).is_empty());
}

#[test]
fn text_that_cannot_fit_at_24px_is_untouched() {
    let (card, rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "An exceptionally long amount",
        "fontSize": 52,
        "textGrowth": "auto"
    }));
    let mut rects = rects;
    rects.insert("text".to_string(), rect(0.0, 0.0, 800.0, 52.0));

    assert!(collect(card, rects).is_empty());
}

#[test]
fn cleanup_records_the_repair_under_text_fit() {
    let tree = json!({
        "type": "frame",
        "id": "root",
        "width": 327,
        "height": 100,
        "clipContent": true,
        "children": [{
            "type": "text",
            "id": "text",
            "content": "$52,480.16",
            "width": 370,
            "height": 52,
            "fontSize": 52,
            "textGrowth": "auto"
        }]
    });
    let node: PenNode = serde_json::from_value(tree).expect("fixture parses");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(&mut sink);
    assert_eq!(repair_text_fit(&mut counting, "root"), 1);
    let mut summary = RepairSummary::default();
    counter.checkpoint(&mut summary, CheckCategory::Overflow, "text-fit");

    let text = serde_json::to_value(&counting.state().active_children()[0]).expect("serialize");
    let font_size = text["children"][0]["fontSize"].as_f64();
    assert_eq!(font_size, Some(45.0));
    assert_eq!(summary.repairs_for(CheckCategory::Overflow), 1);
    assert_eq!(summary.records()[0].pass, "text-fit");
}

#[test]
fn status_bar_subtree_is_untouched() {
    let card = json!({
        "type": "frame",
        "id": "status-bar",
        "role": "status-bar",
        "width": 327,
        "height": 100,
        "children": [{
            "type": "text",
            "id": "text",
            "content": "09:41",
            "fontSize": 52,
            "textGrowth": "auto"
        }]
    });
    let rects = HashMap::from([
        ("status-bar".to_string(), rect(0.0, 0.0, 327.0, 100.0)),
        ("text".to_string(), rect(0.0, 0.0, 370.0, 52.0)),
    ]);

    assert!(collect(card, rects).is_empty());
}

/// arena-m03 minimized: a 48px fixed-width "¥ 268,540.32" hero amount in a
/// 327px card with 14px padding. The layout wrapped it after the currency
/// symbol ("¥" / "268,540.32"); the measured amount path must shrink it back
/// onto one line inside the width it was given. The fixture narrows the
/// screen to 300px so the wrap reproduces even where "DM Mono" is missing
/// and a narrower fallback face measures the amount (CI runners).
#[test]
fn wrapped_currency_amount_is_measured_back_onto_one_line() {
    let tree = json!({
        "type": "frame", "id": "root", "width": 300, "height": 400,
        "layout": "vertical", "padding": [0, 24],
        "children": [{
            "type": "frame", "id": "card", "width": "fill_container",
            "height": "fit_content", "layout": "vertical", "gap": 8, "padding": 14,
            "children": [{
                "type": "text", "id": "amount", "content": "¥ 268,540.32",
                "fontFamily": "DM Mono", "fontSize": 48, "fontWeight": 700,
                "letterSpacing": -0.96, "lineHeight": 1.15,
                "width": "fill_container", "height": "fit_content",
                "textGrowth": "fixed-width"
            }]
        }]
    });
    let node: PenNode = serde_json::from_value(tree).expect("fixture parses");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let before = resolved_rects(sink.state());
    assert!(
        before["amount"].h > 48.0 * 1.15 * 1.5,
        "fixture must reproduce the wrap, got h={}",
        before["amount"].h
    );

    assert_eq!(repair_text_fit(&mut sink, "root"), 1);

    let after = resolved_rects(sink.state());
    let root = serde_json::to_value(&sink.state().active_children()[0]).expect("serialize");
    let amount = &root["children"][0]["children"][0];
    let size = amount["fontSize"].as_f64().expect("font size");
    assert!((24.0..48.0).contains(&size), "shrunk, got {size}");
    assert!(
        after["amount"].h < size * 1.15 * 1.5,
        "the amount sits on one line again, got h={} at {size}px",
        after["amount"].h
    );
    assert_eq!(
        amount["textGrowth"],
        json!("fixed-width"),
        "only the size changes — the authored width mode stays"
    );
}

#[test]
fn amount_token_recognizes_figures_but_not_prose() {
    for yes in [
        "¥ 268,540.32",
        "+¥1,286.40",
        "-0.86%",
        "$52,480.16",
        "1 234 567",
        "€12.5",
    ] {
        assert!(amount::is_amount_token(yes), "{yes} is an amount");
    }
    for no in [
        "近1月",
        "Q3 2025",
        "12 34",
        "¥",
        "268,540.32 元",
        "Total 12",
        "06/30",
    ] {
        assert!(!amount::is_amount_token(no), "{no} is not an amount");
    }
}

/// arena-m03's hero-card stats row: two `fill_container` blocks and a hug
/// chip. The 今日收益 amount is a `fill_container` fixed-width text next to
/// a 14px icon; taffy keeps the text leaf at its min-content width, so it
/// spills out of its block over the 累计收益 block while its own rect equals
/// its natural width. `root_width` narrows the row.
fn stats_row_screen(root_width: f64) -> VecDocSink {
    let tree = json!({
        "type": "frame", "id": "root", "width": root_width, "height": 300,
        "layout": "vertical", "padding": [0, 24],
        "children": [{
            "type": "frame", "id": "card", "width": "fill_container",
            "height": "fit_content", "layout": "vertical", "padding": 24,
            "children": [{
                "type": "frame", "id": "stats-row", "width": "fill_container",
                "layout": "horizontal", "gap": 8, "alignItems": "end",
                "children": [
                    {
                        "type": "frame", "id": "today", "width": "fill_container",
                        "height": "fit_content", "layout": "vertical", "gap": 4,
                        "children": [
                            {"type": "text", "id": "today-label", "content": "今日收益",
                             "fontFamily": "DM Sans", "fontSize": 12, "lineHeight": 1.5},
                            {
                                "type": "frame", "id": "today-value", "width": "fill_container",
                                "layout": "horizontal", "gap": 4, "alignItems": "center",
                                "children": [
                                    {"type": "icon_font", "id": "trend", "iconFontName": "trending-up",
                                     "iconFontFamily": "lucide", "width": 14, "height": 14},
                                    {"type": "text", "id": "today-amount", "content": "+1,286.40",
                                     "fontFamily": "DM Mono", "fontSize": 16, "fontWeight": 500,
                                     "letterSpacing": 0.5, "lineHeight": 1.5,
                                     "width": "fill_container", "height": "fit_content",
                                     "textGrowth": "fixed-width"}
                                ]
                            }
                        ]
                    },
                    {
                        "type": "frame", "id": "total", "width": "fill_container",
                        "height": "fit_content", "layout": "vertical", "gap": 4,
                        "children": [
                            {"type": "text", "id": "total-label", "content": "累计收益",
                             "fontFamily": "DM Sans", "fontSize": 12, "lineHeight": 1.5},
                            {"type": "text", "id": "total-amount", "content": "+38,640.18",
                             "fontFamily": "DM Mono", "fontSize": 14, "fontWeight": 500,
                             "letterSpacing": 0.5, "lineHeight": 1.5}
                        ]
                    },
                    {
                        "type": "frame", "id": "chip", "width": "fit_content",
                        "height": "fit_content", "layout": "horizontal", "padding": [6, 12],
                        "children": [{"type": "text", "id": "chip-text", "content": "持仓 6 只",
                                      "fontFamily": "DM Sans", "fontSize": 12}]
                    }
                ]
            }]
        }]
    });
    let node: PenNode = serde_json::from_value(tree).expect("fixture parses");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink
}

fn node_font_size(sink: &VecDocSink, id: &str) -> Option<f64> {
    fn find<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
        if v.get("id").and_then(Value::as_str) == Some(id) {
            return Some(v);
        }
        v.get("children")
            .and_then(Value::as_array)?
            .iter()
            .find_map(|c| find(c, id))
    }
    let root = serde_json::to_value(&sink.state().active_children()[0]).ok()?;
    find(&root, id)?.get("fontSize")?.as_f64()
}

#[test]
fn amount_spilling_past_its_row_share_shrinks_until_the_blocks_stop_overlapping() {
    // 362px narrows the row to 266px so the spill reproduces under a
    // fallback face as well as DM Mono.
    let mut sink = stats_row_screen(362.0);
    let before = resolved_rects(sink.state());
    let (amount, block, next) = (
        before["today-amount"],
        before["today-value"],
        before["total"],
    );
    assert!(
        amount.x + amount.w > next.x,
        "fixture must reproduce the overlap: amount ends at {}, 累计收益 starts at {}",
        amount.x + amount.w,
        next.x
    );
    assert!(
        amount.x + amount.w > block.x + block.w,
        "the text spills out of its own block"
    );

    assert_eq!(repair_text_fit(&mut sink, "root"), 1);

    let size = node_font_size(&sink, "today-amount").expect("font size");
    assert!((12.0..16.0).contains(&size), "shrunk, got {size}");
    let after = resolved_rects(sink.state());
    let (amount, today, next) = (after["today-amount"], after["today"], after["total"]);
    assert!(
        amount.x + amount.w <= next.x,
        "the amount ends at {} before 累计收益 starts at {}",
        amount.x + amount.w,
        next.x
    );
    assert!(
        today.x + today.w <= next.x,
        "the blocks no longer overlap: {today:?} vs {next:?}"
    );
    assert!(
        amount.x + amount.w <= after["today-value"].x + after["today-value"].w + 0.5,
        "the text ends inside its own row"
    );
}

#[test]
fn amount_that_fits_its_row_share_is_untouched() {
    let mut sink = stats_row_screen(600.0);
    let before = resolved_rects(sink.state());
    assert!(
        before["today-amount"].x + before["today-amount"].w
            <= before["today-value"].x + before["today-value"].w,
        "fixture fits its share"
    );

    assert_eq!(repair_text_fit(&mut sink, "root"), 0);
    assert_eq!(node_font_size(&sink, "today-amount"), Some(16.0));
}
