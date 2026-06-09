//! State-layer mirror of the AI chat panel's bare header buttons.
//!
//! The chat panel's full hit enum (`op_editor_ui::widgets::AIChatHit`)
//! carries owned `String` payloads, so it is not `Copy` and can't live
//! on `EditorUiState`. The header chevron / maximize / new-chat glyphs
//! are the only chat controls painted without their own background, so
//! they are the ones that need a `theme.button_hover` wash. This small
//! `Copy` enum captures just those for the hover state — same wasm32-
//! clean discipline as `topbar_state` / `statusbar_state`.

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
