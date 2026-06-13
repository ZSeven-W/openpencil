//! Shell-core-side derivations over `op_editor_core::EditorUiState`.
//!
//! shell-core's `Document` carried `theme()` + `t()` accessors that the
//! widgets relied on. `EditorState` deliberately has no widget-layer
//! concerns, so the same two derivations live here as free functions
//! over the narrowest sub-struct that carries the inputs —
//! `EditorUiState` (which holds `theme_mode` + `locale`).
//!
//! Widgets that were ported off `Document` onto `EditorState` call
//! these instead of the old `doc.theme()` / `doc.t(key)`.

use crate::theme::Theme;
use op_editor_core::editor_ui_state::{EditorUiState, ThemeMode};

/// Resolve the active editor [`Theme`] from the UI theme mode.
/// Mirrors the old `Document::theme()`.
pub fn theme_for(ui: &EditorUiState) -> Theme {
    match ui.theme_mode {
        ThemeMode::Dark => Theme::dark(),
        ThemeMode::Light => Theme::light(),
    }
}

/// Translate a chrome string key against the active UI locale.
/// Mirrors the old `Document::t()`.
pub fn translate(ui: &EditorUiState, key: &'static str) -> &'static str {
    crate::i18n::translate(ui.locale, key)
}

/// Map an `op_editor_core::ExportFormat` onto the widget-layer
/// `widgets::export_dialog::ExportFormat`. Variant-identical.
pub fn doc_export_format(
    f: op_editor_core::ExportFormat,
) -> crate::widgets::export_dialog::ExportFormat {
    use crate::widgets::export_dialog::ExportFormat as D;
    use op_editor_core::ExportFormat as O;
    match f {
        O::Png => D::Png,
        O::Jpeg => D::Jpeg,
        O::Webp => D::Webp,
        O::Svg => D::Svg,
        O::Pdf => D::Pdf,
    }
}

// ── Widget-layer → canonical reverse converters ───────────────────
//
// The host feeds widget hit-test results back into `EditorState`'s
// `editor_ui_state`. Most widget hit-tests already emit canonical
// `op_editor_core` types (`Tool`, `AlignAction`, `PropertyFocus`, …)
// so no conversion is needed. The remaining widget-local enums below
// still need a one-arm-per-variant bridge into canonical state fields.

/// Map the widget-layer `widgets::toolbar::ToolbarAction` onto the
/// canonical `op_editor_core::ToolbarAction`. Variant-identical;
/// bridges the toolbar hover state.
pub fn toolbar_action(a: crate::widgets::toolbar::ToolbarAction) -> op_editor_core::ToolbarAction {
    use crate::widgets::toolbar::ToolbarAction as W;
    use op_editor_core::ToolbarAction as O;
    match a {
        W::Undo => O::Undo,
        W::Redo => O::Redo,
        W::ToggleVariablesPanel => O::ToggleVariablesPanel,
        W::ToggleDesignPanel => O::ToggleDesignPanel,
    }
}

/// Map a widget-layer `ToolbarHit` onto the canonical
/// `op_editor_core::ToolbarHover` so the host can store the
/// hovered item on `EditorUiState.toolbar_hover`.
pub fn toolbar_hover(hit: crate::widgets::toolbar::ToolbarHit) -> op_editor_core::ToolbarHover {
    use crate::widgets::toolbar::ToolbarHit as W;
    use op_editor_core::ToolbarHover as O;
    match hit {
        W::Tool(t) => O::Tool(t),
        W::Action(a) => O::Action(toolbar_action(a)),
        W::ToggleShapePicker => O::ShapeSlot,
    }
}

/// Map a widget-layer `FigmaImportHit` onto the canonical
/// `op_editor_core::FigmaImportButton` — `Some` for the two hoverable
/// targets, `None` for outside / dead-space hits.
pub fn figma_import_button(
    hit: crate::widgets::figma_import::FigmaImportHit,
) -> Option<op_editor_core::FigmaImportButton> {
    use crate::widgets::figma_import::FigmaImportHit as W;
    use op_editor_core::FigmaImportButton as O;
    match hit {
        W::Close => Some(O::Close),
        W::DropZone => Some(O::DropZone),
        W::Outside | W::Inside => None,
    }
}

/// Map a widget-layer `ExportDialogHit` onto the canonical
/// `op_editor_core::ExportDialogButton` for the modal export dialog's
/// hover wash. The `Format` arm reuses [`export_format`] to canonicalise
/// the widget `ExportFormat`.
pub fn export_dialog_button(
    hit: crate::widgets::export_dialog::ExportDialogHit,
) -> op_editor_core::ExportDialogButton {
    use crate::widgets::export_dialog::ExportDialogHit as W;
    use op_editor_core::ExportDialogButton as O;
    match hit {
        W::Format(f) => O::Format(export_format(f)),
        W::Scale(s) => O::Scale(s),
        W::Cancel => O::Cancel,
        W::Export => O::Export,
    }
}

/// Map a widget-layer `GitPanelHit` onto the canonical
/// `op_editor_core::GitButton` — `Some` only for the plain action
/// buttons that take a hover wash, `None` for inputs / rows / branch
/// trigger / popover-dismiss. Stored on `GitPanelState.button_hover`.
pub fn git_button_hover(
    hit: crate::widgets::git_panel::GitPanelHit,
) -> Option<op_editor_core::GitButton> {
    use crate::widgets::git_panel::GitPanelHit as W;
    use op_editor_core::GitButton as O;
    match hit {
        W::Pull => Some(O::Pull),
        W::Push => Some(O::Push),
        W::Overflow => Some(O::Overflow),
        W::Commit => Some(O::Commit),
        W::CommitMilestone => Some(O::CommitMilestone),
        W::Refresh => Some(O::Refresh),
        W::CloseDiff => Some(O::CloseDiff),
        W::DiffScrollUp => Some(O::DiffScrollUp),
        W::DiffScrollDown => Some(O::DiffScrollDown),
        W::DiffScrollLeft => Some(O::DiffScrollLeft),
        W::DiffScrollRight => Some(O::DiffScrollRight),
        W::SwitchBranch(i) => Some(O::SwitchBranch(i)),
        W::MergeBranch(i) => Some(O::MergeBranch(i)),
        W::ShowWorkingDiff => Some(O::ShowWorkingDiff),
        W::ShowCommitDiff(i) => Some(O::ShowCommitDiff(i)),
        W::RestoreCommit(i) => Some(O::RestoreCommit(i)),
        W::CopyCommitHash(i) => Some(O::CopyCommitHash(i)),
        W::ShowFileDiff(i) => Some(O::ShowFileDiff(i)),
        W::ToggleStageFile(i) => Some(O::ToggleStageFile(i)),
        W::ShowChangedFileDiff(i) => Some(O::ShowChangedFileDiff(i)),
        W::StageHunk(i) => Some(O::StageHunk(i)),
        W::AbortMerge => Some(O::AbortMerge),
        W::CompleteMerge => Some(O::CompleteMerge),
        W::MergeChoiceOurs(i) => Some(O::MergeChoiceOurs(i)),
        W::MergeChoiceTheirs(i) => Some(O::MergeChoiceTheirs(i)),
        W::ApplyMergeResolution => Some(O::ApplyMergeResolution),
        W::CancelMergeResolution => Some(O::CancelMergeResolution),
        W::BranchCreateMode => Some(O::BranchCreateMode),
        W::BranchMergeMode => Some(O::BranchMergeMode),
        W::BranchCreateSubmit => Some(O::BranchCreateSubmit),
        W::BranchPickerCancel => Some(O::BranchPickerCancel),
        W::OverflowRemoteSettings => Some(O::OverflowRemoteSettings),
        W::OverflowSshKeys => Some(O::OverflowSshKeys),
        W::OverflowSwitchTracked => Some(O::OverflowSwitchTracked),
        W::OverflowClearAuthor => Some(O::OverflowClearAuthor),
        W::OverflowCloseRepo => Some(O::OverflowCloseRepo),
        W::OverflowBack => Some(O::OverflowBack),
        W::TrackedPickerBind => Some(O::TrackedPickerBind),
        W::TrackedPickerBindOpen => Some(O::TrackedPickerBindOpen),
        W::TrackedPickerBack => Some(O::TrackedPickerBack),
        W::FetchRemote => Some(O::FetchRemote),
        W::SetRemote => Some(O::SetRemote),
        W::SetupSshAuth => Some(O::SetupSshAuth),
        W::SetHttpsAuth => Some(O::SetHttpsAuth),
        W::AuthorSave => Some(O::AuthorSave),
        W::AuthorCancel => Some(O::AuthorCancel),
        W::SshGenerateKey => Some(O::SshGenerateKey),
        W::SshImportKey => Some(O::SshImportKey),
        W::CloneDestPick => Some(O::CloneDestPick),
        W::CloneSubmit => Some(O::CloneSubmit),
        W::CloneCancel => Some(O::CloneCancel),
        // Inputs, popover-dismiss, branch trigger, empty cards (own state)
        // → no wash.
        _ => None,
    }
}

/// Map a widget-layer `AIChatHit` onto the canonical
/// `op_editor_core::ChatHeaderButton` — `Some` only for the three bare
/// header buttons that need a hover wash, `None` for every other chat
/// hit (input, send, chips, rows, drag handle, resize). Stored on
/// `EditorUiState.chat_header_hover`.
pub fn chat_header_hover(
    hit: &crate::widgets::AIChatHit,
) -> Option<op_editor_core::ChatHeaderButton> {
    use crate::widgets::AIChatHit as W;
    use op_editor_core::ChatHeaderButton as O;
    match hit {
        W::ToggleCollapse => Some(O::ToggleCollapse),
        W::ToggleMaximize => Some(O::ToggleMaximize),
        W::NewChat => Some(O::NewChat),
        _ => None,
    }
}

/// Map a widget-layer `TopBarHit` onto the canonical
/// `op_editor_core::TopBarButton` so the host can store the hovered
/// top-bar chrome button on `EditorUiState.topbar_button_hover`.
pub fn topbar_button_hover(
    hit: crate::widgets::top_bar::TopBarHit,
) -> op_editor_core::TopBarButton {
    use crate::widgets::top_bar::TopBarHit as W;
    use op_editor_core::TopBarButton as O;
    match hit {
        W::ToggleSidebar => O::ToggleSidebar,
        W::ToggleFileMenu => O::ToggleFileMenu,
        W::OpenFigmaImport => O::OpenFigmaImport,
        W::ToggleTheme => O::ToggleTheme,
        W::ToggleLocale => O::ToggleLocale,
        W::OpenAgentSettings => O::OpenAgentSettings,
        W::ToggleGitPanel => O::ToggleGitPanel,
        W::ToggleFullscreen => O::ToggleFullscreen,
    }
}

/// Map the widget-layer `widgets::export_dialog::ExportFormat` onto
/// the canonical `op_editor_core::ExportFormat`. Reverse of
/// [`doc_export_format`].
pub fn export_format(
    f: crate::widgets::export_dialog::ExportFormat,
) -> op_editor_core::ExportFormat {
    use crate::widgets::export_dialog::ExportFormat as W;
    use op_editor_core::ExportFormat as O;
    match f {
        W::Png => O::Png,
        W::Jpeg => O::Jpeg,
        W::Webp => O::Webp,
        W::Svg => O::Svg,
        W::Pdf => O::Pdf,
    }
}
