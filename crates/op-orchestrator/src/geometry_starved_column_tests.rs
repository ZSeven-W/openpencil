//! `collect_starved_text_column_fixes` against the REAL jian layout.
//!
//! The table fixture is the GLM-5.3-Flash `arena-m03` holdings table reduced
//! to its header plus two data rows (dividers kept — they are what hides the
//! table from the name-gated column scaler).

use super::*;
use serde_json::json;

fn state_of(root: Value) -> EditorState {
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_value(json!({ "version": "1.0", "children": [root] })).expect("doc");
    EditorState::from_document(doc)
}

fn root_value(state: &EditorState) -> Value {
    serde_json::to_value(&state.active_children()[0]).expect("root json")
}

fn fixes_for(state: &EditorState) -> Vec<EditorCommand> {
    let rects = resolved_rects(state);
    let mut cmds = Vec::new();
    collect_starved_text_column_fixes(state, &root_value(state), &rects, &mut cmds);
    cmds
}

fn find<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
    if v.get("id").and_then(Value::as_str) == Some(id) {
        return Some(v);
    }
    children(v).iter().find_map(|c| find(c, id))
}

fn text(id: &str, content: &str, width: Value, size: f64) -> Value {
    json!({"type":"text","id":id,"content":content,"width":width,"fontSize":size,
           "fontFamily":"DM Sans","fontWeight":500,"lineHeight":1.5,"textGrowth":"auto"})
}

/// A `fill_container` cell (`{id}c`) holding a 14px text (`id`) that wraps
/// inside whatever width the cell gets — the m03 name-cell shape.
fn wrapping(id: &str, content: &str) -> Value {
    let mut t = text(id, content, json!("fill_container"), 14.0);
    t["textGrowth"] = json!("fixed-width");
    json!({"type":"frame","id":format!("{id}c"),"layout":"vertical",
           "width":"fill_container","height":"fit_content","children":[t]})
}

fn mono(id: &str, content: &str, width: f64) -> Value {
    json!({"type":"text","id":id,"content":content,"width":width,"fontSize":12,
           "fontFamily":"DM Mono","letterSpacing":0.5,"lineHeight":1.5,
           "textAlign":"right","textGrowth":"auto"})
}

fn data_row(p: &str, name: &str, a: &str, b: &str, pct: &str) -> Value {
    json!({"type":"frame","id":format!("{p}row"),"layout":"horizontal","width":"fill_container",
      "height":"fit_content","gap":24,"padding":[12,16],"alignItems":"center","children":[
        {"type":"frame","id":format!("{p}name"),"layout":"vertical","gap":3,
         "width":"fill_container","height":"fit_content","children":[
            {"type":"text","id":format!("{p}nt"),"content":name,"width":"fill_container",
             "fontSize":14,"fontFamily":"DM Sans","lineHeight":1.5,"textGrowth":"fixed-width"},
            {"type":"frame","id":format!("{p}tag"),"layout":"horizontal","width":"fill_container",
             "padding":[1,6],"justifyContent":"center","children":[
                text(&format!("{p}tagt"), "基金", json!("fit_content"), 10.0)]}]},
        mono(&format!("{p}a"), a, 64.0),
        mono(&format!("{p}b"), b, 78.0),
        {"type":"frame","id":format!("{p}pill"),"layout":"horizontal","width":56,
         "padding":[3,0],"justifyContent":"center","cornerRadius":999,"children":[
            text(&format!("{p}pt"), pct, json!("fit_content"), 11.0)]}]})
}

fn divider(id: &str) -> Value {
    json!({"type":"frame","id":id,"layout":"none","width":"fill_container","height":1})
}

/// 375 screen → 327 table card → rows of 327 (inner 295): 72px of gaps and
/// 198px of fixed columns leave the name column ~25px.
fn m03_table() -> Value {
    json!({"type":"frame","id":"root","width":375,"height":"fit_content","layout":"vertical",
      "padding":[0,24],"children":[
        {"type":"frame","id":"card","layout":"vertical","width":"fill_container",
         "height":"fit_content","children":[
            {"type":"frame","id":"head","layout":"horizontal","width":"fill_container",
             "height":"fit_content","gap":24,"padding":[12,16],"alignItems":"center","children":[
                text("h0", "名称", json!("fill_container"), 12.0),
                text("h1", "份额", json!(64), 12.0),
                text("h2", "市值", json!(78), 12.0),
                text("h3", "涨跌幅", json!(56), 12.0)]},
            divider("d0"),
            data_row("r1", "易方达蓝筹精选", "12,430.20", "58,622.44", "+2.35%"),
            divider("d1"),
            data_row("r2", "贵州茅台", "180", "29,142.00", "+1.12%")]}]})
}

#[test]
fn m03_name_column_widens_toward_no_wrap_and_rows_stay_aligned() {
    let mut state = state_of(m03_table());
    let before = resolved_rects(&state);
    assert!(
        before["r1name"].w < 49.0,
        "fixture must reproduce the starved column: {}",
        before["r1name"].w
    );
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    geometry_validate_and_fix(&mut sink, "root");

    let after = resolved_rects(&state);
    assert!(
        after["r1name"].w >= 3.5 * 14.0 - 0.5,
        "name column must hold 3.5 glyphs of 14px: {}",
        after["r1name"].w
    );
    let root = root_value(&state);
    // "贵州茅台" (4 glyphs @14px) must sit on one line: aiming for the floor
    // alone left it split 3+1.
    let four = find(&root, "r2nt").unwrap();
    let natural = measure_texts(&state, &[four]).natural["r2nt"];
    assert!(
        after["r2nt"].w + 0.5 >= natural,
        "贵州茅台 wraps: {} < natural {natural}",
        after["r2nt"].w
    );
    let gap = |id: &str| num(find(&root, id).expect(id), "gap");
    assert_eq!(gap("head"), gap("r1row"), "header and data gaps match");
    assert_eq!(gap("r1row"), gap("r2row"), "data gaps match");
    // The 7-glyph name needs more than the gaps can give, so they bottom out.
    assert_eq!(gap("head"), MIN_GAP);
    for (h, r1, r2) in [
        ("h1", "r1a", "r2a"),
        ("h2", "r1b", "r2b"),
        ("h3", "r1pill", "r2pill"),
    ] {
        let w = |id: &str| fixed_width(find(&root, id).expect(id));
        assert_eq!(w(h), w(r1), "column {h} stays aligned");
        assert_eq!(w(r1), w(r2), "column {r1} stays aligned");
    }
    let x = |id: &str| after[id].x.round();
    assert_eq!(x("h2"), x("r1b"), "resolved column edges line up");
    assert_eq!(x("r1b"), x("r2b"), "resolved column edges line up");
}

#[test]
fn a_comfortably_wide_fill_column_is_untouched() {
    let state = state_of(
        json!({"type":"frame","id":"root","width":375,"layout":"vertical",
      "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container",
        "gap":24,"padding":[12,16],"children":[
            wrapping("t0", "易方达蓝筹精选混合基金"),
            mono("t1", "12,430.20", 72.0)]}]}),
    );
    assert!(fixes_for(&state).is_empty());
}

#[test]
fn at_the_gap_floor_numeric_text_columns_shrink_but_never_below_natural() {
    let state = state_of(
        json!({"type":"frame","id":"root","width":320,"layout":"vertical",
      "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container",
        "gap":8,"padding":[0,0],"children":[
            wrapping("t0", "易方达蓝筹精选"),
            mono("t1", "12.5", 130.0),
            mono("t2", "7", 150.0)]}]}),
    );
    let before = resolved_rects(&state);
    assert!(before["t0"].w < 49.0, "starved: {}", before["t0"].w);
    let cmds = fixes_for(&state);
    assert!(
        !cmds.iter().any(
            |c| matches!(c, EditorCommand::SetNodeLayoutProp { property, .. } if property == "gap")
        ),
        "gap already at its floor: {cmds:?}"
    );
    let root = root_value(&state);
    let measured = measure_texts(
        &state,
        &[find(&root, "t1").unwrap(), find(&root, "t2").unwrap()],
    );
    let mut narrowed = 0;
    for cmd in &cmds {
        if let EditorCommand::UpdateNode { node_id, width, .. } = cmd {
            let natural = measured.natural[node_id.as_str()];
            let w = f64::from(width.expect("width"));
            assert!(w >= natural.ceil(), "{node_id:?} {w} < natural {natural}");
            narrowed += 1;
        }
    }
    assert_eq!(narrowed, 2, "both text columns give back slack: {cmds:?}");
}

#[test]
fn an_unreachable_natural_width_still_lands_the_reachable_maximum() {
    // 32 glyphs @14px can never fit a 343px row beside 280px of columns:
    // the gap drops to 8, both text columns fall to their natural width,
    // and the name column takes exactly what that frees.
    let mut state = state_of(
        json!({"type":"frame","id":"root","width":375,"layout":"vertical",
      "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container",
        "gap":24,"padding":[0,16],"children":[
            wrapping("t0", "易方达蓝筹精选混合型证券投资基金易方达蓝筹精选混合型证券投资基金"),
            mono("t1", "12.5", 140.0),
            mono("t2", "7", 140.0)]}]}),
    );
    let before = resolved_rects(&state);
    assert!(before["t0"].w < 49.0, "starved: {}", before["t0"].w);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    geometry_validate_and_fix(&mut sink, "root");

    let root = root_value(&state);
    assert_eq!(num(find(&root, "row").unwrap(), "gap"), MIN_GAP);
    let measured = measure_texts(
        &state,
        &[find(&root, "t1").unwrap(), find(&root, "t2").unwrap()],
    );
    let w1 = fixed_width(find(&root, "t1").unwrap()).unwrap();
    let w2 = fixed_width(find(&root, "t2").unwrap()).unwrap();
    assert_eq!(w1, measured.natural["t1"].ceil(), "t1 gives all its slack");
    assert_eq!(w2, measured.natural["t2"].ceil(), "t2 gives all its slack");
    let after = resolved_rects(&state);
    // 375px row minus [0,16] padding leaves 343px inside.
    let reachable = 343.0 - 2.0 * MIN_GAP - w1 - w2;
    assert!(
        (after["t0c"].w - reachable).abs() < 1.0,
        "name column {} != reachable {reachable}",
        after["t0c"].w
    );
}

#[test]
fn pills_are_never_narrowed() {
    // The only rigid column besides the gap is a pill: the gap is reduced,
    // the pill keeps its width even though the floor is not reached.
    let state = state_of(
        json!({"type":"frame","id":"root","width":240,"layout":"vertical",
      "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container",
        "gap":12,"children":[
            wrapping("t0", "易方达蓝筹精选"),
            {"type":"frame","id":"pill","layout":"horizontal","width":190,"children":[
                text("pt", "+2%", json!("fit_content"), 11.0)]}]}]}),
    );
    let cmds = fixes_for(&state);
    assert!(
        cmds.iter()
            .all(|c| !matches!(c, EditorCommand::UpdateNode { .. })),
        "{cmds:?}"
    );
    assert!(cmds.iter().any(|c| matches!(
        c,
        EditorCommand::SetNodeLayoutProp { value: LayoutPropValue::Number(g), .. } if *g == MIN_GAP
    )));
}

#[test]
fn a_non_text_fill_child_is_ignored() {
    let state = state_of(
        json!({"type":"frame","id":"root","width":375,"layout":"vertical",
      "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container",
        "gap":24,"children":[
            text("t0", "标题", json!(120), 14.0),
            {"type":"frame","id":"spacer","width":"fill_container","height":1},
            mono("t1", "12,430.20", 200.0)]}]}),
    );
    assert!(fixes_for(&state).is_empty());
}

#[test]
fn a_narrow_chip_row_is_ignored() {
    let state = state_of(
        json!({"type":"frame","id":"root","width":180,"layout":"vertical",
      "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container",
        "gap":24,"children":[
            wrapping("t0", "易方达蓝筹精选"),
            mono("t1", "12.5", 130.0)]}]}),
    );
    assert!(resolved_rects(&state)["t0"].w < 49.0);
    assert!(fixes_for(&state).is_empty());
}

#[test]
fn a_latin_word_sets_the_floor_when_wider_than_three_and_a_half_ems() {
    let state = state_of(
        json!({"type":"frame","id":"root","width":375,"layout":"vertical",
      "children":[{"type":"frame","id":"row","layout":"horizontal","width":"fill_container",
        "gap":24,"padding":[0,16],"children":[
            wrapping("t0", "a.sterling@email.com"),
            mono("t1", "12.5", 120.0),
            mono("t2", "7", 100.0)]}]}),
    );
    let root = root_value(&state);
    let t0 = find(&root, "t0").unwrap();
    let measured = measure_texts(&state, &[t0]);
    let floor = readable_floor(t0, &measured).expect("floor");
    assert!(floor > 3.5 * 14.0, "the unbreakable word decides: {floor}");
    let mut state = state;
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    geometry_validate_and_fix(&mut sink, "root");
    let after = resolved_rects(&state);
    assert!(after["t0"].w + 1.0 >= floor, "{} < {floor}", after["t0"].w);
}
