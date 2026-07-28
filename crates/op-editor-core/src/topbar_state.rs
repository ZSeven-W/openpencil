//! State-layer mirror of the top bar's hit-test enum.
//!
//! `TopBarButton` mirrors `op_editor_ui::widgets::TopBarHit` so the
//! host can record which top-bar chrome button the cursor is over on
//! `EditorUiState.topbar_button_hover` without `op-editor-core`
//! depending on the widget crate (keeps the crate wasm32-clean, same
//! discipline as `toolbar_state::ToolbarHover`).

/// Which top-bar chrome button the cursor is over. `None` on
/// `EditorUiState.topbar_button_hover` = no hover wash. Each variant
/// pairs with the matching `TopBarHit` so the button paints a
/// `theme.button_hover` background while the cursor rests on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopBarButton {
    /// PanelLeft icon — toggles the sidebar (LayerPanel).
    ToggleSidebar,
    /// Folder + chevron compound — toggles the file menu.
    ToggleFileMenu,
    /// Figma logo — opens the .fig import modal.
    OpenImportMenu,
    /// Sun / Moon icon — flips the theme.
    ToggleTheme,
    /// Globe + chevron — opens the locale picker.
    ToggleLocale,
    /// Agents-and-MCP chip — opens the agent settings modal.
    OpenAgentSettings,
    /// Collaboration status / participant chip — opens the shared
    /// collaboration panel.
    OpenCollaboration,
    /// Git-branch button beside the file name — toggles the git panel.
    ToggleGitPanel,
    /// Maximize icon — toggles window fullscreen.
    ToggleFullscreen,
    /// Play / Stop icon — enters / exits canvas Preview (Play) mode.
    TogglePreview,
    /// User-avatar button — opens the sign-in modal (signed out) or the
    /// account dropdown (signed in).
    OpenAccount,
}
