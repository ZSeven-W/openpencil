//! Tests for the report's localized item lines.

use super::*;
use crate::QualityTopic;

fn item(source: &str, name: Option<&str>, detail: &str) -> QualityItem {
    QualityItem {
        topic: QualityTopic::Spacing,
        source: source.to_string(),
        node_id: Some("n1".to_string()),
        node_name: name.map(str::to_string),
        board_id: None,
        detail: detail.to_string(),
    }
}

#[test]
fn every_reported_lint_category_has_a_sentence_in_every_locale() {
    for (category, _) in super::super::topics::LINT_TOPICS {
        let key = issue_key(category).unwrap_or_else(|| panic!("{category} has no issue key"));
        for locale in op_i18n::Locale::ALL {
            assert!(
                op_i18n::translate_dynamic(locale, key).is_some(),
                "{key} missing for {locale:?}"
            );
        }
    }
    assert!(issue_key("unfilled-screen").is_some());
    assert!(
        issue_key("redundant-wrapper").is_none(),
        "code shape stays hidden"
    );
}

#[test]
fn a_remaining_issue_reads_as_its_category_not_the_detector_reason() {
    let issue = item(
        "text-bg-contrast",
        Some("Hero title"),
        "contrast 1.8:1 below 3:1 against #f5f5f5",
    );
    let zh = issue.localized_label(Locale::ZhCn, false);
    assert!(zh.starts_with("Hero title · "), "{zh}");
    assert!(
        !zh.contains("contrast 1.8"),
        "the English reason is not shown: {zh}"
    );
    let en = issue.localized_label(Locale::EnUs, false);
    assert_ne!(zh, en);
}

#[test]
fn a_field_change_keeps_its_values_and_translates_its_field() {
    let fix = item(
        "spacing+footer-sink",
        Some("Card"),
        "gap 24 → 16, padding (unset) → 12",
    );
    let zh = fix.localized_detail(Locale::ZhCn, true);
    assert!(zh.contains("24 → 16"), "{zh}");
    assert!(zh.contains("→ 12"), "{zh}");
    assert!(!zh.contains("gap"), "{zh}");
    assert!(!zh.contains("(unset)"), "{zh}");
    assert_eq!(
        fix.localized_detail(Locale::EnUs, true),
        "Gap 24 → 16, Padding unset → 12"
    );
}

#[test]
fn prose_fixes_fall_back_to_the_generic_line() {
    for detail in [
        "removed frame (+3 descendant(s))",
        "Section: padding 0 → 16; Row: gap 0 → 8",
        "moved under Header at index 2",
        "gap 24 → 16, moved under Header",
    ] {
        let fix = item("chrome-dedupe", None, detail);
        assert_eq!(
            fix.localized_detail(Locale::ZhCn, true),
            op_i18n::translate(Locale::ZhCn, "workspace.quality.fixedGeneric"),
            "{detail}"
        );
    }
}

#[test]
fn an_unnamed_empty_item_still_labels_itself_by_id() {
    let bare = item("chrome-dedupe", None, "");
    assert_eq!(bare.localized_label(Locale::EnUs, true), "n1");
}
