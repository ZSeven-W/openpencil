//! State-layer mirror of the Git panel's action buttons.
//!
//! `op_editor_ui::widgets::GitPanelHit` (the panel's full click enum)
//! lives in the widget crate and carries `usize` row payloads, so it
//! can't be stored on `EditorUiState` without `op-editor-core`
//! depending on the widget crate. `GitButton` captures just the
//! plain action buttons that take a `theme.button_hover` wash — the
//! branch trigger keeps its own `GitPanelState.branch_button_hovered`
//! bool, and rows / inputs / popover-dismiss hits never hover. Same
//! wasm32-clean discipline as the other `*_state` mirrors.

/// Which Git-panel action button the cursor is over. `None` on
/// `GitPanelState.button_hover` = no hover wash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitButton {
    /// Header `↓` — pull from the remote.
    Pull,
    /// Header `↑` — push to the remote.
    Push,
    /// Header `…` — toggle the overflow menu.
    Overflow,
    /// The plain Commit button (classic dirty-tree view).
    Commit,
    /// The "save milestone" button (ready view).
    CommitMilestone,
    /// The Refresh button.
    Refresh,
    /// Diff view: the `✕` close-diff control.
    CloseDiff,
    /// Diff view: the `▲` scroll-up control.
    DiffScrollUp,
    /// Diff view: the `▼` scroll-down control.
    DiffScrollDown,
    /// Diff view: the `◀` scroll-left control.
    DiffScrollLeft,
    /// Diff view: the `▶` scroll-right control.
    DiffScrollRight,
    /// Branch picker: a branch row (click to switch), by index.
    SwitchBranch(usize),
    /// Branch picker: a row's `⤵` merge-into-current button, by index.
    MergeBranch(usize),
    /// Working-tree status line — opens the whole-repo diff.
    ShowWorkingDiff,
    /// A recent-commit row (toggles its detail card), by index.
    ShowCommitDiff(usize),
    /// Expanded commit card "恢复" (restore) button, by index.
    RestoreCommit(usize),
    /// Expanded commit card "复制哈希" (copy hash) button, by index.
    CopyCommitHash(usize),
    /// A conflicted-file row (merge mode), by index.
    ShowFileDiff(usize),
    /// A changed-file row's stage checkbox, by index.
    ToggleStageFile(usize),
    /// A changed-file row's body (open its diff), by index.
    ShowChangedFileDiff(usize),
    /// A diff-view hunk's "Stage" button, by index.
    StageHunk(usize),
    /// "Abort Merge" button.
    AbortMerge,
    /// "Complete Merge" button.
    CompleteMerge,
    /// Merge-resolution row "Ours" choice, by index.
    MergeChoiceOurs(usize),
    /// Merge-resolution row "Theirs" choice, by index.
    MergeChoiceTheirs(usize),
    /// Merge-resolution "Apply" button.
    ApplyMergeResolution,
    /// Merge-resolution "Cancel" button.
    CancelMergeResolution,
    /// Footer "新建分支" — enter create-branch mode.
    BranchCreateMode,
    /// Footer "合并分支" — enter merge mode.
    BranchMergeMode,
    /// Inline create-branch submit button.
    BranchCreateSubmit,
    /// Cancel a create / merge sub-mode.
    BranchPickerCancel,
    /// Overflow menu: "Remote settings" entry.
    OverflowRemoteSettings,
    /// Overflow menu: "SSH keys" entry.
    OverflowSshKeys,
    /// Overflow menu: "Switch tracked file" entry.
    OverflowSwitchTracked,
    /// Overflow menu: "Clear commit author" entry.
    OverflowClearAuthor,
    /// Overflow menu: "Close repository" entry.
    OverflowCloseRepo,
    /// A subview's `‹ Back` row.
    OverflowBack,
    /// Tracked-file-picker candidate row, by index.
    TrackedPickerRow(usize),
    /// Tracked-file picker "Track this file" button.
    TrackedPickerBind,
    /// Tracked-file picker "Track and open" button.
    TrackedPickerBindOpen,
    /// Tracked-file picker Back / Cancel button.
    TrackedPickerBack,
    /// Remote-settings "获取" (fetch) button.
    FetchRemote,
    /// Remote-settings "Set origin" button.
    SetRemote,
    /// Remote-settings "SSH" auth button.
    SetupSshAuth,
    /// Remote-settings HTTPS "Login" button.
    SetHttpsAuth,
    /// Commit-signature form "保存" (save) button.
    AuthorSave,
    /// Commit-signature form "取消" (cancel) button.
    AuthorCancel,
    /// SSH subview "生成新密钥" (generate key) button.
    SshGenerateKey,
    /// SSH subview "导入现有密钥" (import key) button.
    SshImportKey,
    /// Clone view — open the destination folder picker.
    CloneDestPick,
    /// Clone view — submit `git clone`.
    CloneSubmit,
    /// Clone view — cancel back to the empty state.
    CloneCancel,
}
