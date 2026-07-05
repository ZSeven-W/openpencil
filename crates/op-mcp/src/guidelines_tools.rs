//! MCP tool `get_guidelines` — returns product-design guideline text for a topic.
//!
//! Part of the Pencil-style agentic tool-loop (Phase 0 — purely additive). A
//! design agent calls this early in the loop to load the product-design
//! principles for the target surface before generating any nodes.

use std::collections::BTreeMap;

use op_ai_skills::guideline_for;
use op_ai_skills::resolve_style::{
    resolve_style, Fonts, ResolveOutcome, Shadow, StyleGuide, StyleParams, TokenMap,
};

use super::{McpTool, ToolErrorCode, ToolOutcome};

/// Read tool that returns the product-design guidelines for `topic`.
///
/// Supported topics include `"web-app"`, `"mobile"`, and `"code-to-design"`.
/// Unknown topics return the same error envelope shape as `get_style_guide`
/// on a bad argument — `ToolOutcome::Ok` with an `"error"` key in the result
/// map (not a JSON-RPC transport error).
pub struct GetGuidelines;

impl McpTool for GetGuidelines {
    fn name(&self) -> &str {
        "get_guidelines"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let category = args
            .get("category")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("guide")
            .to_ascii_lowercase();

        match category.as_str() {
            "guide" => call_guide(args),
            "style" => call_style(args),
            other => ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("category must be \"guide\" or \"style\", got {other:?}"),
            ),
        }
    }
}

fn call_guide(args: &BTreeMap<String, String>) -> ToolOutcome {
    let topic = match args.get("topic").map(String::as_str) {
        Some(t) if !t.trim().is_empty() => t.trim(),
        _ => {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "topic is required (\"web-app\", \"mobile\", or \"code-to-design\")".into(),
            )
        }
    };

    match guideline_for(topic) {
        Some(content) => {
            let mut out = BTreeMap::new();
            out.insert("topic".into(), topic.to_string());
            out.insert("content".into(), content);
            ToolOutcome::Ok(out)
        }
        None => {
            // Mirror the shape get_style_guide returns when no match is found:
            // ToolOutcome::Ok with an "error" key — not a transport-level error.
            let mut out = BTreeMap::new();
            out.insert(
                "error".into(),
                format!(
                    "Unknown topic: \"{topic}\". Supported topics: web-app, mobile, code-to-design."
                ),
            );
            ToolOutcome::Ok(out)
        }
    }
}

fn call_style(args: &BTreeMap<String, String>) -> ToolOutcome {
    let name = match required_arg(args, "name") {
        Ok(value) => value,
        Err((code, message)) => return ToolOutcome::Err(code, message),
    };
    let params = match style_params(args) {
        Ok(params) => params,
        Err((code, message)) => return ToolOutcome::Err(code, message),
    };

    match resolve_style(&name, &params) {
        ResolveOutcome::Hit(style_guide) => {
            let mut out = BTreeMap::new();
            out.insert("category".into(), "style".into());
            out.insert("name".into(), name);
            out.insert("content".into(), format_style_guide(&style_guide));
            ToolOutcome::Ok(out)
        }
        ResolveOutcome::Miss { missing, suggest } => {
            let mut out = BTreeMap::new();
            out.insert("category".into(), "style".into());
            out.insert("name".into(), name);
            out.insert(
                "error".into(),
                format!(
                    "Unable to resolve style. Missing: {}. Suggested catalog candidates: {}.",
                    join_or_none(&missing),
                    join_or_none(&suggest)
                ),
            );
            out.insert("missing".into(), join_or_none(&missing));
            out.insert("suggest".into(), join_or_none(&suggest));
            ToolOutcome::Ok(out)
        }
    }
}

fn style_params(args: &BTreeMap<String, String>) -> Result<StyleParams, (ToolErrorCode, String)> {
    Ok(StyleParams {
        color_palette: required_arg(args, "colorPalette")?,
        roundness: required_arg(args, "roundness")?,
        elevation: required_arg(args, "elevation")?,
        fonts: Fonts {
            headings: required_arg(args, "headings")?,
            body: required_arg(args, "body")?,
            captions: required_arg(args, "captions")?,
            data: required_arg(args, "data")?,
        },
        decorative_imagery: args
            .get("decorativeImagery")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    })
}

fn required_arg(
    args: &BTreeMap<String, String>,
    key: &str,
) -> Result<String, (ToolErrorCode, String)> {
    match args.get(key).map(String::as_str).map(str::trim) {
        Some(value) if !value.is_empty() => Ok(value.to_string()),
        _ => Err((
            ToolErrorCode::MissingArgument,
            format!("{key} is required for category=\"style\""),
        )),
    }
}

fn format_style_guide(style_guide: &StyleGuide) -> String {
    let mut out = String::new();
    out.push_str(style_guide.prose.trim());
    out.push_str("\n\n## TokenMap\n");
    append_token_map(&mut out, &style_guide.tokens);
    append_contrast(&mut out, style_guide);
    out
}

fn append_token_map(out: &mut String, tokens: &TokenMap) {
    out.push_str("colorPalette:\n");
    append_color_role_map(out, "surface", &tokens.surface);
    append_color_role_map(out, "foreground", &tokens.foreground);
    append_color_role_map(out, "accent", &tokens.accent);
    append_color_role_map(out, "border", &tokens.border);
    append_number_map(out, "roundness", &tokens.rounded);
    append_shadow_map(out, "elevation", &tokens.shadow);
    out.push_str("typography:\n");
    out.push_str(&format!("  headings: {}\n", tokens.typography.headings));
    out.push_str(&format!("  body: {}\n", tokens.typography.body));
    out.push_str(&format!("  captions: {}\n", tokens.typography.captions));
    out.push_str(&format!("  data: {}\n", tokens.typography.data));
    append_string_map(out, "on", &tokens.on);
}

fn append_color_role_map(out: &mut String, title: &str, values: &BTreeMap<String, String>) {
    out.push_str(&format!("  {title}:\n"));
    for (key, value) in values {
        out.push_str(&format!("    {key}: {value}\n"));
    }
}

fn append_string_map(out: &mut String, title: &str, values: &BTreeMap<String, String>) {
    out.push_str(title);
    out.push_str(":\n");
    for (key, value) in values {
        out.push_str(&format!("  {key}: {value}\n"));
    }
}

fn append_number_map(out: &mut String, title: &str, values: &BTreeMap<String, f64>) {
    out.push_str(title);
    out.push_str(":\n");
    for (key, value) in values {
        out.push_str(&format!("  {key}: {}\n", format_number(*value)));
    }
}

fn append_shadow_map(out: &mut String, title: &str, values: &BTreeMap<String, Shadow>) {
    out.push_str(title);
    out.push_str(":\n");
    for (key, shadow) in values {
        out.push_str(&format!("  {key}:\n"));
        out.push_str(&format!("    type: {}\n", shadow.shadow_type));
        out.push_str(&format!("    color: {}\n", shadow.color));
        out.push_str(&format!(
            "    offsetX: {}\n",
            format_number(shadow.offset_x)
        ));
        out.push_str(&format!(
            "    offsetY: {}\n",
            format_number(shadow.offset_y)
        ));
        out.push_str(&format!("    blur: {}\n", format_number(shadow.blur)));
    }
}

fn append_contrast(out: &mut String, style_guide: &StyleGuide) {
    out.push_str("contrast:\n");
    if style_guide.contrast.violations.is_empty() {
        out.push_str("  status: pass\n");
        return;
    }
    out.push_str("  status: review\n");
    out.push_str("  violations:\n");
    for violation in &style_guide.contrast.violations {
        out.push_str(&format!("    - foreground: {}\n", violation.fg));
        out.push_str(&format!("      background: {}\n", violation.bg));
        out.push_str(&format!("      ratio: {:.2}\n", violation.ratio));
        out.push_str(&format!("      target: {:.2}\n", violation.target));
    }
}

fn format_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}")
    }
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

/// Snapshot constructor — mirrors `get_style_guide_snapshot` convention.
pub fn get_guidelines_snapshot() -> GetGuidelines {
    GetGuidelines
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai_skills::resolve_style::{resolve_style, Fonts, ResolveOutcome, StyleParams};

    fn call(topic: &str) -> ToolOutcome {
        let mut args = BTreeMap::new();
        args.insert("topic".into(), topic.into());
        get_guidelines_snapshot().call(&args)
    }

    fn style_args() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("category".into(), "style".into()),
            ("name".into(), "Atlas Grid".into()),
            ("colorPalette".into(), "Alloy Blue".into()),
            ("roundness".into(), "medium".into()),
            ("elevation".into(), "low".into()),
            ("headings".into(), "Inter".into()),
            ("body".into(), "Inter".into()),
            ("captions".into(), "Inter".into()),
            ("data".into(), "IBM Plex Mono".into()),
            (
                "decorativeImagery".into(),
                "product diagrams only when they clarify state".into(),
            ),
        ])
    }

    fn style_params() -> StyleParams {
        StyleParams {
            color_palette: "Alloy Blue".into(),
            roundness: "medium".into(),
            elevation: "low".into(),
            fonts: Fonts {
                headings: "Inter".into(),
                body: "Inter".into(),
                captions: "Inter".into(),
                data: "IBM Plex Mono".into(),
            },
            decorative_imagery: Some("product diagrams only when they clarify state".into()),
        }
    }

    #[test]
    fn get_guidelines_style_roundtrips_our_style() {
        let expected = match resolve_style("Atlas Grid", &style_params()) {
            ResolveOutcome::Hit(guide) => guide,
            other => panic!("expected Atlas Grid hit, got {other:?}"),
        };

        match get_guidelines_snapshot().call(&style_args()) {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("category").map(String::as_str), Some("style"));
                assert_eq!(out.get("name").map(String::as_str), Some("Atlas Grid"));
                let content = out.get("content").expect("content field");
                assert!(
                    content.contains("Atlas Grid is a practical workspace style"),
                    "style prose must be present: {content}"
                );
                assert!(content.contains("colorPalette:"), "{content}");
                assert!(content.contains("roundness:"), "{content}");
                assert!(content.contains("elevation:"), "{content}");
                assert!(content.contains("typography:"), "{content}");
                assert!(content.contains("on:"), "{content}");

                let surface_primary = expected
                    .tokens
                    .surface
                    .get("primary")
                    .expect("surface.primary token");
                let on_surface_primary = expected
                    .tokens
                    .on
                    .get("on-surface.primary")
                    .expect("on-surface.primary token");
                assert!(
                    content.contains(surface_primary),
                    "surface.primary token value must roundtrip: {content}"
                );
                assert!(
                    content.contains(on_surface_primary),
                    "computed on-surface.primary token value must roundtrip: {content}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn get_guidelines_guide_topic_unchanged() {
        match call("web-app") {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("topic").map(String::as_str), Some("web-app"));
                assert!(
                    !out.contains_key("category"),
                    "default guide path should preserve the existing envelope: {out:?}"
                );
                assert_eq!(
                    out.get("content"),
                    op_ai_skills::guideline_for("web-app").as_ref(),
                    "default guide path should return the same guideline content"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn web_app_returns_non_empty_content_with_purpose_first() {
        match call("web-app") {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("topic").map(String::as_str), Some("web-app"));
                let content = out.get("content").expect("content field");
                assert!(!content.is_empty());
                // Verbatim phrase from product-principles.md.
                assert!(
                    content.contains("PURPOSE FIRST"),
                    "web-app guideline must contain PURPOSE FIRST: {content}"
                );
                // Verbatim phrase from design-principles.md.
                assert!(
                    content.contains("DESIGN CRAFT"),
                    "web-app guideline must contain DESIGN CRAFT: {content}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn mobile_returns_three_section_architecture_content() {
        match call("mobile") {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("topic").map(String::as_str), Some("mobile"));
                let content = out.get("content").expect("content field");
                assert!(!content.is_empty());
                // Verbatim phrase from mobile-app.md.
                assert!(
                    content.contains("THREE-SECTION ARCHITECTURE"),
                    "mobile guideline must contain THREE-SECTION ARCHITECTURE: {content}"
                );
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn code_to_design_returns_five_step_content() {
        match call("code-to-design") {
            ToolOutcome::Ok(out) => {
                assert_eq!(out.get("topic").map(String::as_str), Some("code-to-design"));
                let content = out.get("content").expect("content field");
                assert!(content.contains("Five-step"), "{content}");
                assert!(content.contains("upsert_screen"), "{content}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn unknown_topic_returns_ok_with_error_key_mirroring_style_guide_shape() {
        // get_style_guide returns ToolOutcome::Ok { "error": "..." } for unknown
        // inputs — not a transport-level ToolOutcome::Err. We must match that.
        match call("unknown-topic") {
            ToolOutcome::Ok(out) => {
                assert!(
                    out.contains_key("error"),
                    "unknown topic must return an error key in the result map: {out:?}"
                );
                assert!(
                    !out.contains_key("content"),
                    "unknown topic must not return content: {out:?}"
                );
                let msg = out.get("error").unwrap();
                assert!(
                    msg.contains("unknown-topic"),
                    "error message must name the bad topic: {msg}"
                );
            }
            other => panic!("expected Ok(error), got {other:?}"),
        }
    }

    #[test]
    fn missing_topic_arg_returns_err() {
        let out = get_guidelines_snapshot().call(&BTreeMap::new());
        assert!(
            matches!(out, ToolOutcome::Err(ToolErrorCode::MissingArgument, _)),
            "missing topic must return MissingArgument: {out:?}"
        );
    }
}
