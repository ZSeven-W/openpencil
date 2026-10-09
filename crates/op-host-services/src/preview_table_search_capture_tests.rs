//! Real search input on the retained GLM design, using production PNG rendering.
use super::*;
fn replace_query(session: &mut PreviewSession, text: &str) {
    let b = session
        .preview_scene_for_test()
        .active_page()
        .unwrap()
        .find("n843")
        .unwrap()
        .bounds;
    session.dispatch_tap(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
    session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Key {
        key: "a".into(),
        code: "KeyA".into(),
        repeat: false,
        modifiers: jian_core::gesture::pointer::Modifiers::CMD,
    }));
    key(session, "Backspace");
    // A sequence of edits must preserve focus and caret after every row reflow.
    for character in text.chars() {
        session.dispatch_input(PreviewInputEnvelope::new(PreviewInput::Text(
            character.to_string(),
        )));
    }
    let stored = session
        .app_state_value_for_test("op_filter_6e383433")
        .unwrap();
    assert_eq!(stored.as_str(), Some(text));
}
#[test]
#[ignore = "requires OPENPENCIL_QA_SEARCH_DOCUMENT and OPENPENCIL_QA_SEARCH_CAPTURE"]
fn captured_search_input_composes_with_status_and_preserves_chrome() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_SEARCH_DOCUMENT").unwrap()).unwrap();
    let doc = op_pen_loader::payload::load_canonical(&source)
        .unwrap()
        .value;
    let original = serde_json::to_value(&doc).unwrap();
    let root_id = original["children"][0]["id"].as_str().unwrap();
    let table = contract_table(&original).unwrap();
    let table_id = table["id"].as_str().unwrap();
    let role = find(&original, "n884").unwrap();
    let status = find(&original, "n887").unwrap();
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
    let page = initial.active_page().unwrap();
    let stable: Vec<_> = [root_id, "n1093", "n843"]
        .iter()
        .map(|id| ((*id).to_owned(), page.find(id).unwrap().bounds))
        .collect();
    let out = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_SEARCH_CAPTURE").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let mut receipts = vec![];
    for (name, query, role_value, status_value, count) in [
        ("all", "", "all", "__op_all", 10),
        ("name", "  MARCUS  ", "all", "__op_all", 1),
        ("email", "emma.l@nimbus.io", "all", "__op_all", 1),
        ("status-text", "suspended", "all", "__op_all", 2),
        ("combined", "nimbus.io", "editor", "active", 3),
        ("empty", "Marcus", "viewer", "suspended", 0),
        ("escaped", "\"]<script>", "all", "__op_all", 0),
        ("restored", "", "all", "__op_all", 10),
    ] {
        choose(
            &mut session,
            "n884",
            role_value,
            role["options"].as_array().unwrap(),
        );
        choose(
            &mut session,
            "n887",
            status_value,
            status["options"].as_array().unwrap(),
        );
        replace_query(&mut session, query);
        let scene = session.preview_scene_for_test();
        let page = scene.active_page().unwrap();
        let rows = &page.find(table_id).unwrap().children;
        let matches: Vec<_> = rows
            .iter()
            .filter(|row| row_ids.contains(&row.id))
            .collect();
        assert_eq!(matches.len(), count, "{name}");
        for pair in matches.windows(2) {
            assert!(
                (pair[1].bounds.origin.y - pair[0].bounds.origin.y - pair[0].bounds.size.y).abs()
                    < 1.0,
                "{name}"
            );
        }
        for (id, bounds) in &stable {
            assert_eq!(page.find(id).unwrap().bounds, *bounds, "{name}: {id}");
        }
        assert_eq!(
            page.find("n1094").unwrap().text.as_deref(),
            Some(format!("Rows on this page: {count}").as_str())
        );
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
        receipts.push(json!({"case":name,"query":query,"role":role_value,"status":status_value,"count":count,"rows":matches.iter().map(|n|n.id.as_str()).collect::<Vec<_>>(),"footer_and_search_stationary":true,"continuous_typing_verified":true}));
    }
    assert_eq!(serde_json::to_value(&doc).unwrap(), original);
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&json!({"native_skia":true,"canonical_document_unchanged":true,"actual_pointer_keyboard_and_text_input":true,"cases":receipts,"limits":["time filtering","pagination","sorting","desktop window workflow","Android device"]})).unwrap()).unwrap();
}
