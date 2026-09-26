//! The injected table gap stays within the row's provable width budget.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

use crate::table_repair::ensure_table_column_gap;

fn run(root: Value) -> Value {
    let mut node: PenNode = serde_json::from_value(root).expect("node");
    assert!(ensure_table_column_gap(&mut node), "rows get a gap");
    serde_json::to_value(&node).expect("json")
}

fn mono(id: &str, content: &str, width: f64) -> Value {
    json!({"type":"text","id":id,"content":content,"width":width,"fontSize":12})
}

fn row(id: &str, name: &str, name_size: f64) -> Value {
    json!({"type":"frame","id":id,"layout":"horizontal","width":"fill_container",
      "padding":[12,16],"children":[
        {"type":"frame","id":format!("{id}n"),"layout":"vertical","width":"fill_container",
         "children":[{"type":"text","id":format!("{id}t"),"content":name,
                      "fontSize":name_size,"width":"fill_container"}]},
        mono(&format!("{id}a"), "12,430.20", 64.0),
        mono(&format!("{id}b"), "58,622.44", 78.0),
        {"type":"frame","id":format!("{id}p"),"layout":"horizontal","width":56,
         "children":[mono(&format!("{id}pt"), "+2.35%", 40.0)]}]})
}

/// The arena-m03 holdings table: a 375 screen, a [0,24]-padded section and
/// gap-less rows of `[fill name, 64, 78, 56]`.
fn phone_table(width: Value) -> Value {
    json!({"type":"frame","id":"root","width":width,"layout":"vertical","children":[
      {"type":"frame","id":"sec","layout":"vertical","width":"fill_container","padding":[0,24],
       "children":[{"type":"frame","id":"tbl","name":"Holdings Table","layout":"vertical",
         "width":"fill_container","children":[
            {"type":"frame","id":"head","layout":"horizontal","width":"fill_container",
             "padding":[12,16],"children":[
                {"type":"text","id":"h0","content":"名称","fontSize":12,"width":"fill_container"},
                mono("h1", "份额", 64.0), mono("h2", "市值", 78.0), mono("h3", "涨跌幅", 56.0)]},
            row("r1", "易方达蓝筹精选", 14.0),
            row("r2", "贵州茅台", 14.0)]}]}]})
}

fn gaps(v: &Value) -> Vec<f64> {
    let tbl = &v["children"][0]["children"][0];
    tbl["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["gap"].as_f64().expect("gap"))
        .collect()
}

#[test]
fn a_phone_table_gets_a_gap_that_seats_its_name_column() {
    // 295px inside a row, 198px of fixed columns: 3 × 24px would leave the
    // name column 25px. The widest name (7 glyphs @14px) needs ~98px, so
    // every row — header included — gets the 8px floor.
    let g = gaps(&run(phone_table(json!(375))));
    assert_eq!(g, vec![8.0, 8.0, 8.0], "one uniform, budgeted gap");
}

#[test]
fn a_gap_that_fits_is_the_largest_that_seats_the_text() {
    // Only a 4-glyph name: 295 − 198 − 56 = 41px of free space over three
    // gaps → 13px, below the 24px default and above the floor.
    let mut t = phone_table(json!(375));
    let rows = t["children"][0]["children"][0]["children"]
        .as_array_mut()
        .unwrap();
    rows.remove(1);
    let g = gaps(&run(t));
    assert_eq!(g, vec![13.0, 13.0]);
}

#[test]
fn a_wide_table_keeps_the_default_gap() {
    let g = gaps(&run(phone_table(json!(1200))));
    assert_eq!(g, vec![24.0, 24.0, 24.0]);
}

#[test]
fn an_unprovable_width_keeps_the_default_gap() {
    // A hugging root proves nothing about the row's width.
    let g = gaps(&run(phone_table(json!("fit_content"))));
    assert_eq!(g, vec![24.0, 24.0, 24.0]);
}
