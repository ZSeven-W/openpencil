//! Pure bar tracks must not pair with their labels to become a data table.

use super::*;

#[test]
fn retained_workout_metadata_chips_do_not_reserve_data_table_column_floors() {
    let section: Value =
        serde_json::from_str(include_str!("test_fixtures/workout_title_meta_chips.json")).unwrap();
    let root = json!({"type":"frame","id":"root","width":375,
        "height":"fit_content","layout":"vertical","children":[section]});
    let doc = serde_json::from_value(json!({"version":"1.0","children":[root]})).unwrap();
    let state = EditorState::from_document(doc);
    let section = serde_json::to_value(&state.active_children()[0]).unwrap()["children"][0].clone();
    let rects = resolved_rects(&state);
    assert!(table_rows(&section).is_empty());
    assert!(table_columns_exceed_width(&section, &rects).is_none());
    assert!(table_overflow_scale(&section, &rects).is_none());
    assert!(!geometry_diagnostics(&state)
        .iter()
        .any(|s| s.contains("columns cannot fit")));

    // Text-bearing data cells keep their readable floor. Only intrinsic,
    // individually painted compact chips have no shared column contract.
    let mut data = section;
    for row in data["children"].as_array_mut().unwrap().iter_mut().skip(1) {
        for cell in row["children"].as_array_mut().unwrap() {
            cell["width"] = json!("fill_container");
        }
    }
    assert_eq!(table_rows(&data).len(), 2);
    assert_eq!(table_columns_exceed_width(&data, &rects), Some((3, 327)));
}

fn bar_tracks_and_labels() -> Value {
    let tracks: Vec<Value> = [62, 40, 76, 52, 88, 30, 48]
        .into_iter()
        .enumerate()
        .map(|(i, height)| {
            json!({"type":"frame","id":format!("c{i}"),"width":"fill_container",
            "height":88,"layout":"vertical","justifyContent":"end",
            "alignItems":"center","children":[
                {"type":"rectangle","id":format!("b{i}"),"width":"fill_container",
                 "height":height,"cornerRadius":6,"fill":[{"type":"solid","color":"#22C55E"}]}
            ]})
        })
        .collect();
    let labels: Vec<Value> = ["一", "二", "三", "四", "五", "六", "日"]
        .into_iter()
        .enumerate()
        .map(|(i, content)| {
            json!({"type":"frame","id":format!("l{i}"),"width":"fill_container",
            "height":"fit_content","layout":"vertical","alignItems":"center","children":[
                {"type":"text","id":format!("t{i}"),"content":content,"fontSize":12}
            ]})
        })
        .collect();
    // Deliberately no chart names or roles: the distinction is structural.
    json!({"type":"frame","id":"root","width":327,"height":"fit_content",
    "layout":"vertical","padding":16,"gap":8,"children":[
        {"type":"frame","id":"a","width":"fill_container","height":88,
         "layout":"horizontal","gap":8,"alignItems":"end","children":tracks},
        {"type":"frame","id":"z","width":"fill_container","height":"fit_content",
         "layout":"horizontal","gap":8,"children":labels}
    ]})
}

fn known_row_rects() -> HashMap<String, Rect> {
    ["a", "z"]
        .into_iter()
        .map(|id| {
            (
                id.to_string(),
                Rect {
                    x: 16.0,
                    y: 16.0,
                    w: 295.0,
                    h: 88.0,
                },
            )
        })
        .collect()
}

#[test]
fn anonymous_bar_tracks_and_labels_are_not_table_rows_in_real_layout() {
    let doc: jian_ops_schema::PenDocument =
        serde_json::from_value(json!({"version":"1.0","children":[bar_tracks_and_labels()]}))
            .unwrap();
    let state = EditorState::from_document(doc);
    let root = serde_json::to_value(&state.active_children()[0]).unwrap();
    let rects = resolved_rects(&state);

    assert!((rects["a"].w - 295.0).abs() < 0.5);
    assert!(table_rows(&root).is_empty());
    assert!(table_columns_exceed_width(&root, &rects).is_none());
    assert!(table_overflow_scale(&root, &rects).is_none());
    let mut ops = Vec::new();
    collect_scale_ops(&state, &root, &rects, &mut ops);
    assert!(ops.is_empty(), "a valid chart must retain its bar geometry");
    assert!(
        !geometry_diagnostics(&state)
            .iter()
            .any(|message| message.contains("columns cannot fit")),
        "date labels do not need seven full table-text width reservations"
    );
}

#[test]
fn anonymous_seven_column_text_table_keeps_its_readable_floor() {
    for clipped in [false, true] {
        let mut table = bar_tracks_and_labels();
        // Two genuine text rows, including a misleading chart name and
        // clipping, must still report their impossible readable width.
        table["name"] = json!("activity-chart-card");
        table["clipContent"] = json!(clipped);
        let mut first_row = table["children"][1].clone();
        first_row["id"] = json!("a");
        table["children"][0] = first_row;
        assert_eq!(table_rows(&table).len(), 2);
        assert_eq!(
            table_columns_exceed_width(&table, &known_row_rects()),
            Some((7, 295))
        );
    }
}

#[test]
fn content_inside_a_bar_like_row_preserves_table_detection() {
    let content_nodes = [
        json!({"type":"text","content":"42","height":40}),
        json!({"type":"icon_font","iconFontName":"star","height":40}),
        json!({"type":"image","imageSearchQuery":"portrait","height":40}),
        json!({"type":"rectangle","height":40,
            "fill":[{"type":"image","url":"https://example.com/photo.jpg","mode":"fill"}]}),
        json!({"type":"rectangle","height":40,"children":[
            {"type":"text","content":"42"}]}),
    ];
    for node in content_nodes {
        let mut table = bar_tracks_and_labels();
        table["children"][0]["children"][0]["children"][0] = node.clone();
        assert_eq!(table_rows(&table).len(), 2, "content cell: {node}");
        assert_eq!(
            table_columns_exceed_width(&table, &known_row_rects()),
            Some((7, 295))
        );
    }
    let mut table = bar_tracks_and_labels();
    table["children"][0]["children"][0]["fill"] =
        json!([{"type":"image","url":"https://example.com/photo.jpg","mode":"fill"}]);
    assert_eq!(
        table_rows(&table).len(),
        2,
        "track image fill is content too"
    );
}

#[test]
fn rectangles_without_bottom_aligned_tracks_are_not_exempt() {
    let mut table = bar_tracks_and_labels();
    table["children"][0]["children"][0]["justifyContent"] = json!("start");
    assert_eq!(table_rows(&table).len(), 2);
    assert_eq!(
        table_columns_exceed_width(&table, &known_row_rects()),
        Some((7, 295))
    );
}
