//! Tests for the concurrent side-by-side directions driver.

use std::cell::Cell;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::*;
use crate::test_support::VecDocSink;
use crate::variants::choose_variant_style_guides;
use futures::executor::block_on;
use serde_json::json;

const BRIEF: &str =
    "请设计一套可编辑的高保真手机 App 界面（mobile app，375×812）。用户需求：记账 app";

/// Resolves after `polls` pending polls — a stand-in for model latency
/// that lets the other directions' futures run in between.
struct Latency {
    polls: usize,
}

impl Future for Latency {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.polls == 0 {
            return Poll::Ready(());
        }
        self.polls -= 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

fn request() -> DesignRequest {
    DesignRequest {
        prompt: BRIEF.into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 3,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
        reference_skeleton: None,
    }
}

fn blank_sink() -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.active_children_mut().clear();
    sink
}

fn jobs(count: usize) -> Vec<VariantJob> {
    let plans = choose_variant_style_guides(BRIEF, None, count);
    variant_jobs(&request(), &plans, &blank_sink().state)
}

/// A direction's private document after a "run": one or two phone
/// screens plus its own palette.
fn finished_state(mut state: EditorState, index: usize, screens: usize) -> EditorState {
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(
        "accent".to_string(),
        serde_json::from_value(json!({"type": "color", "value": format!("#00000{index}")}))
            .unwrap(),
    );
    state.apply(EditorCommand::SetVariables {
        variables: vars,
        replace: true,
    });
    let roots: Vec<PenNode> = (0..screens)
        .map(|screen| {
            serde_json::from_value(json!({
                "type": "frame", "id": format!("s{screen}"), "name": format!("Screen {screen}"),
                "x": screen as f64 * 415.0, "y": 0, "width": 375, "height": 812,
                "fill": [{"type": "solid", "color": "$accent"}],
                "children": [{"type": "text", "id": format!("t{screen}"), "content": "Hi"}]
            }))
            .unwrap()
        })
        .collect();
    state.apply(EditorCommand::InsertSubtree {
        nodes: roots,
        parent_id: NodeId::NONE,
        page_id: None,
    });
    state
}

fn summary(nodes: usize) -> RunSummary {
    RunSummary {
        root_frame_id: "s0".into(),
        subtasks: Vec::new(),
        total_nodes: nodes,
        unfilled_screens: Vec::new(),
        incomplete_subtask_failure: false,
    }
}

#[test]
fn directions_run_concurrently_and_land_in_slot_order() {
    let in_flight = Cell::new(0usize);
    let peak = Cell::new(0usize);
    let mut finished = Vec::new();
    let mut events = Vec::new();
    let mut sink = blank_sink();
    let result = block_on(run_variants_with(
        jobs(3),
        &mut sink,
        &mut |event| events.push(event),
        |job, tap| {
            let in_flight = &in_flight;
            let peak = &peak;
            async move {
                in_flight.set(in_flight.get() + 1);
                peak.set(peak.get().max(in_flight.get()));
                tap.emit(Progress::SubtaskStarted {
                    id: "hero".into(),
                    label: "Hero".into(),
                });
                // A is the slowest, C the fastest: completion order is
                // C, B, A — the reverse of slot order.
                Latency {
                    polls: 30 - 10 * job.plan.index,
                }
                .await;
                in_flight.set(in_flight.get() - 1);
                let screens = if job.plan.index == 0 { 2 } else { 1 };
                Ok((
                    finished_state(job.state, job.plan.index, screens),
                    summary(3),
                ))
            }
        },
    ));
    for event in &events {
        if let Progress::VariantReady(variant) = event {
            finished.push(variant.index);
        }
    }
    let summary = result.expect("three directions landed");
    assert_eq!(
        peak.get(),
        3,
        "all three directions must be in flight at once"
    );
    assert_eq!(finished, vec![2, 1, 0], "they land as they finish");
    assert_eq!(summary.total_nodes, 9);

    // Four boards (A has two screens), re-flowed A, B, C left to right.
    let roots = sink.state.active_children();
    assert_eq!(roots.len(), 4);
    let mut placed: Vec<(f64, String)> = roots
        .iter()
        .map(|r| (r.base().x.unwrap(), r.base().name.clone().unwrap()))
        .collect();
    placed.sort_by(|a, b| a.0.total_cmp(&b.0));
    let names: Vec<&str> = placed.iter().map(|(_, n)| n.as_str()).collect();
    assert!(names[0].starts_with("Direction A · ") && names[0].ends_with("Screen 0"));
    assert!(names[1].starts_with("Direction A · ") && names[1].ends_with("Screen 1"));
    assert!(names[2].starts_with("Direction B · "));
    assert!(names[3].starts_with("Direction C · "));
    assert_eq!(placed[0].0, 0.0);
    assert_eq!(placed[1].0, 415.0, "a direction keeps its own layout");
    assert_eq!(placed[2].0, 790.0 + VARIANT_GAP);

    // Each direction kept its own colour even though they share a page.
    let fills: Vec<&str> = placed
        .iter()
        .map(|(x, _)| {
            let root = roots.iter().find(|r| r.base().x == Some(*x)).unwrap();
            op_editor_core::fills::first_solid_fill_hex(root).unwrap()
        })
        .collect();
    assert_eq!(fills, vec!["#000000", "#000000", "#000001", "#000002"]);

    // Progress was scoped per direction with prefixed row ids.
    assert!(events.iter().any(|event| matches!(
        event,
        Progress::WorkerScoped(worker)
            if worker.group_idx == 1
                && matches!(worker.event.as_ref(), Progress::SubtaskStarted { id, .. } if id == "B-hero")
    )));
}

#[test]
fn one_failing_direction_does_not_sink_the_others() {
    let mut events = Vec::new();
    let mut sink = blank_sink();
    let result = block_on(run_variants_with(
        jobs(3),
        &mut sink,
        &mut |event| events.push(event),
        |job, _tap| async move {
            Latency { polls: 2 }.await;
            if job.plan.index == 1 {
                return Err(OrchestratorError::AllFailed("provider quota".into()));
            }
            Ok((finished_state(job.state, job.plan.index, 1), summary(2)))
        },
    ));
    assert!(result.is_ok());
    let ready: Vec<usize> = events
        .iter()
        .filter_map(|event| match event {
            Progress::VariantReady(variant) => Some(variant.index),
            _ => None,
        })
        .collect();
    assert_eq!(ready.len(), 2);
    assert!(!ready.contains(&1));
    let failed: Vec<(usize, &str)> = events
        .iter()
        .filter_map(|event| match event {
            Progress::VariantFailed { index, error, .. } => Some((*index, error.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].0, 1);
    assert!(failed[0].1.contains("provider quota"));
    assert_eq!(sink.state.active_children().len(), 2);
}

#[test]
fn a_direction_with_no_boards_counts_as_failed() {
    let mut events = Vec::new();
    let mut sink = blank_sink();
    let result = block_on(run_variants_with(
        jobs(2),
        &mut sink,
        &mut |event| events.push(event),
        |job, _tap| async move {
            let screens = if job.plan.index == 0 { 0 } else { 1 };
            Ok((
                finished_state(job.state, job.plan.index, screens),
                summary(1),
            ))
        },
    ));
    assert!(result.is_ok());
    assert!(events
        .iter()
        .any(|event| matches!(event, Progress::VariantFailed { index: 0, .. })));
    // The document palette follows the first direction that landed (B).
    let accent = sink
        .state
        .doc
        .variables
        .as_ref()
        .and_then(|vars| vars.get("accent"))
        .cloned();
    assert_eq!(
        accent,
        Some(serde_json::from_value(json!({"type": "color", "value": "#000001"})).unwrap())
    );
}

#[test]
fn every_direction_failing_fails_the_run() {
    let mut sink = blank_sink();
    let result = block_on(run_variants_with(
        jobs(2),
        &mut sink,
        &mut |_| {},
        |_job, _tap| async move { Err(OrchestratorError::NoContent) },
    ));
    assert!(matches!(result, Err(OrchestratorError::AllFailed(_))));
    assert!(sink.state.active_children().is_empty());

    let aborted = block_on(run_variants_with(
        jobs(2),
        &mut sink,
        &mut |_| {},
        |_job, _tap| async move { Err(OrchestratorError::Aborted) },
    ));
    assert!(matches!(aborted, Err(OrchestratorError::Aborted)));
}

#[test]
fn jobs_start_from_an_empty_page_with_their_own_pin() {
    let mut base = blank_sink().state;
    base.apply(EditorCommand::InsertSubtree {
        nodes: vec![serde_json::from_value(json!({
            "type": "frame", "id": "old", "name": "Old", "width": 10, "height": 10,
            "children": [{"type": "text", "id": "t", "content": "x"}]
        }))
        .unwrap()],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let plans = choose_variant_style_guides(BRIEF, None, 2);
    let jobs = variant_jobs(&request(), &plans, &base);
    assert_eq!(jobs.len(), 2);
    for (job, plan) in jobs.iter().zip(&plans) {
        assert!(job.state.active_children().is_empty());
        assert_eq!(
            job.request.pinned_style_guide.as_deref(),
            Some(plan.style_guide.as_str())
        );
    }
}

#[test]
fn merged_outcomes_are_addressed_per_direction_and_not_row_retryable() {
    let mut sink = blank_sink();
    let result = block_on(run_variants_with(
        jobs(2),
        &mut sink,
        &mut |_| {},
        |job, _tap| async move {
            let mut summary = summary(1);
            summary.subtasks.push(crate::types::SubtaskOutcome {
                id: "hero".into(),
                node_count: 0,
                error: Some("timeout".into()),
                inserted_root_ids: vec!["private-1".into()],
                headline: None,
                subtask: None,
            });
            Ok((finished_state(job.state, job.plan.index, 1), summary))
        },
    ))
    .expect("both directions landed");
    let ids: Vec<&str> = result.subtasks.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, vec!["A-hero", "B-hero"]);
    assert!(result
        .subtasks
        .iter()
        .all(|o| o.subtask.is_none() && o.inserted_root_ids.is_empty()));
}
