//! Generation history — port of `memory/generation-history.ts`.
//!
//! Records past generations so the `generation` phase can inject a
//! `{{recentHistory}}` block and steer the model away from repeating
//! itself (anti-slop feedback loop).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::{Feedback, HistoryEntry, HistoryInput, HistoryOutput, Phase};

/// Monotonic suffix counter for generated entry ids (TS `idCounter`).
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Parameters for [`create_history_entry`] — mirrors the TS params
/// object.
#[derive(Debug, Clone)]
pub struct HistoryEntryParams {
    pub document_path: String,
    pub prompt: String,
    pub phase: Phase,
    pub skills_used: Vec<String>,
    pub node_count: u32,
    pub section_types: Vec<String>,
    pub validation_score: Option<u32>,
    pub validation_rounds: Option<u32>,
    pub heading_font: Option<String>,
    pub palette: Option<String>,
    pub creative_variant: Option<String>,
}

/// Build a history entry. `now_ms` seeds the id (`gen-{now_ms}-{n}`)
/// and `now_iso` is the recorded ISO-8601 timestamp.
pub fn create_history_entry(
    params: HistoryEntryParams,
    now_ms: u64,
    now_iso: &str,
) -> HistoryEntry {
    let n = ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    HistoryEntry {
        id: format!("gen-{now_ms}-{n}"),
        timestamp: now_iso.to_string(),
        document_path: params.document_path,
        input: HistoryInput {
            prompt: params.prompt,
            phase: params.phase,
            skills_used: params.skills_used,
        },
        output: HistoryOutput {
            node_count: params.node_count,
            section_types: params.section_types,
            validation_score: params.validation_score,
            validation_rounds: params.validation_rounds,
            heading_font: params.heading_font,
            palette: params.palette,
            creative_variant: params.creative_variant,
        },
        feedback: None,
    }
}

/// Return a copy of `entry` with `feedback` applied.
pub fn update_feedback(entry: &HistoryEntry, feedback: Feedback) -> HistoryEntry {
    let mut next = entry.clone();
    next.feedback = Some(feedback);
    next
}

/// The most-recent `limit` entries, optionally scoped to one document.
pub fn get_recent_entries(
    entries: &[HistoryEntry],
    limit: usize,
    document_path: Option<&str>,
) -> Vec<HistoryEntry> {
    let filtered: Vec<&HistoryEntry> = match document_path {
        Some(path) => entries.iter().filter(|e| e.document_path == path).collect(),
        None => entries.iter().collect(),
    };
    let start = filtered.len().saturating_sub(limit);
    filtered[start..].iter().map(|&e| e.clone()).collect()
}

/// Render history entries as a prompt-ready markdown block. Returns an
/// empty string when there are no entries.
pub fn history_to_prompt_string(entries: &[HistoryEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut lines = vec!["## Recent Generation History".to_string()];
    for e in entries {
        let score = e
            .output
            .validation_score
            .map(|s| format!(" (score: {s})"))
            .unwrap_or_default();
        let feedback = e
            .feedback
            .map(|f| format!(" [{}]", feedback_label(f)))
            .unwrap_or_default();
        lines.push(format!(
            "- \"{}\" → {} nodes{score}{feedback}",
            e.input.prompt, e.output.node_count
        ));
    }
    lines.join("\n")
}

/// Lowercase wire token for a [`Feedback`] value.
pub fn feedback_label(f: Feedback) -> &'static str {
    match f {
        Feedback::Accepted => "accepted",
        Feedback::Modified => "modified",
        Feedback::Regenerated => "regenerated",
        Feedback::Deleted => "deleted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(doc: &str, prompt: &str) -> HistoryEntry {
        create_history_entry(
            HistoryEntryParams {
                document_path: doc.into(),
                prompt: prompt.into(),
                phase: Phase::Generation,
                skills_used: vec![],
                node_count: 7,
                section_types: vec![],
                validation_score: None,
                validation_rounds: None,
                heading_font: None,
                palette: None,
                creative_variant: None,
            },
            1_000,
            "2026-05-17T00:00:00Z",
        )
    }

    #[test]
    fn create_assigns_unique_ids() {
        let a = entry("/a.op", "one");
        let b = entry("/a.op", "two");
        assert_ne!(a.id, b.id);
        assert!(a.id.starts_with("gen-1000-"));
        assert_eq!(a.timestamp, "2026-05-17T00:00:00Z");
    }

    #[test]
    fn update_feedback_sets_the_field() {
        let e = entry("/a.op", "one");
        assert!(e.feedback.is_none());
        let e = update_feedback(&e, Feedback::Accepted);
        assert_eq!(e.feedback, Some(Feedback::Accepted));
    }

    #[test]
    fn recent_entries_filters_by_document_and_limits() {
        let entries = vec![
            entry("/a.op", "1"),
            entry("/b.op", "2"),
            entry("/a.op", "3"),
            entry("/a.op", "4"),
        ];
        // Scoped to /a.op, last 2.
        let recent = get_recent_entries(&entries, 2, Some("/a.op"));
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].input.prompt, "3");
        assert_eq!(recent[1].input.prompt, "4");
        // Unscoped, last 3 across all documents.
        let recent_all = get_recent_entries(&entries, 3, None);
        assert_eq!(recent_all.len(), 3);
    }

    #[test]
    fn prompt_string_is_empty_for_no_entries() {
        assert_eq!(history_to_prompt_string(&[]), "");
        let s = history_to_prompt_string(&[entry("/a.op", "design a login")]);
        assert!(s.starts_with("## Recent Generation History"));
        assert!(s.contains("\"design a login\" → 7 nodes"));
    }
}
