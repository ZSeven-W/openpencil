use super::*;
use serde_json::json;

fn day_cell(id: &str, width: Value, align: &str) -> Value {
    json!({"type":"frame","id":id,"width":width,"layout":"none","children":[
        {"type":"rectangle","id":format!("{id}-band")},
        {"type":"ellipse","id":format!("{id}-marker")},
        {"type":"text","id":format!("{id}-number"),"content":"14","textAlign":align}
    ]})
}

/// A DeepSeek calendar week: six 42px days and a `fill_container` Saturday
/// in a 303px row with an 8px gap, so Saturday resolved to 3px.
fn week(gap: f64, cell_w: f64, last_fill: bool, align: &str) -> (Value, HashMap<String, Rect>) {
    let mut cells = Vec::new();
    let mut rects = HashMap::from([(
        "week".to_string(),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 303.0,
            h: 24.0,
        },
    )]);
    let mut x: f64 = 0.0;
    for i in 0..7 {
        let id = format!("d{i}");
        let fill = last_fill && i == 6;
        let width = if fill {
            json!("fill_container")
        } else {
            json!(cell_w)
        };
        let w = if fill { (303.0 - x).max(0.0) } else { cell_w };
        rects.insert(
            id.clone(),
            Rect {
                x,
                y: 0.0,
                w,
                h: 24.0,
            },
        );
        x += w + gap;
        cells.push(day_cell(&id, width, align));
    }
    let row = json!({"type":"frame","id":"week","layout":"horizontal","gap":gap,
        "width":"fill_container","children":cells});
    (row, rects)
}

fn fixes(row: &Value, rects: &HashMap<String, Rect>) -> Vec<String> {
    let mut cmds = Vec::new();
    collect_grid_row_fit_fixes(row, rects, &mut cmds);
    cmds.iter()
        .map(|cmd| match cmd {
            EditorCommand::UpdateNode { node_id, width, .. } => {
                format!("{} width {}", node_id.as_str(), width.unwrap())
            }
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Number(n),
            } => format!("{} {property} {n}", node_id.as_str()),
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

#[test]
fn a_calendar_week_gets_its_collapsed_saturday_back_and_a_fitting_gap() {
    let (row, rects) = week(8.0, 42.0, true, "center");
    assert_eq!(fixes(&row, &rects), ["d6 width 42", "week gap 1"]);
    // All fixed cells that overrun the row only lose gap.
    let (row, rects) = week(8.0, 42.0, false, "center");
    assert_eq!(fixes(&row, &rects), ["week gap 1"]);
}

#[test]
fn a_day_whose_number_was_lifted_above_its_circle_is_still_a_peer_cell() {
    // The buried-overlay pass moves the endpoint day's number to the front in
    // the same validation round; the week must still be fitted.
    let (mut row, rects) = week(8.0, 42.0, true, "center");
    let kids = row["children"][5]["children"].as_array_mut().unwrap();
    let number = kids.remove(2);
    kids.insert(0, number);
    assert_eq!(fixes(&row, &rects), ["d6 width 42", "week gap 1"]);
}

#[test]
fn fitting_rows_impossible_rows_and_ordinary_rows_are_left_alone() {
    // Already fits: 7 × 40 + 6 × 2 = 292.
    let (row, rects) = week(2.0, 40.0, false, "center");
    assert!(fixes(&row, &rects).is_empty());
    // Seven 48px cells cannot fit 303px even with no gap: an intent call.
    let (row, rects) = week(8.0, 48.0, true, "center");
    assert!(fixes(&row, &rects).is_empty());
    // Cells that differ in structure are not one grid.
    let (mut row, rects) = week(8.0, 42.0, true, "center");
    row["children"][3]["children"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"text","id":"extra","content":"!"}));
    assert!(fixes(&row, &rects).is_empty());
    // Four cells are a row of controls, not a grid.
    let (mut row, rects) = week(8.0, 42.0, true, "center");
    row["children"].as_array_mut().unwrap().truncate(4);
    assert!(fixes(&row, &rects).is_empty());
}

#[test]
fn centred_grid_cells_may_touch_but_data_columns_still_get_their_gap() {
    // After the fit the week touches at a 1px gap; its numbers sit mid-cell.
    let (mut row, mut rects) = week(1.0, 42.0, false, "center");
    for i in 0..7 {
        rects.get_mut(&format!("d{i}")).unwrap().x = f64::from(i) * 43.0;
    }
    row["gap"] = json!(1.0);
    assert!(is_centered_grid_row(&row));
    let mut cmds = Vec::new();
    collect_row_gap_fixes(&row, &rects, &mut cmds);
    assert!(cmds.is_empty(), "{cmds:?}");

    // The same geometry with left-aligned text is a jammed data row.
    let (row, rects) = {
        let (mut row, mut rects) = week(1.0, 42.0, false, "left");
        for i in 0..7 {
            rects.get_mut(&format!("d{i}")).unwrap().x = f64::from(i) * 43.0;
        }
        row["gap"] = json!(1.0);
        (row, rects)
    };
    assert!(!is_centered_grid_row(&row));
    let mut cmds = Vec::new();
    collect_row_gap_fixes(&row, &rects, &mut cmds);
    assert_eq!(cmds.len(), 1, "{cmds:?}");
}
