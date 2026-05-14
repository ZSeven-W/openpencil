//! Non-color scalar variable write tools — `set_variable_number`,
//! `set_variable_string`, `set_variable_boolean`. Carved off
//! `write_tools.rs` to stay under the 800-line cap as the
//! catalog grew.
//!
//! All three emit `McpCommand::SetVariableScalar { name, scalar }`;
//! the applier routes through `VariableTable::set_scalar` which
//! honors active-theme routing identically to set_color_hex.

use std::collections::BTreeMap;

use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};

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
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
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
            McpCommand::SetVariableScalar {
                name: name.clone(),
                scalar: crate::mcp::VariableScalarPayload::Number(n),
            },
        )
    }
}

pub fn set_variable_number_snapshot(doc: &crate::document::Document) -> SetVariableNumber {
    use crate::document::VariableKind;
    let known = doc
        .var_table
        .variables
        .iter()
        .filter(|v| matches!(v.kind, VariableKind::Number))
        .map(|v| (v.name.clone(), ()))
        .collect();
    SetVariableNumber { known }
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
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
        };
        let Some(value) = args.get("value") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "value is required".into(),
            );
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
            McpCommand::SetVariableScalar {
                name: name.clone(),
                scalar: crate::mcp::VariableScalarPayload::String(value.clone()),
            },
        )
    }
}

pub fn set_variable_string_snapshot(doc: &crate::document::Document) -> SetVariableString {
    use crate::document::VariableKind;
    let known = doc
        .var_table
        .variables
        .iter()
        .filter(|v| matches!(v.kind, VariableKind::String))
        .map(|v| (v.name.clone(), ()))
        .collect();
    SetVariableString { known }
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
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required".into(),
            );
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
            McpCommand::SetVariableScalar {
                name: name.clone(),
                scalar: crate::mcp::VariableScalarPayload::Boolean(b),
            },
        )
    }
}

pub fn set_variable_boolean_snapshot(doc: &crate::document::Document) -> SetVariableBoolean {
    use crate::document::VariableKind;
    let known = doc
        .var_table
        .variables
        .iter()
        .filter(|v| matches!(v.kind, VariableKind::Boolean))
        .map(|v| (v.name.clone(), ()))
        .collect();
    SetVariableBoolean { known }
}
