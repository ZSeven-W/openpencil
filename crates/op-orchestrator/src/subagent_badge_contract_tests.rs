use super::*;
use crate::types::{GeometryEchoBudget, Progress};
use serde_json::json;

const HERO_FIRST: &str = include_str!("test_fixtures/workout_detail_1018_hero_attempt1.js");
const HERO_SECOND: &str = include_str!("test_fixtures/workout_detail_1018_hero_attempt2.js");
const HERO_EMPTY: &str = include_str!("test_fixtures/workout_detail_1018_hero_attempt3.js");

fn workout_without_requested_count() -> (Subtask, OrchestratorPlan, DesignRequest) {
    let plan = crate::plan::parse_plan(include_str!("test_fixtures/workout_detail_1018_plan.json"))
        .unwrap();
    let mut task = plan.subtasks[0].clone();
    task.parent_frame_id = None;
    let mut request = req();
    // Isolate this gate from the separate source-fact contract: the original
    // brief requested six, so its generated "12 exercises" is not valid copy.
    // Keep the real response and plan intact; only omit that requested total.
    request.prompt = "Workout detail screen (375×812): hero image with a back button, workout title and meta row, difficulty badges, an exercise list with thumbnail, name, sets and reps, a start button pinned at the bottom.".into();
    (task, plan, request)
}

fn badge_labels(nodes: &[PenNode]) -> Vec<String> {
    let mut result = Vec::new();
    for node in nodes {
        if let PenNode::Text(text) = node {
            let value = serde_json::to_value(text).unwrap();
            if let Some(label @ ("Beginner" | "Intermediate")) = value["content"].as_str() {
                result.push(label.to_string());
            }
        }
        if let Some(children) = node.children() {
            result.extend(badge_labels(children));
        }
    }
    result
}

#[test]
fn workout_empty_badges_from_third_response_reject_before_insertion() {
    let (task, plan, request) = workout_without_requested_count();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(HERO_EMPTY.into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    let error = outcome.error.unwrap();
    assert!(error.contains("empty-label-badge"), "{error}");
    assert!(
        error.contains("badge-active") && error.contains("badge-inactive"),
        "{error}"
    );
    assert!(crate::retry::is_self_check_rejection(&error));
    assert_eq!(outcome.node_count, 0);
    assert!(!sink
        .applied
        .iter()
        .any(|cmd| matches!(cmd, EditorCommand::InsertSubtree { .. })));
}

#[test]
fn workout_first_two_responses_contain_complete_badges() {
    for script in [HERO_FIRST, HERO_SECOND] {
        let (nodes, _) = crate::script_gen::parse_script(script).unwrap();
        assert!(empty_controls::rejection(&nodes).is_none());
        assert_eq!(badge_labels(&nodes), ["Beginner", "Intermediate"]);
    }
}

#[test]
fn workout_metadata_count_without_brief_total_does_not_retry_complete_badges() {
    let (task, plan, request) = workout_without_requested_count();
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(HERO_FIRST.into())]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let outcome = block_on(crate::concurrent::run_subtask_retry_ladder_with_outcomes(
        &task,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        crate::model_profile::ModelTier::Full,
        None,
        &GeometryEchoBudget::new(0),
        &mut |event| events.push(event),
        &[],
    ));
    assert!(outcome.error.is_none(), "{:?}", outcome.error);
    assert_eq!(
        llm.user_prompts().len(),
        1,
        "displayed metadata must not require twelve hero children"
    );
    assert!(!events.iter().any(|e| matches!(
        e,
        Progress::SubtaskRetry { .. } | Progress::SubtaskIncomplete { .. }
    )));
    assert_eq!(
        badge_labels(sink.state.active_children()),
        ["Beginner", "Intermediate"]
    );
}

#[test]
fn empty_badge_gate_preserves_visible_graphics_and_non_label_marks() {
    for patch in [
        json!({"children":[{"type":"icon_font","id":"glyph","iconFontName":"award"}]}),
        json!({"children":[{"type":"path","id":"glyph","d":"M0 0 L10 10"}]}),
        json!({"children":[{"type":"text","id":"label","content":"Gold"}]}),
        json!({"fill":[{"type":"image","url":"asset:award"}]}),
        json!({"name":"status-dot badge","width":8,"height":8}),
        json!({"name":"Dot","role":"badge","width":6,"height":6}),
        json!({"name":"Color Chip","width":32,"height":32}),
        json!({"name":"Swatch Badge","width":32,"height":32}),
        json!({"name":"Pattern Badge","width":40,"height":40}),
        json!({"name":"色板徽章","width":32,"height":32}),
        json!({"visible":false}),
        json!({"opacity":0}),
    ] {
        let mut value = json!({"type":"frame","id":"badge","name":"Award Badge","role":"badge",
            "width":100,"height":32,"cornerRadius":99,"fill":[{"type":"solid","color":"#F00"}]});
        value
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let node: PenNode = serde_json::from_value(value.clone()).unwrap();
        assert!(empty_controls::rejection(&[node]).is_none(), "{value}");
    }
    let anonymous: PenNode = serde_json::from_value(json!({"type":"frame","id":"graphic",
        "width":40,"height":40,"cornerRadius":20,"fill":[{"type":"solid","color":"#F00"}]}))
    .unwrap();
    assert!(empty_controls::rejection(&[anonymous]).is_none());
}

#[test]
fn explicit_badges_with_only_hidden_or_blank_children_are_rejected() {
    for identity in [
        json!({"name":"Difficulty Chip"}),
        json!({"role":"pill"}),
        json!({"name":"Badge Active"}),
    ] {
        for child in [
            json!({"type":"text","id":"label","content":"  "}),
            json!({"type":"icon_font","id":"glyph","iconFontName":"award","visible":false}),
            json!({"type":"frame","id":"wrapper","children":[]}),
        ] {
            let mut value =
                json!({"type":"frame","id":"badge","width":90,"height":32,"children":[child]});
            value
                .as_object_mut()
                .unwrap()
                .extend(identity.as_object().unwrap().clone());
            let node: PenNode = serde_json::from_value(value).unwrap();
            assert!(empty_controls::rejection(&[node])
                .unwrap()
                .contains("empty-label-badge"));
        }
    }
}

#[test]
fn complete_badge_preserves_its_explicit_background_and_outline_layers() {
    for (name, paint) in [
        (
            "Badge Background",
            r##"fill:[{type:"solid",color:"#E9F5ED"}]"##,
        ),
        ("badge-bg", r##"fill:[{type:"solid",color:"#E9F5ED"}]"##),
        (
            "Badge Outline",
            r##"stroke:{thickness:1,fill:[{type:"solid",color:"#E9F5ED"}]}"##,
        ),
    ] {
        let script = format!(
            r#"const badge=I(null,{{type:"frame",name:"Difficulty Badge",width:80,height:28,layout:"none"}});
I(badge,{{type:"frame",name:{},width:80,height:28,{paint}}});
I(badge,{{type:"text",content:"Intermediate",fontSize:12}});"#,
            serde_json::to_string(name).unwrap()
        );
        let (nodes, _) = crate::script_gen::parse_script(&script).unwrap();
        let before = serde_json::to_value(&nodes).unwrap();
        assert!(empty_controls::rejection(&nodes).is_none(), "{name}");
        assert_eq!(badge_labels(&nodes), ["Intermediate"]);
        assert_eq!(serde_json::to_value(&nodes).unwrap(), before);
    }
}

#[test]
fn badge_background_exception_does_not_hide_empty_controls_or_sibling_badges() {
    let background = json!({"type":"frame","id":"bg","name":"Badge Background",
        "width":80,"height":28,"fill":[{"type":"solid","color":"#E9F5ED"}]});
    let label = json!({"type":"text","id":"label","content":"Intermediate"});
    let complete = json!({"type":"frame","id":"complete","name":"Complete Badge",
        "children":[label.clone()]});
    for value in [
        // A standalone painted shell remains incomplete, regardless of name.
        background.clone(),
        // Ordinary text outside a badge does not make the shell its background.
        json!({"type":"frame","id":"section","children":[label.clone(),background.clone()]}),
        // A complete child does not exempt a different empty child badge.
        json!({"type":"frame","id":"outer","name":"Difficulty Badge","children":[complete,
            {"type":"frame","id":"missing","name":"Missing Badge","width":80,"height":28}]}),
        // An invisible label cannot establish a complete badge context.
        json!({"type":"frame","id":"outer","name":"Difficulty Badge","children":[
            {"type":"text","id":"hidden","content":"Intermediate","visible":false},background.clone()]}),
        // Naming an icon control as a background must not bypass its own gate.
        json!({"type":"frame","id":"outer","name":"Difficulty Badge","children":[label.clone(),
            {"type":"frame","id":"control","name":"Badge Background","role":"icon-button",
             "width":80,"height":28,"fill":[{"type":"solid","color":"#E9F5ED"}]}]}),
        // Only actual painted drawing layers receive the narrow exception.
        json!({"type":"frame","id":"outer","name":"Difficulty Badge","children":[label,
            {"type":"frame","id":"missing","name":"Badge Background","width":80,"height":28}]}),
    ] {
        let node: PenNode = serde_json::from_value(value.clone()).unwrap();
        assert!(empty_controls::rejection(&[node]).is_some(), "{value}");
    }
}
