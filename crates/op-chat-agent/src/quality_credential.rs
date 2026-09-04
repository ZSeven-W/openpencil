//! User-facing rendering of the deterministic quality passes' tally — the
//! "trust receipt" a finished generation shows instead of leaving every
//! auto-repair in a log line.
//!
//! Both generation paths render through here so the sentence is identical:
//! the built-in agent loop (`chat_agent_loop::finalize_and_report`, from the
//! executor's [`QualitySummary`]) and the orchestrator
//! (`Progress::QualityChecked`, from `op_orchestrator::RepairSummary` via
//! [`quality_summary_from_repairs`]).
//!
//! Honesty contract, in the order it bites:
//!
//! 1. **No checks, no credential.** An empty summary means the passes never
//!    ran (plain chat turn, executor without a live document, a host build
//!    that doesn't report). [`quality_credential_line`] returns `None` — it
//!    never degrades into "0 problems found", which would vouch for work
//!    nobody did.
//! 2. **Only what ran gets listed.** The check names come from the categories
//!    that actually reached a checkpoint, never from a hardcoded roster.
//! 3. **The number is what was counted.** One repair = one document edit a
//!    quality pass applied (see `op_orchestrator::repair_summary`), so the
//!    wording says "auto-repair(s) applied", not "problems found".
//! 4. **Leftovers are stated, not buried.** When the caller knows how many
//!    issues are still open (the loop does: unfilled screens + unresolved
//!    blockers), the line says so plainly. When it genuinely does not yet
//!    know (the orchestrator, whose promise-delivery check runs later), the
//!    clause is omitted rather than optimistically filled in with "none".
//!
//! Text is English and hardcoded, matching every other progress/report line
//! on these paths (`report_unfilled_if_any`, `report_blockers_if_any`,
//! `web_chat_standard::progress_label`); `op-i18n` is not involved in this
//! family of diagnostic lines today.

use op_ai::chat_provider::QualitySummary;

/// Convert the orchestrator's in-crate tally into the transport-free wire
/// shape the renderer takes. Keeps the display order the summary already
/// guarantees.
pub fn quality_summary_from_repairs(summary: &op_orchestrator::RepairSummary) -> QualitySummary {
    QualitySummary {
        checks: summary
            .checked()
            .into_iter()
            .map(|c| c.key().to_string())
            .collect(),
        repairs: summary
            .repaired()
            .into_iter()
            .map(|(check, count)| (check.key().to_string(), count))
            .collect(),
        records: summary
            .records()
            .iter()
            .map(op_orchestrator::RepairRecord::line)
            .collect(),
        notes: summary.notes().to_vec(),
    }
}

/// How many repair lines a caller shows inline before summarizing the rest.
/// A routine run applies dozens; the ceiling keeps one turn from burying the
/// transcript while the log keeps the complete list.
pub const MAX_INLINE_REPAIR_RECORDS: usize = 30;

/// The itemized repair lines, capped at [`MAX_INLINE_REPAIR_RECORDS`].
///
/// Returns the lines only — no header, no "and N more" notice: the callers
/// that have a locale (the desktop progress panel) localize those around
/// this list, and the ones that do not (the diagnostic transcript lines)
/// render them with their own English framing. Empty when the summary
/// carries no records, so a caller can skip the whole block.
pub fn quality_repair_detail_lines(quality: &QualitySummary) -> Vec<String> {
    quality
        .records
        .iter()
        .take(MAX_INLINE_REPAIR_RECORDS)
        .cloned()
        .collect()
}

/// The non-edit statements about the run, rendered ahead of the repair list.
///
/// Uncapped on purpose: notes are decisions, not edits — the orchestrator
/// de-duplicates them, so a run carries 0 or 1 today and would carry a
/// handful at worst. Truncating a "a whole tier of passes did not run" line
/// would hide exactly the fact it exists to surface.
pub fn quality_note_lines(quality: &QualitySummary) -> Vec<String> {
    quality.notes.clone()
}

/// How many records the cap left out — `0` when everything is shown.
pub fn quality_repair_overflow(quality: &QualitySummary) -> usize {
    quality
        .records
        .len()
        .saturating_sub(MAX_INLINE_REPAIR_RECORDS)
}

/// Render the credential, or `None` when nothing was checked.
///
/// `remaining` is how many issues are still open after the passes ran:
/// `Some(0)` earns the "no issues left" clause, `Some(n)` states the count,
/// and `None` omits the clause entirely for a caller that cannot yet know.
///
/// The returned string is a transcript fragment — it carries its own leading
/// blank line, like the other `• …` report lines it is appended after.
pub fn quality_credential_line(
    quality: &QualitySummary,
    remaining: Option<usize>,
) -> Option<String> {
    if !quality.ran() {
        return None;
    }
    let repairs = quality.total_repairs();
    let repair_clause = if repairs == 0 {
        "nothing needed fixing".to_string()
    } else {
        format!("{repairs} auto-repair(s) applied")
    };
    let remaining_clause = match remaining {
        None => String::new(),
        Some(0) => ", no issues left".to_string(),
        Some(n) => format!(", {n} issue(s) still open"),
    };
    let mut line = format!(
        "\n\n• Checked {} — {repair_clause}{remaining_clause}",
        quality.checks.join(", ")
    );
    if repairs > 0 {
        let breakdown: Vec<String> = quality
            .repairs
            .iter()
            .map(|(check, count)| format!("{check} {count}"))
            .collect();
        line.push_str(&format!("\n  ▸ repairs: {}", breakdown.join(", ")));
    }
    Some(line)
}

/// [`quality_credential_line`] plus one `▸` sub-line per applied repair.
///
/// For the callers whose ONLY channel to the user is this text: the agentic
/// loop's transcript stream and the web host's thinking stream. Both render
/// `▸` sub-lines as the owning step's expandable detail
/// (`split_design_progress`), so the itemized list lands under the credential
/// instead of beside it. Hosts that can attach detail to a structured
/// progress row (the desktop panel) use the plain line and pass
/// [`quality_repair_detail_lines`] to that row instead, so nothing is shown
/// twice.
pub fn quality_credential_line_with_records(
    quality: &QualitySummary,
    remaining: Option<usize>,
) -> Option<String> {
    let mut line = quality_credential_line(quality, remaining)?;
    // Notes lead: a tier that was deliberately SKIPPED reframes every number
    // above it ("layout 0" means "not checked", not "nothing wrong"), so it
    // must not sit below a list the reader may never scroll through.
    for note in quality_note_lines(quality) {
        line.push_str(&format!("\n  ▸ {note}"));
    }
    for record in quality_repair_detail_lines(quality) {
        line.push_str(&format!("\n  ▸ {record}"));
    }
    let overflow = quality_repair_overflow(quality);
    if overflow > 0 {
        line.push_str(&format!(
            "\n  ▸ … and {overflow} more repair(s) — full list in the log"
        ));
    }
    Some(line)
}

#[cfg(test)]
#[path = "quality_credential_tests.rs"]
mod tests;
