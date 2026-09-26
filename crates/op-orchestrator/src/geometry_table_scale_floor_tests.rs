//! The table column scaler against the REAL jian layout: fixed cells stop at
//! their content floors and the geometry loop converges.
//!
//! Expectations are derived from measured natural widths, never literals —
//! CI has none of the fixtures' faces (DM Sans / DM Mono / CJK), so every
//! width here is whatever the fallback face produces.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;

/// The face the fixtures ask for. Swap to "NoSuchFace" / "Courier New" /
/// "Arial" locally to re-check the font-agnostic claims.
const MONO: &str = "DM Mono";
const SANS: &str = "DM Sans";

fn sink_of(root: Value) -> VecDocSink {
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_value(json!({ "version": "1.0", "children": [root] })).expect("doc");
    VecDocSink {
        state: EditorState::from_document(doc),
        applied: Vec::new(),
        batch_depth: 0,
    }
}

fn root_json(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state().active_children()[0]).expect("root json")
}

fn find<'a>(v: &'a Value, id: &str) -> &'a Value {
    fn walk<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
        if v.get("id").and_then(Value::as_str) == Some(id) {
            return Some(v);
        }
        children(v).iter().find_map(|c| walk(c, id))
    }
    walk(v, id).unwrap_or_else(|| panic!("node {id} survives"))
}

fn width_of(sink: &VecDocSink, id: &str) -> f64 {
    fixed_width(find(&root_json(sink), id)).unwrap_or_else(|| panic!("{id} keeps a fixed width"))
}

fn label(id: &str, content: &str, family: &str, size: f64) -> Value {
    json!({"type":"text","id":id,"content":content,"width":"fit_content","height":"fit_content",
           "fontFamily":family,"fontSize":size,"fontWeight":500,"letterSpacing":0.5,
           "lineHeight":1.5,"textAlign":"right"})
}

/// A fixed-width cell right-aligning one number — the m03 shares/value cell.
fn num_cell(id: &str, w: f64, content: &str) -> Value {
    json!({"type":"frame","id":id,"width":w,"layout":"vertical","alignItems":"end",
           "children":[label(&format!("{id}-t"), content, MONO, 12.0)]})
}

/// The m03 change cell: fixed width, a padded pill hugging a percentage.
fn chg_cell(id: &str, w: f64, content: &str) -> Value {
    json!({"type":"frame","id":id,"width":w,"layout":"horizontal","justifyContent":"end",
           "children":[{"type":"frame","id":format!("{id}-badge"),"layout":"horizontal",
             "padding":[3,6],"cornerRadius":999,"children":[
                label(&format!("{id}-t"), content, MONO, 11.0)]}]})
}

fn m03_data_row(p: &str, name: &str, shares: &str, value: &str, pct: &str) -> Value {
    json!({"type":"frame","id":p,"layout":"horizontal","width":"fill_container","gap":8,
      "padding":[14,16],"alignItems":"center","children":[
        {"type":"frame","id":format!("{p}-name"),"width":"fill_container","layout":"vertical",
         "gap":2,"children":[
            {"type":"text","id":format!("{p}-nt"),"content":name,"width":"fill_container",
             "fontFamily":SANS,"fontSize":14,"fontWeight":500,"lineHeight":1.3,
             "textGrowth":"fixed-width"},
            label(&format!("{p}-code"), "005827", MONO, 11.0)]},
        num_cell(&format!("{p}-shares"), 64.0, shares),
        num_cell(&format!("{p}-value"), 80.0, value),
        chg_cell(&format!("{p}-chg"), 56.0, pct)]})
}

fn th(id: &str, w: Value, content: &str) -> Value {
    json!({"type":"frame","id":id,"width":w,"layout":"vertical","alignItems":"end",
           "children":[label(&format!("{id}-t"), content, SANS, 12.0)]})
}

/// GLM-5.3-Flash `arena-m03`'s holdings table on a 375 phone, as authored
/// before the geometry loop: header + six data rows, fixed 64 / 80 / 56.
fn m03_screen() -> Value {
    let rows = [
        ("易方达蓝筹精选", "12,480.55", "¥86,432.18", "+2.34%"),
        ("招商中证白酒", "8,235.10", "¥52,890.44", "+1.68%"),
        ("华夏沪深300ETF联接", "20,110.00", "¥41,205.30", "-0.52%"),
        ("富国天惠成长", "6,742.80", "¥28,564.92", "+0.96%"),
        ("中欧医疗健康", "4,015.20", "¥19,820.66", "-1.24%"),
        ("广发纳斯达克100", "2,880.00", "¥9,745.12", "+3.05%"),
    ];
    let mut kids = vec![json!({"type":"frame","id":"hdr","layout":"horizontal",
        "width":"fill_container","gap":8,"padding":[14,16],"alignItems":"center","children":[
            th("th-name", json!("fill_container"), "名称"),
            th("th-shares", json!(64), "份额"),
            th("th-value", json!(80), "市值"),
            th("th-chg", json!(56), "涨跌幅")]})];
    for (i, (name, shares, value, pct)) in rows.iter().enumerate() {
        kids.push(m03_data_row(&format!("r{i}"), name, shares, value, pct));
    }
    json!({"type":"frame","id":"screen","width":375,"height":"fit_content","layout":"vertical",
      "padding":16,"children":[
        {"type":"frame","id":"table","width":"fill_container","height":"fit_content",
         "layout":"vertical","clipContent":true,"cornerRadius":12,"children":kids}]})
}

/// Resolved widths of `cell` and its text; the text must fit inside.
fn assert_text_fits(rects: &HashMap<String, Rect>, cell: &str, text: &str) {
    let c = rects[cell];
    let t = rects[text];
    assert!(
        t.w <= c.w + 0.5,
        "{text} ({:.1}px) spills out of its {:.1}px cell {cell}",
        t.w,
        c.w
    );
}

#[test]
fn m03_fixed_number_cells_stop_at_their_text() {
    let mut sink = sink_of(m03_screen());
    geometry_validate_and_fix(&mut sink, "screen");
    let rects = resolved_rects(sink.state());

    for i in 0..6 {
        let p = format!("r{i}");
        for col in ["shares", "value"] {
            let cell = format!("{p}-{col}");
            assert_text_fits(&rects, &cell, &format!("{cell}-t"));
        }
        let chg = format!("{p}-chg");
        assert_text_fits(&rects, &chg, &format!("{chg}-badge"));
        assert_text_fits(&rects, &format!("{chg}-badge"), &format!("{chg}-t"));
        // Nothing crushed the column gap.
        let row = find(&root_json(&sink), &p).clone();
        assert!(num(&row, "gap") >= MIN_GAP_FLOOR, "row {p} gap crushed");
        // The row's children sit inside its content box.
        let inner = rects[&p].w - horizontal_padding(&row);
        let used: f64 = children(&row)
            .iter()
            .map(|c| rects[c["id"].as_str().unwrap()].w)
            .sum::<f64>()
            + num(&row, "gap") * 3.0;
        assert!(
            used <= inner + 1.0,
            "row {p} overflows: {used:.1} > {inner:.1}"
        );
    }
    // Header and data columns keep one width per index.
    for (th, col) in [
        ("th-shares", "shares"),
        ("th-value", "value"),
        ("th-chg", "chg"),
    ] {
        let hw = width_of(&sink, th);
        for i in 0..6 {
            assert_eq!(
                hw,
                width_of(&sink, &format!("r{i}-{col}")),
                "{th} misaligned"
            );
        }
    }
}

/// Gaps never go below this in the scaler.
const MIN_GAP_FLOOR: f64 = 8.0;

#[test]
fn m03_loop_converges_once_cells_sit_on_their_floors() {
    let mut sink = sink_of(m03_screen());
    geometry_validate_and_fix(&mut sink, "screen");
    let after_first: Vec<f64> = ["r0-shares", "r0-value", "r0-chg", "th-chg"]
        .iter()
        .map(|id| width_of(&sink, id))
        .collect();

    // The scaler itself has nothing left to give …
    let rects = resolved_rects(sink.state());
    let mut ops = Vec::new();
    collect_scale_ops(sink.state(), &root_json(&sink), &rects, &mut ops);
    assert!(
        ops.is_empty(),
        "scaler keeps cutting at its floors: {ops:?}"
    );

    // … and re-running the whole loop moves no column.
    geometry_validate_and_fix(&mut sink, "screen");
    let after_second: Vec<f64> = ["r0-shares", "r0-value", "r0-chg", "th-chg"]
        .iter()
        .map(|id| width_of(&sink, id))
        .collect();
    assert_eq!(
        after_first, after_second,
        "a later loop re-scaled the table"
    );
}

fn text_row(id: &str, widths: &[f64], texts: &[&str]) -> Value {
    let cells: Vec<Value> = widths
        .iter()
        .zip(texts)
        .enumerate()
        .map(|(i, (w, t))| num_cell(&format!("{id}-c{i}"), *w, t))
        .collect();
    json!({"type":"frame","id":id,"layout":"horizontal","width":"fill_container","gap":24,
           "children":cells})
}

fn table_root(root_w: f64, widths: &[f64], rows: &[(&str, &[&str])]) -> Value {
    let kids: Vec<Value> = rows
        .iter()
        .map(|(id, texts)| text_row(id, widths, texts))
        .collect();
    json!({"type":"frame","id":"root","width":root_w,"height":"fit_content",
      "layout":"vertical","children":[{"type":"frame","id":"tbl","layout":"vertical",
        "width":"fill_container","children":kids}]})
}

/// Resolved `(x, w)` per cell, so column alignment is checked on the canvas
/// even after a later pass retargets a column to `fill_container`.
fn column_box(rects: &HashMap<String, Rect>, id: &str) -> (i64, i64) {
    let r = rects[id];
    (r.x.round() as i64, r.w.round() as i64)
}

#[test]
fn oversized_fixed_text_columns_still_scale_to_fit() {
    // Five 240px columns holding short numbers in an 800px table: plenty of
    // slack above every text, so the scaler alone brings the table inside
    // its rows, just as the proportional scaler did.
    let widths = [240.0; 5];
    let texts: &[&str] = &["42", "1,024", "7", "¥3.50", "99%"];
    let rows: &[(&str, &[&str])] = &[
        ("hdr", &["A", "B", "C", "D", "E"]),
        ("r1", texts),
        ("r2", texts),
    ];
    let mut sink = sink_of(table_root(800.0, &widths, rows));
    assert!(fix_table_column_overflow(&mut sink, "root"));
    let rects = resolved_rects(sink.state());
    let v = root_json(&sink);
    for (rid, _) in rows {
        let row = find(&v, rid);
        let used: f64 =
            children(row).iter().filter_map(fixed_width).sum::<f64>() + num(row, "gap") * 4.0;
        assert!(
            used <= rects[*rid].w + 1.0,
            "{rid} still overflows ({used})"
        );
        assert!(num(row, "gap") >= MIN_GAP_FLOOR);
        for i in 0..5 {
            let cell = format!("{rid}-c{i}");
            let w = width_of(&sink, &cell);
            assert!(w < 200.0, "{cell} barely scaled ({w})");
            assert_text_fits(&rects, &cell, &format!("{cell}-t"));
        }
    }

    // The whole loop settles with every number still inside its cell.
    let mut sink = sink_of(table_root(800.0, &widths, rows));
    geometry_validate_and_fix(&mut sink, "root");
    let rects = resolved_rects(sink.state());
    for (rid, _) in rows {
        for i in 0..5 {
            let cell = format!("{rid}-c{i}");
            assert_text_fits(&rects, &cell, &format!("{cell}-t"));
        }
    }
}

#[test]
fn all_fixed_header_stays_aligned_with_its_data_rows() {
    // Header labels are short, data values long: the column floor is the
    // widest text in the column, and every row gets the same width.
    let widths = [140.0, 140.0, 140.0];
    let rows: &[(&str, &[&str])] = &[
        ("hdr", &["Qty", "Price", "Total"]),
        ("r1", &["12,480.55", "¥86,432.18", "¥1,078,660.20"]),
        ("r2", &["8.10", "¥2.44", "¥19.76"]),
    ];
    let mut sink = sink_of(table_root(360.0, &widths, rows));
    assert!(fix_table_column_overflow(&mut sink, "root"));
    let rects = resolved_rects(sink.state());
    for i in 0..3 {
        let hw = width_of(&sink, &format!("hdr-c{i}"));
        assert!(hw < 140.0, "column {i} not scaled");
        for rid in ["r1", "r2"] {
            let cell = format!("{rid}-c{i}");
            assert_eq!(hw, width_of(&sink, &cell), "column {i} misaligned in {rid}");
            assert_text_fits(&rects, &cell, &format!("{cell}-t"));
        }
    }

    let mut sink = sink_of(table_root(360.0, &widths, rows));
    geometry_validate_and_fix(&mut sink, "root");
    let rects = resolved_rects(sink.state());
    for i in 0..3 {
        let head = column_box(&rects, &format!("hdr-c{i}"));
        for rid in ["r1", "r2"] {
            let cell = format!("{rid}-c{i}");
            assert_eq!(
                head,
                column_box(&rects, &cell),
                "column {i} misaligned in {rid}"
            );
            assert_text_fits(&rects, &cell, &format!("{cell}-t"));
        }
    }
}
