//! Exercise continuity through the concurrent worker's real commit boundary.
use super::*;
use crate::plan::Subtask;
use crate::test_support::VecDocSink;
use crate::types::{CallRequest, LlmChunk, LlmError};
use futures::stream::BoxStream;
use op_editor_core::PenNodeExt;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Default)]
struct SectionLlm(Mutex<Vec<String>>);

impl LlmClient for SectionLlm {
    fn call(&self, request: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let label = ["Title A", "Body A", "Title B", "Body B"]
            .into_iter()
            .find(|label| {
                request
                    .user_prompt
                    .contains(&format!("Generate ONLY \"{label}\""))
            })
            .expect("one current section");
        let content = match label {
            "Title A" => "Workout Alpha · 32 min",
            "Title B" => "Workout Beta · 18 min",
            _ => "Exercise details",
        };
        let script = format!(
            "const s=I(null,{{type:'frame',name:'{label}',layout:'vertical',width:327,height:'fit_content'}});I(s,{{type:'text',content:'{content}',fontSize:18}});"
        );
        self.0.lock().unwrap().push(request.user_prompt);
        Box::pin(futures::stream::iter(vec![Ok(LlmChunk::Text(script))]))
    }
}

fn fixture() -> (
    VecDocSink,
    OrchestratorPlan,
    DesignRequest,
    Vec<ScreenGroup>,
) {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(
        serde_json::from_value(json!({
            "version":"1.0.0", "children":[
                {"id":"root-a","type":"frame","width":375,"height":812,"layout":"vertical"},
                {"id":"root-b","type":"frame","width":375,"height":812,"layout":"vertical"}
            ]
        }))
        .unwrap(),
    );
    let tasks: Vec<Subtask> = [
        ("title-a", "Title A", "root-a", "A"),
        ("body-a", "Body A", "root-a", "A"),
        ("title-b", "Title B", "root-b", "B"),
        ("body-b", "Body B", "root-b", "B"),
    ]
    .into_iter()
    .map(|(id, label, parent, screen)| {
        serde_json::from_value(json!({
            "id":id,"label":label,"parentFrameId":parent,"screen":screen,
            "region":{"width":375,"height":140}
        }))
        .unwrap()
    })
    .collect();
    let plan = OrchestratorPlan {
        subtasks: tasks,
        ..serde_json::from_value(json!({
            "rootFrame":{"id":"root-a","name":"Workouts","width":375,"height":812},
            "subtasks":[]
        }))
        .unwrap()
    };
    let groups = crate::screen_groups::group_subtasks_by_screen(&plan.subtasks);
    let request = DesignRequest {
        prompt: "Workout details".into(),
        model: Some("glm-5.3-flash".into()),
        provider: None,
        design_md: None,
        concurrency: 2,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    (sink, plan, request, groups)
}

#[tokio::test]
async fn parallel_screens_keep_committed_copy_with_local_ids_and_no_sibling_leak() {
    let (mut sink, plan, request, groups) = fixture();
    let llm = SectionLlm::default();
    let identities: Vec<_> = ["A", "B"]
        .into_iter()
        .map(|name| AgentIdentity {
            name: name.into(),
            color: "#5B8DEF".into(),
        })
        .collect();
    let result = crate::concurrent::run_screen_groups_concurrent(
        &groups,
        &identities,
        &plan,
        &request,
        &llm,
        &mut sink,
        &AbortFlag::new(),
        ModelTier::Standard,
        2,
        None,
        &GeometryEchoBudget::new(0),
        &mut |_| {},
    )
    .await;
    assert!(!result.aborted_mid);
    for outcome in &result.outcomes {
        assert_eq!(outcome.inserted_root_ids.len(), 1);
        let node = op_editor_core::walkers::find_node(
            sink.state.active_children(),
            &NodeId::new(&outcome.inserted_root_ids[0]),
        )
        .expect("global outcomes must reference committed real nodes");
        let task = plan
            .subtasks
            .iter()
            .find(|task| task.id == outcome.id)
            .unwrap();
        assert_eq!(node.base().name.as_deref(), Some(task.label.as_str()));
    }
    let prompts = llm.0.lock().unwrap();
    assert_eq!(prompts.len(), 4);
    for (label, own, other) in [
        ("Body A", "Workout Alpha · 32 min", "Workout Beta · 18 min"),
        ("Body B", "Workout Beta · 18 min", "Workout Alpha · 32 min"),
    ] {
        let prompt = prompts
            .iter()
            .find(|p| p.contains(&format!("Generate ONLY \"{label}\"")))
            .unwrap();
        assert!(
            prompt.contains("EXISTING SAME-PAGE COPY") && prompt.contains(own),
            "{label} lost its committed copy"
        );
        assert!(!prompt.contains(other), "{label} leaked sibling copy");
    }
}

struct SalvageLlm {
    sections: SectionLlm,
    plan: String,
    body_b_attempts: AtomicUsize,
}

impl LlmClient for SalvageLlm {
    fn call(&self, request: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        if !request.user_prompt.contains("Generate ONLY") {
            return Box::pin(futures::stream::iter(vec![Ok(LlmChunk::Text(
                self.plan.clone(),
            ))]));
        }
        if request.user_prompt.contains("Generate ONLY \"Body B\"")
            && self.body_b_attempts.fetch_add(1, Ordering::SeqCst) < 3
        {
            self.sections.0.lock().unwrap().push(request.user_prompt);
            return Box::pin(futures::stream::iter(vec![Err(LlmError {
                message: "temporary service unavailable".into(),
                aborted: false,
            })]));
        }
        self.sections.call(request)
    }
}

#[tokio::test]
async fn fourth_attempt_salvage_after_concurrent_generation_receives_same_page_committed_copy() {
    use crate::test_support::{
        SkippedPreValidator, SkippedScreenshotProvider, SkippedVisionLlmClient,
    };
    use crate::{Orchestrator, ValidationProviders};
    let (_, plan, request, _) = fixture();
    let llm = SalvageLlm {
        sections: SectionLlm::default(),
        plan: serde_json::to_string(&plan).unwrap(),
        body_b_attempts: AtomicUsize::new(0),
    };
    let providers = ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    };
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = Orchestrator::new()
        .run(
            request,
            &mut sink,
            &llm,
            &mut |event| events.push(event),
            &AbortFlag::new(),
            &providers,
        )
        .await
        .unwrap();
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::ConcurrentGroupsStarted {
            group_count: 2,
            workers: 2,
        }
    )));
    assert_eq!(llm.body_b_attempts.load(Ordering::SeqCst), 4);
    let prompts = llm.sections.0.lock().unwrap();
    let calls: Vec<_> = prompts
        .iter()
        .filter(|p| p.contains("Generate ONLY \"Body B\""))
        .collect();
    assert_eq!(calls.len(), 4);
    for (attempt, prompt) in calls.into_iter().enumerate() {
        let copy = prompt
            .split("EXISTING SAME-PAGE COPY")
            .nth(1)
            .unwrap_or_else(|| panic!("attempt {} lost committed copy", attempt + 1));
        assert!(copy.contains("Workout Beta · 18 min"));
        assert!(!copy.contains("Workout Alpha · 32 min"));
    }
    let salvaged = summary
        .subtasks
        .iter()
        .find(|outcome| outcome.id == "body-b")
        .unwrap();
    assert!(salvaged.error.is_none() && salvaged.node_count > 0);
}

#[tokio::test]
async fn rejected_real_commit_does_not_establish_copy_for_the_next_worker_section() {
    let (sink, plan, request, groups) = fixture();
    let llm = SectionLlm::default();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let abort = AbortFlag::new();
    let budget = GeometryEchoBudget::new(0);
    let worker = run_screen_group_worker(
        0,
        &groups[0],
        &plan,
        &request,
        &llm,
        &abort,
        ModelTier::Standard,
        sink.state,
        Arc::new(Semaphore::new(1)),
        &budget,
        event_tx,
    );
    let writer = async {
        let mut settled = 0;
        while let Some(event) = event_rx.recv().await {
            if let WorkerSignal::SubtaskSettled(replay) = event {
                settled += 1;
                // Simulate the real sink rejecting the first atomic commit.
                replay.ack.send(settled > 1).unwrap();
            }
        }
        settled
    };
    let (result, settled) = tokio::join!(worker, writer);
    assert!(!result.aborted);
    assert_eq!(settled, 2);
    let prompts = llm.0.lock().unwrap();
    assert_eq!(prompts.len(), 2);
    assert!(!prompts[1].contains("EXISTING SAME-PAGE COPY"));
    assert!(!prompts[1].contains("Workout Alpha · 32 min"));
}
