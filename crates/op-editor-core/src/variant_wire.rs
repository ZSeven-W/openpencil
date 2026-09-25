//! Versioned wire projection of a side-by-side directions ("variants") run's
//! per-direction reports.
//!
//! On desktop the design pump folds `Progress::VariantReady` /
//! `Progress::VariantFailed` straight into the workspace. In the browser the
//! run happens in the serve-web daemon, so the same two reports travel over
//! the standard turn's SSE stream as one `data: {"variant": …}` frame each.
//! This module is the contract between the two ends — a separate DTO rather
//! than serde derives on [`WorkspaceVariant`], so internal refactors of the
//! workspace state cannot silently change the wire.
//!
//! Wire shape (camelCase):
//!
//! ```json
//! {"v":1,"index":0,"name":"Direction A","phase":"ready",
//!  "styleGuide":"zen-paper-light","styleLabel":"Zen Paper Light",
//!  "namePrefix":"Direction A · Zen Paper Light · ","rootIds":["…"],
//!  "variables":{…},"themes":{…}}
//! {"v":1,"index":1,"name":"Direction B","phase":"failed","error":"…"}
//! ```

use std::collections::BTreeMap;

use jian_ops_schema::variable::VariableDefinition;
use serde::{Deserialize, Serialize};

use crate::WorkspaceVariant;

/// Bumped whenever an existing field changes meaning or disappears.
///
/// Purely additive changes do not bump it: unknown fields are ignored on
/// decode. A frame from a different version is dropped by [`decode`] —
/// the run still finishes, the workspace simply shows no pick bar.
///
/// [`decode`]: VariantEventWire::decode
pub const VARIANT_WIRE_VERSION: u32 = 1;

/// One direction's settled outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantEventWire {
    /// [`VARIANT_WIRE_VERSION`] of the sender.
    pub v: u32,
    /// Slot of the direction in the run (`0` = A).
    pub index: usize,
    /// The direction's display name (`Direction A`, `方案 A`).
    pub name: String,
    /// What happened to it.
    #[serde(flatten)]
    pub outcome: VariantOutcomeWire,
}

/// `phase` tag of a [`VariantEventWire`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum VariantOutcomeWire {
    /// The direction landed on the shared page.
    Ready(VariantReadyWire),
    /// The direction produced nothing; the others keep going.
    Failed(VariantFailedWire),
}

/// The landed direction: what the workspace needs to offer "use this".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantReadyWire {
    pub style_guide: String,
    pub style_label: String,
    pub name_prefix: String,
    pub root_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, VariableDefinition>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub themes: Option<BTreeMap<String, Vec<String>>>,
}

/// Why a direction produced nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantFailedWire {
    pub error: String,
}

impl VariantEventWire {
    /// The report for a landed direction.
    pub fn ready(variant: &WorkspaceVariant) -> Self {
        Self {
            v: VARIANT_WIRE_VERSION,
            index: variant.index,
            name: variant.name.clone(),
            outcome: VariantOutcomeWire::Ready(VariantReadyWire {
                style_guide: variant.style_guide.clone(),
                style_label: variant.style_label.clone(),
                name_prefix: variant.name_prefix.clone(),
                root_ids: variant.root_ids.clone(),
                variables: variant.variables.clone(),
                themes: variant.themes.clone(),
            }),
        }
    }

    /// The report for a direction that produced nothing.
    pub fn failed(index: usize, name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            v: VARIANT_WIRE_VERSION,
            index,
            name: name.into(),
            outcome: VariantOutcomeWire::Failed(VariantFailedWire {
                error: error.into(),
            }),
        }
    }

    /// Decode one `variant` frame. `None` for a malformed payload or a
    /// frame from another wire version.
    pub fn decode(value: &serde_json::Value) -> Option<Self> {
        let event: Self = serde_json::from_value(value.clone()).ok()?;
        (event.v == VARIANT_WIRE_VERSION).then_some(event)
    }

    /// The landed direction as the workspace records it; `None` for a
    /// failure report.
    pub fn to_workspace_variant(&self) -> Option<WorkspaceVariant> {
        let VariantOutcomeWire::Ready(ready) = &self.outcome else {
            return None;
        };
        Some(WorkspaceVariant {
            index: self.index,
            name: self.name.clone(),
            style_guide: ready.style_guide.clone(),
            style_label: ready.style_label.clone(),
            name_prefix: ready.name_prefix.clone(),
            root_ids: ready.root_ids.clone(),
            variables: ready.variables.clone(),
            themes: ready.themes.clone(),
        })
    }
}

#[cfg(test)]
#[path = "variant_wire_tests.rs"]
mod tests;
