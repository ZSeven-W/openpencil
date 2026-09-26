//! Replay of every `[PLAN] coverage: missing …` / `still missing …` verdict
//! from the GLM-5.3-Flash design-arena runs (0926, 0926b–e, 0927a) against
//! the gate, with a hand-labelled verdict per section.
//!
//! The fixture holds the plan exactly as the run logged it: the `planned:`
//! list (id + label) for a first-plan verdict, the final `[PLAN] subtask`
//! lines (id + label + elements, cut at ~160 chars) for a retry verdict, and
//! `covers` rebuilt from the `(covers)` entries of the `covered-by:` list.
//! Each row carries two verdicts. `truth` is the hand label: `covered` — the
//! logged plan already has the section (a false miss); `missing` — the logged
//! plan really lacks it; `detail` — a sub-part of a sibling section, which
//! must never become a subtask of its own. `gate` is what the gate reports
//! today. Where `truth` is `covered` but `gate` is `missing`, the plan names
//! the section in another script than the brief (`Product Cards` for
//! `产品卡`, `Top Status Summary Cards` for `顶部状态摘要`) or by a word the
//! brief never used (`Welcome Actions` for `two buttons`): only the planner's
//! `covers` backfill can vouch for those, and the appender already refuses to
//! act on a cross-script verdict (`script-mismatch`).
//!
//! Measured on this corpus (233 rows: 197 covered, 25 missing, 11 detail),
//! before → after the position/count/state normalisation and the detail
//! provenance: covered rows reported missing 195 → 52 (the rest are all the
//! cross-script or unworded kind above), true misses reported 25 → 25,
//! retry-plan appends 19 → 1, details appended 2 → 0.
//!
//! Multi-screen `arena-m01`/`arena-m05` page rows are left out: they hinge on
//! the subtasks' `screen`, which the log does not record. `arena-m04` plans
//! are replayed as multi-screen, as every one of its retries logged.

use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::plan_coverage::{check_coverage, required_section_details, required_sections};
use crate::plan_coverage_append::append_missing_sections;
use serde_json::Value;

const CORPUS: &str = include_str!("test_fixtures/plan_coverage_corpus.json");

struct RowOutcome {
    key: String,
    truth: String,
    gate: String,
    reported_missing: bool,
    appended: bool,
}

fn subtask(value: &Value, multi_screen: bool) -> Subtask {
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let id = text("id").expect("subtask id");
    Subtask {
        id: id.clone(),
        label: text("label").expect("subtask label"),
        region: Region {
            width: 1440.0,
            height: 300.0,
        },
        bleed_hero: false,
        id_prefix: id,
        parent_frame_id: None,
        insert_after_sibling_id: None,
        elements: text("elements"),
        screen: multi_screen.then(|| "main".to_string()),
        generated_root_id: None,
        existing_section_labels: None,
        covers: value
            .get("covers")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry.as_str().map(str::to_owned))
                    .collect()
            }),
        retry_feedback: None,
    }
}

fn case_plan(case: &Value) -> OrchestratorPlan {
    let multi_screen = case
        .get("multi_screen")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "page".into(),
            name: "Page".into(),
            width: 1440.0,
            height: 900.0,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: Some(0.0),
            fill: None,
        },
        subtasks: case["plan"]
            .as_array()
            .expect("plan array")
            .iter()
            .map(|st| subtask(st, multi_screen))
            .collect(),
        style_guide_name: None,
    }
}

fn replay() -> (Vec<RowOutcome>, usize) {
    let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
    let mut rows = Vec::new();
    let mut appended_on_retry = 0;
    for case in corpus["cases"].as_array().expect("cases") {
        let brief = corpus["briefs"][case["brief"].as_str().expect("brief key")]
            .as_str()
            .expect("brief text");
        let required = required_sections(brief);
        let details = required_section_details(brief);
        let plan = case_plan(case);
        let check = check_coverage(&required, &plan);
        let mut appended_plan = plan.clone();
        let outcome = append_missing_sections(&mut appended_plan, &required, &details);
        let key = format!(
            "{} {}",
            case["run"].as_str().unwrap_or_default(),
            case["phase"].as_str().unwrap_or_default()
        );
        if case["phase"] == "retry" {
            appended_on_retry += outcome.appended.len();
            for section in &outcome.appended {
                eprintln!("  retry append: {key} / {section}");
            }
        }
        for row in case["rows"].as_array().expect("rows") {
            let section = row["section"].as_str().expect("section");
            assert!(
                required.iter().any(|r| r == section),
                "{key}: `{section}` is no longer a required section of {required:?}"
            );
            rows.push(RowOutcome {
                key: format!("{key} {section}"),
                truth: row["truth"].as_str().expect("truth").to_string(),
                gate: row["gate"].as_str().expect("gate").to_string(),
                reported_missing: check.missing.iter().any(|m| m == section),
                appended: outcome.appended.iter().any(|a| a == section),
            });
        }
    }
    (rows, appended_on_retry)
}

#[test]
fn corpus_verdicts_match_the_hand_labels() {
    let (rows, appended_on_retry) = replay();
    let count = |truth: &str, missing: bool| {
        rows.iter()
            .filter(|row| row.truth == truth && row.reported_missing == missing)
            .count()
    };
    eprintln!(
        "corpus: {} rows | covered-truth: {} still reported missing, {} passing | missing-truth: {} reported, {} passing | detail-truth: {} reported, {} passing, {} appended | retry appends: {}",
        rows.len(),
        count("covered", true),
        count("covered", false),
        count("missing", true),
        count("missing", false),
        count("detail", true),
        count("detail", false),
        rows.iter().filter(|r| r.truth == "detail" && r.appended).count(),
        appended_on_retry
    );
    for row in &rows {
        // A true miss must stay reported.
        if row.truth == "missing" {
            assert!(
                row.reported_missing,
                "true miss passed the gate: {}",
                row.key
            );
        }
        // A detail is never appended as a subtask of its own.
        if row.truth == "detail" {
            assert!(!row.appended, "detail appended as a section: {}", row.key);
        }
        let reported = if row.reported_missing {
            "missing"
        } else {
            "covered"
        };
        assert_eq!(reported, row.gate, "gate verdict moved: {}", row.key);
    }
    // One retry replay still appends: arena-d03 0926c's final plan lost its
    // status-summary subtask to a normalize fold ("normalize dropped
    // 顶部状态摘要"), so the logged plan truly lacks it.
    assert_eq!(appended_on_retry, 1);
}
