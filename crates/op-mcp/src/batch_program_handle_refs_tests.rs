//! Handle references in `children` (`batch_program_handle_refs.rs`): a
//! container that lists already-inserted nodes by binding adopts them in
//! order instead of failing the whole line.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use serde_json::Value;

use super::batch_program_test_support::{binding_id, call_operations, call_operations_best_effort};
use super::test_fixtures::sample;

/// Sidebar shell shared by the scenarios: a vertical root plus one row.
const SHELL: &str = concat!(
    "b1=I(null, {\"type\":\"frame\",\"name\":\"sidebar\",\"width\":240,\"height\":600,\"layout\":\"vertical\"})\n",
    "b8=I(b1, {\"type\":\"frame\",\"name\":\"project-row\",\"layout\":\"horizontal\",\"width\":\"fill_container\",\"height\":32,\"gap\":8})\n",
);

fn run(program: &str) -> (EditorState, Value) {
    let mut state = sample();
    let (envelope, command) = call_operations_best_effort(&state, program);
    assert!(
        state.apply(command.expect("program emits a command")),
        "{envelope}"
    );
    (state, envelope)
}

fn node<'a>(state: &'a EditorState, id: &str) -> &'a PenNode {
    op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(id))
        .unwrap_or_else(|| panic!("node {id} missing"))
}

fn child_ids(state: &EditorState, id: &str) -> Vec<String> {
    node(state, id)
        .children()
        .map(|kids| kids.iter().map(|k| k.id_str().to_string()).collect())
        .unwrap_or_default()
}

fn warnings(envelope: &Value) -> Vec<String> {
    envelope["warnings"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|w| w["warning"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Canonical JSON of a subtree with every id stripped — the "same document
/// modulo id allocation" comparison.
fn shape(node: &PenNode) -> Value {
    fn strip(value: &mut Value) {
        if let Value::Object(map) = value {
            map.remove("id");
            if let Some(Value::Array(children)) = map.get_mut("children") {
                children.iter_mut().for_each(strip);
            }
        }
    }
    let mut value = serde_json::to_value(node).expect("node json");
    strip(&mut value);
    value
}

#[test]
fn w01_dot_created_first_moves_into_its_ring_container() {
    // The measured GLM-5.3-Flash w01 shape: the dot (b9) is inserted into
    // the row, THEN the ring frame lists it by handle.
    let program = format!(
        "{SHELL}b9=I(b8, {{\"type\":\"text\",\"content\":\"●\",\"fontSize\":10}})\n\
         b10=I(b8, {{\"type\":\"frame\",\"width\":20,\"height\":20,\"cornerRadius\":10,\"layout\":\"horizontal\",\"alignItems\":\"center\",\"justifyContent\":\"center\",\"stroke\":{{\"thickness\":2,\"fill\":[{{\"type\":\"solid\",\"color\":\"#FFFFFF\"}}]}},\"children\":[\"b9\"]}})"
    );
    let (state, envelope) = run(&program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let (b8, b9, b10) = (
        binding_id(&envelope, "b8"),
        binding_id(&envelope, "b9"),
        binding_id(&envelope, "b10"),
    );
    assert_eq!(
        child_ids(&state, &b10),
        vec![b9.clone()],
        "dot moved into ring"
    );
    assert_eq!(child_ids(&state, &b8), vec![b10], "row keeps only the ring");
    assert!(warnings(&envelope).is_empty(), "{envelope}");
}

#[test]
fn user_card_adopts_two_handles_in_array_order() {
    // `b56=I(b51, {…,"children":["b54","b55"]})` — the name and the role
    // were inserted first; the meta column must hold them in listed order
    // even when the handles were created in the opposite order.
    let program = format!(
        "{SHELL}b51=I(b1, {{\"type\":\"frame\",\"name\":\"user-card\",\"layout\":\"horizontal\",\"gap\":10}})\n\
         b55=I(b51, {{\"type\":\"text\",\"content\":\"Product lead\",\"fontSize\":12}})\n\
         b54=I(b51, {{\"type\":\"text\",\"content\":\"Lin Xiaoyu\",\"fontSize\":14}})\n\
         b56=I(b51, {{\"type\":\"frame\",\"name\":\"user-meta\",\"layout\":\"vertical\",\"width\":\"fill_container\",\"gap\":2,\"children\":[\"b54\",\"b55\"]}})"
    );
    let (state, envelope) = run(&program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let meta = binding_id(&envelope, "b56");
    assert_eq!(
        child_ids(&state, &meta),
        vec![binding_id(&envelope, "b54"), binding_id(&envelope, "b55")]
    );
    assert_eq!(
        child_ids(&state, &binding_id(&envelope, "b51")),
        vec![meta],
        "the card holds only the meta column — no loose leaves"
    );
}

#[test]
fn unknown_handle_entry_is_dropped_with_a_warning_but_the_container_lands() {
    let program = format!(
        "{SHELL}b9=I(b8, {{\"type\":\"text\",\"content\":\"18\"}})\n\
         b10=I(b8, {{\"type\":\"frame\",\"name\":\"pill\",\"layout\":\"horizontal\",\"children\":[\"b99\",\"b9\"]}})"
    );
    let (state, envelope) = run(&program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let pill = binding_id(&envelope, "b10");
    assert_eq!(child_ids(&state, &pill), vec![binding_id(&envelope, "b9")]);
    let notes = warnings(&envelope);
    assert_eq!(notes.len(), 1, "{envelope}");
    assert!(
        notes[0].contains("\"b99\"") && notes[0].contains("no earlier line bound"),
        "{notes:?}"
    );
}

#[test]
fn ancestor_handle_is_rejected_as_a_cycle() {
    // The row lists its own parent: moving b1 inside would detach the tree.
    let program =
        format!("{SHELL}b10=I(b8, {{\"type\":\"frame\",\"name\":\"loop\",\"children\":[\"b1\"]}})");
    let (state, envelope) = run(&program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let b1 = binding_id(&envelope, "b1");
    let row = binding_id(&envelope, "b8");
    let ring = binding_id(&envelope, "b10");
    assert!(child_ids(&state, &ring).is_empty());
    assert_eq!(child_ids(&state, &b1), vec![row]);
    assert!(
        warnings(&envelope)
            .iter()
            .any(|w| w.contains("\"b1\"") && w.contains("cycle")),
        "{envelope}"
    );
}

#[test]
fn handle_form_equals_the_inline_form() {
    // Mixed inline objects and handles, including a nested container that
    // itself lists a handle: the final tree must match the program that
    // wrote every child inline, in the same order.
    let handle_form = format!(
        "{SHELL}h1=I(b8, {{\"type\":\"icon_font\",\"iconFontName\":\"folder\",\"width\":16,\"height\":16}})\n\
         h2=I(b8, {{\"type\":\"text\",\"content\":\"12\",\"fontSize\":11}})\n\
         b20=I(b8, {{\"type\":\"frame\",\"name\":\"row-body\",\"layout\":\"horizontal\",\"gap\":6,\"children\":[\
         \"h1\",\
         {{\"type\":\"text\",\"content\":\"Roadmap\",\"fontSize\":13}},\
         {{\"type\":\"frame\",\"name\":\"count-pill\",\"layout\":\"horizontal\",\"padding\":[0,8],\"children\":[\"h2\"]}}\
         ]}})"
    );
    let inline_form = format!(
        "{SHELL}b20=I(b8, {{\"type\":\"frame\",\"name\":\"row-body\",\"layout\":\"horizontal\",\"gap\":6,\"children\":[\
         {{\"type\":\"icon_font\",\"iconFontName\":\"folder\",\"width\":16,\"height\":16}},\
         {{\"type\":\"text\",\"content\":\"Roadmap\",\"fontSize\":13}},\
         {{\"type\":\"frame\",\"name\":\"count-pill\",\"layout\":\"horizontal\",\"padding\":[0,8],\"children\":[\
         {{\"type\":\"text\",\"content\":\"12\",\"fontSize\":11}}]}}\
         ]}})"
    );
    let (handle_state, handle_env) = run(&handle_form);
    let (inline_state, inline_env) = run(&inline_form);
    assert!(handle_env.get("errors").is_none(), "{handle_env}");
    assert!(handle_env.get("warnings").is_none(), "{handle_env}");
    let handle_root = node(&handle_state, &binding_id(&handle_env, "b1"));
    let inline_root = node(&inline_state, &binding_id(&inline_env, "b1"));
    assert_eq!(shape(handle_root), shape(inline_root));
}

#[test]
fn transactional_surface_keeps_the_batch_and_reports_the_warning() {
    // The agent-facing path must not roll back over a dropped handle entry:
    // warnings are informational, only `errors[]` aborts the batch.
    let state = sample();
    let program = format!(
        "{SHELL}b10=I(b8, {{\"type\":\"frame\",\"name\":\"pill\",\"children\":[\"ghost\"]}})"
    );
    let (envelope, command) = call_operations(&state, &program);
    assert!(command.is_some(), "{envelope}");
    assert!(envelope.get("errors").is_none(), "{envelope}");
    assert_eq!(warnings(&envelope).len(), 1, "{envelope}");
}
