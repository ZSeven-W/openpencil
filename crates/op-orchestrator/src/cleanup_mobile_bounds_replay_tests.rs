//! Reapply only the owned fallback and mobile chrome contracts to a real draft.

use super::*;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

fn bounds(state: &EditorState, id: &str) -> [f32; 4] {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let b = scene.active_page().unwrap().find(id).unwrap().bounds;
    [b.origin.x, b.origin.y, b.size.x, b.size.y]
}

#[test]
#[ignore = "requires OPENPENCIL_QA_MOBILE_BOUNDS_INPUT and output path"]
fn replay_retained_mobile_bounds() {
    let source =
        std::fs::read_to_string(std::env::var("OPENPENCIL_QA_MOBILE_BOUNDS_INPUT").unwrap())
            .unwrap();
    let loaded = op_pen_loader::payload::load_canonical(&source).unwrap();
    let mut state = EditorState::from_document(loaded.value);
    let before = state.doc.clone();
    let root_id = state.active_children()[0].id_str().to_owned();
    let nav_id = state.active_children()[0]
        .children()
        .unwrap()
        .last()
        .unwrap()
        .id_str()
        .to_owned();
    let issues_before = crate::geometry_validation::geometry_diagnostics(&state);
    let nav_before = bounds(&state, &nav_id);
    let patches = op_image_enrich::image_fallback_policy(&state.active_children()[0], false);
    for patch in &patches {
        state.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(&patch.node_id),
            patch_json: patch.patch_json.clone(),
            page_id: None,
        });
    }
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    assert!(super::super::mobile_scroll::repair(&mut sink, &root_id));
    let nav_after = bounds(sink.state(), &nav_id);
    let board = bounds(sink.state(), &root_id);
    let issues_after = crate::geometry_validation::geometry_diagnostics(sink.state());
    assert!(
        issues_after.is_empty(),
        "{issues_after:?}; nav before={nav_before:?}, after={nav_after:?}, board={board:?}"
    );
    assert!(nav_after[1] + nav_after[3] <= board[1] + board[3] + 1.0);
    let settled = sink.state().doc.clone();
    assert!(!super::super::mobile_scroll::repair(&mut sink, &root_id));
    assert_eq!(sink.state().doc, settled);
    let output =
        std::path::PathBuf::from(std::env::var("OPENPENCIL_QA_MOBILE_BOUNDS_OUTPUT").unwrap());
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(&output).unwrap();
    jian_ops_schema::image_table::write_document_with_extension(
        &mut file,
        &sink.state().doc,
        &jian_ops_schema::image_thumbs::capture_snapshot(),
        "editorMeta",
        &op_pen_loader::EditorMeta::from_state(sink.state()),
    )
    .unwrap();
    std::fs::write(output.with_extension("receipt.json"),serde_json::to_vec_pretty(&json!({
        "issues_before":issues_before,"issues_after":issues_after,"nav_before":nav_before,"nav_after":nav_after,
        "fixed_board":board,"fallback_patches":patches.len(),"root_height_preserved":before.children[0].height_px()==sink.state().doc.children[0].height_px(),
        "extra_model_calls":0,"source_document_mutated":false
    })).unwrap()).unwrap();
}
