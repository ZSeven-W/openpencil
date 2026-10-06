//! `group_subtasks_by_screen` tests — faithfully revived from
//! `aca0d3a0^:crates/op-orchestrator/src/concurrent_tests.rs`'s grouping
//! coverage (the concurrency-flavored tests in that file — `effective_
//! concurrency` / `BufferDocSink` / worker fan-out — are deliberately NOT
//! revived; see the module doc's scope note).

use super::*;
use crate::plan::Region;

fn subtask_with_screen(id: &str, screen: Option<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        label: id.into(),
        bleed_hero: false,
        id_prefix: id.into(),
        region: Region {
            width: 1200.0,
            height: 400.0,
        },
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: None,
        screen: screen.map(|s| s.to_string()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

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
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].screen, "login");
    assert_eq!(groups[0].indices, vec![0, 1]);
    assert_eq!(groups[1].screen, "home");
    assert_eq!(groups[1].indices, vec![2]);
}

/// All subtasks have no screen → empty result (no groups) — the
/// zero-screen-label regression lock (spec point 2).
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

/// Every subtask tagged with the SAME screen → exactly one group — the
/// "all-same-tag" regression lock (spec point 2: callers gate multi-root on
/// `groups.len() > 1`, so this must NOT trigger it).
#[test]
fn group_subtasks_all_same_screen_returns_one_group() {
    let subtasks = vec![
        subtask_with_screen("a", Some("home")),
        subtask_with_screen("b", Some("home")),
        subtask_with_screen("c", Some("home")),
    ];
    let groups = group_subtasks_by_screen(&subtasks);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].indices, vec![0, 1, 2]);
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

/// A None screen at the START falls back to first_screen from a LATER subtask.
#[test]
fn group_subtasks_no_screen_at_start_uses_later_first_screen() {
    let subtasks = vec![
        subtask_with_screen("a", None),         // no screen
        subtask_with_screen("b", Some("home")), // first with screen → first_screen="home"
        subtask_with_screen("c", Some("settings")),
    ];
    let groups = group_subtasks_by_screen(&subtasks);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].screen, "home");
    assert!(groups[0].indices.contains(&0));
    assert!(groups[0].indices.contains(&1));
    assert_eq!(groups[1].screen, "settings");
    assert_eq!(groups[1].indices, vec![2]);
}
