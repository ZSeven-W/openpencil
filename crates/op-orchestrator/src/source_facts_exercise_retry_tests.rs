use super::*;

const REJECTION: &str = "exercise-list-count-conflict";

fn exercise_request() -> DesignRequest {
    DesignRequest {
        prompt: "Workout detail: an exercise list of six with names, sets and reps.".into(),
        model: Some("glm-5.3-flash".into()),
        validation_enabled: false,
        ..Default::default()
    }
}

fn exercise_plan() -> String {
    json!({
        "rootFrame":{"id":"root","name":"Workout","width":375,"height":812,"layout":"vertical","gap":0},
        "subtasks":[{"id":"exercises","label":"Exercise List","elements":"list of six exercise rows with names, sets and reps; meta row (12 exercises)","covers":["names","sets","reps"],"region":{"width":375,"height":600}}]
    }).to_string()
}

fn exercise_script(count: usize) -> String {
    let rows: Vec<_> = (0..6).map(|index| json!({
        "type":"frame","name":format!("Exercise Row {index}"),"layout":"vertical",
        "width":"fill_container","height":"fit_content","gap":4,"children":[
            {"type":"text","name":"Exercise Name","content":format!("Exercise {}",index+1),"fontSize":18,"height":"fit_content","width":"fill_container"},
            {"type":"text","name":"Sets Reps","content":"3 sets × 12 reps","fontSize":14,"height":"fit_content","width":"fill_container"}
        ]
    })).collect();
    format!(
        "I(null, {});",
        json!({
            "type":"frame","name":"Exercise List","width":"fill_container","height":"fit_content",
            "layout":"vertical","gap":16,"padding":24,"children":[
                {"type":"text","name":"Exercise Count","content":format!("{count} exercises"),"fontSize":20,"width":"fill_container","height":"fit_content"},
                {"type":"frame","name":"Exercise Rows","width":"fill_container","height":"fit_content","layout":"vertical","gap":12,"children":rows}
            ]
        })
    )
}

fn inserted_wrong_count(command: &EditorCommand) -> bool {
    match command {
        EditorCommand::InsertSubtree { nodes, .. }
        | EditorCommand::InsertAuthoredSubtree { nodes, .. } => {
            texts(nodes).iter().any(|text| text == "12 exercises")
        }
        EditorCommand::Batch { commands } => commands.iter().any(inserted_wrong_count),
        _ => false,
    }
}

#[test]
fn explicit_exercise_count_retries_before_any_conflicting_copy_is_inserted() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(exercise_plan()),
        ScriptResponse::Text(exercise_script(12)),
        ScriptResponse::Text(exercise_script(6)),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = futures::executor::block_on(Orchestrator::new().run(
        exercise_request(),
        &mut sink,
        &llm,
        &mut |event| events.push(event),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .unwrap();
    assert!(summary
        .subtasks
        .iter()
        .all(|outcome| outcome.error.is_none()));
    assert_eq!(llm.user_prompts().len(), 3);
    assert!(!events
        .iter()
        .any(|event| matches!(event, Progress::PlanCoverageRetry { .. })));
    assert!(llm.user_prompts()[2].contains(REJECTION));
    assert!(events.iter().any(|event| matches!(event,
        Progress::SubtaskRetry { id, attempt: 2, reason }
        if id == "exercises" && reason.contains(REJECTION)
    )));
    assert!(!sink.applied.iter().any(inserted_wrong_count));
    let copy = texts(sink.state.active_children());
    assert!(copy.iter().any(|text| text == "6 exercises"));
    for number in 1..=6 {
        assert!(copy
            .iter()
            .any(|text| text == &format!("Exercise {number}")));
    }
}

#[test]
fn explicit_exercise_count_stays_a_hard_rejection_on_final_salvage() {
    let llm = ScriptedLlm::new(
        std::iter::once(ScriptResponse::Text(exercise_plan()))
            .chain((0..4).map(|_| ScriptResponse::Text(exercise_script(12))))
            .collect(),
    );
    let mut sink = VecDocSink::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        exercise_request(),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(ref reason)) if reason.contains(REJECTION)),
        "{result:?}"
    );
    assert_eq!(
        llm.user_prompts().len(),
        5,
        "one plan, three attempts and final salvage"
    );
    assert!(llm.user_prompts()[2..]
        .iter()
        .all(|prompt| prompt.contains(REJECTION)));
    assert!(!sink.applied.iter().any(inserted_wrong_count));
    assert!(sink.state.active_children().is_empty());
}
