//! Deterministic backstop for the plan-coverage gate.
//!
//! The gate's one re-plan is a request, not a guarantee: arena-m02 (0925)
//! re-planned and still shipped three subtasks for a five-section brief — the
//! goal ring and the six-card course rail simply vanished. When the re-plan
//! (or a failed re-plan) still leaves brief-required sections unplanned, each
//! one is appended as its own subtask, placed where the brief put it.
//!
//! The append is guarded so a matcher miss never duplicates a section the
//! plan really has (see [`append_missing_sections`]).

use crate::plan::{OrchestratorPlan, Region, Subtask};
use crate::plan_coverage::{check_coverage, is_han, CoverageCheck};

/// Region height for an appended section on a phone-width root.
const MOBILE_SECTION_HEIGHT: f64 = 200.0;
/// Region height for an appended section on a wider root.
const WIDE_SECTION_HEIGHT: f64 = 400.0;
/// Widest root that still counts as a phone for the default height.
const MOBILE_ROOT_MAX_WIDTH: f64 = 480.0;

/// Why a missing section was not appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipReason {
    /// The plan groups subtasks by screen; a free-floating section cannot be
    /// assigned to the right screen safely.
    MultiScreen,
    /// More sections look missing than covered — the matcher is failing on
    /// this plan's wording (e.g. English labels for a Chinese brief), not the
    /// planner dropping sections. Appending would duplicate real sections.
    MatcherUnreliable,
    /// A CJK section against a plan whose labels carry no CJK at all: text
    /// evidence cannot tell "missing" from "translated".
    ScriptMismatch,
    /// Status bars are injected by the scaffold, never planned.
    StatusBar,
}

impl SkipReason {
    fn tag(self) -> &'static str {
        match self {
            SkipReason::MultiScreen => "multi-screen",
            SkipReason::MatcherUnreliable => "matcher-unreliable",
            SkipReason::ScriptMismatch => "script-mismatch",
            SkipReason::StatusBar => "status-bar",
        }
    }
}

/// Outcome of one append pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AppendOutcome {
    pub appended: Vec<String>,
    pub skipped: Vec<(String, SkipReason)>,
}

/// Append one subtask per brief-required section the plan still misses.
///
/// Guards, in order:
/// - multi-screen plans are left alone (screen assignment is ambiguous);
/// - when fewer sections are covered than missing, the verdict is treated as
///   a matcher failure and nothing is appended;
/// - a CJK section is only appended when some subtask label carries CJK;
/// - status-bar sections are never appended.
///
/// Each appended subtask is inserted before the subtask covering the next
/// required section in brief order, else before a trailing bottom nav /
/// footer, else at the end — so the brief's reading order survives.
pub(crate) fn append_missing_sections(
    plan: &mut OrchestratorPlan,
    required: &[String],
) -> AppendOutcome {
    let check = check_coverage(required, plan);
    let mut outcome = AppendOutcome::default();
    if check.missing.is_empty() {
        return outcome;
    }
    let global_skip = if plan.subtasks.iter().any(|st| st.screen.is_some()) {
        Some(SkipReason::MultiScreen)
    } else if check.covered_by.len() < check.missing.len() {
        Some(SkipReason::MatcherUnreliable)
    } else {
        None
    };
    if let Some(reason) = global_skip {
        outcome.skipped = check
            .missing
            .iter()
            .map(|section| (section.clone(), reason))
            .collect();
        return outcome;
    }
    let labels_have_han = plan.subtasks.iter().any(|st| st.label.chars().any(is_han));
    for section in &check.missing {
        if is_status_bar_section(section) {
            outcome
                .skipped
                .push((section.clone(), SkipReason::StatusBar));
            continue;
        }
        if section.chars().any(is_han) && !labels_have_han {
            outcome
                .skipped
                .push((section.clone(), SkipReason::ScriptMismatch));
            continue;
        }
        let index = insertion_index(plan, required, &check, section);
        let subtask = section_subtask(plan, section);
        plan.subtasks.insert(index, subtask);
        outcome.appended.push(section.clone());
    }
    outcome
}

/// One-line diagnostic for the `[PLAN]` log, `None` when nothing happened.
pub(crate) fn outcome_log_line(outcome: &AppendOutcome) -> Option<String> {
    if outcome.appended.is_empty() && outcome.skipped.is_empty() {
        return None;
    }
    let skipped = outcome
        .skipped
        .iter()
        .map(|(section, reason)| format!("{section}({})", reason.tag()))
        .collect::<Vec<_>>();
    Some(format!(
        "[PLAN] coverage: appended [{}] skipped [{}]",
        outcome.appended.join(", "),
        skipped.join(", ")
    ))
}

/// Sections that were covered before normalization but are missing after it
/// (a normalize pass folded or dropped the subtask that covered them). Status
/// bars are excluded: normalization strips them by design.
pub(crate) fn dropped_by_normalize(
    before: &CoverageCheck,
    required: &[String],
    plan: &OrchestratorPlan,
) -> Vec<String> {
    let after = check_coverage(required, plan);
    after
        .missing
        .into_iter()
        .filter(|section| !before.missing.contains(section) && !is_status_bar_section(section))
        .collect()
}

fn is_status_bar_section(section: &str) -> bool {
    section.contains("状态栏") || section.to_lowercase().contains("status bar")
}

fn insertion_index(
    plan: &OrchestratorPlan,
    required: &[String],
    check: &CoverageCheck,
    section: &str,
) -> usize {
    let position = required.iter().position(|r| r == section).unwrap_or(0);
    for later in required.iter().skip(position + 1) {
        let Some((_, id, _)) = check.covered_by.iter().find(|(s, _, _)| s == later) else {
            continue;
        };
        if let Some(index) = plan.subtasks.iter().position(|st| &st.id == id) {
            return index;
        }
    }
    match plan.subtasks.last() {
        Some(last) if is_trailing_chrome(last) => plan.subtasks.len() - 1,
        _ => plan.subtasks.len(),
    }
}

fn is_trailing_chrome(st: &Subtask) -> bool {
    let hay = format!("{} {}", st.id, st.label).to_lowercase();
    [
        "底部导航",
        "底栏",
        "bottom nav",
        "bottom-nav",
        "tab bar",
        "tab-bar",
        "footer",
        "页脚",
    ]
    .iter()
    .any(|cue| hay.contains(cue))
}

fn section_subtask(plan: &OrchestratorPlan, section: &str) -> Subtask {
    let mut n = 1;
    let id = loop {
        let candidate = format!("brief-section-{n}");
        if !plan.subtasks.iter().any(|st| st.id == candidate) {
            break candidate;
        }
        n += 1;
    };
    let width = plan.root_frame.width;
    let height = if width <= MOBILE_ROOT_MAX_WIDTH {
        MOBILE_SECTION_HEIGHT
    } else {
        WIDE_SECTION_HEIGHT
    };
    Subtask {
        id: id.clone(),
        label: section.to_string(),
        region: Region { width, height },
        bleed_hero: false,
        id_prefix: id,
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: Some(format!(
            "the brief explicitly requires this section: {section} — build exactly what it names, with every item and count the brief gives for it"
        )),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        covers: Some(vec![section.to_string()]),
        retry_feedback: None,
    }
}

#[cfg(test)]
#[path = "plan_coverage_append_tests.rs"]
mod tests;
