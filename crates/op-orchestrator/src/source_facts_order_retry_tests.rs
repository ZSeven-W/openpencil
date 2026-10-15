use super::*;

const ORDER_ID: &str = "WDS-20240612";

fn order_plan() -> String {
    plan().replace("姓名与评价标签", "订单编号")
}

fn order_script(with_identifier: bool) -> String {
    let base = script(false);
    let mut frame: serde_json::Value = serde_json::from_str(
        base.strip_prefix("I(null, ")
            .unwrap()
            .strip_suffix(");")
            .unwrap(),
    )
    .unwrap();
    if with_identifier {
        frame["children"].as_array_mut().unwrap().push(json!({
            "type":"text","name":"Order Number","content":format!("订单号 {ORDER_ID}"),
            "fontSize":12,"width":"fit_content","height":"fit_content"
        }));
    }
    format!("I(null, {frame});")
}

fn inserted_identifier(command: &EditorCommand) -> bool {
    match command {
        EditorCommand::InsertSubtree { nodes, .. }
        | EditorCommand::InsertAuthoredSubtree { nodes, .. } => {
            texts(nodes).iter().any(|text| text.contains(ORDER_ID))
        }
        EditorCommand::Batch { commands } => commands.iter().any(inserted_identifier),
        _ => false,
    }
}

#[test]
fn source_facts_order_retries_before_insertion_and_preserves_complete_section() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(order_plan()),
        ScriptResponse::Text(order_script(true)),
        ScriptResponse::Text(order_script(false)),
    ]);
    let mut sink = VecDocSink::new();
    let mut events = Vec::new();
    let summary = futures::executor::block_on(Orchestrator::new().run(
        request(),
        &mut sink,
        &llm,
        &mut |event| events.push(event),
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("a corrected identifier-free section should recover");
    assert!(summary.subtasks[0].error.is_none());
    assert_eq!(llm.user_prompts().len(), 3);
    assert!(llm.user_prompts()[2].contains("unsupported-order-identifier"));
    assert_eq!(llm.system_prompts()[1], llm.system_prompts()[2]);
    assert!(!sink.applied.iter().any(inserted_identifier));
    let delivered = texts(sink.state.active_children());
    for required in ["骑手阿明", "联系骑手"] {
        assert_eq!(
            delivered.iter().filter(|t| t.as_str() == required).count(),
            1
        );
    }
    assert!(events.iter().any(
        |event| matches!(event,Progress::SubtaskRetry{id,attempt:2,reason}
        if id=="courier" && reason.contains("unsupported-order-identifier"))
    ));
}

#[test]
fn source_facts_order_final_salvage_cannot_insert_unsupported_identifier() {
    let llm = ScriptedLlm::new(
        std::iter::once(ScriptResponse::Text(order_plan()))
            .chain((0..4).map(|_| ScriptResponse::Text(order_script(true))))
            .collect(),
    );
    let mut sink = VecDocSink::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        request(),
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ));
    assert!(
        matches!(result,Err(OrchestratorError::AllFailed(ref reason))
        if reason.contains("unsupported-order-identifier")),
        "{result:?}"
    );
    assert_eq!(llm.user_prompts().len(), 5);
    assert!(!sink.applied.iter().any(inserted_identifier));
    assert!(sink.state.active_children().is_empty());
}
