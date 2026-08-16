//! Best-of-N candidate selection for smoke runs.
//!
//! `OPENPENCIL_SMOKE_BEST_OF=N` (2..=4) runs the full generation N times
//! against independent fresh states, scores every candidate with the
//! real-layout geometry diagnostics, and keeps the best one. Selection
//! rule: issueCount ascending → nodeCount descending → first to arrive.
//! A candidate whose generation errors scores (9999, 0) so it loses to
//! any candidate that produced a document, without aborting the rest.

use std::path::Path;

use op_editor_core::EditorState;
use op_orchestrator::{AbortFlag, DesignRequest, LlmClient, Orchestrator, ValidationProviders};

use crate::InlineDocSink;

/// Sentinel score for a candidate whose generation failed outright.
const FAILED_ISSUES: usize = 9999;

/// Parse `OPENPENCIL_SMOKE_BEST_OF`. Anything outside 2..=4 (including
/// unset or unparsable) collapses to 1 — the plain single-run path.
pub fn parse_best_of_count() -> usize {
    std::env::var("OPENPENCIL_SMOKE_BEST_OF")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| (2..=4).contains(n))
        .unwrap_or(1)
}

/// Score a single candidate by (issueCount, nodeCount) tuple.
/// Returns (chosen_index, scores) where chosen_index is the best candidate.
///
/// Selection rule: issueCount ascending → nodeCount descending → first to arrive.
pub fn select_best_candidate(scores: &[(usize, usize)]) -> (usize, &[(usize, usize)]) {
    if scores.is_empty() {
        return (0, scores);
    }

    let chosen = scores
        .iter()
        .enumerate()
        .min_by(|(i, (issues_i, nodes_i)), (j, (issues_j, nodes_j))| {
            use std::cmp::Ordering;
            // Primary: fewer issues wins
            match issues_i.cmp(issues_j) {
                Ordering::Equal => {
                    // Tie-breaker: more nodes wins
                    match nodes_j.cmp(nodes_i) {
                        Ordering::Equal => {
                            // Second tie-breaker: first to arrive wins (index ascending)
                            i.cmp(j)
                        }
                        other => other,
                    }
                }
                other => other,
            }
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    (chosen, scores)
}

/// Save a candidate to disk with a numbered suffix.
///
/// `out_path` like `/tmp/out.op` → `/tmp/out.cand0.op`.
pub fn save_candidate(
    candidate_index: usize,
    doc_json: &str,
    out_path: &str,
) -> std::io::Result<()> {
    let path = Path::new(out_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("op");

    let candidate_path = parent.join(format!("{}.cand{}.{}", stem, candidate_index, ext));
    std::fs::write(candidate_path, doc_json)?;
    Ok(())
}

/// The best-of-N driver: N independent generations from clones of the
/// prepared base state (post seed/library merge), one score line per
/// candidate, best document written to `OPENPENCIL_SMOKE_OUT`.
pub async fn run_best_of(
    n: usize,
    template: &DesignRequest,
    base_state: &EditorState,
    llm: &dyn LlmClient,
    abort: &AbortFlag,
    providers: &ValidationProviders<'_>,
) -> std::process::ExitCode {
    let out_path = std::env::var("OPENPENCIL_SMOKE_OUT")
        .ok()
        .filter(|s| !s.is_empty());
    let mut scores: Vec<(usize, usize)> = Vec::new();
    let mut jsons: Vec<Option<String>> = Vec::new();

    for i in 0..n {
        let mut sink = InlineDocSink {
            state: base_state.clone(),
        };
        let request = DesignRequest {
            prompt: template.prompt.clone(),
            design_md: sink.state.doc.design_md.clone(),
            ..template.clone()
        };
        let mut on_progress = |p: op_orchestrator::Progress| {
            eprintln!("[PROGRESS] cand={i} {p:?}");
        };
        let started = std::time::Instant::now();
        let result = Orchestrator::new()
            .run(request, &mut sink, llm, &mut on_progress, abort, providers)
            .await;
        let elapsed = started.elapsed();
        match result {
            Ok(summary) => {
                let issues =
                    op_orchestrator::geometry_validation::geometry_diagnostics(&sink.state).len();
                let nodes = summary.total_nodes;
                eprintln!("[BESTOF] cand={i} issues={issues} nodes={nodes} elapsed={elapsed:?}");
                scores.push((issues, nodes));
                jsons.push(serde_json::to_string_pretty(&sink.state.doc).ok());
            }
            Err(e) => {
                eprintln!(
                    "[BESTOF] cand={i} issues={FAILED_ISSUES} nodes=0 elapsed={elapsed:?} \
                     error={e}"
                );
                scores.push((FAILED_ISSUES, 0));
                jsons.push(None);
            }
        }
        if let (Some(out), Some(json)) = (out_path.as_deref(), jsons[i].as_deref()) {
            if let Err(e) = save_candidate(i, json, out) {
                eprintln!("[BESTOF] cand={i} save failed: {e}");
            }
        }
    }

    let (chosen, _) = select_best_candidate(&scores);
    eprintln!("[BESTOF] chosen={chosen}");
    if scores.iter().all(|(issues, _)| *issues == FAILED_ISSUES) {
        eprintln!("[BESTOF] all {n} candidates failed");
        return std::process::ExitCode::from(1);
    }
    match (out_path.as_deref(), jsons[chosen].as_deref()) {
        (Some(out), Some(json)) => match std::fs::write(out, json) {
            Ok(()) => {
                eprintln!("[BESTOF] saved chosen doc → {out}");
                std::process::ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("[BESTOF] save failed ({out}): {e}");
                std::process::ExitCode::from(4)
            }
        },
        (Some(out), None) => {
            eprintln!("[BESTOF] chosen candidate has no serialized doc ({out} not written)");
            std::process::ExitCode::from(4)
        }
        (None, _) => std::process::ExitCode::SUCCESS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_best_fewest_issues() {
        let scores = vec![(5, 100), (2, 95), (3, 110)];
        let (chosen, _) = select_best_candidate(&scores);
        assert_eq!(chosen, 1, "Index 1 has fewest issues (2)");
    }

    #[test]
    fn select_best_tiebreak_by_nodes() {
        let scores = vec![(3, 100), (3, 95), (3, 110)];
        let (chosen, _) = select_best_candidate(&scores);
        assert_eq!(chosen, 2, "Index 2 has most nodes (110) on issue tie");
    }

    #[test]
    fn select_best_tiebreak_by_arrival() {
        let scores = vec![(3, 105), (3, 105), (3, 105)];
        let (chosen, _) = select_best_candidate(&scores);
        assert_eq!(chosen, 0, "Index 0 arrived first on full tie");
    }

    #[test]
    fn select_best_single_candidate() {
        let scores = vec![(5, 100)];
        let (chosen, _) = select_best_candidate(&scores);
        assert_eq!(chosen, 0, "Only one candidate is always chosen");
    }

    #[test]
    fn failed_candidates_lose_to_any_document() {
        let scores = vec![(FAILED_ISSUES, 0), (7, 12), (FAILED_ISSUES, 0)];
        let (chosen, _) = select_best_candidate(&scores);
        assert_eq!(chosen, 1, "A produced document beats failed candidates");
    }

    #[test]
    fn parse_count_rejects_out_of_range() {
        // Env-free contract check: the parser's range filter is 2..=4.
        for (raw, expect) in [("1", 1usize), ("2", 2), ("4", 4), ("5", 1), ("x", 1)] {
            let parsed = raw
                .parse::<usize>()
                .ok()
                .filter(|n| (2..=4).contains(n))
                .unwrap_or(1);
            assert_eq!(parsed, expect, "raw={raw}");
        }
    }
}
