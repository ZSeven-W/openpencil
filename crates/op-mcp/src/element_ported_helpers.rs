//! Shared helpers for the ported TS element builders (the kinds that
//! previously fell through to the generic kit placeholder). Kept in one
//! place so the per-category builder modules stay under the 800-line cap
//! and share one id counter + node-construction convention.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::{ToolErrorCode, ToolOutcome};

static NEXT_PORTED_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolFailure {
    code: ToolErrorCode,
    message: String,
}

impl ToolFailure {
    fn new(code: ToolErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<ToolFailure> for ToolOutcome {
    fn from(value: ToolFailure) -> Self {
        ToolOutcome::Err(value.code, value.message)
    }
}

/// Fresh unique node id (`__op_<prefix>_<n>`), matching the convention the
/// other element builders + the apply path expect.
pub(crate) fn next_id(prefix: &str) -> String {
    let n = NEXT_PORTED_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_{prefix}_{n}")
}

/// Required non-empty string arg, else `MissingArgument`.
pub(crate) fn required<'a>(
    args: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, ToolFailure> {
    args.get(key)
        .map(String::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ToolFailure::new(ToolErrorCode::MissingArgument, format!("{key} is required"))
        })
}

/// Optional non-empty string arg.
pub(crate) fn opt<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    args.get(key).map(String::as_str).filter(|s| !s.is_empty())
}

/// Numeric arg with a default + minimum clamp.
pub(crate) fn number_arg(
    args: &BTreeMap<String, String>,
    key: &str,
    default: f64,
    min: f64,
) -> f64 {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
        .max(min)
}

/// Boolean arg (`"true"` ⇒ true).
pub(crate) fn bool_arg(args: &BTreeMap<String, String>, key: &str) -> bool {
    args.get(key).map(|value| value == "true").unwrap_or(false)
}

/// String field of a parsed JSON item.
pub(crate) fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// Boolean field of a parsed JSON item.
pub(crate) fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// A solid-fill array `[{ "type":"solid", "color": <color> }]` (the shape
/// the PenNode `fill` field deserializes from).
pub(crate) fn solid(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

/// Numeric arg as `Option<f64>` (no default).
pub(crate) fn parse_f64(args: &BTreeMap<String, String>, key: &str) -> Option<f64> {
    args.get(key).and_then(|value| value.parse::<f64>().ok())
}

/// Free-function clamp for `f64` (some ported builders call `clamp(x, lo, hi)`).
pub(crate) fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

/// A 24×24 lucide icon-font node (override width/height as needed).
pub(crate) fn icon_node(name: &str, icon: &str, width: i32, height: i32) -> Value {
    json!({
        "id": next_id("icon"),
        "type": "icon_font",
        "name": name,
        "iconFontName": icon,
        "iconFontFamily": "lucide",
        "width": width,
        "height": height,
    })
}

/// Parse a JSON-array arg into `Vec<Value>` (no per-item validation).
pub(crate) fn parse_array_items(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<Vec<Value>, ToolFailure> {
    let raw = required(args, key)?;
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        ToolFailure::new(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON array: {e}"),
        )
    })?;
    value.as_array().cloned().ok_or_else(|| {
        ToolFailure::new(
            ToolErrorCode::InvalidArgument,
            format!("{key} must be a JSON array"),
        )
    })
}

/// Like [`parse_array_items`] but each item must carry every field in
/// `required_fields` as a non-empty string. The field set is per-builder
/// because TS item types differ (TabsItem needs `label`, ToolbarItem needs
/// `icon`, SocialLoginProvider needs `name`, StatGridItem needs `label`+
/// `value`, DataTableRowColumn needs `content`, …).
pub(crate) fn parse_object_items(
    args: &BTreeMap<String, String>,
    key: &str,
    required_fields: &[&str],
    error: &str,
) -> Result<Vec<Value>, ToolFailure> {
    let items = parse_array_items(args, key)?;
    for item in &items {
        for field in required_fields {
            string_field(item, field)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| ToolFailure::new(ToolErrorCode::InvalidArgument, error))?;
        }
    }
    Ok(items)
}

#[cfg(test)]
mod ported_smoke_tests {
    use super::*;
    use crate::element_alias_builders::semantic_alias_node;

    fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// Every previously-placeholder kind now builds a REAL role-tagged
    /// subtree (Some), not None (which would fall back to the generic kit
    /// placeholder). One representative arg set per kind.
    #[test]
    fn ported_kinds_build_real_subtrees_not_placeholders() {
        let cases: &[(&str, &[(&str, &str)])] = &[
            ("add_list_row_v0", &[("title", "Inbox")]),
            ("add_member_row_v1", &[("name", "Ada"), ("theme", "dark")]),
            ("add_setting_row_v0", &[("title", "Notifications")]),
            ("add_search_bar_v0", &[("placeholder", "Search")]),
            (
                "add_select_v0",
                &[
                    ("label", "Country"),
                    ("options", r#"[{"label":"US"},{"label":"UK"}]"#),
                ],
            ),
            (
                "add_stat_card_v0",
                &[("label", "Revenue"), ("value", "$12k")],
            ),
            (
                "add_user_card_v1",
                &[("name", "Grace"), ("theme", "system")],
            ),
            (
                "add_tabs_v0",
                &[("items", r#"[{"label":"One"},{"label":"Two"}]"#)],
            ),
            ("add_top_nav_bar_v0", &[("title", "Home")]),
            ("add_modal_shell_v0", &[("title", "Confirm")]),
            (
                "add_metric_row_v0",
                &[("items", r#"[{"label":"Users","value":"1,204"}]"#)],
            ),
        ];
        for (tool, pairs) in cases {
            let node = semantic_alias_node(tool, &args(pairs));
            let node = node.unwrap_or_else(|e| panic!("{tool} dispatch errored: {e:?}"));
            assert!(
                node.is_some(),
                "{tool} must build a real subtree (got None ⇒ placeholder fallback)"
            );
        }
    }
}
