use super::*;
use crate::test_support::{ScriptResponse, ScriptedLlm, VecDocSink};
use crate::types::{AbortFlag, AppendContext};
use futures::executor::block_on;
use op_editor_core::EditorState;
use serde_json::{json, Value};

const FIXTURE: &str = include_str!("test_fixtures/mobile_pinned_action_plan_1018.json");

fn fixture() -> (OrchestratorPlan, Vec<PenNode>) {
    let value: Value = serde_json::from_str(FIXTURE).unwrap();
    (
        serde_json::from_value(value["plan"].clone()).unwrap(),
        vec![serde_json::from_value(value["section"].clone()).unwrap()],
    )
}

#[test]
fn pinned_action_plan_marker_preserves_roles_events_and_survives_save_load() {
    let (plan, mut nodes) = fixture();
    let task = plan.subtasks.last().unwrap();
    let mut original = serde_json::to_value(&nodes).unwrap();
    preserve(&mut nodes, task, &plan, &DesignRequest::default());
    assert!(is_marked(&nodes[0]));
    assert!(crate::mobile_action_bar::is_action_bar(&nodes[0]));
    original[0]["explain"] = json!(MARKER);
    assert_eq!(serde_json::to_value(&nodes).unwrap(), original);
    let once = nodes.clone();
    preserve(&mut nodes, task, &plan, &DesignRequest::default());
    assert_eq!(nodes, once, "do not duplicate the marker");

    crate::role_infer::resolve_forest_roles(&mut nodes, 375.0, crate::role_defaults::Theme::Dark);
    let stored = serde_json::to_string(&nodes).unwrap();
    let loaded: Vec<PenNode> = serde_json::from_str(&stored).unwrap();
    assert!(is_marked(&loaded[0]));
    assert_eq!(loaded[0].base().role.as_deref(), Some("button"));
    assert!(crate::mobile_action_bar::is_action_bar(&loaded[0]));
}

#[test]
fn pinned_action_marker_matches_a_complete_line_only() {
    let (_, mut nodes) = fixture();
    for explain in [format!("prefix {MARKER}"), format!("{MARKER} suffix")] {
        nodes[0].base_mut().explain = Some(explain);
        assert!(!is_marked(&nodes[0]));
        assert!(!crate::mobile_action_bar::is_action_bar(&nodes[0]));
    }
    nodes[0].base_mut().explain = Some(format!("Original explanation\n{MARKER}"));
    assert!(is_marked(&nodes[0]));
}

#[test]
fn ordinary_start_buttons_and_unrelated_sections_do_not_gain_pinning() {
    for elements in [
        "Start Workout button after the exercise list",
        "bottom bar with a Start Workout button, not pinned",
        "fixed top action with Start Workout button",
        "底部开始按钮，不固定，随正文滚动",
    ] {
        let (mut plan, mut nodes) = fixture();
        plan.subtasks.last_mut().unwrap().elements = Some(elements.into());
        let before = nodes.clone();
        preserve(
            &mut nodes,
            plan.subtasks.last().unwrap(),
            &plan,
            &DesignRequest::default(),
        );
        assert_eq!(nodes, before, "{elements}");
        assert!(!crate::mobile_action_bar::is_action_bar(&nodes[0]));
    }
}

#[test]
fn pinned_action_plan_preserves_ambiguous_and_existing_layouts() {
    for case in [
        "positioned",
        "multiple roots",
        "desktop",
        "nonterminal",
        "append",
        "reference",
        "continuation",
        "no action",
    ] {
        let (mut plan, mut nodes) = fixture();
        let mut req = DesignRequest::default();
        match case {
            "positioned" => nodes[0].base_mut().y = Some(0.0),
            "multiple roots" => nodes.push(nodes[0].clone()),
            "desktop" => plan.root_frame.width = 1440.0,
            "nonterminal" => {
                let mut extra = plan.subtasks[0].clone();
                extra.id = "extra".into();
                plan.subtasks.push(extra);
            }
            "append" => {
                req.append_context = Some(AppendContext {
                    target_parent_id: "existing".into(),
                    target_width: 375.0,
                    existing_section_labels: vec![],
                    is_mobile: true,
                })
            }
            "reference" => {
                req.reference_skeleton = Some(
                    serde_json::from_value(json!({
                        "source":"reference","width":375,"sections":[],"navKind":"none",
                        "heroKind":"none","columnRhythm":[]
                    }))
                    .unwrap(),
                )
            }
            "continuation" => {
                req.continuation_context = Some(crate::types::ContinuationContext {
                    screen_width: 375.0,
                    screen_height: 812.0,
                    background_color: None,
                    screen_names: vec!["Workout".into()],
                })
            }
            "no action" => {
                nodes[0].base_mut().role = None;
                if let PenNode::Frame(frame) = &mut nodes[0] {
                    frame.children = Some(vec![]);
                }
            }
            _ => unreachable!(),
        }
        let before = nodes.clone();
        let task = plan
            .subtasks
            .iter()
            .find(|task| task.id == "start")
            .unwrap();
        preserve(&mut nodes, task, &plan, &req);
        assert_eq!(nodes, before, "{case}");
    }
}

#[test]
fn pinned_action_plan_checks_terminal_position_per_screen() {
    let (mut plan, mut nodes) = fixture();
    for task in &mut plan.subtasks {
        task.screen = Some("workout".into());
    }
    let mut extra = plan.subtasks[0].clone();
    extra.id = "next-screen".into();
    extra.screen = Some("history".into());
    plan.subtasks.push(extra);
    let task = plan
        .subtasks
        .iter()
        .find(|task| task.id == "start")
        .unwrap();
    preserve(&mut nodes, task, &plan, &DesignRequest::default());
    assert!(is_marked(&nodes[0]));
    assert!(plan.subtasks.last().unwrap().id != task.id);
}

#[test]
fn pinned_plan_intent_reaches_saved_section_and_visible_button_after_cleanup() {
    let (mut plan, nodes) = fixture();
    let task = plan.subtasks.last_mut().unwrap();
    task.parent_frame_id = Some("page".into());
    let task = task.clone();
    assert_eq!(task.label, "Start Button");
    let req = DesignRequest {
        prompt: "Workout detail screen (375×812): hero image with a back button, workout title and meta row, difficulty badges, an exercise list of six with thumbnail, name, sets and reps, a start button pinned at the bottom.".into(),
        ..DesignRequest::default()
    };
    // Replay the real 01 plan + CTA through the same subagent insertion and
    // cleanup boundaries. Other sections are a tall body, never a patched .op.
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(format!(
        "I(null, {});",
        serde_json::to_string(&nodes[0]).unwrap()
    ))]);
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(serde_json::from_value(json!({
        "version":"1.0.0","children":[{
            "type":"frame","id":"page","name":"Workout Detail","width":375,"height":812,
            "layout":"vertical","gap":16,"fill":[{"type":"solid","color":"#111111"}],
            "children":[
                {"type":"frame","id":"status","name":"Status Bar","role":"status-bar","width":"fill_container","height":62},
                {"type":"frame","id":"body","name":"Workout Content","width":"fill_container","height":900,"layout":"vertical","children":[
                    {"type":"text","id":"body-text","content":"Exercise list","fontSize":16,"height":900}
                ]}
            ]
        }]
    })).unwrap());
    let result = block_on(crate::subagent::run_subtask(
        &task,
        &plan,
        &req,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    assert!(result.error.is_none(), "{:?}", result.error);
    let inserted = sink.state.active_children()[0]
        .children()
        .unwrap()
        .last()
        .unwrap();
    assert!(
        is_marked(inserted),
        "the production insertion hook must persist intent"
    );
    // Match the fresh-root generation entry point. Legacy/default cleanup
    // intentionally cannot infer that an existing artboard is fixed output.
    crate::cleanup::finalize_design_with_summary_and_policy(
        &mut sink,
        &plan,
        &["page"],
        &mut crate::repair_summary::RepairSummary::default(),
        crate::cleanup::CleanupPolicy {
            preserve_requested_root_height: true,
            roots_are_run_output: true,
            is_deck: false,
        },
    );

    let root = &sink.state.active_children()[0];
    let sections = root.children().unwrap();
    let footer = sections.last().unwrap();
    assert!(is_marked(footer));
    assert!(sections
        .iter()
        .any(|node| node.base().role.as_deref() == Some("scroll-area")));
    let button = footer
        .children()
        .unwrap()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("start-workout-button"))
        .unwrap();
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
    let page = scene.active_page().unwrap();
    let board = page.find(root.id_str()).unwrap().bounds;
    let control = page.find(button.id_str()).unwrap().bounds;
    assert_eq!(board.size.y, 812.0);
    assert!(control.size.y >= 44.0, "{control:?}");
    assert!(control.origin.y >= board.origin.y, "{control:?}");
    assert!(
        control.origin.y + control.size.y <= board.origin.y + board.size.y - 24.0,
        "{control:?}"
    );
    assert!(
        control.origin.x >= board.origin.x
            && control.origin.x + control.size.x <= board.origin.x + board.size.x,
        "{control:?}"
    );
}
