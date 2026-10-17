use super::*;

fn home_request(brief: &str) -> DesignRequest {
    DesignRequest {
        prompt: op_editor_core::HomeFamily::AppUi
            .generation_prompt(&op_editor_core::TaskDraft {
                text: brief.into(),
                device: op_editor_core::HomeDevice::Mobile,
                ..Default::default()
            })
            .unwrap(),
        model: Some("glm-5.3-flash".into()),
        ..Default::default()
    }
}

#[test]
fn home_copy_language_contract_reaches_planning_and_every_retry_rung() {
    for (brief, expected) in [
        (
            "Workout detail screen with six exercises",
            "original brief's Latin-script language",
        ),
        ("设计咖啡点单首页", "original brief's CJK language"),
    ] {
        let request = home_request(brief);
        for mode in [
            PlanningMode::Compact,
            PlanningMode::Rich,
            PlanningMode::Minimal,
        ] {
            let prompt = build_orchestrator_prompt(&request, mode, AbortFlag::new()).call_request;
            assert!(prompt.system_prompt.contains(expected), "{mode:?}");
            assert!(prompt.user_prompt.contains(brief));
        }
        for (reduced, minimal) in [(false, false), (true, false), (true, true)] {
            let (prompt, _) = bsp(
                &subtask(),
                &plan(),
                &request,
                AbortFlag::new(),
                reduced,
                minimal,
            );
            assert!(
                prompt.system_prompt.contains(expected),
                "reduced={reduced}, minimal={minimal}"
            );
            assert!(prompt.user_prompt.contains(brief));
        }
    }
}

#[test]
fn bilingual_home_brief_has_no_forced_script_contract() {
    let request = home_request("Design a bilingual English and Chinese UI");
    let (prompt, _) = bsp(&subtask(), &plan(), &request, AbortFlag::new(), true, true);
    assert!(prompt
        .system_prompt
        .contains("Follow explicit language/bilingual requests"));
    assert!(!prompt
        .system_prompt
        .contains("Keep all generated UI copy in the original brief's"));
}
