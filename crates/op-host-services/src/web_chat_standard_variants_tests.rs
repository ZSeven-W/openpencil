//! Tests for the web variants route: the SSE `variant` frame and a scripted
//! variants run landing through the daemon's design sink.

use std::sync::Mutex;

use jian_ops_schema::node::PenNode;
use op_editor_core::variant_wire::VariantOutcomeWire;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt, WorkspaceVariant};
use op_orchestrator::variants_run::{run_variants_with, variant_jobs};
use op_orchestrator::{OrchestratorError, RunSummary};
use serde_json::json;

use super::*;
use crate::web_canvas_server::{SseHub, WebCanvasState};

const BRIEF: &str = "Design a mobile app (375×812) for tracking household expenses";

fn landed(index: usize) -> WorkspaceVariant {
    WorkspaceVariant {
        index,
        name: "Direction A".into(),
        style_guide: "zen-paper-light".into(),
        style_label: "Zen Paper Light".into(),
        name_prefix: "Direction A · Zen Paper Light · ".into(),
        root_ids: vec!["r0".into()],
        variables: None,
        themes: None,
    }
}

/// Split one `data: …\n\n` frame back into its JSON payload.
fn frame_payload(bytes: &[u8]) -> serde_json::Value {
    let text = std::str::from_utf8(bytes).unwrap();
    let body = text
        .strip_prefix("data: ")
        .and_then(|rest| rest.strip_suffix("\n\n"))
        .expect("one SSE data frame");
    serde_json::from_str(body).unwrap()
}

#[test]
fn only_direction_level_reports_become_variant_frames() {
    assert_eq!(
        variant_wire_for(&Progress::VariantReady(landed(0))),
        Some(VariantEventWire::ready(&landed(0)))
    );
    assert_eq!(
        variant_wire_for(&Progress::VariantFailed {
            index: 2,
            name: "Direction C".into(),
            error: "nothing drawn".into(),
        }),
        Some(VariantEventWire::failed(2, "Direction C", "nothing drawn"))
    );
    assert_eq!(variant_wire_for(&Progress::CleanupDone), None);
}

#[test]
fn a_variant_frame_decodes_back_to_the_same_report() {
    let wire = VariantEventWire::ready(&landed(1));
    let mut out = Vec::new();
    write_variant_event(&mut out, &wire).unwrap();
    let payload = frame_payload(&out);
    let decoded = VariantEventWire::decode(&payload["variant"]).expect("decodes");
    assert_eq!(decoded, wire);
    assert_eq!(decoded.to_workspace_variant(), Some(landed(1)));
}

#[test]
fn a_variants_turn_body_carries_its_count_and_the_browser_locale() {
    let body = json!({
        "model": "claude-sonnet",
        "user": BRIEF,
        "launchRoute": "variants",
        "variantCount": 4,
        "locale": "zh-CN",
    })
    .to_string();
    let req = super::super::parse_standard_turn_body(&body).expect("parses");
    assert_eq!(req.launch_route, LaunchRoute::Variants(4));
    assert_eq!(req.locale, Some(Locale::ZhCn));

    let plain = json!({"model": "claude-sonnet", "user": BRIEF, "locale": "xx"}).to_string();
    let req = super::super::parse_standard_turn_body(&plain).expect("parses");
    assert_eq!(req.launch_route, LaunchRoute::Auto);
    assert_eq!(
        req.locale, None,
        "an unknown locale falls back to the daemon's"
    );
}

/// A direction's private document after a scripted "run": one phone
/// screen in its own colour.
fn finished_state(mut state: EditorState, index: usize) -> EditorState {
    let root: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "screen", "name": "Home",
        "x": 0, "y": 0, "width": 375, "height": 812,
        "fill": [{"type": "solid", "color": format!("#00000{index}")}],
        "children": [{"type": "text", "id": "title", "content": "Hi"}]
    }))
    .unwrap();
    state.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    state
}

fn summary() -> RunSummary {
    RunSummary {
        root_frame_id: "screen".into(),
        subtasks: Vec::new(),
        total_nodes: 2,
        unfilled_screens: Vec::new(),
        incomplete_subtask_failure: false,
    }
}

#[test]
fn a_scripted_variants_run_lands_every_direction_in_the_daemon_document() {
    let mut blank = EditorState::new();
    blank.active_children_mut().clear();
    let state = Mutex::new(WebCanvasState::new(blank.clone(), 3100));
    let hub = SseHub::default();
    let sub = hub.subscribe();
    let mut sink = WebDesignDocSink::new(&state, &hub, None, blank.clone());

    let request = DesignRequest {
        prompt: BRIEF.into(),
        model: None,
        provider: None,
        design_md: None,
        continuation_context: None,
        append_context: None,
        concurrency: 3,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    };
    let mut plans = choose_variant_style_guides(BRIEF, None, 3);
    localize_variant_plans(&mut plans, Locale::ZhCn);
    let jobs = variant_jobs(&request, &plans, &blank);

    let mut frames = Vec::new();
    let result = crate::chat_runtime::block_on_anywhere(run_variants_with(
        jobs,
        &mut sink,
        &mut |event| {
            if let Some(wire) = variant_wire_for(&event) {
                let mut out = Vec::new();
                write_variant_event(&mut out, &wire).unwrap();
                frames.push(frame_payload(&out)["variant"].clone());
            }
        },
        |job, _tap| async move {
            // Direction B produces nothing; A and C land.
            if job.plan.index == 1 {
                return Err(OrchestratorError::NoContent);
            }
            Ok((finished_state(job.state, job.plan.index), summary()))
        },
    ));
    let summary = result.expect("two directions landed");
    assert_eq!(summary.total_nodes, 4);

    // Both landed directions are on the daemon's page, named for the
    // user's locale, and every commit bumped the version and ticked SSE.
    let live = state.lock().unwrap();
    let names: Vec<String> = live
        .editor
        .active_children()
        .iter()
        .map(|root| root.base().name.clone().unwrap_or_default())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(
        names.iter().any(|name| name.starts_with("方案 A · ")),
        "{names:?}"
    );
    assert!(
        names.iter().any(|name| name.starts_with("方案 C · ")),
        "{names:?}"
    );
    assert!(live.version >= 2);
    assert_eq!(sub.pending().expect("published").version, live.version);

    // One frame per direction, each decoding to what the browser folds.
    let decoded: Vec<VariantEventWire> = frames
        .iter()
        .map(|frame| VariantEventWire::decode(frame).expect("frame decodes"))
        .collect();
    assert_eq!(decoded.len(), 3);
    let failed = decoded.iter().find(|event| event.index == 1).unwrap();
    assert!(matches!(failed.outcome, VariantOutcomeWire::Failed(_)));
    for event in decoded.iter().filter(|event| event.index != 1) {
        let variant = event.to_workspace_variant().expect("ready");
        for id in &variant.root_ids {
            assert!(
                live.editor
                    .active_children()
                    .iter()
                    .any(|root| root.id_str() == id),
                "landed root {id} is on the daemon page"
            );
        }
    }
}
