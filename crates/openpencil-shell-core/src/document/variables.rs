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
    /// Map of `node_id → variable name` for nodes whose stroke colour
    /// follows a `$ref`. Parallel to `fill_refs`; paint pre-resolves
    /// via `stroke_color_for(node_id)`.
    pub stroke_refs: BTreeMap<super::NodeId, String>,
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
    /// Stroke parallel to `set_fill_ref` — registers the variable
    /// driving a node's stroke colour at paint time.
    pub fn set_stroke_ref(&mut self, node_id: super::NodeId, ref_name: impl Into<String>) {
        self.stroke_refs.insert(node_id, ref_name.into());
    }
    /// Resolve the paint-time stroke color for `node_id`. Same shape
    /// as `fill_for`; paint falls back to `node.stroke.color` when
    /// None.
    pub fn stroke_color_for(&self, node_id: super::NodeId) -> Option<crate::Color> {
        let name = self.stroke_refs.get(&node_id)?;
        self.resolve_color(name)
    }
    /// Set the active value of a theme axis (e.g. `("mode", "dark")`).
    /// Mirrors the TS theme picker — flips colours across every
    /// themed variable in one call. Paint reflows on the next frame.
    pub fn set_active_theme(&mut self, axis: impl Into<String>, value: impl Into<String>) {
        self.active_theme.insert(axis.into(), value.into());
    }
    /// Remove an axis from the active theme map; subsequent
    /// resolutions fall back to the variable's `theme: None` default.
    pub fn clear_active_axis(&mut self, axis: &str) {
        self.active_theme.remove(axis);
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

    /// Mutable lookup parallel to `find`. Returns None when no
    /// variable with that name exists. Editor surfaces use this to
    /// stage an in-progress write back into the live document.
    pub fn find_mut(&mut self, name: &str) -> Option<&mut Variable> {
        self.variables.iter_mut().find(|v| v.name == name)
    }

    /// Write a new hex string into a `Color`-kind variable. Returns
    /// `true` when the variable existed AND was Color-kind AND the
    /// hex parsed cleanly; `false` otherwise (no mutation). For
    /// themed variables this overwrites the entry matching the
    /// current `active_theme` (or creates one if absent); for
    /// scalar variables it overwrites the single value.
    ///
    /// The ColorPicker commits through this helper when the
    /// VariablesPanel routes a row click into the picker — the
    /// model-layer write path is unified across "edit a node's
    /// fill" and "edit a variable", so paint sees the change on
    /// the next frame regardless of which surface the user touched.
    pub fn set_color_hex(&mut self, name: &str, hex: &str) -> bool {
        // Validate the hex up front so a malformed input never
        // corrupts the stored scalar.
        if parse_hex_color(hex).is_none() {
            return false;
        }
        let active = self.active_theme.clone();
        let var = match self.find_mut(name) {
            Some(v) if matches!(v.kind, VariableKind::Color) => v,
            _ => return false,
        };
        let normalized = hex.trim().to_string();
        match &mut var.value {
            VariableValue::Scalar(s) => {
                *s = VariableScalar::Str(normalized);
                true
            }
            VariableValue::Themed(entries) => {
                // Write-routing rules — the resolve walk picks an
                // entry; we must update THAT entry when it's a
                // subset match for the active theme. When no subset
                // entry matches, the user's intent depends on the
                // active theme:
                //
                //   - active_theme EMPTY: the user is editing with
                //     no theme axis selected, so the write targets
                //     the `theme: None` default entry (the resolve
                //     fallback). Creates one if absent.
                //
                //   - active_theme NON-EMPTY: the user is editing
                //     under a specific theme combo. The edit must
                //     scope to THAT combo. Pushing a new entry keyed
                //     to active_theme is correct; mutating the
                //     `theme: None` default would clobber the value
                //     under every OTHER theme axis too (codex stop-
                //     gate flag — fallback writes were silently
                //     reaching across themes).
                let subset_idx = entries.iter().position(|e| match &e.theme {
                    Some(t) => t.iter().all(|(k, v)| active.get(k) == Some(v)),
                    None => false,
                });
                if let Some(i) = subset_idx {
                    entries[i].value = VariableScalar::Str(normalized);
                    return true;
                }
                if active.is_empty() {
                    // No theme selected — target the default entry.
                    let default_idx = entries.iter().position(|e| e.theme.is_none());
                    if let Some(i) = default_idx {
                        entries[i].value = VariableScalar::Str(normalized);
                    } else {
                        entries.push(ThemedValue {
                            value: VariableScalar::Str(normalized),
                            theme: None,
                        });
                    }
                    return true;
                }
                // Active theme set + no subset match — push a new
                // entry keyed to the active theme. The default entry
                // (if any) stays untouched so other axes keep their
                // original values.
                //
                // Insert at the FRONT so the resolve walk reaches
                // the new entry before any pre-existing themed entry
                // whose theme is a superset of ours (resolve uses
                // subset matching against active, so a more-specific
                // entry placed first wins under the active theme).
                entries.insert(
                    0,
                    ThemedValue {
                        value: VariableScalar::Str(normalized),
                        theme: Some(active),
                    },
                );
                true
            }
        }
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
    fn stroke_color_for_resolves_registered_ref() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "border".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#0000ff".into())),
        });
        let node = crate::document::NodeId::new(7);
        tbl.set_stroke_ref(node, "border");
        let c = tbl.stroke_color_for(node).unwrap();
        assert!((c.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn set_active_theme_round_trips_through_axis_picker() {
        let mut tbl = super::VariableTable::default();
        tbl.set_active_theme("mode", "dark");
        assert_eq!(tbl.active_theme.get("mode"), Some(&"dark".to_string()));
        tbl.set_active_theme("density", "comfortable");
        assert_eq!(tbl.active_theme.len(), 2);
        tbl.clear_active_axis("mode");
        assert!(!tbl.active_theme.contains_key("mode"));
        assert_eq!(tbl.active_theme.len(), 1);
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

    #[test]
    fn set_color_hex_writes_scalar_variable() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "color-1".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
        });
        assert!(tbl.set_color_hex("color-1", "#00ff00"));
        assert_eq!(
            tbl.resolve_color("color-1"),
            Some(crate::Color {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            })
        );
    }

    #[test]
    fn set_color_hex_rejects_malformed_input() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "color-1".into(),
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#ff0000".into())),
        });
        assert!(!tbl.set_color_hex("color-1", "not-hex"));
        // Scalar must be unchanged after a rejected write.
        assert_eq!(
            tbl.resolve_color("color-1"),
            Some(crate::Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            })
        );
    }

    #[test]
    fn set_color_hex_returns_false_for_unknown_variable() {
        let mut tbl = super::VariableTable::default();
        assert!(!tbl.set_color_hex("does-not-exist", "#ffffff"));
    }

    #[test]
    fn set_color_hex_returns_false_for_wrong_kind() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "spacing".into(),
            kind: VariableKind::Number,
            value: VariableValue::Scalar(VariableScalar::Num(16.0)),
        });
        // Number-kind variable: write rejected.
        assert!(!tbl.set_color_hex("spacing", "#ffffff"));
    }

    #[test]
    fn set_color_hex_changes_resolved_value_under_subset_active_theme() {
        // Codex stop-gate regression: when the themed entry's theme
        // is a SUBSET of the active theme (e.g. entry only specifies
        // `mode: dark`; active is `mode: dark, density: compact`),
        // `resolve` still picks that entry — so a write must update
        // that entry in place, NOT push a new entry at the end that
        // `resolve` will never reach.
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "bg".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![ThemedValue {
                value: VariableScalar::Str("#111111".into()),
                theme: Some({
                    let mut m = BTreeMap::new();
                    m.insert("mode".into(), "dark".into());
                    m
                }),
            }]),
        });
        tbl.set_active_theme("mode", "dark");
        tbl.set_active_theme("density", "compact");
        // Sanity: resolves through the subset-matching entry.
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0x11 as f32 / 255.0,
                g: 0x11 as f32 / 255.0,
                b: 0x11 as f32 / 255.0,
                a: 1.0,
            })
        );
        // Write under the wider active theme. The original entry
        // (only mode=dark) is the resolved one — the write MUST go
        // through it, not append a new entry that resolve never
        // reaches.
        assert!(tbl.set_color_hex("bg", "#22ff44"));
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0x22 as f32 / 255.0,
                g: 1.0,
                b: 0x44 as f32 / 255.0,
                a: 1.0,
            }),
            "themed write must update the entry resolve picks"
        );
    }

    #[test]
    fn set_color_hex_under_active_theme_does_not_clobber_default() {
        // Codex stop-gate (round 2): when active_theme is non-empty
        // and no subset-matching themed entry exists, the write
        // must scope to the active theme (push a new entry), NOT
        // mutate the `theme: None` default. Mutating the default
        // would silently change the value under EVERY OTHER theme
        // axis the user hasn't explicitly authored, which is
        // never what an editor user clicking a colour swatch in
        // dark mode means.
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "bg".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#ffffff".into()),
                    theme: Some({
                        let mut m = BTreeMap::new();
                        m.insert("mode".into(), "light".into());
                        m
                    }),
                },
                ThemedValue {
                    value: VariableScalar::Str("#888888".into()),
                    theme: None,
                },
            ]),
        });
        tbl.set_active_theme("mode", "dark"); // matches neither
        // Pre-write: resolves to the default (#888) under dark.
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0x88 as f32 / 255.0,
                g: 0x88 as f32 / 255.0,
                b: 0x88 as f32 / 255.0,
                a: 1.0,
            })
        );
        // Write under active=dark must scope to dark only.
        assert!(tbl.set_color_hex("bg", "#000000"));
        // Under dark — new value reads back.
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
        );
        // The default entry must be untouched. Flipping to a
        // never-authored axis value (e.g. mode=sepia) should still
        // resolve to the original default #888888, NOT the new dark
        // value.
        tbl.set_active_theme("mode", "sepia");
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0x88 as f32 / 255.0,
                g: 0x88 as f32 / 255.0,
                b: 0x88 as f32 / 255.0,
                a: 1.0,
            }),
            "default entry must NOT have been clobbered by the dark write"
        );
        // Light entry also untouched.
        tbl.set_active_theme("mode", "light");
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }),
            "light entry must NOT have been clobbered"
        );
    }

    #[test]
    fn set_color_hex_empty_active_theme_targets_default_entry() {
        // The flip side: when active_theme IS empty (no axis
        // selected), writes target the default `theme: None`
        // entry — that's what resolve falls back to in this state.
        // Editing with no theme selected is the explicit "edit the
        // universal value" path.
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "bg".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![ThemedValue {
                value: VariableScalar::Str("#888888".into()),
                theme: None,
            }]),
        });
        // No active theme set.
        assert!(tbl.set_color_hex("bg", "#aabbcc"));
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0xaa as f32 / 255.0,
                g: 0xbb as f32 / 255.0,
                b: 0xcc as f32 / 255.0,
                a: 1.0,
            })
        );
    }

    #[test]
    fn set_color_hex_writes_themed_entry_matching_active_axis() {
        let mut tbl = super::VariableTable::default();
        tbl.variables.push(Variable {
            name: "bg".into(),
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#ffffff".into()),
                    theme: Some({
                        let mut m = BTreeMap::new();
                        m.insert("mode".into(), "light".into());
                        m
                    }),
                },
                ThemedValue {
                    value: VariableScalar::Str("#000000".into()),
                    theme: Some({
                        let mut m = BTreeMap::new();
                        m.insert("mode".into(), "dark".into());
                        m
                    }),
                },
            ]),
        });
        tbl.set_active_theme("mode", "dark");
        // Resolves to dark first.
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            })
        );
        // Write under the active theme → updates only the dark entry.
        assert!(tbl.set_color_hex("bg", "#3344ff"));
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 0x33 as f32 / 255.0,
                g: 0x44 as f32 / 255.0,
                b: 1.0,
                a: 1.0,
            })
        );
        // Flip to light — original light entry untouched.
        tbl.set_active_theme("mode", "light");
        assert_eq!(
            tbl.resolve_color("bg"),
            Some(crate::Color {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            })
        );
    }
}
