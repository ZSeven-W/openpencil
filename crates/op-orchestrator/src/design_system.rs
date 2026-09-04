//! `design_system.rs` — S4 A1 + B1: `DesignSystem` struct, parse, defaults,
//! LLM generator, variable seeding, and prompt context.
//!
//! Port of `design-system-generator.ts` (deleted in commit `0f12b6e9`),
//! re-keyed by the B1 shadcn dictionary: palette keys are the shadcn
//! token names (dash-less in the LLM JSON: `background`, `card`,
//! `muted-foreground`, `--color-success` …) and seeding emits `--`-prefixed
//! document variables that match the bundled `design-systems.json`
//! presets and `apply_design_system` (`--background`, `--card`,
//! `--color-success` …).
//!
//! Port map:
//! - L20-29: `generateDesignSystem` entry (→ `generate_design_system`).
//! - L40-100: `parseDesignSystem` + `tryParseDS` 4-stage fallback chain.
//! - L102-124: `DEFAULT_DESIGN_SYSTEM` constant values.
//! - L134-156: `designSystemToVariables` (→ `design_system_to_seed_commands`).
//! - L161-170: `designSystemToPromptContext` (→ `design_system_to_prompt_context`).
//! - L59-81: `DesignSystem` interface (via `ai-types.ts`).

use futures::StreamExt;
use op_editor_core::{EditorCommand, VariableScalarPayload};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::types::{AbortFlag, CallRequest, LlmChunk, LlmClient};

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
/// Port of the `DesignSystem` interface in `ai-types.ts:59-81`, with the
/// palette re-keyed to the B1 shadcn dictionary:
///
/// ```text
/// palette  — keyed color tokens (`background`, `foreground`, `card`,
///            `card-foreground`, `popover(-foreground)`, `primary(-foreground)`,
///            `secondary(-foreground)`, `muted(-foreground)`,
///            `accent(-foreground)`, `destructive(-foreground)`, `border`,
///            `input`, `ring`, the `sidebar-*` 8, the `color-<status>`
///            pairs, `scrim`)
/// typography — heading/body font names + type scale (px)
/// spacing  — unit grid size + scale steps (px)
/// radius   — corner-radius steps (px), seeded onto --radius-none/xs/m/l/pill
/// aesthetic — prose adjectives describing the visual style
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignSystem {
    /// Color palette tokens. Keys are shadcn token names without the
    /// leading dashes; seeding prefixes them with `--`.
    pub palette: BTreeMap<String, String>,
    pub typography: Typography,
    pub spacing: Spacing,
    /// Corner-radius scale in pixels. Port of `radius: number[]` in TS.
    pub radius: Vec<f64>,
    /// Short aesthetic description (e.g. "clean modern blue").
    pub aesthetic: String,
}

// ── DEFAULT_DESIGN_SYSTEM (mirrors TS L102-124) ───────────────────────────────

/// The palette keys the dictionary carries, in table order. Shared by
/// the default palette and `try_parse_ds`'s fill-missing loop.
const PALETTE_KEYS: &[&str] = &[
    "background",
    "foreground",
    "card",
    "card-foreground",
    "popover",
    "popover-foreground",
    "primary",
    "primary-foreground",
    "secondary",
    "secondary-foreground",
    "muted",
    "muted-foreground",
    "accent",
    "accent-foreground",
    "destructive",
    "destructive-foreground",
    "border",
    "input",
    "ring",
    "sidebar",
    "sidebar-foreground",
    "sidebar-primary",
    "sidebar-primary-foreground",
    "sidebar-accent",
    "sidebar-accent-foreground",
    "sidebar-border",
    "sidebar-ring",
    "--color-success",
    "--color-success-foreground",
    "--color-warning",
    "--color-warning-foreground",
    "--color-error",
    "--color-error-foreground",
    "--color-info",
    "--color-info-foreground",
    "scrim",
];

fn build_default() -> DesignSystem {
    let hexes: &[(&str, &str)] = &[
        ("background", "#F8FAFC"),
        ("foreground", "#0F172A"),
        ("card", "#FFFFFF"),
        ("card-foreground", "#0F172A"),
        ("popover", "#FFFFFF"),
        ("popover-foreground", "#0F172A"),
        ("primary", "#2563EB"),
        ("primary-foreground", "#FFFFFF"),
        ("secondary", "#F1F5F9"),
        ("secondary-foreground", "#0F172A"),
        ("muted", "#F1F5F9"),
        ("muted-foreground", "#64748B"),
        ("accent", "#F3F4F6"),
        ("accent-foreground", "#0F172A"),
        ("destructive", "#EF4444"),
        ("destructive-foreground", "#FFFFFF"),
        ("border", "#E2E8F0"),
        ("input", "#E2E8F0"),
        ("ring", "#2563EB"),
        ("sidebar", "#FFFFFF"),
        ("sidebar-foreground", "#0F172A"),
        ("sidebar-primary", "#2563EB"),
        ("sidebar-primary-foreground", "#FFFFFF"),
        ("sidebar-accent", "#F1F5F9"),
        ("sidebar-accent-foreground", "#0F172A"),
        ("sidebar-border", "#E2E8F0"),
        ("sidebar-ring", "#2563EB"),
        ("--color-success", "#10B981"),
        ("--color-success-foreground", "#FFFFFF"),
        ("--color-warning", "#F59E0B"),
        ("--color-warning-foreground", "#FFFFFF"),
        ("--color-error", "#EF4444"),
        ("--color-error-foreground", "#FFFFFF"),
        ("--color-info", "#3B82F6"),
        ("--color-info-foreground", "#FFFFFF"),
        ("scrim", "#00000080"),
    ];
    let palette = hexes
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

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
        radius: vec![4.0, 8.0, 12.0],
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

    // Build palette — fill missing keys with defaults. Accepts both the
    // dash-less key form taught by the skill and a literal `--token` key
    // (LLMs sometimes echo the CSS custom-property spelling).
    let mut palette = BTreeMap::new();
    for key in PALETTE_KEYS {
        let val = p
            .get(*key)
            .or_else(|| p.get(&format!("--{key}")))
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| default.palette.get(*key).cloned().unwrap_or_default());
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
    let content_start = after_open.strip_prefix("json").unwrap_or(after_open);

    // Skip leading newline
    let content_start = content_start.strip_prefix('\n').unwrap_or(content_start);

    // Find closing fence
    let fence_end = content_start.find("```")?;

    // Strip trailing newline before closing fence
    let inner = &content_start[..fence_end];
    let inner = inner.strip_suffix('\n').unwrap_or(inner);

    Some(inner)
}

// ── B1: generate_design_system (port of TS L20-29) ───────────────────────────

/// Generate a `DesignSystem` from a user prompt via a single LLM call.
///
/// Port of `generateDesignSystem` in `design-system-generator.ts:20-29`:
///  1. Loads the `design-system` skill as system prompt via
///     `op_ai_skills::get_skill_by_name`.
///  2. Makes a single (non-streaming) `LlmClient::call` with the user prompt.
///  3. Collects all text chunks, then parses via `parse_design_system`.
///  4. Falls back to `DEFAULT_DESIGN_SYSTEM` on any parse/LLM failure.
///
/// The `model` / `provider` / `abort` params match the existing `CallRequest`
/// shape used by `subagent.rs` and `prompt.rs`.
pub async fn generate_design_system(
    prompt: &str,
    llm: &dyn LlmClient,
    model: Option<&str>,
    provider: Option<&str>,
    abort: &AbortFlag,
) -> DesignSystem {
    // Load the design-system skill content as system prompt.
    let system_prompt = op_ai_skills::get_skill_by_name("design-system")
        .map(|e| e.content.as_str())
        .unwrap_or("")
        .to_string();

    let req = CallRequest {
        system_prompt,
        user_prompt: prompt.to_string(),
        model: model.map(|s| s.to_string()),
        provider: provider.map(|s| s.to_string()),
        timeout: std::time::Duration::from_secs(30),
        abort: abort.clone(),
        no_text_timeout: None,
        first_text_timeout: None,
    };

    // Collect all text chunks.
    let mut stream = llm.call(req);
    let mut text = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(LlmChunk::Text(t)) => text.push_str(&t),
            Ok(LlmChunk::Thinking(_)) => {}
            Err(_) => break,
        }
    }

    // Parse and return; falls back to DEFAULT on failure.
    parse_design_system(&text)
}

// ── B1: design_system_to_seed_commands (port of TS L134-156) ─────────────────

/// Convert a `DesignSystem` into `EditorCommand::SetVariable*` commands.
///
/// Port of `designSystemToVariables` in
/// `design-system-generator.ts:134-156`, re-keyed by B1. Emits:
///  - Palette: `--{key}` → `SetVariableColor` (one per palette entry;
///    keys are the shadcn token names sans leading dashes)
///  - Fonts: `--font-primary` (body font) + `--font-secondary`
///    (heading font) → `SetVariableScalar::String`
///  - Spacing scale: `spacing-{xs|sm|md|lg|xl|2xl|…}` →
///    `SetVariableScalar::Number` (capped at `spacingNames.length`)
///  - Radius: fixed `--radius-none` (0) + `--radius-pill` (999), with
///    the DS radius scale mapped onto `--radius-xs/m/l` (capped at 3)
///
/// The type SCALE is still not seeded (it reaches the LLM via
/// `design_system_to_prompt_context` only).
///
/// DEFAULT_DESIGN_SYSTEM emits exactly 49 commands:
/// 36 palette + 2 fonts + 6 spacing + 5 radius.
pub fn design_system_to_seed_commands(ds: &DesignSystem) -> Vec<EditorCommand> {
    let mut cmds = Vec::new();

    // Colors: palette → SetVariableColor named `--{key}`.
    // TS: `for (const [key, value] of Object.entries(ds.palette))`
    // Note: BTreeMap iterates in sorted key order, which is fine for determinism.
    for (key, value) in &ds.palette {
        let name = if key.starts_with("--") {
            key.clone()
        } else {
            format!("--{key}")
        };
        cmds.push(EditorCommand::SetVariableColor {
            name,
            hex: value.clone(),
        });
    }

    // Fonts: body font is the primary reading face, heading the
    // secondary display face.
    cmds.push(EditorCommand::SetVariableScalar {
        name: "--font-primary".into(),
        scalar: VariableScalarPayload::String(ds.typography.body_font.clone()),
    });
    cmds.push(EditorCommand::SetVariableScalar {
        name: "--font-secondary".into(),
        scalar: VariableScalarPayload::String(ds.typography.heading_font.clone()),
    });

    // Spacing scale → spacing-xs/sm/md/lg/xl/2xl/3xl/4xl/5xl/6xl
    // TS: `const spacingNames = ['xs', 'sm', 'md', 'lg', 'xl', '2xl', '3xl', '4xl', '5xl', '6xl']`
    const SPACING_NAMES: &[&str] = &[
        "xs", "sm", "md", "lg", "xl", "2xl", "3xl", "4xl", "5xl", "6xl",
    ];
    for (i, &label) in SPACING_NAMES
        .iter()
        .enumerate()
        .take(ds.spacing.scale.len())
    {
        let name = format!("spacing-{label}");
        cmds.push(EditorCommand::SetVariableScalar {
            name,
            scalar: VariableScalarPayload::Number(ds.spacing.scale[i]),
        });
    }

    // Radius → fixed --radius-none (0) + scale on --radius-xs/m/l
    // (capped at 3 steps) + fixed --radius-pill (999).
    const RADIUS_SCALE_NAMES: &[&str] = &["xs", "m", "l"];
    cmds.push(EditorCommand::SetVariableScalar {
        name: "--radius-none".into(),
        scalar: VariableScalarPayload::Number(0.0),
    });
    for (i, &label) in RADIUS_SCALE_NAMES.iter().enumerate().take(ds.radius.len()) {
        let name = format!("--radius-{label}");
        cmds.push(EditorCommand::SetVariableScalar {
            name,
            scalar: VariableScalarPayload::Number(ds.radius[i]),
        });
    }
    cmds.push(EditorCommand::SetVariableScalar {
        name: "--radius-pill".into(),
        scalar: VariableScalarPayload::Number(999.0),
    });

    cmds
}

// ── B1: design_system_to_prompt_context (port of TS L161-170) ────────────────

/// Build a fixed-form design-system context string for AI prompts.
///
/// Port of `designSystemToPromptContext` in `design-system-generator.ts:161-170`
/// (labels re-keyed to the B1 shadcn tokens):
/// ```text
/// DESIGN SYSTEM (use these values consistently):
/// Colors: background {bg}, foreground {fg}, card {card}, muted {muted}, ...
/// Fonts: heading "{headingFont}", body "{bodyFont}"
/// Type scale: {scale}px
/// Spacing: {scale}px ({unit}px grid)
/// Radius: {radius}px
/// Style: {aesthetic}
/// ```
pub fn design_system_to_prompt_context(ds: &DesignSystem) -> String {
    let p = &ds.palette;
    let get = |key: &str| p.get(key).map(|s| s.as_str()).unwrap_or("");

    // Type scale: "14, 16, 20, 28, 40, 56px"
    let type_scale = ds
        .typography
        .scale
        .iter()
        .map(|v| format_num(*v))
        .collect::<Vec<_>>()
        .join(", ");

    // Spacing scale: "8, 16, 24, 32, 48, 64px"
    let spacing_scale = ds
        .spacing
        .scale
        .iter()
        .map(|v| format_num(*v))
        .collect::<Vec<_>>()
        .join(", ");
    let spacing_unit = format_num(ds.spacing.unit);

    // Radius: "4, 8, 12px"
    let radius = ds
        .radius
        .iter()
        .map(|v| format_num(*v))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "DESIGN SYSTEM (use these values consistently):\n\
Colors: background {background}, foreground {foreground}, card {card}, card-foreground {card_foreground}, muted {muted}, muted-foreground {muted_foreground}, primary {primary}, primary-foreground {primary_foreground}, accent {accent}, border {border}, input {input}, ring {ring}\n\
Fonts: heading \"{heading}\", body \"{body}\"\n\
Type scale: {type_scale}px\n\
Spacing: {spacing_scale}px ({spacing_unit}px grid)\n\
Radius: {radius}px\n\
Style: {aesthetic}",
        background = get("background"),
        foreground = get("foreground"),
        card = get("card"),
        card_foreground = get("card-foreground"),
        muted = get("muted"),
        muted_foreground = get("muted-foreground"),
        primary = get("primary"),
        primary_foreground = get("primary-foreground"),
        accent = get("accent"),
        border = get("border"),
        input = get("input"),
        ring = get("ring"),
        heading = ds.typography.heading_font,
        body = ds.typography.body_font,
        type_scale = type_scale,
        spacing_scale = spacing_scale,
        spacing_unit = spacing_unit,
        radius = radius,
        aesthetic = ds.aesthetic,
    )
}

/// Format a float as an integer if it has no fractional part, or with 1
/// decimal place otherwise. Matches the JS number-to-string behaviour for
/// the values in the design system (all are whole numbers in practice).
fn format_num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

#[cfg(test)]
#[path = "design_system_tests.rs"]
mod tests;
