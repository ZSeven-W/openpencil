//! End-to-end generator runtime tests: the real QuickJS sandbox + the
//! `batch_program` executor, driven through `EditorState`.

use jian_ops_schema::node::PenNode;
use op_editor_core::generator::{
    GeneratorError, GeneratorParam, GeneratorParamKind, GeneratorSpec, GENERATOR_STARTERS,
};
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use serde_json::json;

use super::*;

fn frame() -> PenNode {
    serde_json::from_value(json!({
        "type": "frame", "id": "tmp", "name": "Gen", "layout": "vertical",
        "width": "fit_content", "height": "fit_content"
    }))
    .unwrap()
}

fn spec(program: &str) -> GeneratorSpec {
    GeneratorSpec::new(
        program,
        vec![GeneratorParam {
            name: "count".into(),
            label: None,
            kind: GeneratorParamKind::Number,
            value: json!(3),
        }],
    )
}

fn create(program: &str) -> Result<(EditorState, NodeId), GeneratorError> {
    let mut state = EditorState::new();
    let (command, id) = state.generator_create_command(
        &NodeId::NONE,
        None,
        frame(),
        spec(program),
        Some(run_generator),
    )?;
    assert!(state.apply(command));
    Ok((state, id))
}

fn node<'a>(state: &'a EditorState, id: &NodeId) -> &'a PenNode {
    op_editor_core::walkers::find_node(state.active_children(), id).expect("node")
}

fn texts(node: &PenNode, out: &mut Vec<String>) {
    let value = serde_json::to_value(node).unwrap();
    if value["type"] == "text" {
        out.push(value["content"].as_str().unwrap_or_default().to_string());
    }
    for child in node.children().map(Vec::as_slice).unwrap_or(&[]) {
        texts(child, out);
    }
}

const COUNTING: &str = r##"
for (var i = 0; i < params.count; i++) {
  var card = I(root, {type: "frame", name: "Card " + i, width: 40, height: 40, layout: "vertical"});
  I(card, {type: "text", name: "Label", content: "#" + i, fontSize: 12});
}
"##;

#[test]
fn program_output_becomes_owned_children_with_derived_ids() {
    let (state, id) = create(COUNTING).expect("runs");
    let generator = node(&state, &id);
    let children = generator.children().expect("children");
    assert_eq!(children.len(), 3);
    assert_eq!(children[1].id_str(), format!("{id}_g1"));
    assert_eq!(
        children[1].children().unwrap()[0].id_str(),
        format!("{id}_g1_0")
    );
    let mut labels = Vec::new();
    texts(generator, &mut labels);
    assert_eq!(labels, vec!["#0", "#1", "#2"]);
}

#[test]
fn same_program_and_params_are_byte_identical_including_random_and_clock() {
    let program = r#"
      for (var i = 0; i < params.count; i++) {
        I(root, {type: "text", name: "r", content: String(Math.random()) + "|" + Date.now() + "|" + new Date().getTime()});
      }
    "#;
    let (a, id) = create(program).expect("runs");
    let (b, _) = create(program).expect("runs");
    let json_a = serde_json::to_string(node(&a, &id)).unwrap();
    assert_eq!(json_a, serde_json::to_string(node(&b, &id)).unwrap());
    let mut labels = Vec::new();
    texts(node(&a, &id), &mut labels);
    assert!(labels.iter().all(|label| label.ends_with("|0|0")));
    assert_ne!(labels[0], labels[1], "the seeded PRNG still advances");
}

#[test]
fn runaway_program_is_stopped_and_the_document_is_unchanged() {
    let (mut state, id) = create(COUNTING).expect("runs");
    let doc = serde_json::to_string(&state.doc).unwrap();
    // Swap the program for a runaway one by editing the stored spec.
    let mut runaway = state.generator_spec(&id).unwrap();
    runaway.program = "while (true) {}".into();
    let patch = json!({ "explain": runaway.to_explain() }).to_string();
    assert!(state.apply(op_editor_core::EditorCommand::PatchNodeData {
        node_id: id.clone(),
        patch_json: patch,
        page_id: None,
    }));
    let before = serde_json::to_string(&state.doc).unwrap();
    assert_ne!(before, doc);
    let started = std::time::Instant::now();
    let error = state
        .regenerate_generator(&id, Some(run_generator))
        .unwrap_err();
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert_eq!(error, GeneratorError::TimeLimit { budget_ms: 1500 });
    assert_eq!(serde_json::to_string(&state.doc).unwrap(), before);
    assert!(state
        .ui
        .generator_error
        .as_ref()
        .is_some_and(|parked| parked.message.contains("time limit")));
}

#[test]
fn output_floods_are_refused_not_truncated() {
    let error =
        create("for (var i = 0; i < 5000; i++) I(root, {type: 'rectangle', width: 1, height: 1});")
            .unwrap_err();
    assert!(
        matches!(error, GeneratorError::TooManyNodes { .. }),
        "{error}"
    );
    let error =
        create("for (var i = 0; i < 2100; i++) I(root, {type: 'rectangle', width: 1, height: 1});")
            .unwrap_err();
    assert!(
        matches!(error, GeneratorError::TooManyNodes { .. }),
        "{error}"
    );
}

#[test]
fn sandbox_refuses_updates_throws_and_bad_nodes_clearly() {
    let error = create("U('n1', {name: 'x'});").unwrap_err();
    assert!(
        error.to_string().contains("U() is not available"),
        "{error}"
    );
    let error = create("throw new Error('boom');").unwrap_err();
    assert!(error.to_string().contains("boom"), "{error}");
    let error = create("I(null, {type: 'rectangle'});").unwrap_err();
    assert!(error.to_string().contains("insert under root"), "{error}");
    let error = create("I(root, {type: 'teleporter'});").unwrap_err();
    assert!(matches!(error, GeneratorError::Program(_)), "{error}");
    assert!(
        create("fetch('https://example.com')").is_err(),
        "no IO globals"
    );
}

#[test]
fn empty_output_is_a_valid_generator() {
    let (state, id) = create("if (params.count < 0) I(root, {type: 'rectangle'});").expect("runs");
    assert!(node(&state, &id)
        .children()
        .map(Vec::is_empty)
        .unwrap_or(true));
}

#[test]
fn card_grid_starter_follows_its_params() {
    let mut state = EditorState::new();
    let starter = &GENERATOR_STARTERS[0];
    let id = state
        .insert_generator_starter(starter, Some(run_generator))
        .expect("card grid runs");
    let mut labels = Vec::new();
    texts(node(&state, &id), &mut labels);
    assert_eq!(labels[0], "Quarterly metrics");
    assert!(labels.contains(&"Uptime".to_string()));
    assert!(labels.contains(&"99.98%".to_string()));
    // 6 items, 3 columns → title + 2 rows.
    assert_eq!(node(&state, &id).children().unwrap().len(), 3);
    let before = state.snapshot_for_history();
    state
        .set_generator_param_input(&id, 2, "2", Some(run_generator))
        .expect("columns = 2");
    state.history_push_past(before);
    assert_eq!(node(&state, &id).children().unwrap().len(), 4);
    assert_eq!(
        state.toggle_generator_bool_param(&id, 4, Some(run_generator)),
        Ok(true)
    );
    let mut labels = Vec::new();
    texts(node(&state, &id), &mut labels);
    assert!(!labels.contains(&"99.98%".to_string()), "values hidden");
    assert!(state.undo());
    assert!(state.undo());
    assert_eq!(node(&state, &id).children().unwrap().len(), 3);
}

#[test]
fn calendar_starter_lays_out_september_2026() {
    let mut state = EditorState::new();
    let id = state
        .insert_generator_starter(&GENERATOR_STARTERS[1], Some(run_generator))
        .expect("calendar runs");
    let generator = node(&state, &id);
    let children = generator.children().unwrap();
    // Title + weekday header + 5 weeks (Sep 1 2026 is a Tuesday).
    assert_eq!(children.len(), 7);
    let first_week = &children[2];
    let first_day = &first_week.children().unwrap()[1];
    assert_eq!(first_day.base().name.as_deref(), Some("Day 1"));
    let mut labels = Vec::new();
    texts(generator, &mut labels);
    assert_eq!(labels[0], "September 2026");
    assert!(labels.contains(&"30".to_string()));
    assert!(!labels.contains(&"31".to_string()));
}

#[test]
fn saved_generator_renders_through_a_plain_reader_without_a_runtime() {
    let (state, id) = create(COUNTING).expect("runs");
    let saved = serde_json::to_string_pretty(&state.doc).unwrap();
    // A reader that never runs programs: canonical load + layout scene.
    let loaded = op_pen_loader::load_canonical(&saved).expect("loads").value;
    let scene =
        op_pen_loader::pen_document_to_layout_scene(&loaded, &std::collections::BTreeMap::new(), 0);
    let page = scene.active_page().expect("page");
    for index in 0..3 {
        let child = page
            .find(&format!("{id}_g{index}"))
            .expect("generated child is in the render scene");
        assert!(child.bounds.size.x > 0.0 && child.bounds.size.y > 0.0);
        assert!(page.find(&format!("{id}_g{index}_0")).is_some());
    }
    // The spec survives the round trip too.
    let reloaded = EditorState::from_document(loaded);
    assert_eq!(reloaded.generator_spec(&id), state.generator_spec(&id));
}

#[test]
fn large_outputs_build_in_linear_time() {
    let started = std::time::Instant::now();
    let (state, id) =
        create("for (var i = 0; i < 1500; i++) I(root, {type: 'rectangle', width: 1, height: 1});")
            .expect("1 500 nodes are within the limit");
    assert_eq!(node(&state, &id).children().unwrap().len(), 1500);
    // The per-line document simulation this replaced took ~10 s here.
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}
