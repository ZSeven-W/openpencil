//! Derived hero-bleed intent for normalized plan subtasks.

use crate::plan::{OrchestratorPlan, Subtask};
use std::collections::HashSet;

/// Mark the first image-led/full-bleed subtask in each screen group.
///
/// The marker is derived after all plan rewrites so it cannot be supplied by
/// or duplicated by the planning model. Untagged subtasks follow the same
/// first-screen fallback as [`crate::screen_groups::group_subtasks_by_screen`].
pub(super) fn mark_bleed_hero_subtasks(plan: &mut OrchestratorPlan) {
    let first_screen = plan
        .subtasks
        .iter()
        .find_map(|subtask| subtask.screen.clone())
        .unwrap_or_else(|| "page".to_string());
    let has_any_screen = plan.subtasks.iter().any(|subtask| subtask.screen.is_some());
    let mut marked_screens = HashSet::new();

    for subtask in &mut plan.subtasks {
        subtask.bleed_hero = false;
        let screen = if has_any_screen {
            subtask.screen.as_deref().unwrap_or(&first_screen)
        } else {
            "page"
        };
        if !marked_screens.insert(screen.to_string()) {
            continue;
        }
        if is_bleed_hero_candidate(subtask) {
            subtask.bleed_hero = true;
        } else {
            marked_screens.remove(screen);
        }
    }
}

fn is_bleed_hero_candidate(subtask: &Subtask) -> bool {
    let Some(elements) = subtask.elements.as_deref() else {
        return false;
    };
    let normalized = elements.trim_start().to_ascii_lowercase();
    normalized.starts_with("archetype: image-led")
        || normalized.starts_with("archetype: route-map")
        || normalized.contains("edge-to-edge")
        || normalized.contains("full-bleed")
}

#[cfg(test)]
#[path = "plan_normalize_hero_tests.rs"]
mod tests;
