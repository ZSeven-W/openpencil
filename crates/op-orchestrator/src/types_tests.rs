//! Tests for the orchestrator's shared traits + value types.

use super::*;
use crate::test_support::VecDocSink;

#[test]
fn vec_doc_sink_implements_docsink() {
    let mut sink = VecDocSink::new();
    sink.begin_undo_batch();
    sink.end_undo_batch();
    assert_eq!(sink.batch_depth, 0);
    assert!(sink.applied.is_empty());
    let _ = sink.state();
}

#[test]
fn abort_flag_sets_and_reads() {
    let flag = AbortFlag::new();
    assert!(!flag.is_set());
    let clone = flag.clone();
    flag.set();
    assert!(clone.is_set());
}

// ── geometry_echo budget ────────────────────────────────────────────────

#[test]
fn geometry_echo_budget_consumes_down_to_zero_then_refuses() {
    let budget = GeometryEchoBudget::new(2);
    assert!(budget.try_consume());
    assert!(budget.try_consume());
    assert!(!budget.try_consume(), "cap of 2 must refuse a 3rd consume");
    assert!(
        !budget.try_consume(),
        "stays exhausted, does not wrap around"
    );
}

#[test]
fn geometry_echo_budget_of_zero_never_consumes() {
    let budget = GeometryEchoBudget::new(0);
    assert!(!budget.try_consume());
}

#[test]
fn geometry_echo_budget_is_shared_across_clones() {
    // Same shape as `AbortFlag` — clones share the SAME underlying
    // counter (run-wide, not per-clone), since it's threaded by
    // reference/clone through both the sequential loop and every
    // screen-group worker.
    let budget = GeometryEchoBudget::new(1);
    let clone = budget.clone();
    assert!(clone.try_consume());
    assert!(
        !budget.try_consume(),
        "the clone's consume must be visible here too"
    );
}

#[test]
fn geometry_echo_cap_is_min_of_subtask_count_and_six_by_default() {
    assert_eq!(geometry_echo_cap(3, None), 3);
    assert_eq!(geometry_echo_cap(9, None), 6);
    assert_eq!(geometry_echo_cap(0, None), 0);
}

#[test]
fn geometry_echo_cap_is_zero_when_env_says_zero() {
    assert_eq!(geometry_echo_cap(9, Some("0")), 0);
}

#[test]
fn geometry_echo_cap_ignores_other_env_values() {
    // Only the literal "0" disables it — any other value (including a
    // typo'd truthy string) leaves the default cap in effect rather
    // than silently doing something else.
    assert_eq!(geometry_echo_cap(9, Some("1")), 6);
    assert_eq!(geometry_echo_cap(9, Some("false")), 6);
}

// ── Task A1: AppendContext + DesignRequest.append_context ─────────────────

/// AppendContext serde round-trips with all 4 fields (camelCase wire names).
#[test]
fn append_context_serde_round_trip() {
    let ctx = AppendContext {
        target_parent_id: "frame-abc".into(),
        target_width: 390.0,
        existing_section_labels: vec!["Hero".into(), "Pricing".into()],
        is_mobile: true,
    };
    let json = serde_json::to_string(&ctx).expect("serialize");
    // Wire names are camelCase
    assert!(
        json.contains("targetParentId"),
        "expected targetParentId in {json}"
    );
    assert!(
        json.contains("targetWidth"),
        "expected targetWidth in {json}"
    );
    assert!(
        json.contains("existingSectionLabels"),
        "expected existingSectionLabels in {json}"
    );
    assert!(json.contains("isMobile"), "expected isMobile in {json}");
    let back: AppendContext = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.target_parent_id, "frame-abc");
    assert_eq!(back.target_width, 390.0);
    assert_eq!(back.existing_section_labels, vec!["Hero", "Pricing"]);
    assert!(back.is_mobile);
}

#[test]
fn continuation_context_serde_round_trip() {
    let context = ContinuationContext {
        screen_width: 390.0,
        screen_height: 844.0,
        background_color: Some("#050508".into()),
        screen_names: vec!["星图".into(), "观测计划".into(), "我的".into()],
    };
    let json = serde_json::to_string(&context).expect("serialize");
    assert!(json.contains("screenWidth"));
    assert!(json.contains("backgroundColor"));
    let back: ContinuationContext = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, context);
}

/// DesignRequest accepts append_context: None without breaking compilation.
#[test]
fn design_request_append_context_none_compiles() {
    let req = DesignRequest {
        prompt: "test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    };
    assert!(req.append_context.is_none());
}

/// DesignRequest accepts a populated AppendContext.
#[test]
fn design_request_append_context_some_compiles() {
    let ctx = AppendContext {
        target_parent_id: "p1".into(),
        target_width: 1200.0,
        existing_section_labels: vec![],
        is_mobile: false,
    };
    let req = DesignRequest {
        prompt: "extend page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: Some(ctx),
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    };
    assert!(req.append_context.is_some());
}

/// DesignRequest without append_context serializes without the field.
#[test]
fn design_request_append_context_omitted_from_json_when_none() {
    let req = DesignRequest {
        prompt: "test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    assert!(
        !json.contains("appendContext"),
        "appendContext should be omitted when None, got: {json}"
    );
}

// ── Task A1 (S3c): vision validation types ────────────────────────────────

/// `PreValidator` stub returns zero fixes.
#[test]
fn skipped_pre_validator_returns_empty_result() {
    use crate::test_support::SkippedPreValidator;
    let mut sink = VecDocSink::new();
    let result = SkippedPreValidator.run_pre_validation_fixes(&mut sink);
    assert_eq!(result.total, 0);
    assert!(result.by_category.is_empty());
}

/// `ScreenshotProvider` stub returns `None`.
#[test]
fn skipped_screenshot_provider_returns_none() {
    use crate::test_support::SkippedScreenshotProvider;
    let state = op_editor_core::EditorState::new();
    assert!(SkippedScreenshotProvider
        .capture_root_frame(&state)
        .is_none());
}

/// `VisionLlmClient` stub returns `VisionResponse::Skipped`.
#[test]
fn skipped_vision_llm_client_returns_skipped() {
    use crate::test_support::SkippedVisionLlmClient;
    let req = VisionCallRequest {
        system: "sys".into(),
        message: "msg".into(),
        image_base64: "img".into(),
        model: None,
        provider: None,
        timeout: std::time::Duration::from_millis(5_000),
    };
    let resp = SkippedVisionLlmClient.validate(req);
    assert!(matches!(resp, VisionResponse::Skipped { .. }));
}

/// `Progress::Validation*` variants all compile and pattern-match.
#[test]
fn progress_validation_variants_compile() {
    use std::collections::BTreeMap;
    let variants = vec![
        Progress::ValidationStarted,
        Progress::ValidationPreCheckDone {
            applied: 3,
            by_category: BTreeMap::new(),
        },
        Progress::ValidationRoundStarted { round: 1 },
        Progress::ValidationRoundDone {
            round: 1,
            applied: 2,
            quality_score: 7,
        },
        Progress::ValidationDone { total_applied: 5 },
    ];
    // Verify all variants are matchable
    for v in variants {
        match v {
            Progress::ValidationStarted => {}
            Progress::ValidationPreCheckDone { .. } => {}
            Progress::ValidationRoundStarted { .. } => {}
            Progress::ValidationRoundDone { .. } => {}
            Progress::ValidationDone { .. } => {}
            _ => {}
        }
    }
}

// ── Task A2: VisualRefProvider trait + SkippedVisualRefProvider stub ─────────

/// `SkippedVisualRefProvider` returns `None` for any input.
#[test]
fn skipped_visual_ref_provider_returns_none() {
    use crate::stub_providers::SkippedVisualRefProvider;
    let p = SkippedVisualRefProvider;
    assert!(p
        .render_html_to_screenshot("<html></html>", 1280.0, 800.0)
        .is_none());
    assert!(p.render_html_to_screenshot("", 0.0, 0.0).is_none());
    assert!(p
        .render_html_to_screenshot("<html><body>Hello</body></html>", 390.0, 844.0)
        .is_none());
}

/// `Progress::SubtaskSkills` / `SubtaskRetry` + `SkillBrief` compile and match.
#[test]
fn progress_skill_variants_compile() {
    let brief = SkillBrief {
        name: "cjk-typography".into(),
        token_count: 800,
        truncated: false,
    };
    assert_eq!(brief.token_count, 800);
    let skills = Progress::SubtaskSkills {
        id: "header".into(),
        included: vec![brief],
        dropped: vec![("examples".into(), "budget".into())],
        budget_used: 5200,
        budget_max: 12000,
    };
    let retry = Progress::SubtaskRetry {
        id: "header".into(),
        attempt: 2,
        reason: "zero nodes generated".into(),
    };
    for v in [skills, retry] {
        match v {
            Progress::SubtaskSkills { .. } => {}
            Progress::SubtaskRetry { .. } => {}
            _ => {}
        }
    }
}

/// `Progress::SubtaskNodes` compile and match.
#[test]
fn progress_subtask_nodes_compile() {
    let nodes = Progress::SubtaskNodes {
        id: "body".into(),
        nodes_so_far: 12,
    };
    if let Progress::SubtaskNodes { id, nodes_so_far } = nodes {
        assert_eq!(id, "body");
        assert_eq!(nodes_so_far, 12);
    }
}

#[test]
fn worker_scoped_progress_keeps_group_identity_and_boxed_event() {
    let identity = crate::agent_identity::AgentIdentity {
        color: "#5B8DEF".into(),
        name: "Pixel".into(),
    };
    let progress = Progress::worker_scoped(
        2,
        "Saved",
        identity.clone(),
        Progress::SubtaskNodes {
            id: "saved-grid".into(),
            nodes_so_far: 8,
        },
    );

    let Progress::WorkerScoped(worker) = progress else {
        panic!("expected worker-scoped progress");
    };
    assert_eq!(worker.group_idx, 2);
    assert_eq!(worker.screen, "Saved");
    assert_eq!(worker.identity, identity);
    assert!(matches!(
        worker.event.as_ref(),
        Progress::SubtaskNodes { id, nodes_so_far }
            if id == "saved-grid" && *nodes_so_far == 8
    ));
}

/// `VisualRefProvider` trait is `Send + Sync`.
#[test]
fn visual_ref_provider_is_send_sync() {
    use crate::types::VisualRefProvider;
    fn assert_send_sync<T: Send + Sync + ?Sized>() {}
    assert_send_sync::<dyn VisualRefProvider>();
}

/// `op_orchestrator::SkippedVisualRefProvider` resolves from a host-style import.
#[test]
fn skipped_visual_ref_provider_resolves_from_crate_root() {
    use crate::{SkippedVisualRefProvider, VisualRefProvider};
    let p: &dyn VisualRefProvider = &SkippedVisualRefProvider;
    assert!(p
        .render_html_to_screenshot("<p>test</p>", 800.0, 600.0)
        .is_none());
}

// ── Task A2 end ───────────────────────────────────────────────────────────────

// ── Task B2: SkillBrief::from_entry + report_to_progress_parts ───────────────

/// `report_to_progress_parts` maps a `SkillLoadReport` into SubtaskSkills
/// payload parts: briefs, (name, reason) drops, budget_used, budget_max.
#[test]
fn report_to_progress_parts_maps_entries_and_drops() {
    use op_ai_skills::{DropReason, DroppedSkill, SkillCategory, SkillLoadEntry, SkillLoadReport};
    let report = SkillLoadReport {
        included: vec![SkillLoadEntry {
            name: "cjk-typography".into(),
            category: SkillCategory::Domain,
            token_count: 800,
            truncated: true,
        }],
        dropped: vec![DroppedSkill {
            name: "examples".into(),
            reason: DropReason::BudgetExhausted,
        }],
        budget_used: 5200,
        budget_max: 12000,
    };
    let (briefs, drops, used, max) = report_to_progress_parts(&report);
    assert_eq!(briefs.len(), 1);
    assert_eq!(briefs[0].name, "cjk-typography");
    assert_eq!(briefs[0].token_count, 800);
    assert!(briefs[0].truncated);
    assert_eq!(drops, vec![("examples".to_string(), "budget".to_string())]);
    assert_eq!(used, 5200);
    assert_eq!(max, 12000);
}

/// All 8 `DropReason` variants map to distinct, non-empty display strings.
#[test]
fn drop_reason_all_variants_covered() {
    use op_ai_skills::{DropReason, DroppedSkill, SkillLoadReport};
    let reasons = [
        DropReason::IntentMiss,
        DropReason::BudgetExhausted,
        DropReason::TierFiltered,
        DropReason::MinimalMode,
        DropReason::ReducedComplexity,
        DropReason::Deduped,
        DropReason::ContentMismatch,
        DropReason::ModelFamilyMiss,
    ];
    let expected = [
        "intent", "budget", "tier", "minimal", "reduced", "dedup", "mismatch", "family",
    ];
    for (reason, exp) in reasons.iter().zip(expected.iter()) {
        let report = SkillLoadReport {
            included: vec![],
            dropped: vec![DroppedSkill {
                name: "skill".into(),
                reason: *reason,
            }],
            budget_used: 0,
            budget_max: 0,
        };
        let (_, drops, _, _) = report_to_progress_parts(&report);
        assert_eq!(drops[0].1, *exp, "reason {reason:?} should map to {exp}");
    }
}

// ── Task B2 end ───────────────────────────────────────────────────────────────

/// `DesignRequest.validation_enabled` defaults to `true` when omitted from JSON.
#[test]
fn design_request_validation_enabled_defaults_true() {
    // JSON without `validationEnabled` field
    let json = r#"{"prompt":"test","concurrency":1}"#;
    let req: DesignRequest = serde_json::from_str(json).expect("deserialize");
    assert!(
        req.validation_enabled,
        "validation_enabled should default to true"
    );
}

/// `DesignRequest.validation_enabled` round-trips via serde.
#[test]
fn design_request_validation_enabled_serde_roundtrip() {
    let req = DesignRequest {
        prompt: "test".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let back: DesignRequest = serde_json::from_str(&json).expect("deserialize");
    assert!(!back.validation_enabled);
}

/// Config consts have the values faithful to `ai-runtime-config.ts:119-123`.
#[test]
fn validation_config_consts_match_ts() {
    use crate::validation_config::{
        MAX_VALIDATION_ROUNDS, VALIDATION_NODE_COUNT_THRESHOLD, VALIDATION_QUALITY_THRESHOLD,
        VALIDATION_TIMEOUT_MS,
    };
    assert_eq!(VALIDATION_NODE_COUNT_THRESHOLD, 30);
    assert_eq!(MAX_VALIDATION_ROUNDS, 3);
    assert_eq!(VALIDATION_QUALITY_THRESHOLD, 8);
    assert_eq!(VALIDATION_TIMEOUT_MS, 180_000);
}
