//! Serializable Preview debugger and bounded trace DTOs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewRunState {
    #[default]
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewTraceKind {
    Input,
    SemanticEvent,
    Action,
    StateDiff,
    Route,
    Animation,
    Effect,
    EffectResult,
    Diagnostic,
    Control,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewTraceEntry {
    pub sequence: u64,
    pub at_ms: u64,
    pub kind: PreviewTraceKind,
    pub name: String,
    pub node_id: Option<String>,
    pub event: Option<String>,
    pub action: Option<String>,
    pub detail: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewDiagnostic {
    pub sequence: u64,
    pub code: String,
    pub message: String,
    pub node_id: Option<String>,
    pub event: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PreviewStateScope {
    #[serde(rename = "$app")]
    App,
    #[serde(rename = "$page")]
    Page,
    #[serde(rename = "$state")]
    State,
    #[serde(rename = "$self")]
    SelfNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewStateProvenance {
    pub node_id: Option<String>,
    pub event: Option<String>,
    pub action: Option<String>,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewStateRow {
    pub scope: PreviewStateScope,
    pub owner: Option<String>,
    pub key: String,
    pub value: Value,
    pub provenance: Option<PreviewStateProvenance>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewQueueCounts {
    pub action_tasks: usize,
    pub effects: usize,
    pub animations: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewDebugSnapshot {
    pub run_state: PreviewRunState,
    pub route_stack: Vec<String>,
    pub current_screen: Option<String>,
    pub focused_node: Option<String>,
    pub captured_pointers: Vec<u32>,
    pub active_gestures: usize,
    pub state: Vec<PreviewStateRow>,
    pub queues: PreviewQueueCounts,
    pub diagnostics: Vec<PreviewDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_wire_names_are_the_authored_namespaces() {
        assert_eq!(
            serde_json::to_string(&[
                PreviewStateScope::App,
                PreviewStateScope::Page,
                PreviewStateScope::State,
                PreviewStateScope::SelfNode,
            ])
            .unwrap(),
            r#"["$app","$page","$state","$self"]"#
        );
    }
}
