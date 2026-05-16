//! Variables / themes mutators — ported from shell-core's
//! `document/variables.rs` (`VariableTable`), retargeted onto the
//! canonical document model.
//!
//! ## Model split (spec §5.2)
//!
//! shell-core's `VariableTable` mixed *persisted* data with
//! *transient* editor state. Here that split is explicit:
//!
//!   - **Persisted** — `EditorState.doc.variables`
//!     (`Option<BTreeMap<String, VariableDefinition>>`) +
//!     `EditorState.doc.themes` (`Option<BTreeMap<String,
//!     Vec<String>>>`, axis-name → ordered value list). They
//!     serialize with the `.op` file.
//!   - **Transient** — the active-theme selection lives on
//!     `EditorState.ui.variables.active_theme`. Rebuilt on load,
//!     never serialized.
//!
//! Theme-routing discipline for themed values is ported verbatim:
//! a write targets the subset-matching entry, else the `theme: None`
//! default when no theme axis is active, else a fresh entry keyed to
//! the active theme appended at the END of the vec (front insertion
//! would shadow pre-existing entries on other axes).

use crate::state::EditorState;
use jian_ops_schema::variable::{
    ThemedValue, VariableDefinition, VariableKind, VariableScalar, VariableValue,
};
use std::collections::BTreeMap;

impl EditorState {
    // --- Read helpers -----------------------------------------------

    /// Mutable handle to `doc.variables`, creating the map on first
    /// use so the mutators never have to special-case `None`.
    fn variables_mut(&mut self) -> &mut BTreeMap<String, VariableDefinition> {
        self.doc.variables.get_or_insert_with(BTreeMap::new)
    }

    /// Look up a variable definition by name.
    pub fn find_variable(&self, name: &str) -> Option<&VariableDefinition> {
        self.doc.variables.as_ref()?.get(name)
    }

    /// Resolve a variable's current scalar under the active theme.
    /// `None` for an unknown name or an empty themed list.
    pub fn resolve_variable(&self, name: &str) -> Option<&VariableScalar> {
        let def = self.find_variable(name)?;
        resolve_value(&def.value, &self.ui.variables.active_theme)
    }

    // --- Scalar writes ----------------------------------------------

    /// Write a `#rgb` / `#rrggbb` / `#rrggbbaa` hex into a `Color`
    /// variable. `false` when the variable is unknown, not
    /// Color-kind, or the hex doesn't parse. Themed variables route
    /// per the active-theme discipline.
    pub fn set_variable_color(&mut self, name: &str, hex: &str) -> bool {
        if crate::color_picker::parse_hex_rgb(hex).is_none() {
            return false;
        }
        let active = self.ui.variables.active_theme.clone();
        let Some(def) = self.variables_mut().get_mut(name) else {
            return false;
        };
        if !matches!(def.kind, VariableKind::Color) {
            return false;
        }
        write_scalar(&mut def.value, VariableScalar::Str(hex.trim().to_string()), &active);
        true
    }

    /// Write a number into a `Number` variable. Kind-mismatch → false.
    pub fn set_variable_number(&mut self, name: &str, value: f64) -> bool {
        self.set_variable_scalar(name, VariableKind::Number, VariableScalar::Num(value))
    }

    /// Write a string into a `String` variable. Kind-mismatch → false.
    pub fn set_variable_string(&mut self, name: &str, value: impl Into<String>) -> bool {
        self.set_variable_scalar(
            name,
            VariableKind::String,
            VariableScalar::Str(value.into()),
        )
    }

    /// Write a boolean into a `Boolean` variable. Kind-mismatch → false.
    pub fn set_variable_boolean(&mut self, name: &str, value: bool) -> bool {
        self.set_variable_scalar(name, VariableKind::Boolean, VariableScalar::Bool(value))
    }

    /// Shared kind-checked scalar writer for number / string /
    /// boolean. Color variables are FORBIDDEN here — they need hex
    /// validation, which only `set_variable_color` provides.
    fn set_variable_scalar(
        &mut self,
        name: &str,
        expect: VariableKind,
        scalar: VariableScalar,
    ) -> bool {
        let active = self.ui.variables.active_theme.clone();
        let Some(def) = self.variables_mut().get_mut(name) else {
            return false;
        };
        if def.kind != expect {
            return false;
        }
        write_scalar(&mut def.value, scalar, &active);
        true
    }

    // --- Variable lifecycle -----------------------------------------

    /// Create a new theme-agnostic scalar variable. Rejects an empty
    /// (post-trim) name, a name that collides with an existing
    /// variable, or a `default` that doesn't match `kind`. Color
    /// defaults must be a parseable hex string.
    pub fn create_variable(
        &mut self,
        name: &str,
        kind: VariableKind,
        default: VariableScalar,
    ) -> bool {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return false;
        }
        let kind_ok = match (&kind, &default) {
            (VariableKind::Color, VariableScalar::Str(s)) => {
                crate::color_picker::parse_hex_rgb(s).is_some()
            }
            (VariableKind::Number, VariableScalar::Num(_)) => true,
            (VariableKind::String, VariableScalar::Str(_)) => true,
            (VariableKind::Boolean, VariableScalar::Bool(_)) => true,
            _ => false,
        };
        if !kind_ok {
            return false;
        }
        let vars = self.variables_mut();
        if vars.contains_key(trimmed) {
            return false;
        }
        vars.insert(
            trimmed.to_string(),
            VariableDefinition {
                kind,
                value: VariableValue::Scalar(default),
            },
        );
        true
    }

    /// Delete a variable by name. `false` when the name is unknown.
    pub fn delete_variable(&mut self, name: &str) -> bool {
        let Some(vars) = self.doc.variables.as_mut() else {
            return false;
        };
        if vars.remove(name).is_none() {
            return false;
        }
        // Drop any ref caches pointing at the now-deleted variable.
        self.ui.variables.fill_refs.retain(|_, v| v != name);
        self.ui.variables.stroke_refs.retain(|_, v| v != name);
        true
    }

    /// Rename a variable. Rejects an unknown `old`, an empty (post-
    /// trim) `new`, or a `new` that collides with a different
    /// existing variable. `old == new` (after trim) is a no-op
    /// success. Rewrites every `fill_refs` / `stroke_refs` entry.
    pub fn rename_variable(&mut self, old: &str, new: &str) -> bool {
        let new_trimmed = new.trim();
        if new_trimmed.is_empty() {
            return false;
        }
        let Some(vars) = self.doc.variables.as_mut() else {
            return false;
        };
        if old == new_trimmed {
            return vars.contains_key(old);
        }
        if vars.contains_key(new_trimmed) {
            return false;
        }
        let Some(def) = vars.remove(old) else {
            return false;
        };
        vars.insert(new_trimmed.to_string(), def);
        for v in self.ui.variables.fill_refs.values_mut() {
            if v == old {
                *v = new_trimmed.to_string();
            }
        }
        for v in self.ui.variables.stroke_refs.values_mut() {
            if v == old {
                *v = new_trimmed.to_string();
            }
        }
        true
    }

    // --- Theme axis -------------------------------------------------

    /// Set the active value of a theme axis (e.g. `("mode", "dark")`).
    /// `false` when the axis isn't declared in `doc.themes` or the
    /// value isn't one of its declared entries.
    pub fn set_active_axis_value(&mut self, axis: &str, value: &str) -> bool {
        let Some(themes) = self.doc.themes.as_ref() else {
            return false;
        };
        let Some(values) = themes.get(axis) else {
            return false;
        };
        if !values.iter().any(|v| v == value) {
            return false;
        }
        self.ui
            .variables
            .active_theme
            .insert(axis.to_string(), value.to_string());
        true
    }

    /// Cycle the active value of `axis` to the next declared entry,
    /// wrapping at the end. Seeds the first value when the axis
    /// isn't yet in the active theme. `false` when the axis isn't
    /// declared or has no values.
    pub fn cycle_active_axis_value(&mut self, axis: &str) -> bool {
        let Some(themes) = self.doc.themes.as_ref() else {
            return false;
        };
        let Some(values) = themes.get(axis) else {
            return false;
        };
        if values.is_empty() {
            return false;
        }
        let next = match self.ui.variables.active_theme.get(axis) {
            None => values[0].clone(),
            Some(current) => {
                let idx = values
                    .iter()
                    .position(|v| v == current)
                    .map(|i| (i + 1) % values.len())
                    .unwrap_or(0);
                values[idx].clone()
            }
        };
        self.ui
            .variables
            .active_theme
            .insert(axis.to_string(), next);
        true
    }
}

// --- Free helpers ----------------------------------------------------

/// Resolve a `VariableValue` under the active theme — the canonical
/// equivalent of shell-core's `Variable::resolve`.
fn resolve_value<'a>(
    value: &'a VariableValue,
    active: &BTreeMap<String, String>,
) -> Option<&'a VariableScalar> {
    match value {
        VariableValue::Scalar(s) => Some(s),
        VariableValue::Themed(entries) => {
            for e in entries {
                if let Some(t) = &e.theme {
                    if t.iter().all(|(k, v)| active.get(k) == Some(v)) {
                        return Some(&e.value);
                    }
                }
            }
            entries.iter().find(|e| e.theme.is_none()).map(|e| &e.value)
        }
    }
}

/// Write `scalar` into a `VariableValue` with the theme-routing
/// discipline ported from shell-core's `set_color_hex` /
/// `set_scalar`.
fn write_scalar(
    value: &mut VariableValue,
    scalar: VariableScalar,
    active: &BTreeMap<String, String>,
) {
    match value {
        VariableValue::Scalar(s) => *s = scalar,
        VariableValue::Themed(entries) => {
            let subset_idx = entries.iter().position(|e| match &e.theme {
                Some(t) => t.iter().all(|(k, v)| active.get(k) == Some(v)),
                None => false,
            });
            if let Some(i) = subset_idx {
                entries[i].value = scalar;
                return;
            }
            if active.is_empty() {
                if let Some(i) = entries.iter().position(|e| e.theme.is_none()) {
                    entries[i].value = scalar;
                } else {
                    entries.push(ThemedValue {
                        value: scalar,
                        theme: None,
                    });
                }
                return;
            }
            // Active theme set + no subset match — end-push a fresh
            // entry keyed to the active theme.
            entries.push(ThemedValue {
                value: scalar,
                theme: Some(active.clone()),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::state_with;

    fn doc_with_color_var(name: &str, hex: &str) -> EditorState {
        let mut s = state_with(vec![]);
        s.create_variable(name, VariableKind::Color, VariableScalar::Str(hex.to_string()));
        s
    }

    #[test]
    fn create_then_resolve_color_variable() {
        let s = doc_with_color_var("brand", "#ff8800");
        match s.resolve_variable("brand") {
            Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#ff8800"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn create_variable_rejects_duplicate_and_empty() {
        let mut s = doc_with_color_var("brand", "#ff0000");
        assert!(!s.create_variable("brand", VariableKind::Color, VariableScalar::Str("#fff".into())));
        assert!(!s.create_variable("  ", VariableKind::Number, VariableScalar::Num(1.0)));
    }

    #[test]
    fn create_variable_rejects_kind_mismatch() {
        let mut s = state_with(vec![]);
        // Number kind with a string default → rejected.
        assert!(!s.create_variable("x", VariableKind::Number, VariableScalar::Str("nope".into())));
        // Color kind with a bad hex → rejected.
        assert!(!s.create_variable("c", VariableKind::Color, VariableScalar::Str("zzz".into())));
    }

    #[test]
    fn set_variable_color_writes_and_validates() {
        let mut s = doc_with_color_var("brand", "#ff0000");
        assert!(s.set_variable_color("brand", "#00ff00"));
        match s.resolve_variable("brand") {
            Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#00ff00"),
            other => panic!("unexpected {other:?}"),
        }
        // Bad hex → rejected, value unchanged.
        assert!(!s.set_variable_color("brand", "nothex"));
        // Unknown name → rejected.
        assert!(!s.set_variable_color("missing", "#000000"));
    }

    #[test]
    fn set_variable_scalar_kind_checks() {
        let mut s = state_with(vec![]);
        s.create_variable("n", VariableKind::Number, VariableScalar::Num(1.0));
        assert!(s.set_variable_number("n", 42.0));
        // String write into a Number variable → rejected.
        assert!(!s.set_variable_string("n", "no"));
        // Color write into a Number variable → rejected.
        s.create_variable("flag", VariableKind::Boolean, VariableScalar::Bool(false));
        assert!(s.set_variable_boolean("flag", true));
        assert!(!s.set_variable_number("flag", 3.0));
    }

    #[test]
    fn delete_variable_drops_it() {
        let mut s = doc_with_color_var("brand", "#ff0000");
        assert!(s.delete_variable("brand"));
        assert!(s.find_variable("brand").is_none());
        assert!(!s.delete_variable("brand"));
    }

    #[test]
    fn rename_variable_moves_definition() {
        let mut s = doc_with_color_var("brand", "#ff0000");
        assert!(s.rename_variable("brand", "primary"));
        assert!(s.find_variable("brand").is_none());
        assert!(s.find_variable("primary").is_some());
        // Unknown old → rejected.
        assert!(!s.rename_variable("brand", "x"));
        // Empty new → rejected.
        assert!(!s.rename_variable("primary", "  "));
    }

    #[test]
    fn rename_variable_collision_rejected() {
        let mut s = doc_with_color_var("a", "#000000");
        s.create_variable("b", VariableKind::Color, VariableScalar::Str("#ffffff".into()));
        assert!(!s.rename_variable("a", "b"));
    }

    #[test]
    fn active_axis_value_requires_declared_theme() {
        let mut s = state_with(vec![]);
        // No themes declared → false.
        assert!(!s.set_active_axis_value("mode", "dark"));
        let mut themes = BTreeMap::new();
        themes.insert("mode".to_string(), vec!["light".to_string(), "dark".to_string()]);
        s.doc.themes = Some(themes);
        assert!(s.set_active_axis_value("mode", "dark"));
        assert_eq!(s.ui.variables.active_theme.get("mode").map(|v| v.as_str()), Some("dark"));
        // Undeclared value → false.
        assert!(!s.set_active_axis_value("mode", "sepia"));
    }

    #[test]
    fn cycle_active_axis_wraps() {
        let mut s = state_with(vec![]);
        let mut themes = BTreeMap::new();
        themes.insert("mode".to_string(), vec!["light".to_string(), "dark".to_string()]);
        s.doc.themes = Some(themes);
        // First cycle seeds the first value.
        assert!(s.cycle_active_axis_value("mode"));
        assert_eq!(s.ui.variables.active_theme["mode"], "light");
        assert!(s.cycle_active_axis_value("mode"));
        assert_eq!(s.ui.variables.active_theme["mode"], "dark");
        // Wraps back to the first.
        assert!(s.cycle_active_axis_value("mode"));
        assert_eq!(s.ui.variables.active_theme["mode"], "light");
        // Unknown axis → false.
        assert!(!s.cycle_active_axis_value("density"));
    }

    #[test]
    fn undo_of_variable_create_removes_it() {
        // Gap 3: `EditorSnapshot` clones the whole `PenDocument`
        // (variables included), so a variable create/delete/rename
        // undoes for free — no separate `var_table` snapshot needed.
        let mut s = state_with(vec![]);
        s.commit_history(); // capture the pre-create state
        assert!(s.create_variable("brand", VariableKind::Color, VariableScalar::Str("#ff0000".into())));
        assert!(s.find_variable("brand").is_some());
        assert!(s.undo(), "undo must pop the pre-create snapshot");
        assert!(s.find_variable("brand").is_none(), "undo removes the variable");
        // Redo brings it back.
        assert!(s.redo());
        assert!(s.find_variable("brand").is_some());
    }

    #[test]
    fn undo_of_variable_delete_and_rename_round_trips() {
        let mut s = doc_with_color_var("brand", "#ff8800");
        // Delete under history.
        s.commit_history();
        assert!(s.delete_variable("brand"));
        assert!(s.find_variable("brand").is_none());
        assert!(s.undo());
        assert!(s.find_variable("brand").is_some(), "undo restores deleted var");
        // Rename under history.
        s.commit_history();
        assert!(s.rename_variable("brand", "primary"));
        assert!(s.undo());
        assert!(s.find_variable("brand").is_some(), "undo restores old name");
        assert!(s.find_variable("primary").is_none());
    }

    #[test]
    fn themed_write_targets_active_theme_entry() {
        let mut s = state_with(vec![]);
        let mut themes = BTreeMap::new();
        themes.insert("mode".to_string(), vec!["light".to_string(), "dark".to_string()]);
        s.doc.themes = Some(themes);
        // Seed a themed color variable directly.
        let mut vars = BTreeMap::new();
        vars.insert(
            "bg".to_string(),
            VariableDefinition {
                kind: VariableKind::Color,
                value: VariableValue::Themed(vec![ThemedValue {
                    value: VariableScalar::Str("#ffffff".into()),
                    theme: None,
                }]),
            },
        );
        s.doc.variables = Some(vars);
        // Under no active theme, write hits the default entry.
        assert!(s.set_variable_color("bg", "#eeeeee"));
        // Switch to dark and write — a new dark-keyed entry appends.
        s.set_active_axis_value("mode", "dark");
        assert!(s.set_variable_color("bg", "#111111"));
        match s.resolve_variable("bg") {
            Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#111111"),
            other => panic!("unexpected {other:?}"),
        }
        // Switch back to light — the default entry still resolves.
        s.set_active_axis_value("mode", "light");
        match s.resolve_variable("bg") {
            Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#eeeeee"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
