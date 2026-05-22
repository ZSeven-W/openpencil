//! S3b-2 Task A1: concurrency decision + screen grouping.
//!
//! Port of `orchestrator.ts:780-810`.
//! - `DesignRequest.concurrency` (added in `types.rs`)
//! - `clamp_concurrency` — defensive [1, 6] clamp
//! - `group_subtasks_by_screen` — group subtasks by screen; faithful to TS L785-801
//! - `effective_concurrency` — concurrency decision; faithful to TS L803-810
//!   (minus append-mode gate, which is S3b-4)
//!
//! Callers land in later S3b-2 tasks; scaffolding symbols are allowed to be unused.
#![allow(dead_code)]

use crate::plan::Subtask;

/// A group of subtask indices that share the same screen.
/// `screen` is the screen name; `indices` are into the plan's `subtasks` slice.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScreenGroup {
    pub screen: String,
    pub indices: Vec<usize>,
}

/// Clamps a raw concurrency value to the valid range `[1, 6]`.
///
/// This mirrors the store-side clamp in TS (store clamps to [1,6] before
/// writing to `request.concurrency`). The Rust crate clamps defensively on
/// the way in so callers need not worry about out-of-range values.
pub(crate) fn clamp_concurrency(v: u32) -> u32 {
    v.clamp(1, 6)
}

/// Groups subtasks by screen, faithfully porting `orchestrator.ts:785-801`.
///
/// Rules:
/// - Only called when `concurrency > 1` (caller's responsibility).
/// - `first_screen` = the `screen` of the first subtask that has one, else `"page"`.
/// - A subtask with no `screen` falls back to `first_screen`.
/// - Group order = first-seen order of distinct screen values.
/// - If no subtask has a `screen`, returns an empty `Vec` (caller treats as
///   single-screen; `effective_concurrency` will return 1 in that case).
pub(crate) fn group_subtasks_by_screen(subtasks: &[Subtask]) -> Vec<ScreenGroup> {
    let has_any_screen = subtasks.iter().any(|st| st.screen.is_some());
    if !has_any_screen {
        return vec![];
    }

    // first_screen = screen of first subtask that has one, else "page".
    let first_screen: String = subtasks
        .iter()
        .find(|st| st.screen.is_some())
        .and_then(|st| st.screen.clone())
        .unwrap_or_else(|| "page".to_string());

    let mut groups: Vec<ScreenGroup> = Vec::new();
    // Map from screen name to index into `groups`.
    let mut screen_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (i, subtask) in subtasks.iter().enumerate() {
        let screen = subtask
            .screen
            .clone()
            .unwrap_or_else(|| first_screen.clone());

        if let Some(&group_idx) = screen_map.get(&screen) {
            groups[group_idx].indices.push(i);
        } else {
            screen_map.insert(screen.clone(), groups.len());
            groups.push(ScreenGroup {
                screen,
                indices: vec![i],
            });
        }
    }

    groups
}

/// Computes the effective concurrency for a run.
///
/// Port of `orchestrator.ts:803-810`, minus the append-mode gate (S3b-4).
///
/// - `screen_group_count > 1` → `clamp_concurrency(concurrency)`.
/// - else → `1`.
pub(crate) fn effective_concurrency(concurrency: u32, screen_group_count: usize) -> u32 {
    if screen_group_count > 1 {
        clamp_concurrency(concurrency)
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Region, Subtask};

    fn subtask_with_screen(id: &str, screen: Option<&str>) -> Subtask {
        Subtask {
            id: id.into(),
            label: id.into(),
            id_prefix: id.into(),
            region: Region {
                width: 1200.0,
                height: 400.0,
            },
            parent_frame_id: None,
            elements: None,
            screen: screen.map(|s| s.to_string()),
        }
    }

    // ── effective_concurrency ──────────────────────────────────────────────────

    /// (concurrency=1, 3 screens) → 1  (single-threaded even with many screens)
    #[test]
    fn effective_concurrency_one_concurrency_three_screens_gives_one() {
        assert_eq!(effective_concurrency(1, 3), 1);
    }

    /// (concurrency=4, 1 screen) → 1  (only one group → sequential)
    #[test]
    fn effective_concurrency_four_concurrency_one_screen_gives_one() {
        assert_eq!(effective_concurrency(4, 1), 1);
    }

    /// (concurrency=4, 3 screens) → 4
    #[test]
    fn effective_concurrency_four_concurrency_three_screens_gives_four() {
        assert_eq!(effective_concurrency(4, 3), 4);
    }

    /// Clamp: (concurrency=99, 3 screens) → 6
    #[test]
    fn effective_concurrency_clamps_to_six() {
        assert_eq!(effective_concurrency(99, 3), 6);
    }

    // ── group_subtasks_by_screen ───────────────────────────────────────────────

    /// Basic grouping: [login, home, login] → 2 groups {login:[0,2], home:[1]}
    #[test]
    fn group_subtasks_three_entries_two_screens() {
        let subtasks = vec![
            subtask_with_screen("a", Some("login")),
            subtask_with_screen("b", Some("home")),
            subtask_with_screen("c", Some("login")),
        ];
        let groups = group_subtasks_by_screen(&subtasks);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].screen, "login");
        assert_eq!(groups[0].indices, vec![0, 2]);
        assert_eq!(groups[1].screen, "home");
        assert_eq!(groups[1].indices, vec![1]);
    }

    /// A subtask with no screen falls back to first_screen.
    #[test]
    fn group_subtasks_no_screen_falls_back_to_first_screen() {
        let subtasks = vec![
            subtask_with_screen("a", Some("login")),
            subtask_with_screen("b", None), // no screen → "login"
            subtask_with_screen("c", Some("home")),
        ];
        let groups = group_subtasks_by_screen(&subtasks);
        // "login" gets index 0 and 1 (b bucketed under first_screen="login")
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].screen, "login");
        assert_eq!(groups[0].indices, vec![0, 1]);
        assert_eq!(groups[1].screen, "home");
        assert_eq!(groups[1].indices, vec![2]);
    }

    /// All subtasks have no screen → empty result (no groups).
    #[test]
    fn group_subtasks_all_no_screen_returns_empty() {
        let subtasks = vec![
            subtask_with_screen("a", None),
            subtask_with_screen("b", None),
        ];
        let groups = group_subtasks_by_screen(&subtasks);
        assert!(groups.is_empty());
    }

    /// Empty subtask slice → empty result.
    #[test]
    fn group_subtasks_empty_slice_returns_empty() {
        let groups = group_subtasks_by_screen(&[]);
        assert!(groups.is_empty());
    }

    /// Group order is first-seen order of distinct screen values.
    #[test]
    fn group_subtasks_preserves_first_seen_order() {
        let subtasks = vec![
            subtask_with_screen("a", Some("profile")),
            subtask_with_screen("b", Some("settings")),
            subtask_with_screen("c", Some("profile")),
            subtask_with_screen("d", Some("dashboard")),
        ];
        let groups = group_subtasks_by_screen(&subtasks);
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].screen, "profile");
        assert_eq!(groups[1].screen, "settings");
        assert_eq!(groups[2].screen, "dashboard");
    }

    /// first_screen fallback is "page" when no subtask has a screen
    /// (but `has_any_screen` is false, so returns empty — tested separately).
    /// Here we test that a None screen at the START falls back to first_screen
    /// from a LATER subtask — i.e., first_screen is derived from the first
    /// subtask that HAS a screen.
    #[test]
    fn group_subtasks_no_screen_at_start_uses_later_first_screen() {
        let subtasks = vec![
            subtask_with_screen("a", None),         // no screen
            subtask_with_screen("b", Some("home")), // first with screen → first_screen="home"
            subtask_with_screen("c", Some("settings")),
        ];
        let groups = group_subtasks_by_screen(&subtasks);
        // "home" first (b has screen), but "a" has no screen → bucketed under first_screen="home"
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].screen, "home");
        // "a" (index 0) bucketed under "home" — it's first because it's processed first
        // and maps to first_screen="home" which is first seen when processing "a".
        assert!(groups[0].indices.contains(&0));
        assert!(groups[0].indices.contains(&1));
        assert_eq!(groups[1].screen, "settings");
        assert_eq!(groups[1].indices, vec![2]);
    }
}
