//! Tests for the flat-table regroup pass. Positive = the glm "Client Roster"
//! flat-cell shape; negatives are the false-positives the adversarial review
//! flagged (toolbar/tab-bar header, no row index, ragged run, already grouped,
//! plain text feed).

use super::*;

fn node(v: Value) -> PenNode {
    serde_json::from_value::<PenNode>(v).expect("valid PenNode fixture")
}
fn val(n: &PenNode) -> Value {
    serde_json::to_value(n).expect("serialize PenNode")
}
fn txt(id: &str, name: &str, w: i64) -> Value {
    json!({"type":"text","id":id,"name":name,"width":w,"content":name})
}

fn header_5col() -> Value {
    json!({"type":"frame","id":"hd","name":"Table Header","layout":"horizontal",
        "width":"fill_container","padding":[12,16],"gap":16,"children":[
        txt("th1","TH Client",452), txt("th2","TH Last Visit",120),
        txt("th3","TH Barber",140), txt("th4","TH Spend",100), txt("th5","TH Status",80)]})
}

/// `Main Content` (vertical) holding a prefix section, the header, then 2×5 flat
/// row-indexed cells — the reported shape.
fn flat_table_root() -> PenNode {
    node(json!({
        "type":"frame","id":"main","name":"Main Content","width":940,"layout":"vertical",
        "children":[
            {"type":"frame","id":"kpi","name":"Key Metrics","layout":"horizontal","width":"fill_container","children":[]},
            header_5col(),
            {"type":"frame","id":"r1c","name":"R1 Client Cell","width":940,"layout":"horizontal","children":[]},
            txt("r1v","R1 Visit",120), txt("r1b","R1 Barber",140), txt("r1s","R1 Spend",100),
            {"type":"frame","id":"r1st","name":"R1 Status Badge","width":940,"layout":"horizontal","children":[]},
            {"type":"frame","id":"r2c","name":"R2 Client Cell","width":940,"layout":"horizontal","children":[]},
            txt("r2v","R2 Visit",120), txt("r2b","R2 Barber",140), txt("r2s","R2 Spend",100),
            {"type":"frame","id":"r2st","name":"R2 Status Badge","width":940,"layout":"horizontal","children":[]}
        ]
    }))
}

#[test]
fn positive_flat_cells_grouped_into_table_rows() {
    let mut root = flat_table_root();
    assert!(
        regroup_flat_table_rows(&mut root),
        "flat table must regroup"
    );
    let v = val(&root);
    let kids = v["children"].as_array().unwrap();
    // Prefix section preserved; header+10 cells collapsed into one Table frame.
    assert_eq!(kids.len(), 2, "[Key Metrics, Table]");
    assert_eq!(kids[0]["name"], json!("Key Metrics"));
    let table = &kids[1];
    assert_eq!(table["role"], json!("table"));
    assert_eq!(layout_str(table), Some("vertical"));
    let tkids = table["children"].as_array().unwrap();
    assert_eq!(tkids.len(), 3, "[header, Row 1, Row 2]");
    assert_eq!(tkids[0]["name"], json!("Table Header"));

    let row1 = &tkids[1];
    assert_eq!(row1["role"], json!("table-row"));
    assert_eq!(layout_str(row1), Some("horizontal"));
    let r1: Vec<&str> = row1["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        r1,
        [
            "R1 Client Cell",
            "R1 Visit",
            "R1 Barber",
            "R1 Spend",
            "R1 Status Badge"
        ]
    );
    // Body cells take the header column widths (fixes the full-width status bar).
    let w: Vec<f64> = row1["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["width"].as_f64().unwrap())
        .collect();
    assert_eq!(w, [452.0, 120.0, 140.0, 100.0, 80.0]);

    let row2: Vec<&str> = tkids[2]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        row2,
        [
            "R2 Client Cell",
            "R2 Visit",
            "R2 Barber",
            "R2 Spend",
            "R2 Status Badge"
        ]
    );
}

fn assert_untouched(mut root: PenNode, why: &str) {
    let before = val(&root);
    assert!(
        !regroup_flat_table_rows(&mut root),
        "must NOT regroup: {why}"
    );
    assert_eq!(val(&root), before, "unchanged: {why}");
}

#[test]
fn negative_toolbar_not_a_header() {
    // A horizontal toolbar of short labels is NOT tagged table-header → ignored.
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Main","width":940,"layout":"vertical","children":[
            {"type":"frame","id":"tb","name":"Toolbar","layout":"horizontal","children":[
                txt("a","Filter",60), txt("b","Sort",50), txt("c","Export",60)]},
            txt("x1","R1 Item",100), txt("x2","R2 Item",100), txt("x3","R3 Item",100)]}),
        ),
        "toolbar is not a tagged table header",
    );
}

#[test]
fn negative_cells_without_row_index() {
    // Header present, but the cells carry no R{n} index → never guess → abort.
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Main","width":940,"layout":"vertical","children":[
            header_5col(),
            txt("a","Client A",452), txt("b","Oct 12",120), txt("c","Marcus",140),
            txt("d","$1,240",100), {"type":"frame","id":"e","name":"Status","width":80,"children":[]}]}),
        ),
        "cells lack explicit row index",
    );
}

#[test]
fn negative_ragged_run() {
    // R1 has 5 cells, R2 has only 3 → group size != header columns → abort.
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Main","width":940,"layout":"vertical","children":[
            header_5col(),
            txt("a","R1 Client",452), txt("b","R1 Visit",120), txt("c","R1 Barber",140),
            txt("d","R1 Spend",100), txt("e","R1 Status",80),
            txt("f","R2 Client",452), txt("g","R2 Visit",120), txt("h","R2 Barber",140)]}),
        ),
        "row 2 is ragged (3 != 5 columns)",
    );
}

#[test]
fn negative_already_grouped() {
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Main","width":940,"layout":"vertical","children":[
            header_5col(),
            {"type":"frame","id":"row1","name":"Row 1","role":"table-row","layout":"horizontal","children":[]},
            {"type":"frame","id":"row2","name":"Row 2","role":"table-row","layout":"horizontal","children":[]}]}),
        ),
        "rows already grouped (table-row role present)",
    );
}

#[test]
fn negative_plain_text_feed() {
    // A vertical list of texts with no table header → nothing to regroup.
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Activity Feed","width":600,"layout":"vertical","children":[
            txt("a","Alice commented",400), txt("b","Bob shared a file",400),
            txt("c","Carol joined",400), txt("d","Dave updated status",400)]}),
        ),
        "plain text feed, no header",
    );
}
