//! Native input and raster acceptance on a retained production OP draft.
use op_preview_core::{PreviewInput, PreviewInputEnvelope, PreviewSession};
use serde_json::{json, Value};

fn key(session: &mut PreviewSession, key: &str) {
    session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: key.into(),
        code: key.into(),
        repeat: false,
        modifiers: Default::default(),
    }));
}
fn choose(session: &mut PreviewSession, id: &str, value: &str, options: &[Value]) {
    let scene = session.preview_scene_for_test();
    let b = scene.active_page().unwrap().find(id).unwrap().bounds;
    session.dispatch_tap(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
    let current = session
        .app_state_value_for_test(&format!(
            "op_filter_{}",
            id.bytes().map(|b| format!("{b:02x}")).collect::<String>()
        ))
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap();
    let a = options.iter().position(|v| v["value"] == current).unwrap();
    let b = options.iter().position(|v| v["value"] == value).unwrap();
    for _ in 0..((b + options.len() - a) % options.len()) {
        key(session, "ArrowDown");
    }
}
fn find<'a>(value: &'a Value, id: &str) -> Option<&'a Value> {
    if value["id"] == id {
        return Some(value);
    }
    value["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|v| find(v, id))
}
fn contract_table(value: &Value) -> Option<&Value> {
    if value["explain"]
        .as_str()
        .is_some_and(|s| s.starts_with(op_editor_core::table_filter_contract::TABLE_MARKER))
    {
        return Some(value);
    }
    value["children"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(contract_table)
}
#[test]
#[ignore = "requires OPENPENCIL_QA_TABLE_DOCUMENT and OPENPENCIL_QA_TABLE_CAPTURE"]
fn retained_console_filters_reflow_and_restore_the_canonical_dataset() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_TABLE_DOCUMENT").unwrap()).unwrap();
    let doc = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let original = serde_json::to_value(&doc).unwrap();
    let root_id = original["children"][0]["id"].as_str().unwrap();
    let table = contract_table(&original).unwrap();
    let table_id = table["id"].as_str().unwrap();
    let footer_id = "n1093";
    let counter_id = find(&original, footer_id).unwrap()["children"][0]["id"]
        .as_str()
        .unwrap();
    let role = find(&original, "n884").unwrap();
    let status = find(&original, "n887").unwrap();
    let role_options = role["options"].as_array().unwrap();
    let status_options = status["options"].as_array().unwrap();
    let row_ids: std::collections::BTreeSet<_> = table["children"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| op_editor_core::table_filter_contract::is_row(v))
        .map(|v| v["id"].as_str().unwrap().to_owned())
        .collect();
    let mut session = PreviewSession::enter(
        &doc,
        (1600.0, 1100.0),
        &Default::default(),
        0,
        false,
        false,
        std::rc::Rc::new(jian_skia::SkiaMeasure::new()),
        0,
    )
    .unwrap();
    session.begin_lifecycle(0);
    session.pump(1000);
    let _ = session.preview_scene_for_test();
    session.pump(2000);
    let initial = session.preview_scene_for_test();
    let initial_page = initial.active_page().unwrap();
    let footer_bounds = initial_page.find(footer_id).unwrap().bounds;
    let root_bounds = initial_page.find(root_id).unwrap().bounds;
    let out = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_TABLE_CAPTURE").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let mut receipts = vec![];
    for (name, role_value, status_value, count) in [
        ("all", "all", "__op_all", 10),
        ("active", "all", "active", 6),
        ("suspended", "all", "suspended", 2),
        ("editor", "editor", "__op_all", 4),
        ("editor-active", "editor", "active", 3),
        ("empty", "viewer", "suspended", 0),
        ("restored", "all", "__op_all", 10),
    ] {
        choose(&mut session, "n884", role_value, role_options);
        choose(&mut session, "n887", status_value, status_options);
        let scene = session.preview_scene_for_test();
        let page = scene.active_page().unwrap();
        let rows = &page.find(table_id).unwrap().children;
        let visible: Vec<_> = rows.iter().filter(|n| row_ids.contains(&n.id)).collect();
        assert_eq!(visible.len(), count, "{name}");
        for pair in visible.windows(2) {
            assert!(
                (pair[1].bounds.origin.y - pair[0].bounds.origin.y - pair[0].bounds.size.y).abs()
                    < 1.0,
                "{name} gap {:?} {:?}",
                pair[0].bounds,
                pair[1].bounds
            );
        }
        assert_eq!(
            page.find(footer_id).unwrap().bounds,
            footer_bounds,
            "{name}"
        );
        assert_eq!(page.find(root_id).unwrap().bounds, root_bounds, "{name}");
        let counter = page.find(counter_id).unwrap().text.as_deref().unwrap();
        assert_eq!(counter, format!("Rows on this page: {count}"), "{name}");
        assert_eq!(
            page.find(&format!("{table_id}-filter-empty-label"))
                .is_some(),
            count == 0
        );
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
        receipts.push(json!({"case":name,"role":role_value,"status":status_value,"rows":visible.iter().map(|n|n.id.as_str()).collect::<Vec<_>>(),"count":count,"counter":counter,"footer_stationary":true,"row_gaps_compact":true}));
    }
    assert_eq!(serde_json::to_value(&doc).unwrap(), original);
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&json!({"native_skia":true,"real_pointer_and_key_input":true,"canonical_dataset_unchanged":true,"root":root_id,"table":table_id,"cases":receipts,"limits":["time filtering","pagination","sorting"]})).unwrap()).unwrap();
}

#[path = "preview_table_search_capture_tests.rs"]
mod search;
