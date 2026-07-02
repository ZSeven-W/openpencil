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

/// A list/appointment area where each Row frame is followed by 2 flat orphan
/// cells (initials + status) — the glm barbershop "Upcoming Appointments" shape.
fn appointment_list_root() -> PenNode {
    let row = |n: u32| {
        json!({"type":"frame","id":format!("row{n}"),"name":format!("Appointment Row {n}"),
            "layout":"horizontal","width":"fill_container","children":[
            {"type":"text","id":format!("t{n}"),"name":"Time Slot","content":"09:00"}]})
    };
    let orphan = |n: u32, kind: &str| json!({"type":"text","id":format!("{kind}{n}"),"name":kind,"content":"x"});
    node(json!({
        "type":"frame","id":"main","name":"Main Content","width":940,"layout":"vertical","children":[
            row(1), orphan(1,"Initials"), orphan(1,"Status Text"),
            row(2), orphan(2,"Initials"), orphan(2,"Status Text"),
            row(3), orphan(3,"Initials"), orphan(3,"Status Text")
        ]
    }))
}

#[test]
fn positive_orphan_cells_reparented_into_rows() {
    let mut root = appointment_list_root();
    assert!(
        regroup_flat_table_rows(&mut root),
        "orphan rows must reparent"
    );
    let v = val(&root);
    let kids = v["children"].as_array().unwrap();
    // The 3 rows remain; the 6 orphan siblings are gone (folded into rows).
    assert_eq!(kids.len(), 3, "only the 3 rows remain at top level");
    for (i, row) in kids.iter().enumerate() {
        assert!(row["name"].as_str().unwrap().starts_with("Appointment Row"));
        let names: Vec<&str> = row["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["Time Slot", "Initials", "Status Text"],
            "row {} absorbed its orphans",
            i + 1
        );
    }
}

#[test]
fn negative_single_row_orphans_untouched() {
    // Only one row → not a regular list → never guess.
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Main","width":940,"layout":"vertical","children":[
            {"type":"frame","id":"r1","name":"Appointment Row 1","layout":"horizontal","children":[
                {"type":"text","id":"t","name":"Time","content":"9"}]},
            {"type":"text","id":"o","name":"Initials","content":"JC"}]}),
        ),
        "single row is not a regular list",
    );
}

#[test]
fn negative_irregular_orphan_counts_untouched() {
    // Row 1 has 2 orphans, row 2 has 1 → irregular → abort.
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Main","width":940,"layout":"vertical","children":[
            {"type":"frame","id":"r1","name":"Appointment Row 1","layout":"horizontal","children":[
                {"type":"text","id":"t1","name":"Time","content":"9"}]},
            {"type":"text","id":"i1","name":"Initials","content":"JC"},
            {"type":"text","id":"s1","name":"Status Text","content":"OK"},
            {"type":"frame","id":"r2","name":"Appointment Row 2","layout":"horizontal","children":[
                {"type":"text","id":"t2","name":"Time","content":"10"}]},
            {"type":"text","id":"i2","name":"Initials","content":"MR"}]}),
        ),
        "irregular orphan counts (2 vs 1)",
    );
}

#[test]
fn negative_section_list_not_reparented() {
    // A vertical stack of real sections (no row frames) must be left alone.
    assert_untouched(
        node(
            json!({"type":"frame","id":"m","name":"Main","width":940,"layout":"vertical","children":[
            {"type":"frame","id":"a","name":"Metrics Section","layout":"horizontal","children":[
                {"type":"text","id":"x","name":"X","content":"1"}]},
            {"type":"frame","id":"b","name":"Chart Section","layout":"vertical","children":[
                {"type":"text","id":"y","name":"Y","content":"2"}]},
            {"type":"frame","id":"c","name":"Table Section","layout":"vertical","children":[
                {"type":"text","id":"z","name":"Z","content":"3"}]}]}),
        ),
        "vertical section stack, no row frames",
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

// ── ensure_table_column_gap ──

#[test]
fn table_named_rows_get_column_gap() {
    // A "Client Table" whose rows have NO gap → the columns touch. Each ≥3-column
    // row must gain the column gap.
    let mut root = node(json!({
        "type":"frame","id":"tbl","name":"Client Table","layout":"vertical","gap":16,"children":[
            {"type":"frame","id":"hd","name":"Header Row","layout":"horizontal","children":[
                txt("h1","CLIENT",200), txt("h2","SPEND",100), txt("h3","STATUS",80)]},
            {"type":"frame","id":"r1","name":"Row 1","layout":"horizontal","children":[
                txt("c1","Alice",200), txt("c2","$100",100), txt("c3","VIP",80)]}
        ]
    }));
    assert!(
        ensure_table_column_gap(&mut root),
        "gap-less table rows get a column gap"
    );
    let v = val(&root);
    for row in v["children"].as_array().unwrap() {
        assert_eq!(
            num(row, "gap"),
            Some(TABLE_COLUMN_GAP),
            "each >=3-col row is spaced"
        );
    }
}

#[test]
fn nav_rows_do_not_get_table_gap() {
    // A "Navigation"-named container is NOT a table (name gate) — even though its
    // items are multi-child horizontal rows, they must not be re-gapped.
    let mut root = node(json!({
        "type":"frame","id":"nav","name":"Navigation","layout":"vertical","children":[
            {"type":"frame","id":"n1","name":"Nav Dashboard","layout":"horizontal","children":[
                txt("i","dot",16), txt("l","Dashboard",120), txt("b","3",20)]},
            {"type":"frame","id":"n2","name":"Nav Clients","layout":"horizontal","children":[
                txt("i2","dot",16), txt("l2","Clients",120), txt("b2","12",20)]}
        ]
    }));
    assert!(
        !ensure_table_column_gap(&mut root),
        "a nav container is not a table — left untouched"
    );
}

#[test]
fn gap_reaches_rows_behind_an_unnamed_wrapper() {
    // test07021.op's verbatim shape: "Client List Table" holds a toolbar row
    // and an UNNAMED vertical wrapper that holds the actual gap-less rows —
    // the gap must reach through the wrapper.
    let mut root: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"tbl","name":"Client List Table","layout":"vertical","children":[
            {"type":"frame","id":"toolbar","layout":"horizontal","gap":12,"children":[
                {"type":"text_input","id":"search"},{"type":"frame","id":"filter"}
            ]},
            {"type":"frame","id":"wrap","layout":"vertical","children":[
                {"type":"frame","id":"hdr","layout":"horizontal","children":[
                    {"type":"frame","id":"h1"},{"type":"frame","id":"h2"},{"type":"frame","id":"h3"}
                ]},
                {"type":"frame","id":"r1","layout":"horizontal","children":[
                    {"type":"frame","id":"c1"},{"type":"frame","id":"c2"},{"type":"frame","id":"c3"}
                ]}
            ]}
        ]
    }))
    .expect("valid PenNode");
    assert!(
        ensure_table_column_gap(&mut root),
        "wrapped gap-less rows must be repaired"
    );
    let v = serde_json::to_value(&root).unwrap();
    let wrap = &v["children"][1];
    for row in wrap["children"].as_array().unwrap() {
        assert!(
            row.get("gap")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
                > 0.0,
            "row behind the wrapper got a gap: {row}"
        );
    }
    // The 2-item toolbar row must stay untouched.
    assert!(
        v["children"][0]
            .get("gap")
            .and_then(serde_json::Value::as_f64)
            == Some(12.0)
    );
}
