//! `design_system.rs` — S4 A1: `DesignSystem` struct, `DEFAULT_DESIGN_SYSTEM`
//! const, and `parse_design_system` JSON-cleaning fallback chain.
//!
//! Port of `design-system-generator.ts` (deleted in commit `0f12b6e9`),
//! specifically:
//! - L40-100: `parseDesignSystem` + `tryParseDS` 4-stage fallback chain.
//! - L102-124: `DEFAULT_DESIGN_SYSTEM` constant values.
//! - L59-81: `DesignSystem` interface (via `ai-types.ts`).
//!
//! No LLM call yet — that is added in Task B1.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

// ── DesignSystem struct (mirrors TS ai-types.ts:59-81) ────────────────────────

/// Typography section of the design system.
///
/// Port of `DesignSystem.typography` in `ai-types.ts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Typography {
    pub heading_font: String,
    pub body_font: String,
    pub scale: Vec<f64>,
}

/// Spacing section of the design system.
///
/// Port of `DesignSystem.spacing` in `ai-types.ts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spacing {
    pub unit: f64,
    pub scale: Vec<f64>,
}

/// Structured design tokens (colors, typography, spacing, radius, aesthetic).
///
/// Port of the `DesignSystem` interface in `ai-types.ts:59-81`.
///
/// ```text
/// palette  — keyed color tokens (background, surface, text, textSecondary,
///            primary, primaryLight, accent, border)
/// typography — heading/body font names + type scale (px)
/// spacing  — unit grid size + scale steps (px)
/// radius   — corner-radius steps (px)
/// aesthetic — prose adjectives describing the visual style
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignSystem {
    /// Color palette tokens. Keys match TS camelCase names
    /// (`background`, `surface`, `text`, `textSecondary`, `primary`,
    /// `primaryLight`, `accent`, `border`).
    pub palette: BTreeMap<String, String>,
    pub typography: Typography,
    pub spacing: Spacing,
    /// Corner-radius scale in pixels. Port of `radius: number[]` in TS.
    pub radius: Vec<f64>,
    /// Short aesthetic description (e.g. "clean modern blue").
    pub aesthetic: String,
}

// ── DEFAULT_DESIGN_SYSTEM (mirrors TS L102-124) ───────────────────────────────

fn build_default() -> DesignSystem {
    let mut palette = BTreeMap::new();
    palette.insert("background".into(), "#F8FAFC".into());
    palette.insert("surface".into(), "#FFFFFF".into());
    palette.insert("text".into(), "#0F172A".into());
    palette.insert("textSecondary".into(), "#475569".into());
    palette.insert("primary".into(), "#2563EB".into());
    palette.insert("primaryLight".into(), "#DBEAFE".into());
    palette.insert("accent".into(), "#0EA5E9".into());
    palette.insert("border".into(), "#E2E8F0".into());

    DesignSystem {
        palette,
        typography: Typography {
            heading_font: "Space Grotesk".into(),
            body_font: "Inter".into(),
            scale: vec![14.0, 16.0, 20.0, 28.0, 40.0, 56.0],
        },
        spacing: Spacing {
            unit: 8.0,
            scale: vec![8.0, 16.0, 24.0, 32.0, 48.0, 64.0],
        },
        radius: vec![8.0, 12.0, 16.0],
        aesthetic: "clean modern blue".into(),
    }
}

/// Lazily-initialized default design system.
///
/// Mirrors `DEFAULT_DESIGN_SYSTEM` in `design-system-generator.ts:102-124`.
/// Using `OnceLock` gives a `&'static DesignSystem` that callers can deref
/// without cloning in the common case; callers that need an owned copy call
/// `.clone()`.
pub static DEFAULT_DESIGN_SYSTEM: OnceLock<DesignSystem> = OnceLock::new();

/// Convenience accessor — initialises on first call.
pub fn default_design_system() -> &'static DesignSystem {
    DEFAULT_DESIGN_SYSTEM.get_or_init(build_default)
}

// ── parse_design_system: 4-stage fallback chain (mirrors TS L40-100) ─────────

/// Parse a `DesignSystem` from LLM response text.
///
/// Tolerant 4-stage fallback chain — faithfully mirrors `parseDesignSystem`
/// in `design-system-generator.ts:40-100`:
///
/// 1. Direct `serde_json::from_str` of trimmed input.
/// 2. Strip code fences (` ```json ... ``` ` or ` ``` ... ``` `).
/// 3. Find first `{` and last `}`, substring, retry parse.
/// 4. Return `DEFAULT_DESIGN_SYSTEM` (cloned).
pub fn parse_design_system(text: &str) -> DesignSystem {
    let trimmed = text.trim();

    // Stage 1: direct parse
    if let Some(ds) = try_parse_ds(trimmed) {
        return ds;
    }

    // Stage 2: strip code fences (```json ... ``` or ``` ... ```)
    // Regex pattern: ```(?:json)?\s*\n?([\s\S]*?)\n?```
    if let Some(inner) = extract_code_fence(trimmed) {
        if let Some(ds) = try_parse_ds(inner.trim()) {
            return ds;
        }
    }

    // Stage 3: first `{` … last `}`
    if let (Some(first), Some(last)) = (trimmed.find('{'), trimmed.rfind('}')) {
        if last > first {
            if let Some(ds) = try_parse_ds(&trimmed[first..=last]) {
                return ds;
            }
        }
    }

    // Stage 4: fallback
    default_design_system().clone()
}

/// Try to parse JSON into a `DesignSystem`, returning `None` on any failure.
///
/// Port of `tryParseDS` in `design-system-generator.ts:62-100`:
/// validates that `palette` and `typography` fields are present objects,
/// then fills missing sub-fields with defaults.
fn try_parse_ds(json: &str) -> Option<DesignSystem> {
    let obj: serde_json::Value = serde_json::from_str(json).ok()?;

    // Must have palette object and typography object
    let p = obj.get("palette").and_then(|v| v.as_object())?;
    let t = obj.get("typography").and_then(|v| v.as_object())?;

    let default = default_design_system();

    // Build palette — fill missing keys with defaults
    let mut palette = BTreeMap::new();
    let dp = &default.palette;
    let palette_keys = [
        "background",
        "surface",
        "text",
        "textSecondary",
        "primary",
        "primaryLight",
        "accent",
        "border",
    ];
    for key in palette_keys {
        let val = p
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| dp.get(key).map(|s| s.as_str()).unwrap_or(""))
            .to_string();
        palette.insert(key.to_string(), val);
    }

    // Typography — fill missing fields with defaults
    let heading_font = t
        .get("headingFont")
        .and_then(|v| v.as_str())
        .unwrap_or(&default.typography.heading_font)
        .to_string();
    let body_font = t
        .get("bodyFont")
        .and_then(|v| v.as_str())
        .unwrap_or(&default.typography.body_font)
        .to_string();
    let type_scale: Vec<f64> = t
        .get("scale")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().map(|v| v.as_f64()).collect::<Option<Vec<_>>>())
        .unwrap_or_else(|| default.typography.scale.clone());

    // Spacing — optional, fill with defaults
    let s_obj = obj.get("spacing").and_then(|v| v.as_object());
    let spacing_unit = s_obj
        .and_then(|s| s.get("unit"))
        .and_then(|v| v.as_f64())
        .unwrap_or(default.spacing.unit);
    let spacing_scale: Vec<f64> = s_obj
        .and_then(|s| s.get("scale"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().map(|v| v.as_f64()).collect::<Option<Vec<_>>>())
        .unwrap_or_else(|| default.spacing.scale.clone());

    // Radius — optional array
    let radius: Vec<f64> = obj
        .get("radius")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().map(|v| v.as_f64()).collect::<Option<Vec<_>>>())
        .unwrap_or_else(|| default.radius.clone());

    // Aesthetic — optional string
    let aesthetic = obj
        .get("aesthetic")
        .and_then(|v| v.as_str())
        .unwrap_or(&default.aesthetic)
        .to_string();

    Some(DesignSystem {
        palette,
        typography: Typography {
            heading_font,
            body_font,
            scale: type_scale,
        },
        spacing: Spacing {
            unit: spacing_unit,
            scale: spacing_scale,
        },
        radius,
        aesthetic,
    })
}

/// Extract the content between code fences (` ```json ... ``` ` or ` ``` ... ``` `).
/// Returns `None` if no fence is found.
fn extract_code_fence(text: &str) -> Option<&str> {
    // Find opening fence: ``` optionally followed by "json"
    let fence_start = text.find("```")?;
    let after_open = &text[fence_start + 3..];

    // Skip optional "json" language tag and the following newline
    let content_start = if after_open.starts_with("json") {
        &after_open[4..]
    } else {
        after_open
    };

    // Skip leading newline
    let content_start = content_start.strip_prefix('\n').unwrap_or(content_start);

    // Find closing fence
    let fence_end = content_start.find("```")?;

    // Strip trailing newline before closing fence
    let inner = &content_start[..fence_end];
    let inner = inner.strip_suffix('\n').unwrap_or(inner);

    Some(inner)
}

#[cfg(test)]
#[path = "design_system_tests.rs"]
mod tests;
