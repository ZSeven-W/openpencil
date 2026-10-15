use super::*;

fn delivery_script(extra: bool, brand: bool) -> String {
    let base = script(false);
    let mut frame: serde_json::Value = serde_json::from_str(
        base.strip_prefix("I(null, ")
            .unwrap()
            .strip_suffix(");")
            .unwrap(),
    )
    .unwrap();
    let children = frame["children"].as_array_mut().unwrap();
    for (name, value) in [
        ("brand", "晚点食堂"),
        ("meal", "双人晚餐"),
        ("eta", "预计12分钟送达"),
    ] {
        if name == "brand" && !brand {
            continue;
        }
        children.push(json!({"type":"text","name":name,"content":value,"fontSize":16,"width":"fit_content","height":"fit_content"}));
    }
    if extra {
        children.push(json!({"type":"text","name":"arrival","content":"预计19:32送达 · ¥88.00 · ×1","fontSize":14,"width":"fit_content","height":"fit_content"}));
    }
    format!("I(null, {frame});")
}

fn inserted_extra(command: &EditorCommand) -> bool {
    match command {
        EditorCommand::InsertSubtree { nodes, .. }
        | EditorCommand::InsertAuthoredSubtree { nodes, .. } => texts(nodes)
            .iter()
            .any(|text| text.contains("19:32") || text.contains("¥88")),
        EditorCommand::Batch { commands } => commands.iter().any(inserted_extra),
        _ => false,
    }
}

#[test]
fn delivery_copy_and_optional_facts_use_same_section_retry_before_insertion() {
    let mut request = request();
    request
        .prompt
        .push_str("品牌“晚点食堂”，预计12分钟送达，双人晚餐。");
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(plan().replace(
            "姓名与评价标签",
            "晚点食堂、双人晚餐、预计19:32送达、价格88元、数量1",
        )),
        ScriptResponse::Text(delivery_script(false, false)),
        ScriptResponse::Text(delivery_script(true, true)),
        ScriptResponse::Text(delivery_script(false, true)),
    ]);
    let mut sink = VecDocSink::new();
    let summary = futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut |_| {},
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .unwrap();
    assert!(summary.subtasks[0].error.is_none());
    let prompts = llm.user_prompts();
    assert_eq!(prompts.len(), 4);
    assert!(prompts[2].contains("source-copy-missing"));
    assert!(prompts[3].contains("unsupported-delivery-facts"));
    assert!(!sink.applied.iter().any(inserted_extra));
    let delivered = texts(sink.state.active_children());
    for required in [
        "晚点食堂",
        "双人晚餐",
        "预计12分钟送达",
        "骑手阿明",
        "联系骑手",
    ] {
        assert_eq!(
            delivered
                .iter()
                .filter(|text| text.as_str() == required)
                .count(),
            1
        );
    }
}

#[test]
fn delivery_unsupported_facts_cannot_survive_final_salvage() {
    let llm = ScriptedLlm::new(
        std::iter::once(ScriptResponse::Text(plan()))
            .chain((0..4).map(|_| ScriptResponse::Text(delivery_script(true, true))))
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
        matches!(result, Err(OrchestratorError::AllFailed(ref reason)) if reason.contains("unsupported-delivery-facts")),
        "{result:?}"
    );
    assert_eq!(llm.user_prompts().len(), 5);
    assert!(!sink.applied.iter().any(inserted_extra));
    assert!(sink.state.active_children().is_empty());
}

#[test]
fn normal_mode_dimensions_do_not_authorize_quantity_or_a_completed_delivery_action() {
    for extra in ["双人晚餐 ×1", "距离您约1.5公里", "确认收货"] {
        let bad = delivery_script(false, true).replace("双人晚餐", extra);
        let mut req = request();
        req.prompt.push_str("预计12分钟送达，双人晚餐。");
        let llm = ScriptedLlm::new(vec![
            ScriptResponse::Text(plan()),
            ScriptResponse::Text(bad),
            ScriptResponse::Text(delivery_script(false, true)),
        ]);
        let mut sink = VecDocSink::new();
        let summary = futures::executor::block_on(Orchestrator::new().run(
            req,
            &mut sink,
            &llm,
            &mut |_| {},
            &AbortFlag::new(),
            &stub_providers(),
        ))
        .unwrap();
        assert!(summary.subtasks[0].error.is_none(), "{extra}");
        let prompts = llm.user_prompts();
        assert_eq!(prompts.len(), 3, "{extra}");
        assert!(prompts[2].contains("unsupported-delivery-facts"), "{extra}");
        assert!(!texts(sink.state.active_children())
            .iter()
            .any(|text| text == extra));
    }
}
