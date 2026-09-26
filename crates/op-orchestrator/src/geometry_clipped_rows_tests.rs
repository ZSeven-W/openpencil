//! Grow-to-fit for a clipped fixed-height card that truncates a table / list
//! (arena-m03, glm-5.3-flash: a 282px holdings table card showed two and a
//! half of its six rows and cut the last visible one mid-glyph).

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

fn with_ids(v: &mut serde_json::Value, next: &mut usize) {
    if let Some(obj) = v.as_object_mut() {
        obj.insert("id".into(), json!(format!("r{next}")));
        *next += 1;
        if let Some(children) = obj.get_mut("children").and_then(|c| c.as_array_mut()) {
            for child in children {
                with_ids(child, next);
            }
        }
    }
}

fn run_geometry(mut root: serde_json::Value) -> serde_json::Value {
    with_ids(&mut root, &mut 0);
    let root: PenNode = serde_json::from_value(root).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();
    geometry_validate_and_fix(&mut sink, &root_id);
    serde_json::to_value(sink.state().active_children()[0].clone()).unwrap()
}

fn find<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
    if v.get("name").and_then(|x| x.as_str()) == Some(name) {
        return Some(v);
    }
    v.get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
        .find_map(|c| find(c, name))
}

fn screen(children: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "type": "frame", "name": "Screen", "width": 375, "height": "fit_content",
        "layout": "vertical", "gap": 16, "padding": [0, 16],
        "children": children
    })
}

fn divider() -> serde_json::Value {
    json!({"type": "frame", "name": "row-divider", "width": "fill_container", "height": 1,
           "layout": "none", "fill": [{"type": "solid", "color": "#EEEEEE"}]})
}

fn cell(text: &str, width: serde_json::Value) -> serde_json::Value {
    json!({"type": "text", "content": text, "fontSize": 14, "width": width,
           "height": "fit_content", "textGrowth": "fixed-width"})
}

/// The m03 holdings row, minimized: a two-line name cell + three fixed cells.
fn holding_row(name: &str) -> serde_json::Value {
    json!({
        "type": "frame", "name": format!("holding-row-{name}"), "width": "fill_container",
        "height": "fit_content", "layout": "horizontal", "gap": 8, "padding": [12, 16],
        "alignItems": "center",
        "children": [
            {"type": "frame", "name": "name-cell", "width": "fill_container",
             "height": "fit_content", "layout": "vertical", "gap": 3,
             "children": [cell(name, json!("fill_container")), cell("股票", json!("fill_container"))]},
            cell("12,430.20", json!(64)),
            cell("58,622.44", json!(69)),
            cell("+2.35%", json!(56))
        ]
    })
}

/// The m03 table card: 282px, clipped, header + 6 rows with dividers between.
fn table_card(height: f64, rows: usize) -> serde_json::Value {
    let mut kids = vec![json!({
        "type": "frame", "name": "table-head-row", "width": "fill_container",
        "height": "fit_content", "layout": "horizontal", "gap": 8, "padding": [12, 16],
        "children": [cell("名称", json!("fill_container")), cell("份额", json!(64)),
                     cell("市值", json!(69)), cell("涨跌幅", json!(56))]
    })];
    for i in 0..rows {
        kids.push(divider());
        kids.push(holding_row(&format!("基金{i}")));
    }
    json!({
        "type": "frame", "name": "table-card", "width": "fill_container", "height": height,
        "layout": "vertical", "clipContent": true, "cornerRadius": 16,
        "fill": [{"type": "solid", "color": "#FFFFFF"}],
        "children": kids
    })
}

#[test]
fn clipped_table_card_hugs_all_its_rows() {
    let v = run_geometry(screen(vec![table_card(282.0, 6)]));
    let card = find(&v, "table-card").expect("card survives");
    assert_eq!(
        card.get("height").and_then(|h| h.as_str()),
        Some("fit_content"),
        "a clipped card truncating its table rows must hug them, got {:?}",
        card.get("height")
    );
    assert_eq!(
        card.get("clipContent").and_then(|c| c.as_bool()),
        Some(true)
    );
}

#[test]
fn clipped_table_card_that_already_fits_is_untouched() {
    let v = run_geometry(screen(vec![table_card(600.0, 3)]));
    let card = find(&v, "table-card").expect("card survives");
    assert_eq!(card.get("height").and_then(|h| h.as_f64()), Some(600.0));
}

#[test]
fn clipped_cover_photo_card_is_untouched() {
    // The image is what the clip crops; the rows under it are the authored
    // crop's casualties, not a truncated table.
    let mut kids = vec![
        json!({"type": "image", "name": "cover", "width": "fill_container",
                               "height": 240, "src": ""}),
    ];
    for i in 0..3 {
        kids.push(holding_row(&format!("基金{i}")));
    }
    let v = run_geometry(screen(vec![json!({
        "type": "frame", "name": "cover-card", "width": "fill_container", "height": 180,
        "layout": "vertical", "clipContent": true, "children": kids
    })]));
    let card = find(&v, "cover-card").expect("card survives");
    assert_eq!(card.get("height").and_then(|h| h.as_f64()), Some(180.0));
}

#[test]
fn clipped_horizontal_carousel_is_untouched() {
    let item = |i: usize| {
        json!({"type": "frame", "name": format!("item-{i}"), "width": 160, "height": "fit_content",
               "layout": "vertical", "gap": 4,
               "children": [cell("标题", json!("fill_container")), cell("说明文字", json!("fill_container")),
                            cell("¥ 99", json!("fill_container"))]})
    };
    let v = run_geometry(screen(vec![json!({
        "type": "frame", "name": "carousel", "width": "fill_container", "height": 40,
        "layout": "horizontal", "gap": 12, "clipContent": true,
        "children": [item(0), item(1), item(2), item(3)]
    })]));
    let carousel = find(&v, "carousel").expect("carousel survives");
    assert_eq!(carousel.get("height").and_then(|h| h.as_f64()), Some(40.0));
}

#[test]
fn clipped_card_with_a_single_tall_block_keeps_the_small_bound() {
    // One non-repeated block far past the edge (> the small-overshoot bound):
    // not a truncated list, so the existing behaviour (no growth) holds.
    let v = run_geometry(screen(vec![json!({
        "type": "frame", "name": "teaser-card", "width": "fill_container", "height": 120,
        "layout": "vertical", "padding": 16, "clipContent": true,
        "children": [
            cell("标题", json!("fill_container")),
            {"type": "frame", "name": "body-block", "width": "fill_container", "height": 300,
             "layout": "vertical",
             "children": [cell("很长的一段正文内容", json!("fill_container"))]}
        ]
    })]));
    let card = find(&v, "teaser-card").expect("card survives");
    assert_eq!(card.get("height").and_then(|h| h.as_f64()), Some(120.0));
}

#[test]
fn clipped_parent_of_a_grown_table_card_follows_it() {
    // The parent section is itself a clipped fixed-height card whose edge
    // cuts the same rows: it qualifies too, so it grows once the card does.
    let v = run_geometry(screen(vec![json!({
        "type": "frame", "name": "holdings-section", "width": "fill_container", "height": 320,
        "layout": "vertical", "gap": 12, "clipContent": true,
        "children": [cell("持仓", json!("fill_container")), table_card(282.0, 6)]
    })]));
    let section = find(&v, "holdings-section").expect("section survives");
    assert_eq!(
        section.get("height").and_then(|h| h.as_str()),
        Some("fit_content")
    );
    let card = find(&v, "table-card").expect("card survives");
    assert_eq!(
        card.get("height").and_then(|h| h.as_str()),
        Some("fit_content")
    );
}

#[test]
fn clipped_screen_root_is_never_grown() {
    let mut root = table_card(282.0, 6);
    root["name"] = json!("Screen");
    root["width"] = json!(375);
    let v = run_geometry(root);
    assert_eq!(v.get("height").and_then(|h| h.as_f64()), Some(282.0));
}

#[test]
fn repeated_row_run_skips_dividers() {
    let card = table_card(282.0, 6);
    // Header differs structurally; the six data rows form the run.
    assert_eq!(geometry_clipped_rows::repeated_row_run(&card).len(), 6);
    assert!(table_rows(&card).len() < 6, "table_rows stops at dividers");
}
