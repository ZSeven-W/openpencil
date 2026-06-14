//! The diagnostics data model. Wire-compatible with the TS `pen-ai-skills`
//! `Issue` type — `#[serde(rename_all = "camelCase")]` keeps the JSON shape
//! byte-identical so `debug_validation_report` clients are unaffected.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Severity of a detected issue. All current detectors emit `Warning`;
/// `Info` is detect-only (an auto-fix would not be safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// Which detector produced an issue — one variant per detector (14 total).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueCategory {
    InvisibleContainer,
    EmptyPath,
    TextExplicitHeight,
    SiblingInconsistency,
    UnexpectedRotation,
    TextCornerRadius,
    MixedSiblingCornerRadius,
    TextEffect,
    TextStroke,
    MixedSiblingPadding,
    ExcessiveFrameEffects,
    EdgeSectionPadding,
    TextBgContrast,
    StackedHorizontalPadding,
    WidgetA11y,
}

/// The node property a fix targets. The detectors emit only this closed set
/// of 9 values (spec §4, Codex Q3); `Remove` is the `"__remove"` sentinel.
/// `#[serde(rename)]` keeps each on-wire string identical to TS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixProperty {
    #[serde(rename = "__remove")]
    Remove,
    #[serde(rename = "cornerRadius")]
    CornerRadius,
    #[serde(rename = "effects")]
    Effects,
    #[serde(rename = "fill")]
    Fill,
    #[serde(rename = "fontSize")]
    FontSize,
    #[serde(rename = "height")]
    Height,
    #[serde(rename = "padding")]
    Padding,
    #[serde(rename = "rotation")]
    Rotation,
    #[serde(rename = "stroke")]
    Stroke,
    /// An accessible label / placeholder target (widget-a11y). Detect-only —
    /// there is no safe auto-fix (the label text must be authored), so the
    /// apply paths treat it as a no-op like `Fill`.
    #[serde(rename = "label")]
    Label,
}

impl FixProperty {
    /// The on-wire property string — identical to the `#[serde(rename)]`
    /// attribute on each variant. Used to build the `{nodeId}:{property}`
    /// dedup key shared by `detect_sibling_inconsistencies` and `detect_all`,
    /// without round-tripping through `serde_json`.
    pub fn wire_str(self) -> &'static str {
        match self {
            FixProperty::Remove => "__remove",
            FixProperty::CornerRadius => "cornerRadius",
            FixProperty::Effects => "effects",
            FixProperty::Fill => "fill",
            FixProperty::FontSize => "fontSize",
            FixProperty::Height => "height",
            FixProperty::Padding => "padding",
            FixProperty::Rotation => "rotation",
            FixProperty::Stroke => "stroke",
            FixProperty::Label => "label",
        }
    }
}

/// One diagnostic finding. `current_value` / `suggested_value` are arbitrary
/// JSON so the wire shape matches the TS `unknown`-typed fields exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Node id where the issue was detected (jian `NodeId` is a string).
    pub node_id: String,
    /// Which detector produced this issue.
    pub category: IssueCategory,
    /// Severity for reporting.
    pub severity: IssueSeverity,
    /// Property the fix targets, or `Remove`.
    pub property: FixProperty,
    /// Current value on the node, raw.
    pub current_value: serde_json::Value,
    /// Value the detector suggests as the fix.
    pub suggested_value: serde_json::Value,
    /// Human-readable reason, mirrors the TS `fix.reason` strings.
    pub reason: String,
}

/// Per-category count of fixes ACTUALLY applied by `apply_fixes` (Plan B).
/// Defined here so the type model is complete in one place.
/// `rename_all = "camelCase"` makes `by_category` serialize as `byCategory`,
/// matching the TS `PreValidationResult` wire shape.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixReport {
    pub total: u32,
    pub by_category: BTreeMap<IssueCategory, u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_category_serializes_kebab_case() {
        let j = serde_json::to_string(&IssueCategory::TextBgContrast).unwrap();
        assert_eq!(j, "\"text-bg-contrast\"");
    }

    #[test]
    fn fix_property_serializes_to_ts_wire_strings() {
        assert_eq!(
            serde_json::to_string(&FixProperty::Remove).unwrap(),
            "\"__remove\""
        );
        assert_eq!(
            serde_json::to_string(&FixProperty::CornerRadius).unwrap(),
            "\"cornerRadius\""
        );
    }

    #[test]
    fn wire_str_matches_serde_rename_for_every_variant() {
        for property in [
            FixProperty::Remove,
            FixProperty::CornerRadius,
            FixProperty::Effects,
            FixProperty::Fill,
            FixProperty::FontSize,
            FixProperty::Height,
            FixProperty::Padding,
            FixProperty::Rotation,
            FixProperty::Stroke,
        ] {
            let serde_wire = serde_json::to_value(property).unwrap();
            assert_eq!(
                serde_wire,
                serde_json::Value::String(property.wire_str().to_string())
            );
        }
    }

    #[test]
    fn issue_round_trips_with_camel_case_fields() {
        let issue = Issue {
            node_id: "n7".into(),
            category: IssueCategory::EmptyPath,
            severity: IssueSeverity::Warning,
            property: FixProperty::Remove,
            current_value: serde_json::Value::Null,
            suggested_value: serde_json::Value::Null,
            reason: "empty path".into(),
        };
        let j = serde_json::to_string(&issue).unwrap();
        assert!(j.contains("\"nodeId\":\"n7\""));
        assert!(j.contains("\"currentValue\":null"));
        let back: Issue = serde_json::from_str(&j).unwrap();
        assert_eq!(back, issue);
    }

    #[test]
    fn fix_report_serializes_by_category_as_camel_case() {
        let mut report = FixReport {
            total: 2,
            ..Default::default()
        };
        report.by_category.insert(IssueCategory::EmptyPath, 2);
        let j = serde_json::to_string(&report).unwrap();
        // TS `PreValidationResult` wire shape uses `byCategory`, not `by_category`.
        assert!(j.contains("\"byCategory\""));
        assert!(!j.contains("by_category"));
    }
}
