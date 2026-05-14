//! Design variables + themes (color tokens, themed values).
//! Mirrors the canonical `jian_ops_schema::variable` model:
//!   - `Document.variables` — name → typed value (color/number/bool/string)
//!   - `Document.themes` — axis name → ordered list of values
//!   - `Document.active_theme` — current value per axis ("mode" → "dark")
//! Used by the canonical `.op` loader to round-trip designs that
//! depend on `$ref` color tokens + multi-axis themes (TS pen-core
//! `variables/resolve.ts` is the algorithmic equivalent).
//!
//! v1 scope: data preservation + lookup. Paint-time `$ref` resolution
//! lands in a follow-up alongside a Variables panel in the Right rail.

use std::collections::BTreeMap;

/// Variable type discriminator. Mirrors `VariableKind` in
/// jian_ops_schema for direct round-trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    Color,
    Number,
    Boolean,
    String,
}

/// Single typed scalar — the leaf value of a variable. Untagged
/// because `.op` JSON uses raw scalars: `"#ff0000"` / `12.5` / `true`.
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScalar {
    Bool(bool),
    Num(f64),
    Str(String),
}

/// A scalar value paired with the theme combination it applies to.
/// `theme = None` is the "default" entry used when no themed entry
/// matches the active selection.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemedValue {
    pub value: VariableScalar,
    /// Axis → axis-value map. e.g. `{"mode": "dark"}`. Match the active
    /// theme against this map to pick the right value.
    pub theme: Option<BTreeMap<String, String>>,
}

/// Variable value: either a single scalar (theme-agnostic) or a list
/// of themed alternatives.
#[derive(Debug, Clone, PartialEq)]
pub enum VariableValue {
    Scalar(VariableScalar),
    Themed(Vec<ThemedValue>),
}

/// A named variable definition. `name` is the lookup key referenced
/// by `$color-1` style refs in node fill / stroke fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    pub kind: VariableKind,
    pub value: VariableValue,
}

impl Variable {
    /// Resolve the variable's current scalar under `active_theme`.
    /// Themed values pick the entry whose `theme` map is a subset of
    /// `active_theme`; falls back to the entry with `theme = None`
    /// if no match. Returns None for empty `Themed([])`.
    pub fn resolve<'a>(&'a self, active_theme: &BTreeMap<String, String>) -> Option<&'a VariableScalar> {
        match &self.value {
            VariableValue::Scalar(s) => Some(s),
            VariableValue::Themed(entries) => {
                // First pass: pick the entry whose every theme axis
                // matches active_theme.
                for e in entries {
                    if let Some(t) = &e.theme {
                        if t.iter().all(|(k, v)| active_theme.get(k) == Some(v)) {
                            return Some(&e.value);
                        }
                    }
                }
                // Fallback: entry with theme = None.
                for e in entries {
                    if e.theme.is_none() {
                        return Some(&e.value);
                    }
                }
                None
            }
        }
    }
}

/// One theme axis (e.g. "mode" with values ["light", "dark"]).
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeAxis {
    pub name: String,
    pub values: Vec<String>,
}

/// Per-document variable + theme registry. Populated by the
/// canonical `.op` loader from `PenDocument.variables` /
/// `PenDocument.themes`. v1 preserves the data + supports lookup +
/// active-theme selection; paint-time `$ref` substitution is a
/// follow-up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VariableTable {
    pub variables: Vec<Variable>,
    pub themes: Vec<ThemeAxis>,
    /// Current selection per axis. e.g. {"mode": "dark"}.
    pub active_theme: BTreeMap<String, String>,
    /// Map of `node_id → variable name` for nodes whose fill is
    /// `$ref:name`. Paint reads this first, falls back to `node.fill`.
    /// Side-table avoids touching every `Node { ... }` literal.
    pub fill_refs: BTreeMap<super::NodeId, String>,
}

impl VariableTable {
    /// Register that `node_id`'s fill follows variable `ref_name`.
    /// Subsequent `fill_for(node_id)` looks the variable up under
    /// the current `active_theme`.
    pub fn set_fill_ref(&mut self, node_id: super::NodeId, ref_name: impl Into<String>) {
        self.fill_refs.insert(node_id, ref_name.into());
    }
    /// Resolve the paint-time fill color for `node_id`. Returns
    /// None when no `fill_ref` is registered or the referenced
    /// variable doesn't resolve (paint then falls back to
    /// `node.fill`).
    pub fn fill_for(&self, node_id: super::NodeId) -> Option<crate::Color> {
        let name = self.fill_refs.get(&node_id)?;
        self.resolve_color(name)
    }
    /// Look up a variable by name. None if unknown.
    pub fn find(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }
    /// Resolve `$ref` against the table under the active theme. Returns
    /// the scalar leaf or None on unknown name / empty Themed.
    pub fn resolve(&self, name: &str) -> Option<&VariableScalar> {
        self.find(name)?.resolve(&self.active_theme)
    }
    /// Resolve a `$ref` into a paintable `Color`. Returns None when
    /// the variable is unknown, isn't of `Color` kind, or its scalar
    /// isn't a parseable hex string (`#rgb`, `#rrggbb`, `#rrggbbaa`).
    /// Used by paint-time `$ref` substitution.
    pub fn resolve_color(&self, name: &str) -> Option<crate::Color> {
        let v = self.find(name)?;
        if !matches!(v.kind, VariableKind::Color) {
            return None;
        }
        let scalar = v.resolve(&self.active_theme)?;
        if let VariableScalar::Str(s) = scalar {
            return parse_hex_color(s);
        }
        None
    }
}

/// Parse `#rgb` / `#rrggbb` / `#rrggbbaa` into a `Color`. Mirrors the
/// TS paint helpers — lenient on case, requires the leading `#`.
fn parse_hex_color(s: &str) -> Option<crate::Color> {
    let s = s.trim().strip_prefix('#')?;
    let (r, g, b, a) = match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255)
        }
        6 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            u8::from_str_radix(&s[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some(crate::Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(name: &str, value: &str) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert(name.to_string(), value.to_string());
        m
    }

    #[test]
    fn scalar_variable_resolves_regardless_of_theme() {
        let v = Variable {
            name: "color-1".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
        };
        let theme = axis("mode", "dark");
        match v.resolve(&theme).unwrap() {
            VariableScalar::Str(s) => assert_eq!(s, "#ff0000"),
            _ => panic!(),
        }
    }

    #[test]
    fn themed_variable_picks_active_axis() {
        let v = Variable {
            name: "bg".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#ffffff".into()),
                    theme: Some(axis("mode", "light")),
                },
                ThemedValue {
                    value: VariableScalar::Str("#000000".into()),
                    theme: Some(axis("mode", "dark")),
                },
            ]),
        };
        let dark = axis("mode", "dark");
        let light = axis("mode", "light");
        match v.resolve(&dark).unwrap() {
            VariableScalar::Str(s) => assert_eq!(s, "#000000"),
            _ => panic!(),
        }
        match v.resolve(&light).unwrap() {
            VariableScalar::Str(s) => assert_eq!(s, "#ffffff"),
            _ => panic!(),
        }
    }

    #[test]
    fn themed_variable_falls_back_to_default_when_no_match() {
        let v = Variable {
            name: "accent".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#888888".into()),
                    theme: None,
                },
                ThemedValue {
                    value: VariableScalar::Str("#000000".into()),
                    theme: Some(axis("mode", "dark")),
                },
            ]),
        };
        let light = axis("mode", "light");
        match v.resolve(&light).unwrap() {
            VariableScalar::Str(s) => assert_eq!(s, "#888888"),
            _ => panic!(),
        }
    }

    #[test]
    fn resolve_color_parses_rrggbb_hex() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "accent".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff8040".into())),
        });
        let c = tbl.resolve_color("accent").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - (128.0 / 255.0)).abs() < 0.01);
        assert!((c.b - (64.0 / 255.0)).abs() < 0.01);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn resolve_color_picks_themed_active_value() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "bg".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#ffffff".into()),
                    theme: Some(axis("mode", "light")),
                },
                ThemedValue {
                    value: VariableScalar::Str("#000000".into()),
                    theme: Some(axis("mode", "dark")),
                },
            ]),
        });
        tbl.active_theme = axis("mode", "dark");
        let c = tbl.resolve_color("bg").unwrap();
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn resolve_color_rejects_non_color_variables() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "spacing".into(),
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(12.0)),
        });
        assert!(tbl.resolve_color("spacing").is_none());
    }

    #[test]
    fn fill_for_resolves_registered_node_ref_to_themed_color() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "accent".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#ff0000".into()),
                    theme: Some(axis("mode", "light")),
                },
                ThemedValue {
                    value: VariableScalar::Str("#00ff00".into()),
                    theme: Some(axis("mode", "dark")),
                },
            ]),
        });
        let node = crate::document::NodeId::new(42);
        tbl.set_fill_ref(node, "accent");
        tbl.active_theme = axis("mode", "dark");
        let c = tbl.fill_for(node).unwrap();
        assert!((c.g - 1.0).abs() < 0.01, "dark mode → green; got {c:?}");
        // Switch theme; same ref resolves differently.
        tbl.active_theme = axis("mode", "light");
        let c2 = tbl.fill_for(node).unwrap();
        assert!((c2.r - 1.0).abs() < 0.01, "light mode → red; got {c2:?}");
    }

    #[test]
    fn fill_for_returns_none_when_no_ref_registered() {
        let tbl = super::VariableTable::default();
        assert!(tbl.fill_for(crate::document::NodeId::new(99)).is_none());
    }

    #[test]
    fn resolve_color_rejects_invalid_hex() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "broken".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("not-hex".into())),
        });
        assert!(tbl.resolve_color("broken").is_none());
    }

    #[test]
    fn themed_variable_returns_none_when_empty_and_no_default() {
        let v = Variable {
            name: "x".into(),
            kind: VariableKind::Number,
            value: VariableValue::Themed(vec![]),
        };
        let theme = BTreeMap::new();
        assert!(v.resolve(&theme).is_none());
    }
}
