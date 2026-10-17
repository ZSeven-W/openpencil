use super::*;
use serde_json::json;

const BRIEF: &str = "Workout detail screen (375×812): hero image with a back button, workout title and meta row, difficulty badges, an exercise list of six with thumbnail, name, sets and reps, a start button pinned at the bottom.";

fn request(brief: &str) -> DesignRequest {
    DesignRequest {
        prompt: brief.into(),
        ..Default::default()
    }
}

fn text(name: &str, content: &str) -> PenNode {
    serde_json::from_value(json!({"type":"text","id":name,"name":name,"content":content})).unwrap()
}

fn list(sets: &[usize], summary: &str) -> PenNode {
    let rows: Vec<_> = sets.iter().enumerate().map(|(index, sets)| json!({
        "type":"frame","id":format!("row-{index}"),"name":"Exercise Row","children":[
            {"type":"text","id":format!("name-{index}"),"content":format!("Exercise {}", index + 1)},
            {"type":"text","id":format!("sets-{index}"),"content":format!("{sets} sets × 12 reps")}
        ]
    })).collect();
    serde_json::from_value(
        json!({"type":"frame","id":"section","name":"Exercise List","children":[
            {"type":"text","id":"summary","name":"Section Hint","content":summary},
            {"type":"frame","id":"rows","children":rows}
        ]}),
    )
    .unwrap()
}

#[test]
fn explicit_exercise_list_counts_come_only_from_one_complete_requested_list() {
    for brief in [
        BRIEF,
        "Workout: a list of 6 exercises with name and sets/reps.",
        "Workout: an exercise list of SIX.",
        "训练详情：六个动作列表，每项显示名称和组数。",
        "训练详情：6项训练动作列表，底部开始按钮。",
    ] {
        assert_eq!(
            ExerciseList::from_request(&request(brief)).unwrap().count,
            6
        );
    }
    for brief in [
        "Workout: hero with 12 exercises metadata, a list with names.",
        "Workout: an exercise list of six to eight.",
        "Workout: an exercise list of 6–8.",
        "Workout: first six of twelve exercises in a list.",
        "Workout: exercise list of six out of twelve.",
        "Workout: exercise list of six; another exercise list of four.",
        "Workout: exercise list of six and another list of stretches.",
        "Two workout pages, each with an exercise list of six.",
        "Workout: exercise list of six with pagination.",
        "训练详情：六到八个动作列表。",
        "训练详情：六个动作列表，每页显示三项。",
        "Replicate screenshot: exercise list of six.",
        "参考图：六个动作列表。",
        "Workout: exercise list of sixteen.",
    ] {
        assert!(
            ExerciseList::from_request(&request(brief)).is_none(),
            "{brief}"
        );
    }
    let mut req = request(BRIEF);
    req.continuation_context = Some(crate::types::ContinuationContext {
        screen_width: 375.0,
        screen_height: 812.0,
        background_color: None,
        screen_names: vec!["Workout".into()],
    });
    assert!(ExerciseList::from_request(&req).is_none());
    req.continuation_context = None;
    req.append_context = Some(crate::types::AppendContext {
        target_parent_id: "existing".into(),
        target_width: 375.0,
        existing_section_labels: vec!["Workout".into()],
        is_mobile: true,
    });
    assert!(ExerciseList::from_request(&req).is_none());
}

#[test]
fn explicit_exercise_count_reads_whole_quantity_before_applying_range() {
    for quantity in [
        "16",
        "26",
        "116",
        "十六",
        "二十",
        "一百零六",
        "廿六",
        "壹佰零六",
        "1",
        "一",
        "0",
        "零",
    ] {
        let req = request(&format!(
            "训练详情：{quantity}个动作列表，每项显示名称和组数。"
        ));
        assert!(ExerciseList::from_request(&req).is_none(), "{quantity}");
        let mut prompt = "Existing instructions".to_string();
        append_instruction(&req, &mut prompt);
        assert_eq!(prompt, "Existing instructions", "{quantity}");
        assert!(rejection(&req, &[text("meta", "12 exercises")]).is_none());
    }
    for (quantity, expected) in [("6", 6), ("六", 6), ("12", 12), ("十二", 12)] {
        let source = format!("{quantity}个动作列表");
        assert_eq!(
            ExerciseList::from_request(&request(&format!("训练详情：{source}。"))),
            Some(ExerciseList {
                count: expected,
                source,
            })
        );
    }
    // The original English regression brief keeps exactly the same contract.
    assert_eq!(
        ExerciseList::from_request(&request(BRIEF)),
        Some(ExerciseList {
            count: 6,
            source: "exercise list of six".into(),
        })
    );
    assert!(rejection(&request(BRIEF), &[text("meta", "6 exercises")]).is_none());
    assert!(rejection(&request(BRIEF), &[text("meta", "12 exercises")])
        .is_some_and(|reason| reason.contains("exercise-list-count-conflict")));
}

#[test]
fn explicit_exercise_count_rejects_conflicting_metadata_without_changing_nodes() {
    for content in [
        "45 min · 12 exercises · 320 kcal",
        "Exercises (12)",
        "12个动作",
    ] {
        let nodes = vec![text("workout-meta", content)];
        let before = serde_json::to_value(&nodes).unwrap();
        let reason = rejection(&request(BRIEF), &nodes).unwrap();
        assert!(
            reason.contains("exercise-list-count-conflict") && reason.contains("\"expected\":6")
        );
        assert_eq!(serde_json::to_value(&nodes).unwrap(), before);
    }
    for content in [
        "45 min · 6 exercises · 320 kcal",
        "Exercises (6)",
        "6个动作",
        "3 sets × 12 reps",
        "12-week program",
        "Exercise 12",
        "Exercise 1: Warm-up",
        "Exercises 3 sets",
        "12 exercises completed this week",
        "6 of 12 exercises",
    ] {
        assert!(
            rejection(&request(BRIEF), &[text("meta", content)]).is_none(),
            "{content}"
        );
    }
    let hidden =
        serde_json::from_value(json!({"type":"frame","id":"hidden","opacity":0,"children":[
            {"type":"text","id":"bad","content":"12 exercises"}
        ]}))
        .unwrap();
    assert!(rejection(&request(BRIEF), &[hidden]).is_none());
    // No count label is required merely because a list count was requested.
    assert!(rejection(&request(BRIEF), &[text("title", "Full Body Strength")]).is_none());
}

#[test]
fn explicit_exercise_set_summary_checks_only_one_fully_parseable_complete_list() {
    let nodes = vec![list(&[1, 3, 3, 3, 3, 1], "3 sets each")];
    let before = serde_json::to_value(&nodes).unwrap();
    let reason = rejection(&request(BRIEF), &nodes).unwrap();
    assert!(reason.contains("exercise-set-summary-conflict"));
    assert_eq!(serde_json::to_value(&nodes).unwrap(), before);
    for (sets, summary) in [
        (vec![3; 6], "3 sets each"),
        (
            vec![1, 3, 3, 3, 3, 1],
            "Follow each exercise's instructions",
        ),
        (vec![1, 3, 3], "3 sets each"),
    ] {
        assert!(rejection(&request(BRIEF), &[list(&sets, summary)]).is_none());
    }
    let mut unknown = serde_json::to_value(list(&[1, 3, 3, 3, 3, 1], "3 sets each")).unwrap();
    unknown["children"][1]["children"][0]["children"][1]["content"] = json!("As instructed");
    assert!(rejection(&request(BRIEF), &[serde_json::from_value(unknown).unwrap()]).is_none());
    assert!(rejection(
        &request(BRIEF),
        &[
            list(&[1, 3, 3, 3, 3, 1], "3 sets each"),
            list(&[3; 6], "3 sets each")
        ]
    )
    .is_none());
}

#[test]
fn brief_derived_exercise_fact_reaches_every_prompt_rung_despite_poisoned_plan() {
    use crate::{plan::OrchestratorPlan, types::AbortFlag};
    let plan: OrchestratorPlan = serde_json::from_value(json!({
        "rootFrame":{"id":"root","name":"Workout","width":375,"height":812},
        "subtasks":[{"id":"hero","label":"Hero with Title & Meta","elements":"meta row (45 min · 12 exercises · 320 kcal)","region":{"width":375,"height":220}}]
    })).unwrap();
    let mut req = request(BRIEF);
    req.model = Some("glm-5.3-flash".into());
    for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
        let (call, _) = crate::prompt::build_subagent_prompt(
            &plan.subtasks[0],
            &plan,
            &req,
            AbortFlag::new(),
            reduced,
            minimal,
            &op_editor_core::ComponentLibrary::default(),
        );
        assert!(call.user_prompt.contains("EXPLICIT EXERCISE LIST FACT"));
        assert!(call.user_prompt.contains("\"count\":6"));
        assert!(call.user_prompt.contains(BRIEF));
        assert!(call
            .user_prompt
            .contains("does not request an extra count label"));
    }
}
