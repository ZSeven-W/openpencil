//! Validation-fix whitelist and validators.
//!
//! Faithful port of
//! `apps/web/src/services/ai/design-validation-fixes.ts:67-157`
//! (`SAFE_FIX_PROPERTIES`, `isValidFixValue`, `isValidStructuralFix`).
//!
//! This module contains the whitelist + validation helpers (Task B1)
//! and wires in the fix-application submodule (Task B2).
//! `buildNodeFromSpec` is Task B3.

#![allow(dead_code)]

use serde_json::Value;

// ── Value-shape kinds ──────────────────────────────────────────────────────────

/// The shape / type category used to validate a fix value for one property.
///
/// Port of the union type used as the value in `SAFE_FIX_PROPERTIES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueKind {
    /// Any finite number (no sign constraint — some properties e.g.
    /// `letterSpacing` / `lineHeight` may be non-negative but the TS
    /// validator only checks `typeof value === 'number'`).
    Number,
    /// A finite non-negative number **or** one of the sizing keyword strings
    /// (`"fill_container"`, `"fit_content"`).
    Sizing,
    /// A finite number **or** an array of finite numbers.
    NumberOrArray,
    /// A font-weight integer: one of {100,200,300,400,500,600,700,800,900}.
    FontWeight,
    /// A hex-color string matching
    /// `#[0-9a-fA-F]{3}`, `#[0-9a-fA-F]{4}`, `#[0-9a-fA-F]{6}`,
    /// or `#[0-9a-fA-F]{8}`.
    Color,
    /// One of `{"left","center","right"}`.
    EnumTextAlign,
    /// One of `{"auto","fixed-width","fixed-width-height"}`.
    EnumTextGrowth,
    /// One of `{"start","center","end"}`.
    EnumAlign,
    /// One of `{"start","center","end","space_between","space_around"}`.
    EnumJustify,
}

// ── SAFE_FIX_PROPERTIES ────────────────────────────────────────────────────────

/// One entry in the safe-property whitelist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SafeFixProperty {
    pub name: &'static str,
    pub kind: ValueKind,
}

/// All 17 whitelisted fix properties.
///
/// Faithful port of `SAFE_FIX_PROPERTIES` in
/// `design-validation-fixes.ts:67-96`.
pub(crate) const SAFE_FIX_PROPERTIES: &[SafeFixProperty] = &[
    SafeFixProperty {
        name: "width",
        kind: ValueKind::Sizing,
    },
    SafeFixProperty {
        name: "height",
        kind: ValueKind::Sizing,
    },
    SafeFixProperty {
        name: "padding",
        kind: ValueKind::NumberOrArray,
    },
    SafeFixProperty {
        name: "gap",
        kind: ValueKind::Number,
    },
    SafeFixProperty {
        name: "fontSize",
        kind: ValueKind::Number,
    },
    SafeFixProperty {
        name: "fontWeight",
        kind: ValueKind::FontWeight,
    },
    SafeFixProperty {
        name: "letterSpacing",
        kind: ValueKind::Number,
    },
    SafeFixProperty {
        name: "lineHeight",
        kind: ValueKind::Number,
    },
    SafeFixProperty {
        name: "cornerRadius",
        kind: ValueKind::Number,
    },
    SafeFixProperty {
        name: "opacity",
        kind: ValueKind::Number,
    },
    SafeFixProperty {
        name: "fillColor",
        kind: ValueKind::Color,
    },
    SafeFixProperty {
        name: "strokeColor",
        kind: ValueKind::Color,
    },
    SafeFixProperty {
        name: "strokeWidth",
        kind: ValueKind::Number,
    },
    SafeFixProperty {
        name: "textAlign",
        kind: ValueKind::EnumTextAlign,
    },
    SafeFixProperty {
        name: "textGrowth",
        kind: ValueKind::EnumTextGrowth,
    },
    SafeFixProperty {
        name: "alignItems",
        kind: ValueKind::EnumAlign,
    },
    SafeFixProperty {
        name: "justifyContent",
        kind: ValueKind::EnumJustify,
    },
];

/// Returns the `ValueKind` for `property`, or `None` if not whitelisted.
pub(crate) fn property_kind(property: &str) -> Option<ValueKind> {
    SAFE_FIX_PROPERTIES
        .iter()
        .find(|p| p.name == property)
        .map(|p| p.kind)
}

// ── Constant sets (ported from TS `const` declarations) ─────────────────────

const VALID_SIZING_STRINGS: &[&str] = &["fill_container", "fit_content"];
const VALID_ALIGN: &[&str] = &["start", "center", "end"];
const VALID_JUSTIFY: &[&str] = &["start", "center", "end", "space_between", "space_around"];
const VALID_TEXT_ALIGN: &[&str] = &["left", "center", "right"];
const VALID_TEXT_GROWTH: &[&str] = &["auto", "fixed-width", "fixed-width-height"];
const VALID_FONT_WEIGHTS: &[u64] = &[100, 200, 300, 400, 500, 600, 700, 800, 900];

// ── Hex-color validation ───────────────────────────────────────────────────────

/// Returns `true` when `s` is a valid hex color string.
///
/// Accepted forms (matching the TS regex
/// `/^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/`):
/// - `#RGB`  (3 hex digits)
/// - `#RGBA` (4 hex digits)
/// - `#RRGGBB` (6 hex digits)
/// - `#RRGGBBAA` (8 hex digits)
fn is_valid_hex_color(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('#') else {
        return false;
    };
    let len = rest.len();
    if len != 3 && len != 4 && len != 6 && len != 8 {
        return false;
    }
    rest.chars().all(|c| c.is_ascii_hexdigit())
}

// ── is_valid_fix_value ─────────────────────────────────────────────────────────

/// Returns `true` when `value` is a valid fix value for `property`.
///
/// Returns `false` for unknown (non-whitelisted) properties.
///
/// Faithful port of `isValidFixValue` in
/// `design-validation-fixes.ts:110-141`.
pub(crate) fn is_valid_fix_value(property: &str, value: &Value) -> bool {
    let Some(kind) = property_kind(property) else {
        return false;
    };
    match kind {
        ValueKind::Number => value.is_number(),
        ValueKind::Sizing => {
            if let Some(n) = value.as_f64() {
                n.is_finite()
            } else if let Some(s) = value.as_str() {
                VALID_SIZING_STRINGS.contains(&s)
            } else {
                false
            }
        }
        ValueKind::NumberOrArray => {
            if value.is_number() {
                true
            } else if let Some(arr) = value.as_array() {
                arr.iter().all(|v| v.is_number())
            } else {
                false
            }
        }
        ValueKind::FontWeight => {
            if let Some(n) = value.as_u64() {
                VALID_FONT_WEIGHTS.contains(&n)
            } else {
                false
            }
        }
        ValueKind::Color => {
            if let Some(s) = value.as_str() {
                is_valid_hex_color(s)
            } else {
                false
            }
        }
        ValueKind::EnumTextAlign => {
            if let Some(s) = value.as_str() {
                VALID_TEXT_ALIGN.contains(&s)
            } else {
                false
            }
        }
        ValueKind::EnumTextGrowth => {
            if let Some(s) = value.as_str() {
                VALID_TEXT_GROWTH.contains(&s)
            } else {
                false
            }
        }
        ValueKind::EnumAlign => {
            if let Some(s) = value.as_str() {
                VALID_ALIGN.contains(&s)
            } else {
                false
            }
        }
        ValueKind::EnumJustify => {
            if let Some(s) = value.as_str() {
                VALID_JUSTIFY.contains(&s)
            } else {
                false
            }
        }
    }
}

// ── is_valid_structural_fix ────────────────────────────────────────────────────

const VALID_NODE_TYPES: &[&str] = &["frame", "text", "path", "rectangle", "ellipse"];

/// Returns `true` when `fix` is a well-formed structural fix object.
///
/// Accepted shapes:
/// - `{action: "addChild", parentId: <non-empty string>, node: {type: <valid>, ...}}`
///   with optional `index: number`.
/// - `{action: "removeNode", nodeId: <non-empty string>}`.
///
/// Faithful port of `isValidStructuralFix` in
/// `design-validation-fixes.ts:143-157`.
pub(crate) fn is_valid_structural_fix(fix: &Value) -> bool {
    let Some(obj) = fix.as_object() else {
        return false;
    };
    match obj.get("action").and_then(|v| v.as_str()) {
        Some("addChild") => {
            // parentId must be a non-empty string
            let parent_id_ok = obj
                .get("parentId")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            if !parent_id_ok {
                return false;
            }
            // node must be an object with a valid `type` string
            let node_ok = obj
                .get("node")
                .and_then(|v| v.as_object())
                .and_then(|n| n.get("type"))
                .and_then(|t| t.as_str())
                .map(|t| VALID_NODE_TYPES.contains(&t))
                .unwrap_or(false);
            node_ok
        }
        Some("removeNode") => obj
            .get("nodeId")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        _ => false,
    }
}

// ── B2: Fix application (split to stay under 800-line ceiling) ───────────────

#[path = "validation_fixes_apply.rs"]
pub(crate) mod apply;

// Re-exported for call sites outside this module (vision pass, B3, tests).
#[allow(unused_imports)]
pub(crate) use apply::{
    apply_validation_fixes, auto_fix_parent_layout_after_add_child, ApplyResult, StructuralFix,
    ValidationFix,
};

#[cfg(test)]
#[path = "validation_fixes_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "validation_fixes_b2_tests.rs"]
mod tests_b2;

#[cfg(test)]
#[path = "validation_fixes_b3_tests.rs"]
mod tests_b3;
