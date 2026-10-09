//! Native acceptance for one retained draft and a separate controlled dataset.
use op_editor_core::table_pagination::Spec;
use op_preview_core::{PreviewInput, PreviewInputEnvelope, PreviewSession};
use serde_json::{json, Value};
fn find<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
    if v["id"] == id {
        return Some(v);
    }
    v["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|n| find(n, id))
}
fn click(s: &mut PreviewSession, id: &str) {
    let b = s
        .preview_scene_for_test()
        .active_page()
        .unwrap()
        .find(id)
        .unwrap()
        .bounds;
    s.dispatch_tap(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
}
fn key(s: &mut PreviewSession, name: &str) {
    s.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: name.into(),
        code: name.into(),
        repeat: false,
        modifiers: Default::default(),
    }));
}
fn choose(s: &mut PreviewSession, node: &Value, key_name: &str, value: &str) {
    click(s, node["id"].as_str().unwrap());
    let current = s
        .app_state_value_for_test(key_name)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();
    let options = node["options"].as_array().unwrap();
    let a = options.iter().position(|v| v["value"] == current).unwrap();
    let b = options.iter().position(|v| v["value"] == value).unwrap();
    for _ in 0..((b + options.len() - a) % options.len()) {
        key(s, "ArrowDown");
    }
}
struct Capture<'a> {
    table_id: &'a str,
    root_id: &'a str,
    output: &'a std::path::Path,
    row_ids: std::collections::BTreeSet<String>,
}
fn record(
    s: &PreviewSession,
    spec: &Spec,
    capture: &Capture<'_>,
    case: (&str, usize, i64, i64),
) -> Value {
    let (name, count, page, total) = case;
    let table_id = capture.table_id;
    let root_id = capture.root_id;
    let out = capture.output;
    let scene = s.preview_scene_for_test();
    let p = scene.active_page().unwrap();
    let rows: Vec<_> = p
        .find(table_id)
        .unwrap()
        .children
        .iter()
        .filter(|n| capture.row_ids.contains(&n.id))
        .collect();
    assert_eq!(rows.len(), count, "{name}");
    let board = p.find(root_id).unwrap().bounds;
    assert!(
        rows.iter()
            .all(|row| row.bounds.origin.y + row.bounds.size.y
                <= board.origin.y + board.size.y + 1.0),
        "{name}: rows must remain within the exported board"
    );
    assert_eq!(
        s.app_state_value_for_test(&spec.page).unwrap().as_i64(),
        Some(page),
        "{name}"
    );
    assert_eq!(
        s.app_state_value_for_test(&spec.total).unwrap().as_i64(),
        Some(total),
        "{name}"
    );
    for pair in rows.windows(2) {
        assert!(
            (pair[1].bounds.origin.y - pair[0].bounds.origin.y - pair[0].bounds.size.y).abs() < 1.0,
            "{name}"
        );
    }
    std::fs::write(
        out.join(format!("{name}.png")),
        crate::export::render_node_raster_bytes(
            &scene,
            root_id,
            crate::export::RasterFormat::Png,
            1.0,
        )
        .unwrap(),
    )
    .unwrap();
    json!({"case":name,"visible_rows":count,"page":page,"total":total,"pages":s.app_state_value_for_test(&spec.pages).unwrap().0,"rows":rows.iter().map(|n|n.id.as_str()).collect::<Vec<_>>()})
}
#[test]
#[ignore = "requires OPENPENCIL_QA_PAGING_DOCUMENT and OPENPENCIL_QA_PAGING_CAPTURE"]
fn captured_local_pages_and_explicit_time_filters_are_consistent() {
    let path = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_PAGING_DOCUMENT").unwrap());
    let out = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_PAGING_CAPTURE").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let mut receipts = vec![];
    for (file, table_id, root_id, retained) in [
        (path.clone(), "n898", "n799", true),
        (
            path.with_file_name("23-record-demo.op"),
            "table",
            "root",
            false,
        ),
    ] {
        let doc = op_pen_loader::payload::load_canonical(&std::fs::read_to_string(&file).unwrap())
            .unwrap()
            .value;
        let original = serde_json::to_value(&doc).unwrap();
        let spec = Spec::from_node(find(&original, table_id).unwrap()).unwrap();
        let capture = Capture {
            table_id,
            root_id,
            output: &out,
            row_ids: find(&original, table_id).unwrap()["children"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|n| op_editor_core::table_filter_contract::is_row(n))
                .map(|n| n["id"].as_str().unwrap().to_owned())
                .collect(),
        };
        let mut s = PreviewSession::enter(
            &doc,
            (1600.0, 1200.0),
            &Default::default(),
            0,
            false,
            false,
            std::rc::Rc::new(jian_skia::SkiaMeasure::new()),
            0,
        )
        .unwrap();
        s.begin_lifecycle(0);
        s.pump(1000);
        let _ = s.preview_scene_for_test();
        s.pump(2000);
        if retained {
            let scene = s.preview_scene_for_test();
            let date = scene
                .active_page()
                .unwrap()
                .find("n890")
                .unwrap()
                .widget
                .as_ref()
                .unwrap();
            assert_eq!(date.placeholder.as_deref(), Some("Dates unavailable"));
            assert_eq!(find(&original, "n890").unwrap()["enabled"], false);
            assert_eq!(
                scene
                    .active_page()
                    .unwrap()
                    .find(&spec.items_id)
                    .unwrap()
                    .children
                    .len(),
                3
            );
            receipts.push(record(
                &s,
                &spec,
                &capture,
                ("retained-one-page", 10, 1, 10),
            ));
            click(&mut s, spec.next["id"].as_str().unwrap());
            receipts.push(record(
                &s,
                &spec,
                &capture,
                ("retained-next-disabled", 10, 1, 10),
            ));
            assert!(find(&original, "n890").unwrap()["bindings"].is_null());
        } else {
            receipts.push(record(&s, &spec, &capture, ("demo-page-1", 10, 1, 23)));
            click(&mut s, "next");
            receipts.push(record(&s, &spec, &capture, ("demo-page-2", 10, 2, 23)));
            click(&mut s, "next");
            receipts.push(record(&s, &spec, &capture, ("demo-page-3", 3, 3, 23)));
            choose(&mut s, find(&original, "size").unwrap(), &spec.size, "5");
            receipts.push(record(&s, &spec, &capture, ("demo-size-5-reset", 5, 1, 23)));
            click(&mut s, "pager-local-page-5");
            receipts.push(record(&s, &spec, &capture, ("demo-page-5", 3, 5, 23)));
            choose(
                &mut s,
                find(&original, "time").unwrap(),
                "op_filter_74696d65",
                "24h",
            );
            receipts.push(record(&s, &spec, &capture, ("demo-time-24h", 2, 1, 2)));
            choose(
                &mut s,
                find(&original, "time").unwrap(),
                "op_filter_74696d65",
                "7d",
            );
            receipts.push(record(&s, &spec, &capture, ("demo-time-7d", 5, 1, 8)));
            choose(
                &mut s,
                find(&original, "status").unwrap(),
                "op_filter_737461747573",
                "active",
            );
            receipts.push(record(
                &s,
                &spec,
                &capture,
                ("demo-time-and-status", 4, 1, 4),
            ));
            choose(
                &mut s,
                find(&original, "status").unwrap(),
                "op_filter_737461747573",
                "all",
            );
            choose(
                &mut s,
                find(&original, "time").unwrap(),
                "op_filter_74696d65",
                "__op_all",
            );
            choose(&mut s, find(&original, "size").unwrap(), &spec.size, "25");
            receipts.push(record(&s, &spec, &capture, ("demo-size-25", 23, 1, 23)));
            choose(
                &mut s,
                find(&original, "time").unwrap(),
                "op_filter_74696d65",
                "30d",
            );
            receipts.push(record(
                &s,
                &spec,
                &capture,
                ("demo-missing-activity-excluded", 22, 1, 22),
            ));
        }
        assert_eq!(serde_json::to_value(&doc).unwrap(), original);
    }
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&json!({"actual_native_input":true,"canonical_records_unchanged":true,"cases":receipts,"controlled_23_record_dataset_is_not_a_new_model_generation":true})).unwrap()).unwrap();
}
