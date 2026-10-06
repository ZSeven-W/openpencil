//! Reject a named empty title/body slot when a generated card contains only chrome.
//! This is a narrow completeness check, not semantic coverage or an aesthetic score.

use super::*;

fn kids(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn chrome(v: &Value) -> bool {
    let name = string_prop(v, "name").unwrap_or("").to_lowercase();
    [
        "page-no",
        "page-num",
        "page-count",
        "series",
        "eyebrow",
        "caption",
        "footer",
        "kicker",
        "serial",
        "meta",
    ]
    .iter()
    .any(|token| name.contains(token))
        || matches!(string_prop(v, "role"), Some("caption" | "label"))
}

fn visible(v: &Value) -> bool {
    v.get("visible") != Some(&Value::Bool(false))
        && !v
            .get("opacity")
            .and_then(Value::as_f64)
            .is_some_and(|v| v <= 0.0)
}

fn reading_content(v: &Value) -> bool {
    if !visible(v) {
        return false;
    }
    if matches!(string_prop(v, "type"), Some("image" | "ref" | "path")) {
        return true;
    }
    if string_prop(v, "type") == Some("text") && !chrome(v) {
        match v.get("content") {
            Some(Value::String(text)) if !text.trim().is_empty() => return true,
            Some(Value::Array(segments))
                if segments.iter().any(|s| {
                    s.get("text")
                        .and_then(Value::as_str)
                        .is_some_and(|t| !t.trim().is_empty())
                }) =>
            {
                return true
            }
            _ => {}
        }
    }
    kids(v).iter().any(reading_content)
}

fn empty_reading_slot(v: &Value) -> Option<String> {
    if !visible(v) {
        return None;
    }
    let name = string_prop(v, "name").unwrap_or("").to_lowercase();
    let reading = matches!(string_prop(v, "role"), Some("heading" | "body-text"))
        || [
            "headline",
            "title-block",
            "title-section",
            "body-block",
            "正文区",
            "标题区",
        ]
        .iter()
        .any(|s| name.contains(s));
    if matches!(string_prop(v, "type"), Some("frame" | "group")) && reading && kids(v).is_empty() {
        return string_prop(v, "id").map(str::to_string);
    }
    kids(v).iter().find_map(empty_reading_slot)
}

pub(super) fn check_reading_content(nodes: &[PenNode], prompt: &str, report: &mut SelfCheckReport) {
    if crate::design_type::detect_design_type(prompt).type_ != crate::design_type::DesignType::Card
    {
        return;
    }
    let lower = prompt.to_lowercase();
    if [
        "skeleton",
        "loading placeholder",
        "blank canvas",
        "骨架屏",
        "空白画布",
        "只做页眉",
        "header only",
    ]
    .iter()
    .any(|s| lower.contains(s))
    {
        return;
    }
    let Ok(value) = serde_json::to_value(nodes) else {
        return;
    };
    let Some(forest) = value.as_array() else {
        return;
    };
    if forest.iter().any(reading_content) {
        return;
    }
    if let Some(id) = forest.iter().find_map(empty_reading_slot) {
        report.issues.push(SelfCheckIssue {
            code:"missing-card-reading-content", node_id:Some(id),
            message:"a named headline/body slot is empty and the card contains only series headers or page numbers. Complete the requested title and body copy before returning the card; preserve the supplied facts instead of treating page chrome as completed content".into(),
            severity:SelfCheckSeverity::Fatal,
        });
    }
}

#[cfg(test)]
#[path = "orchestration_self_check_card_copy_tests.rs"]
mod tests;
