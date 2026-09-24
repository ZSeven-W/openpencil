use std::collections::BTreeMap;

use super::*;
use crate::Locale;

fn record(
    pass: &str,
    family: &str,
    node_id: &str,
    name: &str,
    detail: &str,
) -> QualityRepairRecord {
    QualityRepairRecord {
        pass: pass.into(),
        family: family.into(),
        node_id: node_id.into(),
        node_name: Some(name.into()),
        detail: detail.into(),
    }
}

fn sample_event_report() -> QualityReport {
    let mut report = QualityReport::default();
    report.ingest_repairs(
        &["layout".into(), "overflow".into(), "palette".into()],
        &[
            record(
                "geometry-validation",
                "overflow",
                "n2",
                "Hero Title",
                "width 420 → 327",
            ),
            record(
                "text_contrast_repair",
                "layout",
                "n3",
                "Price",
                "fill #ccc → #333",
            ),
            record(
                "loop-finalize:container-geometry",
                "layout",
                "n4",
                "Card",
                "gap 0 → 12",
            ),
            record("brand-new-pass", "overflow", "", "", "clip on"),
        ],
        &["intent tier skipped".into()],
    );
    report
}

#[test]
fn pass_names_translate_to_user_topics() {
    assert_eq!(
        topic_for_pass("geometry-validation", "overflow"),
        QualityTopic::Overflow
    );
    assert_eq!(
        topic_for_pass("text_contrast_repair", "layout"),
        QualityTopic::Contrast
    );
    assert_eq!(
        topic_for_pass("loop-finalize:container-geometry", "layout"),
        QualityTopic::Sizing
    );
    // An unlisted pass lands under its family, never nowhere.
    assert_eq!(topic_for_pass("new-pass", "palette"), QualityTopic::Palette);
    assert_eq!(
        topic_for_lint("text-bg-contrast"),
        Some(QualityTopic::Contrast)
    );
    assert_eq!(topic_for_lint("redundant-wrapper"), None);
    assert_eq!(topic_for_lint("slop/rounded-card-wall"), None);
}

#[test]
fn every_topic_has_a_label_in_every_locale() {
    for topic in QualityTopic::ALL {
        let key = topic.label_key();
        assert_ne!(op_i18n::translate(Locale::EnUs, key), key, "{key} missing");
        assert_ne!(op_i18n::translate(Locale::ZhCn, key), key, "{key} missing");
    }
}

#[test]
fn report_from_quality_checked_counts_exactly_the_records() {
    let report = sample_event_report();
    assert_eq!(report.total_fixed(), 4);
    assert_eq!(report.total_remaining(), 0);
    assert!(!report.audited, "no audit ran yet — remaining is unknown");
    let overflow = report
        .topics
        .iter()
        .find(|entry| entry.topic == QualityTopic::Overflow)
        .expect("overflow topic");
    assert!(overflow.checked);
    assert_eq!(overflow.fixed.len(), 2);
    assert_eq!(overflow.fixed[0].label(), "Hero Title · width 420 → 327");
    assert_eq!(overflow.fixed[1].node_id, None, "empty id is not a node");
    let contrast = report
        .topics
        .iter()
        .find(|entry| entry.topic == QualityTopic::Contrast)
        .expect("contrast topic");
    assert_eq!(contrast.fixed.len(), 1);
    // Families that ran vouch for their topics even when clean.
    let palette = report
        .topics
        .iter()
        .find(|entry| entry.topic == QualityTopic::Palette)
        .expect("palette topic");
    assert!(palette.checked);
    assert_eq!(palette.found_count(), 0);
    // A family that never reported is not claimed as checked.
    assert!(!report
        .topics
        .iter()
        .any(|entry| entry.topic == QualityTopic::Hierarchy));
    assert_eq!(report.notes, vec!["intent tier skipped".to_string()]);
    // Display order follows the topic enum.
    let order: Vec<_> = report.topics.iter().map(|entry| entry.topic).collect();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted);
}

#[test]
fn count_only_fixes_and_audit_findings_are_folded_in() {
    let mut report = sample_event_report();
    let mut by_category = BTreeMap::new();
    by_category.insert("text-bg-contrast".to_string(), 2);
    by_category.insert("redundant-wrapper".to_string(), 1);
    by_category.insert("empty-path".to_string(), 0);
    report.ingest_lint_fixes(&by_category);
    report.ingest_visual_review(3);
    assert_eq!(report.total_fixed(), 4 + 2 + 1 + 3);

    report.ingest_audit(
        &[QualityTopic::Contrast, QualityTopic::Charts],
        vec![QualityItem {
            topic: QualityTopic::Contrast,
            source: "text-bg-contrast".into(),
            node_id: Some("n9".into()),
            node_name: Some("Caption".into()),
            board_id: None,
            detail: "contrast 1.8".into(),
        }],
    );
    assert!(report.audited);
    assert_eq!(report.total_remaining(), 1);
    let charts = report
        .topics
        .iter()
        .find(|entry| entry.topic == QualityTopic::Charts)
        .expect("audited clean topic is listed");
    assert!(charts.checked);
    // A second audit replaces the first instead of doubling it.
    report.ingest_audit(&[QualityTopic::Contrast], Vec::new());
    assert_eq!(report.total_remaining(), 0);
}

#[test]
fn empty_report_claims_nothing() {
    let report = QualityReport::default();
    assert!(report.is_empty());
    assert_eq!(report.checked_count(), 0);
}

#[test]
fn chip_and_transcript_render_the_real_counts() {
    let mut report = sample_event_report();
    report.ingest_audit(
        &[QualityTopic::Contrast],
        vec![QualityItem {
            topic: QualityTopic::Contrast,
            source: "text-bg-contrast".into(),
            node_id: None,
            node_name: None,
            board_id: None,
            detail: "low".into(),
        }],
    );
    let chip = report.chip_text(Locale::ZhCn);
    assert!(chip.contains('4') && chip.contains('1'), "{chip}");
    assert!(!chip.contains("{{"), "{chip}");
    let line = report.transcript_line(Locale::EnUs);
    assert!(!line.contains("{{"), "{line}");
    assert!(line.contains('4'), "{line}");
}

#[test]
fn boards_attribute_items_by_containing_top_level_frame() {
    let mut state = EditorState::default();
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [
            {"id": "b1", "type": "frame", "name": "Home", "width": 375, "height": 812,
             "children": [{"id": "n2", "type": "text", "content": "Hi"}]},
            {"id": "b2", "type": "frame", "name": "Detail", "width": 375, "height": 812,
             "children": [{"id": "n3", "type": "text", "content": "Yo"}]}
        ]
    }))
    .expect("fixture doc");
    state.doc = doc;
    let mut report = sample_event_report();
    report.attribute_boards(&state, &["b1".to_string(), "b2".to_string()]);
    assert_eq!(report.boards.len(), 2);
    assert_eq!(report.boards[0].board_name, "Home");
    assert_eq!(report.boards[0].fixed, 1, "n2 lives in b1");
    assert_eq!(report.boards[1].fixed, 1, "n3 lives in b2");
    let item = &report.topics[0].fixed[0];
    assert_eq!(item.board_id.as_deref(), Some("b1"));
}

#[test]
fn report_round_trips_through_serde() {
    let report = sample_event_report();
    let json = serde_json::to_string(&report).expect("serialize");
    assert!(json.contains("\"overflow\""));
    let back: QualityReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, report);
}
