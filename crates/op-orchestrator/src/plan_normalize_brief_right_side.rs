//! Carry a brief's "right-side X" onto the subtask that builds X.
//!
//! The desktop shell grows a third column only for a subtask whose id/label
//! names a right-hand surface (`scaffold_right_rail::is_right_rail_subtask`).
//! Planners routinely translate the place away: arena-d03's brief asks for
//! `右侧告警列表五条` and every model planned it as `alerts (Alert List)`, so
//! the list was stacked under the device grid in both the Opus 5.5 and the
//! Sonnet 5.5 runs. The brief is the user's word on placement; this restores
//! it by tagging the one subtask that builds the named region.

use crate::dashboard_columns::is_sidebar_subtask;
use crate::plan::OrchestratorPlan;
use crate::scaffold_right_rail::is_right_rail_subtask;

/// Tag appended to a label so `is_right_rail_subtask` reads it as a rail.
const RIGHT_PANEL_TAG: &str = "(right panel)";
/// The shell only adds a third column on a wide artboard.
const MIN_ROOT_WIDTH: f64 = 1200.0;

const CJK_CUES: &[&str] = &["右侧的", "右边的", "右侧", "右边", "右栏"];
const ASCII_CUES: &[&str] = &["right-side ", "right side ", "right-hand "];

/// Regions the brief places on the right, as written in the brief.
fn brief_right_side_regions(prompt: &str) -> Vec<String> {
    let mut regions = Vec::new();
    for cue in CJK_CUES {
        for (at, _) in prompt.match_indices(cue) {
            let noun: String = prompt[at + cue.len()..]
                .chars()
                .take_while(|ch| is_cjk_noun_char(*ch))
                .collect();
            if noun.chars().count() >= 2 && !regions.contains(&noun) {
                regions.push(noun);
            }
        }
    }
    let lower = prompt.to_lowercase();
    for cue in ASCII_CUES {
        for (at, _) in lower.match_indices(cue) {
            let noun = lower[at + cue.len()..]
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == ' ' || ch == '-'))
                .next()
                .unwrap_or("")
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            if noun.len() >= 3 && !regions.contains(&noun) {
                regions.push(noun);
            }
        }
    }
    regions
}

/// A CJK character that can continue a region noun: a quantity (`五条`) or
/// any punctuation ends it.
fn is_cjk_noun_char(ch: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&ch)
        && !"一二三四五六七八九十两个条项张和与及带含或".contains(ch)
}

/// Tag the subtask that builds each right-side region the brief names.
/// Returns how many subtasks were tagged.
pub(super) fn mark_brief_right_side_subtasks(plan: &mut OrchestratorPlan, prompt: &str) -> usize {
    if plan.root_frame.width < MIN_ROOT_WIDTH || plan.subtasks.iter().any(is_right_rail_subtask) {
        return 0;
    }
    let regions = brief_right_side_regions(prompt);
    for region in &regions {
        let candidates: Vec<usize> = plan
            .subtasks
            .iter()
            .enumerate()
            .filter(|(index, st)| *index > 0 && !is_sidebar_subtask(st))
            .map(|(index, _)| index)
            .collect();
        let names = |index: usize| {
            let st = &plan.subtasks[index];
            format!("{} {}", st.id, st.label).to_lowercase()
        };
        let elements = |index: usize| {
            plan.subtasks[index]
                .elements
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
        };
        // A label naming the region beats a sibling that merely mentions it.
        let by_name: Vec<usize> = candidates
            .iter()
            .copied()
            .filter(|&index| names(index).contains(region.as_str()))
            .collect();
        let matches = if by_name.is_empty() {
            candidates
                .iter()
                .copied()
                .filter(|&index| elements(index).contains(region.as_str()))
                .collect()
        } else {
            by_name
        };
        // Only an unambiguous match moves a section across the page.
        let [index] = matches[..] else {
            continue;
        };
        tag(plan, index);
        return 1;
    }
    // The brief's noun did not survive translation (`告警列表` planned as
    // `Alert List` with `实时告警` elements, arena-d03 Opus 5.5 2026-10-01) but the
    // planner still wrote the placement into the elements ("right panel
    // header …"). One subtask saying so is the region the brief meant.
    if !regions.is_empty() {
        let placed: Vec<usize> = plan
            .subtasks
            .iter()
            .enumerate()
            .filter(|(index, st)| *index > 0 && !is_sidebar_subtask(st))
            .filter(|(_, st)| {
                let elements = st.elements.as_deref().unwrap_or("").to_lowercase();
                ELEMENT_PLACEMENT_CUES
                    .iter()
                    .any(|cue| elements.contains(cue))
            })
            .map(|(index, _)| index)
            .collect();
        if let [index] = placed[..] {
            tag(plan, index);
            return 1;
        }
    }
    0
}

/// Placement phrases a planner writes into a subtask's elements.
const ELEMENT_PLACEMENT_CUES: &[&str] = &[
    "right panel",
    "right rail",
    "right column",
    "right side",
    "right-side",
    "右侧",
    "右栏",
];

fn tag(plan: &mut OrchestratorPlan, index: usize) {
    let st = &mut plan.subtasks[index];
    st.label = format!("{} {RIGHT_PANEL_TAG}", st.label);
}

#[cfg(test)]
#[path = "plan_normalize_brief_right_side_tests.rs"]
mod tests;
