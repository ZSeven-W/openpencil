//! Shared pressed-state target for chrome buttons.
//!
//! Hover state stays per-family because cursor-move hit tests derive it
//! independently. Pressed feedback is mutually exclusive for the primary
//! pointer, so one enum on `EditorUiState` can cover button families without
//! adding parallel `*_pressed` fields.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonPressTarget {
    Toolbar(crate::toolbar_state::ToolbarHover),
    TopBar(crate::topbar_state::TopBarButton),
    StatusBar(crate::statusbar_state::StatusBarButton),
    ChatHeader(crate::chat_button_state::ChatHeaderButton),
    ChatFooter(crate::chat_button_state::ChatFooterButton),
    Git(crate::git_button_state::GitButton),
    ExportDialog(crate::export_dialog_state::ExportDialogButton),
}
