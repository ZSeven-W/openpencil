//! Canonical pressed-state targets for the agent-settings modal.

/// `EditorUiState.pressed_button` target for plain settings-modal
/// buttons that use shared ghost button feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsButton {
    Close,
    AddProvider,
    AddAcpAgent,
    BuiltinEdit(usize),
    BuiltinRemove(usize),
    BuiltinSaveDraft,
    BuiltinCancelDraft,
    AcpEdit(usize),
    AcpRemove(usize),
    AcpConnection(usize),
    AcpSaveDraft,
    AcpCancelDraft,
    McpServer,
    McpClientConfigCopy,
    ImageSearchTest,
    ImageGenAdd,
    ImageProfileHeader(usize),
    ImageProfileRemove(usize),
    ImageProfileProvider(usize),
    ImageProviderOption {
        index: usize,
        provider: crate::agent_settings::ImageGenProvider,
    },
    ImageProfileTest(usize),
}
