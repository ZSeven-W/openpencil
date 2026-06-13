//! Canonical pressed-state targets for the agent-settings modal.

/// `EditorUiState.pressed_button` target for plain settings-modal
/// buttons that use shared ghost button feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsButton {
    Close,
    McpServer,
    McpClientConfigCopy,
}
