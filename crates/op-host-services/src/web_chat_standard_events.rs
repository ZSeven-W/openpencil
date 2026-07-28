//! SSE event serialization and progress labels for the standard web turn.

use std::io::Write;

use op_ai::chat_provider::{ChatDelta, StopReason};
use op_orchestrator::Progress;

pub(super) fn write_delta_event<W: Write>(out: &mut W, text: &str) -> std::io::Result<()> {
    out.write_all(
        crate::ai_proxy::delta_to_sse(&ChatDelta::TextDelta(text.to_string())).as_bytes(),
    )?;
    out.flush()
}

pub(super) fn write_thinking_event<W: Write>(out: &mut W, text: &str) -> std::io::Result<()> {
    out.write_all(
        crate::ai_proxy::delta_to_sse(&ChatDelta::Thinking(text.to_string())).as_bytes(),
    )?;
    out.flush()
}

pub(super) fn write_agent_identity_event<W: Write>(
    out: &mut W,
    identity: &op_orchestrator::agent_identity::AgentIdentity,
) -> std::io::Result<()> {
    let payload = serde_json::json!({
        "agent": {
            "name": identity.name,
            "color": identity.color,
        }
    });
    out.write_all(format!("data: {payload}\n\n").as_bytes())?;
    out.flush()
}

pub(super) fn web_identity_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64)
        .unwrap_or(0)
}

pub(super) fn write_done_event<W: Write>(out: &mut W) -> std::io::Result<()> {
    out.write_all(
        crate::ai_proxy::delta_to_sse(&ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        })
        .as_bytes(),
    )?;
    out.flush()
}

pub(super) fn write_error_event<W: Write>(out: &mut W, message: &str) -> std::io::Result<()> {
    out.write_all(
        crate::ai_proxy::delta_to_sse(&ChatDelta::Error(message.to_string())).as_bytes(),
    )?;
    out.flush()
}

pub(super) fn progress_label(p: &Progress) -> String {
    match p {
        Progress::Planning => "• Planning…".into(),
        Progress::Planned { subtasks } => {
            format!("• Plan — {} section(s)", subtasks.len())
        }
        Progress::ScaffoldDone => "• Scaffold ready".into(),
        Progress::SubtaskStarted { id, label } => format!("• Subtask `{id}` — {label}"),
        Progress::SubtaskDone { id, node_count } => {
            format!("• Subtask `{id}` done ({node_count} nodes)")
        }
        Progress::SubtaskFailed { id, error } => format!("• Subtask `{id}` failed: {error}"),
        Progress::SubtaskSkills {
            id,
            included,
            dropped,
            budget_used,
            budget_max,
        } => format_subtask_skills(id, included, dropped, *budget_used, *budget_max),
        Progress::SubtaskRetry {
            attempt, reason, ..
        } => {
            format!("  ▸ retry #{attempt}: {reason}")
        }
        Progress::GeometryEcho { issue_count, .. } => {
            format!("  ▸ geometry echo: {issue_count} issue(s) → retry")
        }
        Progress::SubtaskNodes { id, nodes_so_far } => {
            format!("• Subtask `{id}` — {nodes_so_far} node(s) so far")
        }
        Progress::ConcurrentGroupsStarted {
            group_count,
            workers,
        } => format!("• {group_count} screen groups · {workers} workers"),
        Progress::ScreenGroupsSequential {
            group_count,
            requested_workers,
        } => format!(
            "• {group_count} screen groups · sequential (parallel setting: {requested_workers})"
        ),
        Progress::WorkerScoped(worker) => {
            let detail = progress_label(worker.event.as_ref());
            format!(
                "• {} · {} — {}",
                worker.identity.name,
                worker.screen,
                detail.trim_start_matches("• ")
            )
        }
        Progress::CleanupDone => "• Cleanup done".into(),
        // The classic path's quality credential. `remaining` is `None` on
        // purpose: the promise-delivery check runs later in the pipeline, so
        // claiming anything about leftover work here would be a guess.
        Progress::QualityChecked { checks, repairs } => {
            crate::quality_credential::quality_credential_line(
                &op_ai::chat_provider::QualitySummary {
                    checks: checks.clone(),
                    repairs: repairs.clone(),
                },
                None,
            )
            .unwrap_or_default()
            .trim_start()
            .to_string()
        }
        Progress::ValidationStarted => "• Validation started".into(),
        Progress::ValidationPreCheckDone { applied, .. } => {
            format!("• Pre-validation applied {applied} fix(es)")
        }
        Progress::ValidationRoundStarted { round } => format!("• Vision round {round} started"),
        Progress::ValidationRoundDone {
            round,
            applied,
            quality_score,
        } => {
            format!("• Vision round {round} done — {applied} fix(es), quality {quality_score}/100")
        }
        Progress::ValidationDone { total_applied } => {
            format!("• Validation done — {total_applied} fix(es) total")
        }
        Progress::VisualRefStarted => "• Visual-ref pipeline started".into(),
        Progress::VisualRefDesignSystem { var_count } => {
            format!("• Design system ready — {var_count} variable(s) seeded")
        }
        Progress::VisualRefHtmlGenerated { byte_len } => {
            format!("• Visual-ref HTML generated ({byte_len} bytes)")
        }
        Progress::VisualRefScreenshotReady { skipped } => {
            if *skipped {
                "• Visual-ref screenshot skipped".into()
            } else {
                "• Visual-ref screenshot captured".into()
            }
        }
        Progress::VisualRefFallback { reason } => format!("• Visual-ref fallback: {reason}"),
        Progress::UnfilledScreens { names } => {
            format!(
                "• {} screen(s) left unfilled: {}",
                names.len(),
                names.join(", ")
            )
        }
    }
}

/// Format a `SubtaskSkills` payload into the concise summary line plus
/// indented `▸ skills:` / `▸ dropped:` detail sub-lines (spec Component 5).
fn format_subtask_skills(
    id: &str,
    included: &[op_orchestrator::SkillBrief],
    dropped: &[(String, String)],
    budget_used: u32,
    budget_max: u32,
) -> String {
    let mut out = format!(
        "• Subtask `{id}`  ·  {} skills · {budget_used}/{budget_max} tok · {} dropped",
        included.len(),
        dropped.len(),
    );
    if !included.is_empty() {
        let names: Vec<String> = included
            .iter()
            .map(|s| {
                if s.truncated {
                    format!("{} (truncated)", s.name)
                } else {
                    s.name.clone()
                }
            })
            .collect();
        out.push_str(&format!("\n  ▸ skills: {}", names.join(", ")));
    }
    if !dropped.is_empty() {
        let drops: Vec<String> = dropped.iter().map(|(n, r)| format!("{n} ({r})")).collect();
        out.push_str(&format!("\n  ▸ dropped: {}", drops.join(", ")));
    }
    out
}
