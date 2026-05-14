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
    /// Current selection per axis. e.g. {"mode": "dark"}. Empty map
    /// means "use the default (theme = None) entry of every Themed
    /// variable".
    pub active_theme: BTreeMap<String, String>,
}

impl VariableTable {
    /// Look up a variable by name. None if unknown.
    pub fn find(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }
    /// Resolve `$ref` against the table under the active theme. Returns
    /// the scalar leaf or None on unknown name / empty Themed.
    pub fn resolve(&self, name: &str) -> Option<&VariableScalar> {
        self.find(name)?.resolve(&self.active_theme)
    }
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
