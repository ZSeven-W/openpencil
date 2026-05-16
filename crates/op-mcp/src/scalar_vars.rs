//! Non-color scalar variable write tools — `set_variable_number`,
//! `set_variable_string`, `set_variable_boolean`. Carved off
//! `write_tools.rs` to stay under the 800-line cap as the
//! catalog grew.
//!
//! All three emit `McpCommand::SetVariableScalar { name, scalar }`;
//! the applier routes through `VariableTable::set_scalar` which
//! honors active-theme routing identically to set_color_hex.

use std::collections::BTreeMap;

use jian_ops_schema::variable::VariableKind;
use op_editor_core::EditorState;

use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome, VariableScalarPayload};

/// First-party `set_variable_number` tool — set a Number-kind
/// variable's value. Mirrors `set_variable_color` for non-color
/// scalars.
pub struct SetVariableNumber {
    /// Snapshot of which Number variables exist. Validation only;
    /// applier re-checks against live state.
    pub known: BTreeMap<String, ()>,
}

impl McpTool for SetVariableNumber {
    fn name(&self) -> &str {
        "set_variable_number"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        let Some(value) = args.get("value") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "value is required (decimal number, may be negative or fractional)".into(),
            );
        };
        if !self.known.contains_key(name) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {name:?} not found or not Number-kind"),
            );
        }
        let n: f64 = match value.parse::<f64>() {
            Ok(n) if n.is_finite() => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("value must be a finite decimal, got {value:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetVariableScalar {
                name: name.clone(),
                scalar: VariableScalarPayload::Number(n),
            },
        )
    }
}

/// Snapshot the variables of `kind` into a `name → ()` map.
fn known_of_kind(state: &EditorState, kind: VariableKind) -> BTreeMap<String, ()> {
    state
        .doc
        .variables
        .as_ref()
        .map(|vars| {
            vars.iter()
                .filter(|(_, def)| def.kind == kind)
                .map(|(name, _)| (name.clone(), ()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn set_variable_number_snapshot(state: &EditorState) -> SetVariableNumber {
    SetVariableNumber {
        known: known_of_kind(state, VariableKind::Number),
    }
}

/// First-party `set_variable_string` tool — set a String-kind
/// variable's value. Mirrors `set_variable_color` for string
/// scalars (free-form text, no validation beyond presence).
pub struct SetVariableString {
    pub known: BTreeMap<String, ()>,
}

impl McpTool for SetVariableString {
    fn name(&self) -> &str {
        "set_variable_string"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        let Some(value) = args.get("value") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "value is required".into());
        };
        if !self.known.contains_key(name) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {name:?} not found or not String-kind"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetVariableScalar {
                name: name.clone(),
                scalar: VariableScalarPayload::String(value.clone()),
            },
        )
    }
}

pub fn set_variable_string_snapshot(state: &EditorState) -> SetVariableString {
    SetVariableString {
        known: known_of_kind(state, VariableKind::String),
    }
}

/// First-party `set_variable_boolean` tool — set a Boolean-kind
/// variable's value. Accepts the strings `"true"` / `"false"`
/// (case-sensitive); anything else returns `InvalidArgument`.
pub struct SetVariableBoolean {
    pub known: BTreeMap<String, ()>,
}

impl McpTool for SetVariableBoolean {
    fn name(&self) -> &str {
        "set_variable_boolean"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        let Some(value) = args.get("value") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "value is required (\"true\" or \"false\")".into(),
            );
        };
        if !self.known.contains_key(name) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {name:?} not found or not Boolean-kind"),
            );
        }
        let b = match value.as_str() {
            "true" => true,
            "false" => false,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("value must be \"true\" or \"false\", got {value:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetVariableScalar {
                name: name.clone(),
                scalar: VariableScalarPayload::Boolean(b),
            },
        )
    }
}

pub fn set_variable_boolean_snapshot(state: &EditorState) -> SetVariableBoolean {
    SetVariableBoolean {
        known: known_of_kind(state, VariableKind::Boolean),
    }
}

/// The four MCP `create_variable` kind strings.
const VARIABLE_KINDS: &[&str] = &["color", "number", "boolean", "string"];

/// Light call-time validation of a `default_value` against its
/// declared `kind`. Mirrors `document::variables::
/// parse_variable_kind_and_default` (which re-runs at apply time);
/// duplicated here so a bad default fails as `InvalidArgument`
/// up front instead of an `Internal` applier demotion.
fn default_value_ok(kind: &str, default_value: &str) -> bool {
    match kind {
        "color" => {
            // #rgb / #rrggbb / #rrggbbaa, leading '#'.
            let Some(hex) = default_value.trim().strip_prefix('#') else {
                return false;
            };
            matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
        }
        "number" => default_value
            .parse::<f64>()
            .map(|n| n.is_finite())
            .unwrap_or(false),
        "boolean" => matches!(default_value, "true" | "false"),
        "string" => true,
        _ => false,
    }
}

/// First-party `create_variable` tool — author a new design
/// token. Carries a snapshot of existing variable names so a
/// duplicate name is rejected at call time.
pub struct CreateVariable {
    pub existing: BTreeMap<String, ()>,
}

impl McpTool for CreateVariable {
    fn name(&self) -> &str {
        "create_variable"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        let Some(kind) = args.get("kind") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "kind is required (color / number / boolean / string)".into(),
            );
        };
        let Some(default_value) = args.get("default_value") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "default_value is required".into(),
            );
        };
        if name.trim().is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "name must not be empty after trimming".into(),
            );
        }
        if !VARIABLE_KINDS.contains(&kind.as_str()) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("kind must be one of {VARIABLE_KINDS:?}, got {kind:?}"),
            );
        }
        if self.existing.contains_key(name.trim()) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {:?} already exists", name.trim()),
            );
        }
        if !default_value_ok(kind, default_value) {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("default_value {default_value:?} invalid for kind {kind:?}"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::CreateVariable {
                name: name.clone(),
                kind: kind.clone(),
                default_value: default_value.clone(),
            },
        )
    }
}

/// Snapshot every variable name into a `name → ()` map.
fn all_variable_names(state: &EditorState) -> BTreeMap<String, ()> {
    state
        .doc
        .variables
        .as_ref()
        .map(|vars| vars.keys().map(|name| (name.clone(), ())).collect())
        .unwrap_or_default()
}

pub fn create_variable_snapshot(state: &EditorState) -> CreateVariable {
    CreateVariable {
        existing: all_variable_names(state),
    }
}

/// First-party `delete_variable` tool — remove a design token by
/// name. Snapshots existing names so an unknown name is rejected
/// at call time.
pub struct DeleteVariable {
    pub existing: BTreeMap<String, ()>,
}

impl McpTool for DeleteVariable {
    fn name(&self) -> &str {
        "delete_variable"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "name is required".into());
        };
        if !self.existing.contains_key(name) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {name:?} not found"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::DeleteVariable { name: name.clone() })
    }
}

pub fn delete_variable_snapshot(state: &EditorState) -> DeleteVariable {
    DeleteVariable {
        existing: all_variable_names(state),
    }
}

/// First-party `rename_variable` tool — rename a design token and
/// rewrite every node `$ref` pointing at it. Snapshots existing
/// names so unknown-old / colliding-new are rejected at call time.
pub struct RenameVariable {
    pub existing: BTreeMap<String, ()>,
}

impl McpTool for RenameVariable {
    fn name(&self) -> &str {
        "rename_variable"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(old_name) = args.get("old_name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "old_name is required".into(),
            );
        };
        let Some(new_name) = args.get("new_name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "new_name is required".into(),
            );
        };
        if new_name.trim().is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "new_name must not be empty after trimming".into(),
            );
        }
        if !self.existing.contains_key(old_name) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {old_name:?} not found"),
            );
        }
        // Collision check — but old == new (after trim) is a
        // legitimate no-op rename, not a collision.
        if old_name != new_name.trim() && self.existing.contains_key(new_name.trim()) {
            return ToolOutcome::Err(
                ToolErrorCode::ToolFailed,
                format!("variable {:?} already exists", new_name.trim()),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::RenameVariable {
                old_name: old_name.clone(),
                new_name: new_name.clone(),
            },
        )
    }
}

pub fn rename_variable_snapshot(state: &EditorState) -> RenameVariable {
    RenameVariable {
        existing: all_variable_names(state),
    }
}
