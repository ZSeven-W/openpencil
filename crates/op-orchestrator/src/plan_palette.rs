//! An explicit palette shared by planning, generation, and owned components.
//!
//! Only named design roles are accepted. This is not a color-similarity repair:
//! status, chart, image, and authored node colors are never rewritten here.

use op_util::hex_color::{parse_hex_rgba8, HexOptions};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// A coverage retry repairs missing sections, not the established palette.
/// Retain roles if the model omits them; an explicit revised palette wins.
pub(crate) fn preserve_on_replan(
    previous: &crate::plan::OrchestratorPlan,
    revised: &mut crate::plan::OrchestratorPlan,
    request: &crate::types::DesignRequest,
) {
    if revised.palette.is_none() && request.design_md.is_none() {
        revised.palette = previous.palette.clone();
        crate::plan_repair::finalize_plan(revised, None, None);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanPalette {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_foreground: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
}

impl PlanPalette {
    /// Canonical roles win; accept the same misplaced root palette on both
    /// strict parsing and JSON-repair paths.
    pub(crate) fn from_plan_value(value: &Value) -> Option<Self> {
        Self::from_value(&value["palette"])
            .or_else(|| Self::from_value(&value["rootFrame"]["palette"]))
    }

    /// Parse supported roles independently, so one malformed value does not
    /// discard a valid brand color or make the otherwise valid plan fail.
    pub fn from_value(value: &Value) -> Option<Self> {
        let get = |key: &str| value.get(key)?.as_str().and_then(normalize_color);
        let palette = Self {
            background: get("background"),
            surface: get("surface"),
            primary: get("primary"),
            primary_foreground: get("primaryForeground"),
            text: get("text"),
            muted_text: get("mutedText"),
            border: get("border"),
        };
        (!palette.token_overrides().is_empty()).then_some(palette)
    }

    pub fn normalized(&self) -> Option<Self> {
        Self::from_value(&serde_json::to_value(self).ok()?)
    }

    /// The exact variable slots this declaration owns. No status or chart
    /// token is included, even if it happens to have the same concrete color.
    pub fn token_overrides(&self) -> Vec<(&'static str, String)> {
        let primary_foreground = self.resolved_primary_foreground();
        [
            ("--background", &self.background),
            ("--card", &self.surface),
            ("--primary", &self.primary),
            ("--primary-foreground", &primary_foreground),
            ("--foreground", &self.text),
            ("--muted-foreground", &self.muted_text),
            ("--border", &self.border),
        ]
        .into_iter()
        .filter_map(|(token, color)| {
            color
                .as_deref()
                .and_then(normalize_color)
                .map(|color| (token, color))
        })
        .collect()
    }

    /// A declared accent needs a readable on-color even when the planner only
    /// specifies the accent. Explicit foregrounds retain their ownership;
    /// absent/invalid foregrounds choose the higher WCAG contrast of black or
    /// white. Both prompt construction and seeding use this same resolution.
    fn resolved_primary_foreground(&self) -> Option<String> {
        self.primary_foreground
            .as_deref()
            .and_then(normalize_color)
            .or_else(|| {
                let primary = self.primary.as_deref().and_then(normalize_color)?;
                let black = op_design_lint::color_contrast(&primary, "#000000");
                let white = op_design_lint::color_contrast(&primary, "#FFFFFF");
                Some(if black >= white { "#000000" } else { "#FFFFFF" }.into())
            })
    }
}

fn normalize_color(color: &str) -> Option<String> {
    let [r, g, b, _] = parse_hex_rgba8(
        color,
        HexOptions {
            require_hash: true,
            allow_rgb_shorthand: true,
            allow_rgba_shorthand: false,
            allow_alpha: false,
        },
    )?;
    Some(format!("#{r:02X}{g:02X}{b:02X}"))
}

pub(crate) fn deserialize_palette<'de, D>(deserializer: D) -> Result<Option<PlanPalette>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(PlanPalette::from_value(&Value::deserialize(deserializer)?))
}

#[cfg(test)]
#[path = "plan_palette_tests.rs"]
mod tests;
