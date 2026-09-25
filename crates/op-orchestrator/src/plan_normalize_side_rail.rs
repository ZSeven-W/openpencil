//! Fold unsupported side progress rails into the navigation bar.

use crate::design_type::contains_word;
use crate::plan::{OrchestratorPlan, Subtask};

const PROGRESS_BAR_ELEMENTS: &str =
    "4px scroll progress bar pinned under the nav row (width bound to $scroll.progress)";

const PROGRESS_CUES: &[&str] = &["progress", "进度", "scroll indicator", "阅读进度"];
const SIDE_CUES: &[&str] = &[
    "side", "vertical", "right", "left", "rail", "竖向", "纵向", "右侧", "左侧", "侧边",
];

/// Cues that mark a progress subtask as something other than a page-edge
/// scroll rail: a horizontal card rail whose cards carry progress bars, or a
/// goal/progress ring. arena-m02 (0925): `今日目标环形进度` ("circular goal ring
/// … calories left") and a course-card rail ("horizontal rail … progress per
/// card") both matched progress + side (`left` / `rail`) and would have been
/// folded into the bottom nav as a 4px bar — silently deleting two sections
/// the brief asked for.
const NOT_SIDE_RAIL_CUES: &[&str] = &[
    "horizontal",
    "carousel",
    "card",
    "cards",
    "ring",
    "circular",
    "goal",
    "横向",
    "水平",
    "卡片",
    "课程卡",
    "环形",
    "圆环",
    "目标",
];

fn matches_cue(text: &str, cues: &[&str]) -> bool {
    let lower = text.to_lowercase();
    cues.iter().any(|cue| {
        if cue.is_ascii() {
            contains_word(&lower, cue)
        } else {
            lower.contains(cue)
        }
    })
}

fn matches_side_progress_field(text: &str) -> bool {
    matches_cue(text, PROGRESS_CUES) && matches_cue(text, SIDE_CUES)
}

fn is_page_navigation_label(label: &str) -> bool {
    let lower = label.to_lowercase();
    ["nav", "navigation", "navbar", "header"]
        .iter()
        .any(|cue| contains_word(&lower, cue))
        || ["导航", "页头"].iter().any(|cue| lower.contains(cue))
}

fn is_side_progress_candidate(st: &Subtask) -> bool {
    let not_a_side_rail = matches_cue(&st.label, NOT_SIDE_RAIL_CUES)
        || st
            .elements
            .as_deref()
            .is_some_and(|elements| matches_cue(elements, NOT_SIDE_RAIL_CUES));
    if not_a_side_rail {
        return false;
    }
    matches_side_progress_field(&st.label)
        || st
            .elements
            .as_deref()
            .is_some_and(matches_side_progress_field)
}

/// Fold a planned side/vertical progress rail into the Navigation Bar
/// subtask. Returns the number of subtasks removed.
pub(crate) fn fold_side_progress_rail(plan: &mut OrchestratorPlan) -> usize {
    if plan.root_frame.height == 1080.0 || plan.subtasks.len() < 3 {
        return 0;
    }
    // Multi-screen plans keep one nav per screen, so a rail on screen 2 must
    // never fold into screen 1's nav. Folding across screens is worse than
    // leaving the rail alone, and a scroll rail on a multi-screen app is rare.
    let mut screens: Vec<&str> = plan
        .subtasks
        .iter()
        .filter_map(|st| st.screen.as_deref())
        .collect();
    screens.sort_unstable();
    screens.dedup();
    if screens.len() > 1 {
        return 0;
    }

    let navigation_index = plan
        .subtasks
        .iter()
        .position(|st| is_page_navigation_label(&st.label))
        .unwrap_or(0);

    let removed = plan
        .subtasks
        .iter()
        .enumerate()
        .filter(|(index, st)| {
            *index != 0 && *index != navigation_index && is_side_progress_candidate(st)
        })
        .count();

    if removed == 0 {
        return 0;
    }

    let nav_elements = plan.subtasks[navigation_index]
        .elements
        .get_or_insert_with(String::new);
    if !nav_elements.contains(PROGRESS_BAR_ELEMENTS) {
        if !nav_elements.is_empty() {
            nav_elements.push_str(", ");
        }
        nav_elements.push_str(PROGRESS_BAR_ELEMENTS);
    }

    // The nav now draws the folded rail, so it inherits what the rail
    // claimed to cover — otherwise the coverage gate reports a brief
    // section as dropped although its content lives on in the nav.
    let inherited: Vec<String> = plan
        .subtasks
        .iter()
        .enumerate()
        .filter(|(index, st)| {
            *index != 0 && *index != navigation_index && is_side_progress_candidate(st)
        })
        .flat_map(|(_, st)| {
            st.covers
                .iter()
                .flatten()
                .cloned()
                .chain(std::iter::once(st.label.clone()))
        })
        .filter(|entry| !entry.trim().is_empty())
        .collect();
    let nav_covers = plan.subtasks[navigation_index]
        .covers
        .get_or_insert_with(Vec::new);
    for entry in inherited {
        if !nav_covers.contains(&entry) {
            nav_covers.push(entry);
        }
    }

    let subtasks = std::mem::take(&mut plan.subtasks);
    plan.subtasks = subtasks
        .into_iter()
        .enumerate()
        .filter_map(|(index, st)| {
            if index != 0 && index != navigation_index && is_side_progress_candidate(&st) {
                None
            } else {
                Some(st)
            }
        })
        .collect();

    removed
}

#[cfg(test)]
#[path = "plan_normalize_side_rail_tests.rs"]
mod tests;
