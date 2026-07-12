//! Built-in design systems — complete shadcn-vocabulary variable presets
//! (`--background` / `--card` / `--primary` / `--muted` / the `--sidebar-*`
//! family, plus font and radius tokens), each with Light + Dark theme
//! values. Applying one gives a generated design a real token system to
//! reference (`$--primary`) and free light/dark theming, and aligns our
//! variable naming with the shadcn convention the wider ecosystem shares.

use std::sync::OnceLock;

const PRESETS_JSON: &str = include_str!("../skills/design-systems.json");

/// One built-in design system: its id and the raw `variables` object in the
/// exact TS shape `set_variables` accepts.
#[derive(Debug, Clone)]
pub struct DesignSystemPreset {
    pub name: &'static str,
    /// JSON object string: `name -> { type, value }` definitions.
    pub variables_json: String,
    pub variable_count: usize,
}

fn presets() -> &'static Vec<DesignSystemPreset> {
    static CACHE: OnceLock<Vec<DesignSystemPreset>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let root: serde_json::Value =
            serde_json::from_str(PRESETS_JSON).expect("bundled design-systems.json parses");
        let mut out = Vec::new();
        for name in ["halo", "lunaris", "nitro", "shadcn"] {
            let Some(vars) = root.get(name).and_then(|ds| ds.get("variables")) else {
                continue;
            };
            out.push(DesignSystemPreset {
                name: match name {
                    "halo" => "halo",
                    "lunaris" => "lunaris",
                    "nitro" => "nitro",
                    _ => "shadcn",
                },
                variables_json: vars.to_string(),
                variable_count: vars.as_object().map(|o| o.len()).unwrap_or(0),
            });
        }
        out
    })
}

/// All built-in design systems.
pub fn design_system_presets() -> &'static [DesignSystemPreset] {
    presets()
}

/// Look up one preset by (case-insensitive) name.
pub fn design_system_preset(name: &str) -> Option<&'static DesignSystemPreset> {
    let lowered = name.trim().to_ascii_lowercase();
    presets().iter().find(|preset| preset.name == lowered)
}

/// The theme axis every preset shares.
pub fn design_system_themes_json() -> &'static str {
    r#"{"Mode":["Light","Dark"]}"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_four_presets_load_with_shadcn_core_tokens() {
        let presets = design_system_presets();
        assert_eq!(presets.len(), 4);
        for preset in presets {
            assert!(preset.variable_count >= 28, "{}", preset.name);
            for token in [
                "--background",
                "--primary",
                "--muted-foreground",
                "--sidebar",
            ] {
                assert!(
                    preset.variables_json.contains(token),
                    "{} missing {token}",
                    preset.name
                );
            }
        }
        assert!(design_system_preset("HALO").is_some());
        assert!(design_system_preset("nope").is_none());
    }
}
