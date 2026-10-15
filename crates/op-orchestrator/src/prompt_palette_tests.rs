use super::*;

#[test]
fn explicit_palette_contract_reaches_all_planner_modes() {
    for mode in [
        PlanningMode::Rich,
        PlanningMode::Minimal,
        PlanningMode::Compact,
    ] {
        let mut request = req();
        request.prompt = "配送追踪，深色海军蓝配暖橙色".into();
        let call = build_orchestrator_prompt(&request, mode, AbortFlag::new()).call_request;
        assert!(call.system_prompt.contains("SHARED PALETTE"));
        assert!(call.system_prompt.contains("primaryForeground"));
        assert!(call.system_prompt.contains("Omit palette when"));
    }
}

#[test]
fn seeded_palette_wins_over_guide_in_every_section_and_retry_prompt() {
    let mut p = plan();
    p.style_guide_name = Some("music-dark-mobile".into());
    p.palette = Some(
        serde_json::from_value(serde_json::json!({
            "background":"#0F1B2D", "primary":"#F97316", "primaryForeground":"#0F1B2D"
        }))
        .unwrap(),
    );
    let mut request = req();
    request.model = Some("glm-5.3-flash".into());
    for minimal in [false, true] {
        for has_variables in [false, true] {
            let (call, _) = build_subagent_prompt_with_screen_routes(
                &subtask(),
                &p,
                &request,
                AbortFlag::new(),
                minimal,
                minimal,
                has_variables,
                &ComponentLibrary::default(),
                &[],
            );
            let prompt = call.system_prompt;
            let palette_at = prompt.find("SHARED BRIEF PALETTE").unwrap();
            let guide_at = prompt.find("VISUAL STYLE GUIDE").unwrap();
            assert!(palette_at > guide_at);
            let block = &prompt[palette_at..];
            assert!(block.contains("#F97316"));
            assert!(!block.contains("#84CC16"));
            assert!(block.contains("Keep semantic success"));
            if has_variables {
                assert!(block.contains("$--primary"));
                assert!(block.contains("Existing document brand values take precedence"));
            } else {
                assert!(block.contains("--primary: #F97316"));
            }
        }
    }
}

#[test]
fn absent_or_invalid_palette_does_not_add_color_override() {
    let mut p = plan();
    assert!(build_plan_palette_instruction(&p, true).is_none());
    p.palette = Some(serde_json::from_value(serde_json::json!({"primary":"orange"})).unwrap());
    assert!(build_plan_palette_instruction(&p, true).is_none());
}

#[test]
fn design_md_does_not_receive_conflicting_planner_palette() {
    let mut p = plan();
    p.palette = Some(serde_json::from_value(serde_json::json!({"primary":"#F97316"})).unwrap());
    let mut request = req();
    request.design_md = Some(serde_json::from_value(serde_json::json!({
        "raw":"Brand color: #0E7C66", "colorPalette":[{"name":"Brand", "hex":"#0E7C66", "role":"primary"}]
    })).unwrap());
    let (call, _) = bsp(&subtask(), &p, &request, AbortFlag::new(), false, false);
    assert!(!call.system_prompt.contains("SHARED BRIEF PALETTE"));
    assert!(call.system_prompt.contains("#0E7C66"));
}
