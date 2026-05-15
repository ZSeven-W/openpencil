//! Variable-table round-trip for the desktop's private `DocPayload`
//! save format. Carved off `persistence.rs` to stay under the
//! 800-line cap.
//!
//! Codex stop-gate: `DocPayload` originally serialized only pages,
//! so every variable mutation — `set_variable_*` AND the newer
//! `create/delete/rename_variable` CRUD — was applied in memory
//! but silently dropped on the MCP save path. This module mirrors
//! `VariableTable` into serde structs so a saved `.op` round-trips
//! design tokens.

use std::collections::BTreeMap;

use openpencil_shell_core::document::{
    ThemeAxis, ThemedValue, Variable, VariableKind, VariableScalar, VariableTable,
    VariableValue,
};
use openpencil_shell_core::document::NodeId;
use serde::{Deserialize, Serialize};

/// Tagged scalar — mirrors `VariableScalar`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
pub enum ScalarPayload {
    #[serde(rename = "bool")]
    Bool(bool),
    #[serde(rename = "num")]
    Num(f64),
    #[serde(rename = "str")]
    Str(String),
}

impl ScalarPayload {
    fn from_scalar(s: &VariableScalar) -> Self {
        match s {
            VariableScalar::Bool(b) => ScalarPayload::Bool(*b),
            VariableScalar::Num(n) => ScalarPayload::Num(*n),
            VariableScalar::Str(s) => ScalarPayload::Str(s.clone()),
        }
    }
    fn to_scalar(&self) -> VariableScalar {
        match self {
            ScalarPayload::Bool(b) => VariableScalar::Bool(*b),
            ScalarPayload::Num(n) => VariableScalar::Num(*n),
            ScalarPayload::Str(s) => VariableScalar::Str(s.clone()),
        }
    }
}

/// One themed alternative — mirrors `ThemedValue`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemedEntryPayload {
    pub value: ScalarPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<BTreeMap<String, String>>,
}

/// Variable value — mirrors `VariableValue` (scalar OR themed list).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum ValuePayload {
    #[serde(rename = "scalar")]
    Scalar { value: ScalarPayload },
    #[serde(rename = "themed")]
    Themed { entries: Vec<ThemedEntryPayload> },
}

/// One named variable — mirrors `Variable`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariablePayload {
    pub name: String,
    pub kind: String,
    pub value: ValuePayload,
}

/// One theme axis — mirrors `ThemeAxis`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeAxisPayload {
    pub name: String,
    pub values: Vec<String>,
}

/// Full `VariableTable` snapshot. Every field is `#[serde(default)]`
/// so a legacy `.op` saved before this struct existed still loads
/// (the table simply comes back empty).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VarTablePayload {
    #[serde(default)]
    pub variables: Vec<VariablePayload>,
    #[serde(default)]
    pub themes: Vec<ThemeAxisPayload>,
    #[serde(default)]
    pub active_theme: BTreeMap<String, String>,
    /// `(node_id, variable_name)` pairs — node fills bound to a
    /// `$ref`. Stored as a list because JSON object keys must be
    /// strings; a list keeps the `u64` ids exact.
    #[serde(default)]
    pub fill_refs: Vec<(u64, String)>,
    #[serde(default)]
    pub stroke_refs: Vec<(u64, String)>,
}

fn kind_to_str(k: &VariableKind) -> &'static str {
    match k {
        VariableKind::Color => "color",
        VariableKind::Number => "number",
        VariableKind::Boolean => "boolean",
        VariableKind::String => "string",
    }
}

fn kind_from_str(s: &str) -> Option<VariableKind> {
    match s {
        "color" => Some(VariableKind::Color),
        "number" => Some(VariableKind::Number),
        "boolean" => Some(VariableKind::Boolean),
        "string" => Some(VariableKind::String),
        _ => None,
    }
}

fn value_to_payload(v: &VariableValue) -> ValuePayload {
    match v {
        VariableValue::Scalar(s) => ValuePayload::Scalar {
            value: ScalarPayload::from_scalar(s),
        },
        VariableValue::Themed(entries) => ValuePayload::Themed {
            entries: entries
                .iter()
                .map(|e| ThemedEntryPayload {
                    value: ScalarPayload::from_scalar(&e.value),
                    theme: e.theme.clone(),
                })
                .collect(),
        },
    }
}

fn value_from_payload(p: &ValuePayload) -> VariableValue {
    match p {
        ValuePayload::Scalar { value } => VariableValue::Scalar(value.to_scalar()),
        ValuePayload::Themed { entries } => VariableValue::Themed(
            entries
                .iter()
                .map(|e| ThemedValue {
                    value: e.value.to_scalar(),
                    theme: e.theme.clone(),
                })
                .collect(),
        ),
    }
}

/// Serialize a live `VariableTable` into its payload form.
pub fn var_table_to_payload(t: &VariableTable) -> VarTablePayload {
    VarTablePayload {
        variables: t
            .variables
            .iter()
            .map(|v| VariablePayload {
                name: v.name.clone(),
                kind: kind_to_str(&v.kind).to_string(),
                value: value_to_payload(&v.value),
            })
            .collect(),
        themes: t
            .themes
            .iter()
            .map(|a| ThemeAxisPayload {
                name: a.name.clone(),
                values: a.values.clone(),
            })
            .collect(),
        active_theme: t.active_theme.clone(),
        fill_refs: t
            .fill_refs
            .iter()
            .map(|(id, name)| (id.raw(), name.clone()))
            .collect(),
        stroke_refs: t
            .stroke_refs
            .iter()
            .map(|(id, name)| (id.raw(), name.clone()))
            .collect(),
    }
}

/// Rebuild a `VariableTable` from its payload form. Variables with
/// an unknown `kind` string are skipped (forward-compat: a newer
/// file shouldn't crash an older build).
pub fn var_table_from_payload(p: &VarTablePayload) -> VariableTable {
    let mut t = VariableTable::default();
    for v in &p.variables {
        let Some(kind) = kind_from_str(&v.kind) else {
            continue;
        };
        t.variables.push(Variable {
            name: v.name.clone(),
            kind,
            value: value_from_payload(&v.value),
        });
    }
    for a in &p.themes {
        t.themes.push(ThemeAxis {
            name: a.name.clone(),
            values: a.values.clone(),
        });
    }
    t.active_theme = p.active_theme.clone();
    for (id, name) in &p.fill_refs {
        t.fill_refs.insert(NodeId::new(*id), name.clone());
    }
    for (id, name) in &p.stroke_refs {
        t.stroke_refs.insert(NodeId::new(*id), name.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpencil_shell_core::document::VariableValue;

    fn sample_table() -> VariableTable {
        let mut t = VariableTable::default();
        t.create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#ff8800".into()),
        );
        t.create_variable("spacing", VariableKind::Number, VariableScalar::Num(16.0));
        t.create_variable(
            "dark-mode",
            VariableKind::Boolean,
            VariableScalar::Bool(true),
        );
        t.themes.push(ThemeAxis {
            name: "mode".into(),
            values: vec!["light".into(), "dark".into()],
        });
        t.active_theme.insert("mode".into(), "dark".into());
        t.fill_refs.insert(NodeId::new(7), "brand".into());
        t.stroke_refs.insert(NodeId::new(9), "brand".into());
        t
    }

    #[test]
    fn var_table_survives_json_round_trip() {
        let original = sample_table();
        let payload = var_table_to_payload(&original);
        // Through the actual serde JSON path the `.op` file uses.
        let json = serde_json::to_string(&payload).expect("serialize");
        let back: VarTablePayload =
            serde_json::from_str(&json).expect("deserialize");
        let restored = var_table_from_payload(&back);

        assert_eq!(restored.variables.len(), 3);
        assert_eq!(restored.themes.len(), 1);
        assert_eq!(restored.themes[0].values, vec!["light", "dark"]);
        assert_eq!(restored.active_theme.get("mode"), Some(&"dark".to_string()));
        assert_eq!(
            restored.fill_refs.get(&NodeId::new(7)),
            Some(&"brand".to_string())
        );
        assert_eq!(
            restored.stroke_refs.get(&NodeId::new(9)),
            Some(&"brand".to_string())
        );
        // Spot-check a scalar value survived intact.
        let spacing = restored
            .variables
            .iter()
            .find(|v| v.name == "spacing")
            .expect("spacing var");
        match &spacing.value {
            VariableValue::Scalar(VariableScalar::Num(n)) => assert_eq!(*n, 16.0),
            other => panic!("expected Num(16), got {other:?}"),
        }
    }

    #[test]
    fn legacy_payload_without_var_table_field_loads_empty() {
        // A `.op` saved before this field existed: the JSON object
        // simply omits `var_table`. serde(default) must yield an
        // empty table, not a parse error.
        let legacy = r#"{"variables":[],"themes":[],"active_theme":{}}"#;
        let payload: VarTablePayload =
            serde_json::from_str(legacy).expect("legacy parse");
        let table = var_table_from_payload(&payload);
        assert!(table.variables.is_empty());
        assert!(table.themes.is_empty());
    }

    #[test]
    fn unknown_variable_kind_is_skipped_not_fatal() {
        // Forward-compat: a newer file with a kind this build
        // doesn't know must skip that variable, not crash.
        let json = r#"{"variables":[
            {"name":"future","kind":"gradient","value":{"mode":"scalar","value":{"t":"str","v":"x"}}},
            {"name":"ok","kind":"number","value":{"mode":"scalar","value":{"t":"num","v":3.0}}}
        ],"themes":[],"active_theme":{}}"#;
        let payload: VarTablePayload =
            serde_json::from_str(json).expect("parse");
        let table = var_table_from_payload(&payload);
        assert_eq!(table.variables.len(), 1, "unknown kind skipped");
        assert_eq!(table.variables[0].name, "ok");
    }
}
