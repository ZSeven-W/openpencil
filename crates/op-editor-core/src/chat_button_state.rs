//! State-layer mirror of the AI chat panel's hoverable buttons.
//!
//! The chat panel's full hit enum (`op_editor_ui::widgets::AIChatHit`)
//! carries owned `String` payloads, so it is not `Copy` and can't live
//! on `EditorUiState`. These small `Copy` enums capture just the chat
//! controls that need hover paint state — same wasm32-clean discipline
//! as `topbar_state` / `statusbar_state`.

/// Which bare header button of the AI chat panel the cursor is over.
/// `None` on `EditorUiState.chat_header_hover` = no hover wash. The
/// send / attach / model / effort controls already paint their own
/// backgrounds and so are intentionally excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatHeaderButton {
    /// Chevron at the top-left — collapses the panel to a pill.
    ToggleCollapse,
    /// Maximize / restore glyph in the header.
    ToggleMaximize,
    /// Plus glyph in the header — starts a new chat.
    NewChat,
}

/// Which footer control of the AI chat panel the cursor is over.
/// `None` on `EditorUiState.chat_footer_hover` = no hover wash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFooterButton {
    /// Bottom-left model chip — opens the model picker.
    ModelPicker,
    /// Speed/effort chip (⚡ + label) — cycles the effort level.
    SpeedChip,
    /// Compact Agent Team size chip in the bottom toolbar.
    AgentTeam,
    /// Paperclip attachment button.
    AddAttachment,
    /// Palette button — currently inert (#27 future affordance).
    Palette,
    /// Send button in its normal idle state.
    Send,
    /// Stop button shown while a response streams.
    Stop,
}
