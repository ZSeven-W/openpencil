//! The quality-ledger half of the design-turn progress adapter: what the
//! deterministic passes repaired, and what they deliberately did not run.
//!
//! Split out of `design_session_tests.rs` at the 800-line cap. These share
//! that file's `super::*` surface and drive the same `apply_progress` entry
//! point; they are grouped here because they all assert on ONE thing — the
//! "Polishing the layout" row's expandable detail, which is the only place a
//! user can see which node a pass touched and why.

use super::*;

#[test]
fn quality_records_land_on_the_polish_row_as_expandable_detail() {
    // The user's complaint was that the check stage reported "41 auto-repair(s)
    // applied" and nothing else. The narration keeps the headline; the itemized
    // list rides the "Polishing the layout" row, which the transcript renders
    // as expandable detail.
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    let events = vec![
        Progress::CleanupDone,
        Progress::QualityChecked {
            checks: vec!["layout".into(), "palette".into()],
            repairs: vec![("layout".into(), 2), ("palette".into(), 1)],
            records: vec![
                "layout · table-gap · Pricing Row [n42] · gap 0 → 16".into(),
                "layout · container-geometry · Hero [n7] · padding [32,32] → [16,16]".into(),
                "palette · light-mobile-nav-surface · Tab Bar [n9] · fill #F8FAFC → #FFFFFF".into(),
            ],
            notes: Vec::new(),
            items: Vec::new(),
        },
    ];

    assert!(super::apply_progress(&mut message, &events, Locale::EnUs));

    let polish = message
        .activities
        .iter()
        .find(|activity| activity.id == "__polish")
        .expect("cleanup must have created the polish row");
    let detail = polish.detail.as_deref().expect("itemized detail");
    assert!(
        detail.starts_with("3 auto-repair(s) applied"),
        "the head line states the count in the user's locale: {detail}"
    );
    for record in [
        "gap 0 → 16",
        "padding [32,32] → [16,16]",
        "fill #F8FAFC → #FFFFFF",
    ] {
        assert!(
            detail.contains(record),
            "`{record}` must be listed on the row: {detail}"
        );
    }
    assert!(
        message.content.contains("41 auto-repair(s)")
            || message.content.contains("3 auto-repair(s)"),
        "the narration keeps the headline credential: {}",
        message.content
    );
    assert_eq!(
        message.content.matches("gap 0 → 16").count(),
        0,
        "the itemized list must not also be dumped into the narration"
    );
}

#[test]
fn a_long_repair_list_is_capped_on_the_row_with_a_localized_remainder_notice() {
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    let records: Vec<String> = (0..45)
        .map(|i| format!("layout · container-geometry · Card {i} [n{i}] · gap 24 → 16"))
        .collect();
    let events = vec![
        Progress::CleanupDone,
        Progress::QualityChecked {
            checks: vec!["layout".into()],
            repairs: vec![("layout".into(), 45)],
            records,
            notes: Vec::new(),
            items: Vec::new(),
        },
    ];

    assert!(super::apply_progress(&mut message, &events, Locale::ZhCn));

    let detail = message
        .activities
        .iter()
        .find(|activity| activity.id == "__polish")
        .and_then(|activity| activity.detail.clone())
        .expect("itemized detail");
    let lines: Vec<&str> = detail.lines().collect();
    assert_eq!(
        lines.len(),
        32,
        "head line + 30 records + remainder notice: {lines:?}"
    );
    assert!(
        lines[0].contains("45"),
        "the head line counts every repair, not just the shown ones: {}",
        lines[0]
    );
    assert!(
        lines[31].contains("15"),
        "the remainder notice states how many were withheld: {}",
        lines[31]
    );
    assert!(
        !lines[31].is_ascii(),
        "the notice must come from the locale table, not a hardcoded English string: {}",
        lines[31]
    );
}

const TIER_SKIP_NOTE: &str = "intent-tier passes skipped (template provenance: slide-deck via \
                              namespaced-variables) — authored spacing, surfaces and palette \
                              kept as designed; contract-tier checks still ran";

#[test]
fn a_skipped_tier_note_heads_the_polish_row_detail() {
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    let events = vec![
        Progress::CleanupDone,
        Progress::QualityChecked {
            checks: vec!["layout".into()],
            repairs: vec![("layout".into(), 1)],
            records: vec!["layout · table-gap · Pricing Row [n42] · gap 0 → 16".into()],
            notes: vec![TIER_SKIP_NOTE.to_string()],
            items: Vec::new(),
        },
    ];

    assert!(super::apply_progress(&mut message, &events, Locale::EnUs));

    let detail = message
        .activities
        .iter()
        .find(|activity| activity.id == "__polish")
        .and_then(|activity| activity.detail.clone())
        .expect("itemized detail");
    let lines: Vec<&str> = detail.lines().collect();
    assert!(
        lines[0].starts_with("intent-tier passes skipped"),
        "what was deliberately NOT run must be the first thing read: {lines:?}"
    );
    assert!(
        lines[1].contains("1 auto-repair(s) applied"),
        "the count head line follows the note: {lines:?}"
    );
    assert!(
        lines[2].contains("table-gap"),
        "then the itemized repairs: {lines:?}"
    );
}

#[test]
fn a_template_run_that_repaired_nothing_still_shows_why() {
    // The shape tiering actually produces: authored input needs no repairs,
    // so there is no record list at all. Before notes reached the row, this
    // rendered as a bare "Polishing the layout ✓" with nothing behind it —
    // indistinguishable from "we checked everything and it was perfect".
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    let events = vec![
        Progress::CleanupDone,
        Progress::QualityChecked {
            checks: vec!["layout".into(), "structure".into()],
            repairs: Vec::new(),
            records: Vec::new(),
            notes: vec![TIER_SKIP_NOTE.to_string()],
            items: Vec::new(),
        },
    ];

    assert!(super::apply_progress(&mut message, &events, Locale::EnUs));

    let detail = message
        .activities
        .iter()
        .find(|activity| activity.id == "__polish")
        .and_then(|activity| activity.detail.clone())
        .expect("a note alone must still open the row");
    assert!(detail.starts_with("intent-tier passes skipped"), "{detail}");
    assert!(
        !detail.contains("auto-repair(s) applied"),
        "no repairs ran, so no count line should claim any: {detail}"
    );
}

#[test]
fn a_run_with_neither_notes_nor_records_leaves_the_row_bare() {
    // The honesty counterpart: nothing to itemize must not manufacture an
    // empty expandable row that reveals nothing when clicked.
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    let events = vec![
        Progress::CleanupDone,
        Progress::QualityChecked {
            checks: vec!["layout".into()],
            repairs: Vec::new(),
            records: Vec::new(),
            notes: Vec::new(),
            items: Vec::new(),
        },
    ];

    super::apply_progress(&mut message, &events, Locale::EnUs);

    let polish = message
        .activities
        .iter()
        .find(|activity| activity.id == "__polish")
        .expect("polish row");
    assert!(polish.detail.is_none(), "{:?}", polish.detail);
}
