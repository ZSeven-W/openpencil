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

    /// Apply an MCP write command against this table. Returns true
    /// when the command actually changed something (caller pushes
    /// an undo snapshot). False on validation failure — the command
    /// is already validated by the tool's `call`, so a `false`
    /// here means the underlying mutator rejected (unknown name /
    /// wrong kind / malformed hex / unknown axis).
    pub fn apply_mcp_command(&mut self, cmd: &crate::mcp::McpCommand) -> bool {
        match cmd {
            crate::mcp::McpCommand::SetVariableColor { name, hex } => {
                self.set_color_hex(name, hex)
            }
            crate::mcp::McpCommand::SetVariableScalar { name, scalar } => {
                let s = match scalar {
                    crate::mcp::VariableScalarPayload::Number(n) => VariableScalar::Num(*n),
                    crate::mcp::VariableScalarPayload::String(s) => {
                        VariableScalar::Str(s.clone())
                    }
                    crate::mcp::VariableScalarPayload::Boolean(b) => VariableScalar::Bool(*b),
                };
                self.set_scalar(name, s)
            }
            crate::mcp::McpCommand::SetActiveAxisValue { axis, value } => {
                let Some(theme_axis) = self.themes.iter().find(|t| t.name == *axis) else {
                    return false;
                };
                if !theme_axis.values.iter().any(|v| v == value) {
                    return false;
                }
                self.active_theme.insert(axis.clone(), value.clone());
                true
            }
            crate::mcp::McpCommand::InsertNode { .. }
            | crate::mcp::McpCommand::UpdateNode { .. }
            | crate::mcp::McpCommand::DeleteNode { .. }
            | crate::mcp::McpCommand::MoveNode { .. }
            | crate::mcp::McpCommand::CopyNode { .. }
            | crate::mcp::McpCommand::ReplaceNode { .. }
            | crate::mcp::McpCommand::BatchInsert { .. }
            | crate::mcp::McpCommand::InstantiateComponent { .. }
            | crate::mcp::McpCommand::CreateComponent { .. } => {
                // Not VariableTable mutations — Pages-level commands
                // live on `Document::apply_mcp_command`. Return false
                // so callers with only a VariableTable handle know
                // these variants aren't theirs.
                false
            }
        }
    }

    /// Cycle the active value of `axis` to the next entry in the
    /// `ThemeAxis::values` list. Returns `false` (no-op) when the
    /// axis isn't in `themes` or has no values. When the axis
    /// isn't currently in `active_theme`, the first value seeds
    /// it. When the current value matches the last entry, wraps
    /// back to the first. Used by the VariablesPanel axis-chip
    /// click to give users a one-click flip — light↔dark,
    /// compact↔comfortable, etc.
    pub fn cycle_active_axis_value(&mut self, axis: &str) -> bool {
        let Some(theme_axis) = self.themes.iter().find(|t| t.name == axis) else {
            return false;
        };
        if theme_axis.values.is_empty() {
            return false;
        }
        let next = match self.active_theme.get(axis) {
            None => theme_axis.values[0].clone(),
            Some(current) => {
                let idx = theme_axis
                    .values
                    .iter()
                    .position(|v| v == current)
                    .map(|i| (i + 1) % theme_axis.values.len())
                    .unwrap_or(0);
                theme_axis.values[idx].clone()
            }
        };
        self.active_theme.insert(axis.to_string(), next);
        true
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
                // Active theme set + no subset match — append a
                // new entry keyed to the active theme at the END
                // of the vec (codex stop-gate: front insertion
                // shadowed pre-existing themed entries on OTHER
                // axes — e.g. an existing {density:compact} entry
                // would never resolve under active=dark+compact if
                // a {mode:dark} entry was front-inserted because
                // resolve's first-match walk picks the front entry
                // for ANY active whose keys are a superset).
                //
                // End-push is safe because Step 1 (subset match)
                // already proved no existing entry is a subset of
                // `active`. So under the active theme our new
                // entry is the unique subset match regardless of
                // position; under OTHER actives, every pre-existing
                // entry retains its original resolve precedence.
                entries.push(ThemedValue {
                    value: VariableScalar::Str(normalized),
                    theme: Some(active),
                });
                true
            }
        }
    }

    /// Set a scalar variable's value with the same theme-routing
    /// discipline as `set_color_hex`, but generic over any
    /// `VariableScalar` (Number / Str / Bool). Used by the MCP
    /// write tools `set_variable_number` / `set_variable_string`
    /// / `set_variable_boolean`. Caller is responsible for
    /// matching the scalar's variant to the variable's kind
    /// (e.g. don't pass a Str to a Number-kind variable) —
    /// kind-mismatch returns `false`.
    pub fn set_scalar(&mut self, name: &str, scalar: VariableScalar) -> bool {
        let active = self.active_theme.clone();
        let var = match self.find_mut(name) {
            Some(v) => v,
            None => return false,
        };
        // Reject kind/scalar mismatch — a Number variable must
        // receive a Num scalar, etc. Color variables are
        // FORBIDDEN from this path: they need hex validation
        // before mutation, which only `set_color_hex` provides.
        // Codex stop-gate flagged that accepting Color+Str here
        // would let a misrouted `set_variable_string` call write
        // garbage into a Color variable (the tool snapshot
        // wouldn't admit the name, but defense in depth says the
        // apply layer should reject too).
        let kind_matches = match (&var.kind, &scalar) {
            (VariableKind::Number, VariableScalar::Num(_)) => true,
            (VariableKind::String, VariableScalar::Str(_)) => true,
            (VariableKind::Boolean, VariableScalar::Bool(_)) => true,
            (VariableKind::Color, _) => false,
            _ => false,
        };
        if !kind_matches {
            return false;
        }
        match &mut var.value {
            VariableValue::Scalar(s) => {
                *s = scalar;
                true
            }
            VariableValue::Themed(entries) => {
                let subset_idx = entries.iter().position(|e| match &e.theme {
                    Some(t) => t.iter().all(|(k, v)| active.get(k) == Some(v)),
                    None => false,
                });
                if let Some(i) = subset_idx {
                    entries[i].value = scalar;
                    return true;
                }
                if active.is_empty() {
                    let default_idx = entries.iter().position(|e| e.theme.is_none());
                    if let Some(i) = default_idx {
                        entries[i].value = scalar;
                    } else {
                        entries.push(ThemedValue {
                            value: scalar,
                            theme: None,
                        });
                    }
                    return true;
                }
                entries.push(ThemedValue {
                    value: scalar,
                    theme: Some(active),
                });
                true
            }
        }
    }
}

/// Parse `#rgb` / `#rrggbb` / `#rrggbbaa` into a `Color`. Mirrors the
/// TS paint helpers — lenient on case, requires the leading `#`.
pub(super) fn parse_hex_color(s: &str) -> Option<crate::Color> {
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
