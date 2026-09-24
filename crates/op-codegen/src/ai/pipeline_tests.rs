use super::*;
use crate::ai::types::{CodegenInput, PipelineStep, RequestKind};
use op_ai::chat_provider::{EffortLevel, ThinkingMode};
use op_editor_core::codegen::Framework;

fn input() -> CodegenInput {
    CodegenInput {
        nodes_json: "[{\"type\":\"frame\",\"id\":\"n1\",\"children\":[]}]".into(),
        framework: Framework::React,
        variables_json: None,
        themes_json: None,
        components_json: None,
        max_output_tokens: 3000,
        thinking: ThinkingMode::Adaptive,
        effort: EffortLevel::Low,
    }
}

#[test]
fn deterministic_target_finishes_on_the_first_step_without_a_model() {
    let mut p = CodegenPipeline::new(CodegenInput {
        framework: Framework::ReactTailwind,
        ..input()
    });
    let step = p.step();
    let PipelineStep::Done { code, degraded, .. } = &step else {
        panic!("expected Done, got {step:?}");
    };
    assert!(!degraded, "a deterministic target is the real output");
    assert!(code.contains("export default function"), "{code}");
    assert_eq!(p.step(), step, "terminal step is sticky");
    let progress = p.progress();
    assert_eq!(progress.planning_done, Some(true));
    assert_eq!(progress.assembly_done, Some(true));
}

#[test]
fn single_chunk_run_reaches_done() {
    let mut p = CodegenPipeline::new(input());
    let reqs = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("{other:?}"),
    };
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].kind, RequestKind::Planning);
    let plan_id = reqs[0].id;
    p.on_delta(plan_id, "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"root\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}");
    p.on_complete(plan_id);
    let reqs = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("{other:?}"),
    };
    assert_eq!(reqs.len(), 1);
    let chunk_id = reqs[0].id;
    p.on_delta(
            chunk_id,
            "export default function Root(){ return null }\n---CONTRACT---\n{\"componentName\":\"Root\"}",
        );
    p.on_complete(chunk_id);
    let reqs = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("{other:?}"),
    };
    let asm_id = reqs[0].id;
    p.on_delta(asm_id, "export default function App(){ return <Root/> }");
    p.on_complete(asm_id);
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(code.contains("function App"));
            assert!(!degraded);
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn planning_parse_failure_retries_once_then_uses_fallback_plan() {
    let mut p = CodegenPipeline::new(CodegenInput {
        nodes_json: r#"[{"type":"frame","id":"hero","name":"Hero","children":[]}]"#.into(),
        ..input()
    });
    let id1 = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        _ => panic!(),
    };
    p.on_delta(id1, "not json at all");
    p.on_complete(id1);
    let reqs = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("{other:?}"),
    };
    assert!(reqs[0].user_message.contains("ONLY valid JSON"));
    let id2 = reqs[0].id;
    p.on_delta(id2, "still not json");
    p.on_complete(id2);
    let reqs = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("expected fallback chunk dispatch, got {other:?}"),
    };
    assert_eq!(reqs.len(), 1);
    assert_eq!(
        reqs[0].kind,
        RequestKind::Chunk {
            chunk_id: "chunk-1".into()
        }
    );
    assert!(
        reqs[0].user_message.contains("Hero"),
        "fallback plan should preserve the selected node name in the chunk prompt"
    );
}

#[test]
fn assembly_failure_uses_rescue_then_deterministic_file() {
    let mut p = CodegenPipeline::new(input());
    let id = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        _ => panic!(),
    };
    p.on_delta(id, "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}");
    p.on_complete(id);
    let cid = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        _ => panic!(),
    };
    p.on_delta(
        cid,
        "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}",
    );
    p.on_complete(cid);
    let a1 = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        _ => panic!(),
    };
    p.on_error(a1, "boom".into());
    let a2 = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        _ => panic!(),
    };
    p.on_error(a2, "boom again".into());
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].clone(),
        other => panic!("expected whole-document rescue, got {other:?}"),
    };
    assert!(rescue.user_message.contains("boom"));
    assert!(rescue.user_message.contains("boom again"));
    p.on_error(rescue.id, "rescue unavailable".into());
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(degraded);
            assert_eq!(code.matches("export default").count(), 1, "{code}");
        }
        other => panic!("expected deterministic Done, got {other:?}"),
    }
}

/// Drive `p` through planning with `plan_json`, returning the dispatched
/// chunk request ids (in order).
fn run_planning(p: &mut CodegenPipeline, plan_json: &str) -> Vec<RequestId> {
    let id = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        other => panic!("expected planning dispatch, got {other:?}"),
    };
    p.on_delta(id, plan_json);
    p.on_complete(id);
    match p.step() {
        PipelineStep::Dispatch(r) => r.iter().map(|q| q.id).collect(),
        other => panic!("expected chunk dispatch, got {other:?}"),
    }
}

fn assert_unsafe_plan_uses_fallback(plan_json: &str) {
    let mut p = CodegenPipeline::new(CodegenInput {
        nodes_json: r#"[
            {"type":"frame","id":"a","name":"A","children":[]},
            {"type":"frame","id":"b","name":"B","children":[]}
        ]"#
        .into(),
        ..input()
    });
    let planning = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected planning dispatch, got {other:?}"),
    };
    p.on_delta(planning, plan_json);
    p.on_complete(planning);
    let chunks = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("unsafe plan must dispatch fallback chunks, got {other:?}"),
    };
    let mut logical_ids = Vec::new();
    for (index, request) in chunks.iter().enumerate() {
        let RequestKind::Chunk { chunk_id } = &request.kind else {
            panic!("fallback dispatched a non-chunk request")
        };
        assert!(!logical_ids.contains(chunk_id), "duplicate fallback id");
        logical_ids.push(chunk_id.clone());
        let component = format!("Part{index}");
        p.on_delta(
            request.id,
            &format!(
                "export function {component}(){{return null}}\n---CONTRACT---\n{{\"componentName\":\"{component}\"}}"
            ),
        );
        p.on_complete(request.id);
    }
    let assembly = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("fallback chunks must reach assembly, got {other:?}"),
    };
    p.on_delta(assembly, "export default function App(){ return <main/> }");
    p.on_complete(assembly);
    assert!(matches!(p.step(), PipelineStep::Done { .. }));
}

#[test]
fn unsafe_planner_graphs_use_fallback_instead_of_waiting_forever() {
    for plan in [
        r#"{"chunks":[
            {"id":"same","nodeIds":["a"],"suggestedComponentName":"A","dependencies":[]},
            {"id":"same","nodeIds":["b"],"suggestedComponentName":"B","dependencies":[]}
        ],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#,
        r#"{"chunks":[
            {"id":"a","nodeIds":["a"],"suggestedComponentName":"A","dependencies":["missing"]}
        ],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#,
        r#"{"chunks":[
            {"id":"a","nodeIds":["a"],"suggestedComponentName":"A","dependencies":["b"]},
            {"id":"b","nodeIds":["b"],"suggestedComponentName":"B","dependencies":["a"]}
        ],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#,
    ] {
        assert_unsafe_plan_uses_fallback(plan);
    }
}

// ── FIX 3: parsed-but-poor chunk is Degraded, not Failed ──────────────

#[test]
fn chunk_with_code_but_no_contract_ends_degraded_not_failed() {
    let mut p = CodegenPipeline::new(input());
    let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
        );
    let cid = chunk_ids[0];
    // Non-empty code with NO contract and NO matching component name in
    // the body → validation fails → Degraded (never retried/Failed).
    p.on_delta(cid, "const x = 1; // no component, no contract");
    p.on_complete(cid);
    // Advancing to assembly proves the chunk settled (Degraded) and the
    // pipeline proceeded rather than retrying.
    let asm = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("expected assembly dispatch, got {other:?}"),
    };
    assert_eq!(asm[0].kind, RequestKind::Assembly);
    let prog = p.progress();
    assert_eq!(prog.chunks[0].status, ChunkStatus::Degraded);
}

#[test]
fn chunk_on_error_retries_then_rescue_failure_uses_deterministic_output() {
    let mut p = CodegenPipeline::new(input());
    let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
        );
    // First on_error → retry (re-dispatch the same chunk).
    p.on_error(chunk_ids[0], "stream broke".into());
    let retry = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("expected retry dispatch, got {other:?}"),
    };
    assert!(matches!(retry[0].kind, RequestKind::Chunk { .. }));
    // Second failure exhausts the chunk retry and dispatches the one-shot
    // whole-document rescue with the complete failure trail.
    p.on_error(retry[0].id, "stream broke again".into());
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("expected rescue dispatch, got {other:?}"),
    };
    assert_eq!(rescue[0].kind, RequestKind::Assembly);
    assert!(rescue[0].user_message.contains("stream broke"));
    assert!(rescue[0].user_message.contains("stream broke again"));

    // If the AI rescue also fails, canonical inputs still get a deterministic
    // framework generator rather than an empty terminal failure.
    p.on_error(rescue[0].id, "rescue provider unavailable".into());
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(degraded);
            assert!(!code.trim().is_empty());
            assert!(code.contains("React") || code.contains("function"));
        }
        other => panic!("expected deterministic Done, got {other:?}"),
    }
}

#[test]
fn failed_dependency_recursively_skips_descendants_without_waiting() {
    let mut p = CodegenPipeline::new(CodegenInput {
        nodes_json: r#"[
            {"type":"frame","id":"a","children":[]},
            {"type":"frame","id":"b","children":[]},
            {"type":"frame","id":"c","children":[]}
        ]"#
        .into(),
        ..input()
    });
    let first = run_planning(
        &mut p,
        r#"{"chunks":[
            {"id":"a","nodeIds":["a"],"suggestedComponentName":"A","dependencies":[]},
            {"id":"b","nodeIds":["b"],"suggestedComponentName":"B","dependencies":["a"]},
            {"id":"c","nodeIds":["c"],"suggestedComponentName":"C","dependencies":["b"]}
        ],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#,
    );
    assert_eq!(first.len(), 1);

    p.on_error(first[0], "root chunk failed".into());
    let retry = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("expected root retry, got {other:?}"),
    };
    assert_eq!(
        retry[0].kind,
        RequestKind::Chunk {
            chunk_id: "a".into()
        }
    );

    p.on_error(retry[0].id, "root retry failed".into());
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("dependency chain must reach rescue without waiting, got {other:?}"),
    };
    assert_eq!(rescue.len(), 1);
    assert_eq!(rescue[0].kind, RequestKind::Assembly);

    let progress = p.progress();
    assert_eq!(progress.chunks.len(), 3);
    assert_eq!(progress.chunks[0].status, ChunkStatus::Failed);
    assert_eq!(progress.chunks[1].status, ChunkStatus::Skipped);
    assert_eq!(progress.chunks[2].status, ChunkStatus::Skipped);
}

#[test]
fn empty_chunk_output_retries_once_then_whole_document_rescue_succeeds() {
    let mut p = CodegenPipeline::new(input());
    let chunk = run_planning(
        &mut p,
        "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
    )[0];
    p.on_delta(chunk, "   \n");
    p.on_complete(chunk);
    let retry = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("expected empty-chunk retry, got {other:?}"),
    };
    assert!(matches!(retry[0].kind, RequestKind::Chunk { .. }));
    p.on_complete(retry[0].id);
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("expected rescue after second empty chunk, got {other:?}"),
    };
    assert!(rescue[0].user_message.contains("model returned empty code"));
    p.on_delta(
        rescue[0].id,
        "export default function Recovered(){ return null; }",
    );
    p.on_complete(rescue[0].id);
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(degraded);
            assert!(code.contains("Recovered"));
        }
        other => panic!("expected rescued Done, got {other:?}"),
    }
}

#[test]
fn empty_assembly_output_retries_then_uses_whole_document_rescue() {
    let mut p = CodegenPipeline::new(input());
    let chunk = run_planning(
        &mut p,
        "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
    )[0];
    p.on_delta(
        chunk,
        "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}",
    );
    p.on_complete(chunk);
    let first = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected assembly, got {other:?}"),
    };
    p.on_complete(first);
    let retry = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected assembly retry, got {other:?}"),
    };
    p.on_delta(retry, " \n ");
    p.on_complete(retry);
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected whole-document rescue, got {other:?}"),
    };
    p.on_delta(
        rescue,
        "export default function Rescued(){ return <main/> }",
    );
    p.on_complete(rescue);
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(degraded);
            assert!(code.contains("function Rescued"));
        }
        other => panic!("expected rescued Done, got {other:?}"),
    }
}

#[test]
fn oversized_assembly_prompt_uses_rescue_not_raw_chunk_output() {
    let mut p = CodegenPipeline::new(input());
    let chunk = run_planning(
        &mut p,
        "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
    )[0];
    let payload = "x".repeat(MAX_USER_PROMPT_BYTES + 1_000);
    p.on_delta(
        chunk,
        &format!(
            "export default function Root(){{return <main data-value=\"{payload}\"/>}}\n---CONTRACT---\n{{\"componentName\":\"Root\"}}"
        ),
    );
    p.on_complete(chunk);
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].clone(),
        other => panic!("oversized assembly must use rescue, got {other:?}"),
    };
    assert!(rescue
        .user_message
        .contains("Generate one complete, self-contained react source file"));
    assert!(!rescue.user_message.contains(&payload));
    p.on_error(rescue.id, "rescue unavailable".into());
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(degraded);
            assert!(!code.contains(&payload));
            assert_eq!(code.matches("export default").count(), 1, "{code}");
        }
        other => panic!("expected deterministic fallback, got {other:?}"),
    }
}

#[test]
fn prose_assembly_output_retries_then_accepts_structured_code() {
    let mut p = CodegenPipeline::new(input());
    let chunk = run_planning(
        &mut p,
        "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
    )[0];
    p.on_delta(
        chunk,
        "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}",
    );
    p.on_complete(chunk);
    let first = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected assembly, got {other:?}"),
    };
    p.on_delta(
        first,
        "Here is the requested implementation with a responsive layout.",
    );
    p.on_complete(first);
    let retry = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected assembly retry, got {other:?}"),
    };
    p.on_delta(retry, "export default function App(){ return <Root/> }");
    p.on_complete(retry);
    match p.step() {
        PipelineStep::Done { code, .. } => assert!(code.contains("function App")),
        other => panic!("expected structured assembly, got {other:?}"),
    }
}

#[test]
fn prose_rescue_output_uses_deterministic_fallback() {
    let mut p = CodegenPipeline::new(input());
    let chunk = run_planning(
        &mut p,
        "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
    )[0];
    p.on_error(chunk, "chunk transport failed".into());
    let retry = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected chunk retry, got {other:?}"),
    };
    p.on_error(retry, "chunk retry failed".into());
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected rescue, got {other:?}"),
    };
    p.on_delta(rescue, "I could not generate the requested source file.");
    p.on_complete(rescue);
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(degraded);
            assert!(!code.contains("could not generate"));
            assert!(code.contains("function") || code.contains("React"));
        }
        other => panic!("expected deterministic fallback, got {other:?}"),
    }
}

#[test]
fn final_failure_retains_every_stage_reason_when_nodes_are_invalid() {
    let mut broken = input();
    broken.nodes_json = "not-json".into();
    let mut p = CodegenPipeline::new(broken);
    let first = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected planning, got {other:?}"),
    };
    p.on_error(first, "planner transport one".into());
    let second = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected planning retry, got {other:?}"),
    };
    p.on_error(second, "planner transport two".into());
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected rescue, got {other:?}"),
    };
    p.on_error(rescue, "rescue transport failed".into());
    match p.step() {
        PipelineStep::Failed { message } => {
            assert!(message.contains("planner transport one"), "{message}");
            assert!(message.contains("planner transport two"), "{message}");
            assert!(message.contains("rescue transport failed"), "{message}");
            assert!(message.contains("not valid JSON"), "{message}");
        }
        other => panic!("expected detailed terminal failure, got {other:?}"),
    }
}

#[test]
fn single_chunk_above_twenty_four_kib_still_dispatches_to_the_model() {
    let huge = "x".repeat(25 * 1024);
    let nodes = serde_json::json!([{
        "type": "frame",
        "id": "root",
        "name": "Root",
        "children": [{ "type": "text", "id": "body", "content": huge }]
    }]);
    let mut p = CodegenPipeline::new(CodegenInput {
        nodes_json: nodes.to_string(),
        ..input()
    });
    let plan = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected planning, got {other:?}"),
    };
    p.on_delta(plan, "{\"chunks\":[{\"id\":\"whole\",\"name\":\"Whole\",\"nodeIds\":[\"root\"],\"role\":\"root\",\"suggestedComponentName\":\"Whole\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}");
    p.on_complete(plan);
    let requests = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("valid chunk below the prompt limit must dispatch, got {other:?}"),
    };
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0].kind, RequestKind::Chunk { .. }));
    assert!(requests[0].user_message.contains(&huge));
}

#[test]
fn single_file_rescue_above_twenty_four_kib_still_dispatches_to_the_model() {
    let huge = "x".repeat(25 * 1024);
    let mut p = CodegenPipeline::new(CodegenInput {
        nodes_json: serde_json::json!([{
            "type": "text",
            "id": "body",
            "content": huge
        }])
        .to_string(),
        ..input()
    });
    let first = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected planning, got {other:?}"),
    };
    p.on_error(first, "planner unavailable".into());
    let second = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected planning retry, got {other:?}"),
    };
    p.on_error(second, "planner still unavailable".into());
    let requests = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("valid rescue below the prompt limit must dispatch, got {other:?}"),
    };
    assert_eq!(requests.len(), 1);
    assert!(matches!(requests[0].kind, RequestKind::Assembly));
    assert!(requests[0].user_message.contains(&huge));
}

#[test]
fn oversized_planning_prompt_uses_fallback_without_dispatching_it() {
    let long_name = "N".repeat(500);
    let nodes = (0..300)
        .map(|index| {
            serde_json::json!({
                "type": "frame",
                "id": format!("n{index}"),
                "name": long_name,
                "children": []
            })
        })
        .collect::<Vec<_>>();
    let mut p = CodegenPipeline::new(CodegenInput {
        nodes_json: serde_json::to_string(&nodes).unwrap(),
        ..input()
    });
    let requests = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("expected fallback chunk dispatch, got {other:?}"),
    };
    assert!(!requests.is_empty());
    assert!(requests
        .iter()
        .all(|request| matches!(request.kind, RequestKind::Chunk { .. })));
    assert!(requests
        .iter()
        .all(|request| request.user_message.len() <= 120_000));
}

// ── FIX 4: nested selection keeps wrapper context without copying siblings ──

#[test]
fn chunk_node_json_slices_nested_child_and_carries_wrapper_context() {
    let mut input = input();
    // A frame whose child is the chunk's only node_id. The child carries a
    // distinctive name/type so we can confirm slicing (the chunk request's
    // `compact_nodes` strips the raw `id` field, so we assert on content).
    input.nodes_json = "[{\"type\":\"frame\",\"id\":\"root\",\"name\":\"RootFrame\",\"width\":1440,\"height\":900,\"fill\":\"#123456\",\"children\":[{\"type\":\"text\",\"id\":\"label\",\"name\":\"HelloChild\"}]}]".into();
    let mut p = CodegenPipeline::new(input);
    // Capture the chunk dispatch directly so we can inspect its user msg.
    let plan_id = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        other => panic!("expected planning dispatch, got {other:?}"),
    };
    p.on_delta(plan_id, "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Label\",\"nodeIds\":[\"label\"],\"role\":\"r\",\"suggestedComponentName\":\"Label\",\"dependencies\":[]}],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}");
    p.on_complete(plan_id);
    let chunk_reqs = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("expected chunk dispatch, got {other:?}"),
    };
    // The node payload stays sliced to the child, while the separate wrapper
    // context retains the parent metadata needed for faithful composition.
    let msg = &chunk_reqs[0].user_message;
    assert!(msg.contains("HelloChild"));
    assert!(msg.contains("text"));
    assert!(msg.contains("Ancestor Wrapper Context"));
    assert!(msg.contains("RootFrame"));
    assert!(msg.contains("#123456"));
    let nodes_section = msg.split("Nodes (JSON):\n").nth(1).expect("nodes section");
    assert!(!nodes_section.contains("RootFrame"));

    p.on_delta(
        chunk_reqs[0].id,
        "export default function Label(){return <div/>}\n---CONTRACT---\n{\"componentName\":\"Label\"}",
    );
    p.on_complete(chunk_reqs[0].id);
    let assembly = match p.step() {
        PipelineStep::Dispatch(requests) => requests,
        other => panic!("expected assembly dispatch, got {other:?}"),
    };
    assert!(assembly[0]
        .user_message
        .contains("Ancestor wrapper context"));
    assert!(assembly[0].user_message.contains("#123456"));
}

// ── FIX 5: assembly carries full rootLayout + sharedStyles ────────────

#[test]
fn assembly_request_includes_layout_gap_and_shared_style_name() {
    let mut p = CodegenPipeline::new(input());
    let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[{\"id\":\"c1\",\"name\":\"Root\",\"nodeIds\":[\"n1\"],\"role\":\"r\",\"suggestedComponentName\":\"Root\",\"dependencies\":[]}],\"sharedStyles\":[{\"name\":\"brandPrimary\",\"description\":\"main\"}],\"rootLayout\":{\"direction\":\"row\",\"gap\":24,\"responsive\":true}}",
        );
    p.on_delta(
            chunk_ids[0],
            "export default function Root(){ return null }\n---CONTRACT---\n{\"componentName\":\"Root\"}",
        );
    p.on_complete(chunk_ids[0]);
    let asm = match p.step() {
        PipelineStep::Dispatch(r) => r,
        other => panic!("expected assembly dispatch, got {other:?}"),
    };
    let msg = &asm[0].user_message;
    assert!(msg.contains("\"gap\":24"));
    assert!(msg.contains("\"responsive\":true"));
    assert!(msg.contains("brandPrimary"));
}

// ── FIX 6: fallback filters empty-code chunks ─────────────────────────

#[test]
fn failed_assembly_never_concatenates_multiple_default_exports() {
    let mut input = input();
    input.nodes_json =
            "[{\"type\":\"frame\",\"id\":\"a\",\"children\":[]},{\"type\":\"frame\",\"id\":\"b\",\"children\":[]}]"
                .into();
    let mut p = CodegenPipeline::new(input);
    // Two independently valid chunk files each own a default export. Raw
    // concatenation would produce an invalid file with two defaults.
    let chunk_ids = run_planning(
            &mut p,
            "{\"chunks\":[\
                {\"id\":\"c1\",\"name\":\"Good\",\"nodeIds\":[\"a\"],\"role\":\"r\",\"suggestedComponentName\":\"Good\",\"dependencies\":[]},\
                {\"id\":\"c2\",\"name\":\"Bad\",\"nodeIds\":[\"b\"],\"role\":\"r\",\"suggestedComponentName\":\"Bad\",\"dependencies\":[]}\
             ],\"sharedStyles\":[],\"rootLayout\":{\"direction\":\"column\",\"gap\":0,\"responsive\":false}}",
        );
    assert_eq!(chunk_ids.len(), 2);
    for (id, name) in chunk_ids.into_iter().zip(["Good", "Bad"]) {
        p.on_delta(
            id,
            &format!(
                "export default function {name}(){{}}\n---CONTRACT---\n{{\"componentName\":\"{name}\"}}"
            ),
        );
        p.on_complete(id);
    }
    let a1 = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        other => panic!("expected assembly dispatch, got {other:?}"),
    };
    p.on_error(a1, "asm boom".into());
    let a2 = match p.step() {
        PipelineStep::Dispatch(r) => r[0].id,
        other => panic!("expected assembly retry dispatch, got {other:?}"),
    };
    p.on_error(a2, "asm boom2".into());
    let rescue = match p.step() {
        PipelineStep::Dispatch(requests) => requests[0].id,
        other => panic!("expected rescue dispatch, got {other:?}"),
    };
    p.on_error(rescue, "rescue boom".into());
    match p.step() {
        PipelineStep::Done { code, degraded, .. } => {
            assert!(degraded);
            assert_eq!(code.matches("export default").count(), 1, "{code}");
            assert!(!code.contains("function Good"), "{code}");
            assert!(!code.contains("function Bad"), "{code}");
        }
        other => panic!("expected deterministic Done, got {other:?}"),
    }
}
