//! Generator spec, materialization, and EditorState flow tests. The real
//! JS runtime lives in op-mcp (and has its own tests); these drive the
//! editor half through small Rust runners.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

use super::*;
use crate::{EditorCommand, EditorState, NodeId};

fn rect(name: &str, w: f64) -> PenNode {
    serde_json::from_value(json!({
        "type": "rectangle", "id": "raw", "name": name, "width": w, "height": 10,
        "fill": [{"type": "solid", "color": "#112233"}]
    }))
    .expect("rect")
}

/// Emits `count` rectangles, each `width` wide, nested one level when
/// `nested` is true.
fn counting_runner(request: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
    let params = request.spec.params_object();
    let count = params["count"].as_f64().unwrap_or(0.0) as usize;
    let width = params.get("width").and_then(Value::as_f64).unwrap_or(10.0);
    Ok((0..count)
        .map(|i| rect(&format!("Item {i}"), width))
        .collect())
}

fn failing_runner(_: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
    Err(GeneratorError::TimeLimit { budget_ms: 1500 })
}

fn flood_runner(_: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
    Ok((0..MAX_GENERATED_NODES + 1)
        .map(|_| rect("x", 1.0))
        .collect())
}

fn spec(count: f64) -> GeneratorSpec {
    GeneratorSpec::new(
        "for (var i = 0; i < params.count; i++) I(root, {type: 'rectangle'});",
        vec![
            GeneratorParam {
                name: "count".into(),
                label: Some("Count".into()),
                kind: GeneratorParamKind::Number,
                value: json!(count),
            },
            GeneratorParam {
                name: "width".into(),
                label: None,
                kind: GeneratorParamKind::Number,
                value: json!(10),
            },
            GeneratorParam {
                name: "dark".into(),
                label: None,
                kind: GeneratorParamKind::Boolean,
                value: json!(false),
            },
        ],
    )
}

fn frame() -> PenNode {
    serde_json::from_value(json!({
        "type": "frame", "id": "tmp", "name": "Gen", "explain": "human note",
        "layout": "horizontal", "width": "fit_content", "height": "fit_content"
    }))
    .expect("frame")
}

fn state_with_generator(count: f64) -> (EditorState, NodeId) {
    let mut state = EditorState::new();
    let (command, id) = state
        .generator_create_command(
            &NodeId::NONE,
            None,
            frame(),
            spec(count),
            Some(counting_runner),
        )
        .expect("create");
    assert!(state.apply(command));
    (state, id)
}

fn node<'a>(state: &'a EditorState, id: &NodeId) -> &'a PenNode {
    crate::walkers::find_node(state.active_children(), id).expect("node")
}

fn child_ids(state: &EditorState, id: &NodeId) -> Vec<String> {
    node(state, id)
        .children()
        .map(|c| c.iter().map(|n| n.id_str().to_string()).collect())
        .unwrap_or_default()
}

#[test]
fn spec_round_trips_through_explain() {
    let spec = spec(3.0);
    let explain = spec.to_explain();
    assert!(explain.starts_with(GENERATOR_EXPLAIN_PREFIX));
    let parsed = GeneratorSpec::from_explain(&explain)
        .expect("marker")
        .expect("parses");
    assert_eq!(parsed, spec);
    assert!(GeneratorSpec::from_explain("just a note").is_none());
    assert!(matches!(
        GeneratorSpec::from_explain("op:generator {oops"),
        Some(Err(GeneratorError::InvalidSpec(_)))
    ));
}

#[test]
fn params_parse_per_kind_and_reject_bad_input() {
    let color = GeneratorParam {
        name: "accent".into(),
        label: None,
        kind: GeneratorParamKind::Color,
        value: json!("#FFFFFF"),
    };
    assert_eq!(color.parse_input("#abc").unwrap(), json!("#AABBCC"));
    assert!(color.parse_input("red").is_err());
    let number = &spec(1.0).params[0];
    assert_eq!(number.parse_input(" -2.5 ").unwrap(), json!(-2.5));
    assert!(number.parse_input("NaN").is_err());
    let flag = &spec(1.0).params[2];
    assert_eq!(flag.parse_input("on").unwrap(), json!(true));
    let mut bad = spec(1.0);
    bad.params[1].name = "not valid".into();
    assert!(matches!(
        bad.validate(),
        Err(GeneratorError::InvalidSpec(_))
    ));
    let mut unknown = spec(1.0);
    let mut values = serde_json::Map::new();
    values.insert("nope".into(), json!(1));
    assert_eq!(
        unknown.apply_param_values(&values),
        Err(GeneratorError::UnknownParam("nope".into()))
    );
}

#[test]
fn materialized_ids_derive_from_generator_id_and_path() {
    let mut outer = rect("outer", 1.0);
    if let PenNode::Rectangle(r) = &mut outer {
        r.children = Some(vec![rect("a", 1.0), rect("b", 1.0)]);
    }
    let out = materialize_children("n7", vec![rect("x", 1.0), outer]).expect("ok");
    assert_eq!(out[0].id_str(), "n7_g0");
    assert_eq!(out[1].id_str(), "n7_g1");
    let nested: Vec<_> = out[1]
        .children()
        .unwrap()
        .iter()
        .map(|n| n.id_str())
        .collect();
    assert_eq!(nested, vec!["n7_g1_0", "n7_g1_1"]);
}

#[test]
fn create_materializes_children_and_keeps_human_explain_inside_spec() {
    let (state, id) = state_with_generator(3.0);
    assert_eq!(
        child_ids(&state, &id),
        vec![format!("{id}_g0"), format!("{id}_g1"), format!("{id}_g2")]
    );
    let spec = state.generator_spec(&id).expect("spec");
    assert_eq!(spec.original_explain.as_deref(), Some("human note"));
    assert!(!state.generator_children_edited(&id));
}

#[test]
fn regenerating_with_same_params_is_byte_identical_and_adds_no_undo_step() {
    let (mut state, id) = state_with_generator(4.0);
    let before = serde_json::to_string(node(&state, &id)).unwrap();
    let depth = state.history.past.len();
    assert_eq!(
        state.regenerate_generator(&id, Some(counting_runner)),
        Ok(false)
    );
    assert_eq!(serde_json::to_string(node(&state, &id)).unwrap(), before);
    assert_eq!(state.history.past.len(), depth);
    // A second, independent editor produces the same bytes too.
    let (other, other_id) = state_with_generator(4.0);
    assert_eq!(other_id, id);
    assert_eq!(
        serde_json::to_string(node(&other, &other_id)).unwrap(),
        before
    );
}

#[test]
fn param_edit_regenerates_as_one_undo_step() {
    let (mut state, id) = state_with_generator(2.0);
    let before = state.snapshot_for_history();
    let changed = state
        .set_generator_param_input(&id, 0, "5", Some(counting_runner))
        .expect("regenerates");
    assert!(changed);
    // The panel commit path owns the snapshot: one push for param + children.
    state.history_push_past(before);
    assert_eq!(child_ids(&state, &id).len(), 5);
    assert_eq!(
        state.generator_spec(&id).unwrap().params[0].value,
        json!(5.0)
    );
    assert!(state.undo());
    assert_eq!(child_ids(&state, &id).len(), 2);
    assert_eq!(
        state.generator_spec(&id).unwrap().params[0].value,
        json!(2.0)
    );
}

#[test]
fn bool_toggle_records_exactly_one_history_entry() {
    let (mut state, id) = state_with_generator(1.0);
    let depth = state.history.past.len();
    assert_eq!(
        state.toggle_generator_bool_param(&id, 2, Some(counting_runner)),
        Ok(true)
    );
    assert_eq!(state.history.past.len(), depth + 1);
    assert_eq!(
        state.generator_spec(&id).unwrap().params[2].value,
        json!(true)
    );
}

#[test]
fn failures_leave_the_document_unchanged_and_report_inline() {
    let (mut state, id) = state_with_generator(2.0);
    let doc = serde_json::to_string(&state.doc).unwrap();
    let depth = state.history.past.len();
    for runner in [
        Some(failing_runner as GeneratorRunner),
        Some(flood_runner),
        None,
    ] {
        let error = state
            .set_generator_param_input(&id, 0, "9", runner)
            .unwrap_err();
        assert_eq!(serde_json::to_string(&state.doc).unwrap(), doc, "{error}");
        let parked = state.ui.generator_error.as_ref().expect("parked error");
        assert_eq!(parked.node_id, id);
        assert_eq!(parked.message, error.to_string());
    }
    assert!(matches!(
        state.set_generator_param_input(&id, 0, "abc", Some(counting_runner)),
        Err(GeneratorError::InvalidParamValue { .. })
    ));
    assert_eq!(state.history.past.len(), depth);
    assert_eq!(
        state.regenerate_generator(&id, Some(counting_runner)),
        Ok(false)
    );
    assert!(state.ui.generator_error.is_none());
}

#[test]
fn derived_id_collision_is_refused() {
    let (mut state, id) = state_with_generator(1.0);
    let squatter = rect("squatter", 1.0);
    let mut squatter = squatter;
    squatter.base_mut().id = format!("{id}_g1");
    assert!(
        state.apply(EditorCommand::InsertAuthoredSubtreePreservingRoots {
            nodes: vec![squatter],
            parent_id: NodeId::NONE,
            page_id: None,
        })
    );
    let mut values = serde_json::Map::new();
    values.insert("count".into(), json!(2));
    assert_eq!(
        state.generator_regenerate_command(&id, Some(&values), Some(counting_runner)),
        Err::<Option<EditorCommand>, _>(GeneratorError::IdCollision {
            id: format!("{id}_g1")
        })
    );
}

#[test]
fn detach_keeps_children_and_restores_the_human_explain() {
    let (mut state, id) = state_with_generator(3.0);
    let children = child_ids(&state, &id);
    assert!(state.detach_generator(&id));
    assert!(!is_generator(node(&state, &id)));
    assert_eq!(
        node(&state, &id).base().explain.as_deref(),
        Some("human note")
    );
    assert_eq!(child_ids(&state, &id), children);
    assert!(state.undo());
    assert!(is_generator(node(&state, &id)));
}

#[test]
fn hand_edited_children_are_flagged_and_overwritten_on_regenerate() {
    let (mut state, id) = state_with_generator(2.0);
    let child = NodeId::new(format!("{id}_g0"));
    assert!(state.apply(EditorCommand::SetNodeName {
        node_id: child.clone(),
        name: "Edited".into(),
    }));
    assert!(state.generator_children_edited(&id));
    assert_eq!(
        state.regenerate_generator(&id, Some(counting_runner)),
        Ok(true)
    );
    assert!(!state.generator_children_edited(&id));
    assert_eq!(node(&state, &child).base().name.as_deref(), Some("Item 0"));
}

#[test]
fn save_load_round_trip_preserves_program_params_and_children() {
    let (state, id) = state_with_generator(3.0);
    let saved = serde_json::to_string(&state.doc).unwrap();
    let loaded: crate::PenDocument = serde_json::from_str(&saved).unwrap();
    let reloaded = EditorState::from_document(loaded);
    assert_eq!(reloaded.generator_spec(&id), state.generator_spec(&id));
    assert_eq!(
        serde_json::to_string(node(&reloaded, &id)).unwrap(),
        serde_json::to_string(node(&state, &id)).unwrap()
    );
    assert!(!reloaded.generator_children_edited(&id));
}

#[test]
fn materialize_enforces_node_and_depth_limits() {
    assert!(matches!(
        flood_runner(&GeneratorRequest {
            generator_id: "g",
            spec: &spec(1.0),
            frame: &frame(),
        })
        .and_then(|raw| materialize_children("g", raw)),
        Err(GeneratorError::TooManyNodes { .. })
    ));
    let mut deep = rect("leaf", 1.0);
    for _ in 0..MAX_GENERATED_DEPTH {
        let mut parent = rect("wrap", 1.0);
        if let PenNode::Rectangle(r) = &mut parent {
            r.children = Some(vec![deep]);
        }
        deep = parent;
    }
    assert!(matches!(
        materialize_children("g", vec![deep]),
        Err(GeneratorError::TooDeep { .. })
    ));
}

#[test]
fn starters_have_valid_specs_and_frames() {
    for starter in GENERATOR_STARTERS.iter() {
        starter.spec().validate().expect("starter spec validates");
        assert!(matches!(starter.frame(0.0, 0.0), PenNode::Frame(_)));
        assert_eq!(GeneratorStarter::by_id(starter.id), Some(starter));
    }
}

#[test]
fn inserting_a_starter_selects_it_as_one_undo_step() {
    let mut state = EditorState::new();
    let depth = state.history.past.len();
    let starter = &GENERATOR_STARTERS[0];
    assert_eq!(
        state.insert_generator_starter(starter, None),
        Err(GeneratorError::RuntimeUnavailable)
    );
    assert_eq!(state.history.past.len(), depth);
    fn empty_runner(_: &GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError> {
        Ok(vec![rect("one", 4.0)])
    }
    let id = state
        .insert_generator_starter(starter, Some(empty_runner))
        .expect("inserted");
    assert_eq!(state.selection.anchor, id);
    assert_eq!(state.history.past.len(), depth + 1);
    assert_eq!(
        state.generator_spec(&id).unwrap().starter.as_deref(),
        Some("card-grid")
    );
}
