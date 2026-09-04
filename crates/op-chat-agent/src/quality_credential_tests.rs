use super::{
    quality_credential_line, quality_credential_line_with_records, quality_note_lines,
    quality_summary_from_repairs, MAX_INLINE_REPAIR_RECORDS,
};
use op_ai::chat_provider::QualitySummary;
use op_orchestrator::{CheckCategory, RepairSummary};

fn summary(checks: &[&str], repairs: &[(&str, usize)]) -> QualitySummary {
    QualitySummary {
        checks: checks.iter().map(|c| c.to_string()).collect(),
        repairs: repairs.iter().map(|(c, n)| (c.to_string(), *n)).collect(),
        records: Vec::new(),
        notes: Vec::new(),
    }
}

#[test]
fn nothing_checked_renders_no_credential() {
    assert_eq!(
        quality_credential_line(&QualitySummary::default(), Some(0)),
        None,
        "a turn that never ran the passes must not be credited with checking"
    );
}

#[test]
fn clean_run_still_earns_a_positive_credential() {
    let line =
        quality_credential_line(&summary(&["layout", "overflow", "hierarchy"], &[]), Some(0))
            .expect("checks ran, so a credential is owed");

    assert_eq!(
        line,
        "\n\n• Checked layout, overflow, hierarchy — nothing needed fixing, no issues left"
    );
    assert!(
        !line.contains("▸ repairs"),
        "no repairs means no breakdown sub-line"
    );
}

#[test]
fn repairs_are_reported_with_a_per_check_breakdown() {
    let line = quality_credential_line(
        &summary(
            &["layout", "overflow", "hierarchy", "structure"],
            &[("layout", 2), ("overflow", 1), ("structure", 3)],
        ),
        Some(0),
    )
    .expect("credential owed");

    assert_eq!(
        line,
        "\n\n• Checked layout, overflow, hierarchy, structure — 6 auto-repair(s) applied, \
         no issues left\n  ▸ repairs: layout 2, overflow 1, structure 3"
    );
}

#[test]
fn leftover_issues_are_stated_not_papered_over() {
    let line = quality_credential_line(&summary(&["layout"], &[("layout", 1)]), Some(2))
        .expect("credential owed");

    assert!(
        line.contains("2 issue(s) still open"),
        "unresolved work must survive into the credential: {line}"
    );
    assert!(
        !line.contains("no issues left"),
        "must not claim a clean finish while issues remain: {line}"
    );
}

#[test]
fn unknown_remaining_omits_the_clause_entirely() {
    let line = quality_credential_line(&summary(&["structure"], &[("structure", 4)]), None)
        .expect("credential owed");

    assert_eq!(
        line, "\n\n• Checked structure — 4 auto-repair(s) applied\n  ▸ repairs: structure 4",
        "a caller that cannot know the leftover count must not imply there is none"
    );
}

#[test]
fn repair_summary_converts_preserving_checked_and_repaired_split() {
    let mut repairs = RepairSummary::default();
    repairs.record(CheckCategory::Layout, 2);
    repairs.record(CheckCategory::Overflow, 0);
    repairs.record(CheckCategory::Structure, 1);

    let wire = quality_summary_from_repairs(&repairs);

    assert_eq!(wire.checks, vec!["layout", "overflow", "structure"]);
    assert_eq!(
        wire.repairs,
        vec![("layout".to_string(), 2), ("structure".to_string(), 1)],
        "checked-but-clean categories are listed as checked, not as repairs"
    );
    assert_eq!(wire.total_repairs(), 3);
}

#[test]
fn the_itemized_variant_lists_each_repair_under_the_credential() {
    // For the callers whose only channel is this text (the agentic loop, the
    // web thinking stream): the `▸` sub-lines are what the transcript turns
    // into the credential step's expandable detail.
    let quality = QualitySummary {
        checks: vec!["layout".into()],
        repairs: vec![("layout".into(), 2)],
        records: vec![
            "layout · table-gap · Pricing Row [n42] · gap 0 → 16".into(),
            "layout · container-geometry · Hero [n7] · padding [32,32] → [16,16]".into(),
        ],
        notes: Vec::new(),
    };

    let line = quality_credential_line_with_records(&quality, Some(0)).expect("credential");

    assert!(line.contains("2 auto-repair(s) applied"));
    assert!(
        line.contains("\n  ▸ layout · table-gap · Pricing Row [n42] · gap 0 → 16"),
        "each record is its own sub-line: {line}"
    );
    assert!(
        !line.contains("more repair(s)"),
        "nothing was withheld, so no remainder notice: {line}"
    );
}

#[test]
fn the_itemized_variant_caps_the_list_and_says_how_many_it_withheld() {
    let records: Vec<String> = (0..MAX_INLINE_REPAIR_RECORDS + 11)
        .map(|i| format!("layout · container-geometry · Card {i} [n{i}] · gap 24 → 16"))
        .collect();
    let total = records.len();
    let quality = QualitySummary {
        checks: vec!["layout".into()],
        repairs: vec![("layout".into(), total)],
        records,
        notes: Vec::new(),
    };

    let line = quality_credential_line_with_records(&quality, None).expect("credential");

    assert_eq!(
        line.matches("· Card ").count(),
        MAX_INLINE_REPAIR_RECORDS,
        "the inline list stops at the cap"
    );
    assert!(
        line.contains("… and 11 more repair(s) — full list in the log"),
        "the withheld count must be stated, never silently dropped: {line}"
    );
    assert!(
        line.contains(&format!("{total} auto-repair(s) applied")),
        "the headline still counts every repair: {line}"
    );
}

#[test]
fn record_lines_ride_the_wire_conversion_alongside_the_counts() {
    let mut repairs = RepairSummary::default();
    repairs.record_all(
        CheckCategory::Layout,
        vec![op_orchestrator::RepairRecord {
            pass: "table-gap".into(),
            category: CheckCategory::Layout,
            node_id: "n42".into(),
            node_name: Some("Pricing Row".into()),
            detail: "gap 0 → 16".into(),
        }],
    );

    let wire = quality_summary_from_repairs(&repairs);

    assert_eq!(wire.repairs, vec![("layout".to_string(), 1)]);
    assert_eq!(
        wire.records,
        vec!["layout · table-gap · Pricing Row [n42] · gap 0 → 16".to_string()],
        "the count and the line it stands for must cross the wire together"
    );
}

const SKIP_NOTE: &str = "intent-tier passes skipped (template provenance: slide-deck via \
                         namespaced-variables) — authored spacing, surfaces and palette kept \
                         as designed; contract-tier checks still ran";

#[test]
fn a_skipped_tier_is_stated_before_the_repairs_that_did_run() {
    // Order is the point. "layout 1" under a skipped intent tier means "one
    // contract-tier fix", not "the layout was reviewed and needed one fix" —
    // a reader who stops after the first detail line must already know that.
    let quality = QualitySummary {
        checks: vec!["layout".into()],
        repairs: vec![("layout".into(), 1)],
        records: vec!["layout · table-gap · Pricing Row [n42] · gap 0 → 16".into()],
        notes: vec![SKIP_NOTE.to_string()],
    };

    let line = quality_credential_line_with_records(&quality, Some(0)).expect("credential");

    let note_at = line
        .find("intent-tier passes skipped")
        .expect("note rendered");
    let record_at = line.find("table-gap").expect("record rendered");
    assert!(
        note_at < record_at,
        "the skipped-tier note must precede the repair list: {line}"
    );
}

#[test]
fn a_note_survives_a_run_that_repaired_nothing() {
    // The case the tiering exists to produce: an authored template needs no
    // repairs, so the record list is empty. Gating the note on repairs would
    // hide the decision exactly where it is the ONLY thing worth reporting.
    let quality = QualitySummary {
        checks: vec!["layout".into(), "structure".into()],
        repairs: Vec::new(),
        records: Vec::new(),
        notes: vec![SKIP_NOTE.to_string()],
    };

    let line = quality_credential_line_with_records(&quality, Some(0)).expect("credential");

    assert!(line.contains("nothing needed fixing"));
    assert!(
        line.contains("\n  ▸ intent-tier passes skipped"),
        "the note must still be reported: {line}"
    );
}

#[test]
fn a_note_is_never_counted_as_a_repair() {
    let quality = QualitySummary {
        checks: vec!["layout".into()],
        repairs: Vec::new(),
        records: Vec::new(),
        notes: vec![SKIP_NOTE.to_string()],
    };

    assert_eq!(
        quality.total_repairs(),
        0,
        "a statement about the run is not an edit to the document"
    );
    assert_eq!(quality_note_lines(&quality).len(), 1);
}

#[test]
fn notes_ride_the_wire_conversion_beside_the_records() {
    let mut repairs = RepairSummary::default();
    repairs.record(CheckCategory::Layout, 0);
    repairs.note(SKIP_NOTE);

    let wire = quality_summary_from_repairs(&repairs);

    assert_eq!(wire.notes, vec![SKIP_NOTE.to_string()]);
    assert!(
        wire.records.is_empty(),
        "notes must not leak into the itemized repair list"
    );
}
