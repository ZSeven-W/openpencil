use super::*;
use serde_json::json;

fn state(root: Value) -> EditorState {
    EditorState::from_document(
        serde_json::from_value(json!({"version":"1.0.0","children":[root]})).unwrap(),
    )
}

#[test]
fn a_fixed_slide_spends_padding_and_keeps_its_text_and_type() {
    let mut state = state(
        json!({"type":"frame","id":"root","width":1920,"height":1080,"layout":"vertical","children":[
            {"type":"frame","id":"body","width":"fill_container","height":"fit_content","layout":"vertical","padding":[144,120,108,120],"gap":100,"children":[
                {"type":"text","id":"title","content":"Heading","width":"fill_container","height":300,"fontSize":88},
                {"type":"text","id":"foot","content":"Deliverable","width":"fill_container","height":600,"fontSize":40}
            ]}
        ]}),
    );
    let before = state.doc.clone();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    repair(&mut sink, "root");
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    let board = page.find("root").unwrap();
    let foot = page.find("foot").unwrap();
    assert!(foot.bounds.origin.y + foot.bounds.size.y <= board.bounds.size.y + 1.0);
    let value = serde_json::to_value(&sink.state().doc).unwrap();
    assert_eq!(value["children"][0]["height"], json!(1080.0));
    assert_eq!(
        value["children"][0]["children"][0]["children"],
        serde_json::to_value(&before).unwrap()["children"][0]["children"][0]["children"]
    );
    let settled = sink.state().doc.clone();
    repair(&mut sink, "root");
    assert_eq!(sink.state().doc, settled);
}

#[test]
fn a_fill_wrapper_cannot_hide_overflow_in_a_nested_open_stack() {
    let mut state = state(
        json!({"type":"frame","id":"root","width":1920,"height":1080,"layout":"vertical","children":[
            {"type":"frame","id":"viewport","width":"fill_container","height":"fill_container","layout":"none","children":[
                {"type":"frame","id":"content","width":1920,"height":1400,"layout":"vertical","padding":[144,120,108,120],"gap":100,"children":[
                    {"type":"text","id":"title","content":"Heading","width":1680,"height":300,"fontSize":88},
                    {"type":"text","id":"foot","content":"Deliverable","width":1680,"height":600,"fontSize":40}
                ]}
            ]}
        ]}),
    );
    assert!(!crate::geometry_validation::fixed_board_content_diagnostics(&state).is_empty());
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    repair(&mut sink, "root");
    assert!(crate::geometry_validation::fixed_board_content_diagnostics(sink.state()).is_empty());
    let value = serde_json::to_value(&sink.state().doc).unwrap();
    assert_eq!(value["children"][0]["height"], json!(1080.0));
    assert_eq!(
        value["children"][0]["children"][0]["height"],
        "fill_container"
    );
    assert_eq!(
        value["children"][0]["children"][0]["children"][0]["children"][1]["fontSize"],
        40.0
    );
}

#[test]
fn a_dense_phone_keeps_navigation_visible_and_preserves_content_in_a_real_scroll_area() {
    let mut state = state(
        json!({"type":"frame","id":"root","width":390,"height":844,"layout":"vertical","gap":4,"children":[
            {"type":"frame","id":"status","role":"status-bar","width":"fill_container","height":62},
            {"type":"frame","id":"body","width":"fill_container","height":"fit_content","layout":"vertical","children":[
                {"type":"text","id":"copy","content":"The complete menu","width":390,"height":800,"fontSize":20}
            ]},
            {"type":"frame","id":"nav","role":"bottom-tab-bar","width":"fill_container","height":72,"children":[
                {"type":"text","id":"nav-text","content":"Home","width":100,"height":20,"fontSize":12}
            ]}
        ]}),
    );
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    repair(&mut sink, "root");
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let page = scene.active_page().unwrap();
    assert_eq!(page.find("root").unwrap().bounds.size.y, 844.0);
    let nav = page.find("nav").unwrap();
    assert!(nav.bounds.origin.y + nav.bounds.size.y <= 845.0);
    let value = serde_json::to_value(&sink.state().doc).unwrap();
    let viewport = &value["children"][0]["children"][1];
    assert_eq!(viewport["role"], "scroll-area");
    assert_eq!(viewport["clipContent"], true);
    assert!(!viewport["events"]["onScroll"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(page.find("copy").is_some());
    assert!(crate::geometry_validation::fixed_board_content_diagnostics(sink.state()).is_empty());
    let before = sink.state().doc.clone();
    repair(&mut sink, "root");
    assert_eq!(sink.state().doc, before);
}

#[test]
fn dense_content_without_excess_space_is_not_shrunk_or_deleted() {
    let mut state = state(
        json!({"type":"frame","id":"root","width":390,"height":844,"layout":"vertical","gap":4,"children":[
            {"type":"frame","id":"nav","role":"bottom-tab-bar","width":390,"height":72,"padding":20,"children":[]},
            {"type":"text","id":"copy","content":"Too much content","width":390,"height":900,"fontSize":40}
        ]}),
    );
    let before = state.doc.clone();
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    repair(&mut sink, "root");
    assert_eq!(sink.state().doc, before);
}

#[test]
fn chinese_and_unnamed_empty_flow_spacers_yield_before_a_footer_is_clipped() {
    for name in [json!("标题至日期"), json!("报头留白"), Value::Null] {
        let mut draft = state(
            json!({"type":"frame","id":"root","width":1080,"height":1440,"layout":"vertical","clipContent":true,"children":[
                {"type":"frame","id":"body","width":"fill_container","height":"fit_content","layout":"vertical","gap":16,"padding":[16,80,16,80],"children":[
                    {"type":"text","id":"title","content":"咖啡小聚","width":"fill_container","height":400,"fontSize":120},
                    {"type":"frame","id":"space-a","name":name,"width":"fill_container","height":300},
                    {"type":"text","id":"details","content":"2026年10月17日 14:30–16:00","width":"fill_container","height":180,"fontSize":36},
                    {"type":"frame","id":"space-b","name":name,"width":"fill_container","height":300},
                    {"type":"text","id":"prices","content":"拿铁 28元 · 美式 22元 · 燕麦拿铁 32元","width":"fill_container","height":180,"fontSize":36},
                    {"type":"frame","id":"space-c","name":name,"width":"fill_container","height":300},
                    {"type":"text","id":"footer","content":"到店自取，不含配送。","width":"fill_container","height":60,"fontSize":36}
                ]}
            ]}),
        );
        let before = draft.doc.clone();
        assert!(!crate::geometry_validation::fixed_board_content_diagnostics(&draft).is_empty());
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut draft };
        repair(&mut sink, "root");
        let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
        let page = scene.active_page().unwrap();
        let footer = page.find("footer").unwrap();
        assert!(
            footer.bounds.origin.y + footer.bounds.size.y <= 1441.0,
            "{name:?}: {:?}",
            footer.bounds
        );
        assert!(
            crate::geometry_validation::fixed_board_content_diagnostics(sink.state()).is_empty()
        );
        for id in ["title", "details", "prices", "footer"] {
            assert_eq!(
                op_editor_core::walkers::find_node(
                    sink.state().active_children(),
                    &NodeId::new(id)
                ),
                op_editor_core::walkers::find_node(&before.children, &NodeId::new(id))
            );
        }
        assert_eq!(
            find_root(sink.state(), "root").unwrap().height_px(),
            Some(1440.0)
        );
        let settled = sink.state().doc.clone();
        repair(&mut sink, "root");
        assert_eq!(sink.state().doc, settled);
    }
}

#[test]
fn empty_frames_with_paint_interaction_or_authored_geometry_are_not_spacing() {
    for protected in [
        json!({"fill":[{"type":"solid","color":"#FFFFFF"}]}),
        json!({"stroke":{"thickness":1,"fill":[{"type":"solid","color":"#111111"}]}}),
        json!({"effects":[{"type":"shadow"}]}),
        json!({"events":{"onTap":[{"toast":"Click"}]}}),
        json!({"role":"input"}),
        json!({"x":0}),
        json!({"y":0}),
        json!({"rotation":15}),
        json!({"constraints":{"vertical":"bottom"}}),
        json!({"clipContent":true}),
        json!({"reusable":true}),
        json!({"minHeight":200}),
        json!({"screen":"/"}),
    ] {
        let mut node =
            json!({"type":"frame","id":"space","name":"留白","height":300,"children":[]});
        node.as_object_mut()
            .unwrap()
            .extend(protected.as_object().unwrap().clone());
        let before = node.clone();
        assert_eq!(trim(&mut node, 16.0, 1.0, true), 0.0, "{protected}");
        assert_eq!(node, before);
    }
    let mut small = json!({"type":"frame","id":"small","height":4,"children":[]});
    assert_eq!(trim(&mut small, 16.0, 1.0, true), 0.0);
    assert_eq!(small["height"], 4.0);
}

/// Explicit local corpus replay; no provider or network calls. The same
/// request-scoped passes operate on a copy of each retained generated draft.
#[test]
#[ignore = "requires explicit retained local QA input/output directories"]
fn replay_retained_fixed_board_drafts() {
    if std::env::var("OPENPENCIL_QA_NATIVE_BOARD_FONTS").as_deref() == Ok("1") {
        let fonts =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/shared/fonts");
        jian_skia::register_bundled_fonts(
            [
                "Archivo-VF.ttf",
                "LibreCaslonText-VF.ttf",
                "NotoSerifSC-VF.ttf",
            ]
            .iter()
            .map(|name| std::fs::read(fonts.join(name)).unwrap())
            .collect(),
        );
    }
    let input = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_DRAFT_INPUT").unwrap());
    let output = std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_DRAFT_OUTPUT").unwrap());
    for entry in std::fs::read_dir(input).unwrap() {
        let entry = entry.unwrap();
        let source = entry.path().join("first-draft.op");
        if !source.exists() {
            continue;
        }
        let source_text = std::fs::read_to_string(&source).unwrap();
        let loaded = op_pen_loader::payload::load_canonical(&source_text).unwrap();
        let mut state = EditorState::from_document(loaded.value);
        let roots: Vec<_> = state
            .active_children()
            .iter()
            .map(|n| n.id_str().to_string())
            .collect();
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        for id in roots {
            let root = find_root(sink.state(), &id).unwrap();
            let value = serde_json::to_value(root).unwrap();
            let mut plan = crate::loop_finalize::synthesize_plan(
                std::slice::from_ref(root),
                value["width"].as_f64().unwrap(),
            );
            plan.root_frame.height = value["height"].as_f64().unwrap();
            super::super::fixed_board_backdrops::repair(&mut sink, &id, &plan);
            {
                let mut guarded = super::super::PreserveRootHeightSink {
                    inner: &mut sink,
                    root_id: &id,
                };
                crate::geometry_validation::geometry_validate_and_fix(&mut guarded, &id);
            }
            repair(&mut sink, &id);
            crate::text_contrast_repair::repair_text_contrast(&mut sink, &id);
        }
        let folder = output.join(entry.file_name());
        std::fs::create_dir_all(&folder).unwrap();
        let mut file = std::fs::File::create(folder.join("first-draft.op")).unwrap();
        // Match the host's canonical save: plain serde omits the image table
        // collected by ImageSrc serialization and produces empty screenshots.
        jian_ops_schema::image_table::write_document_with_extension(
            &mut file,
            &sink.state().doc,
            &jian_ops_schema::image_thumbs::capture_snapshot(),
            "editorMeta",
            &op_pen_loader::EditorMeta::from_state(sink.state()),
        )
        .unwrap();
    }
}
