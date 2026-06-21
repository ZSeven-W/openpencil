//! Editor-UI overlay + panel state for `EditorState`.
//!
//! This module ports the ~30 widget-layer UI fields that
//! `openpencil-shell-core::document::UiState` carries beyond the
//! editor-state subset already modelled by [`crate::ui_draft`]:
//!
//!   - menu / dropdown open flags + their hover targets
//!     (file menu, locale picker, shape picker, fill-type picker, …)
//!   - modal open flags (export dialog, figma import, agent settings)
//!   - the agent-settings modal struct (see [`crate::agent_settings`])
//!   - panel widths, theme mode, locale
//!   - layer / page hover + right-click context menu + page-rename
//!   - the property-panel tab + flex-layout + size toggles
//!   - export scale + format, recent files, pending file action
//!
//! ### Move STATE, not RENDER code
//!
//! Many of these types are *declared* under shell-core's `widgets/`
//! module — for example `ExportFormat` in `widgets/export_dialog.rs`.
//! They are data/state enums, not rendering code, so their type
//! definitions belong in the state layer. The widget *painting /
//! hit-test* code stays in shell-core untouched.
//!
//! All types here are plain data (enums + structs of primitives /
//! strings / ids), so `op-editor-core` stays wasm32-clean.

use crate::node_id::NodeId;
use crate::tool::Tool;
use std::collections::HashSet;

// `Locale` is the i18n locale enum — dependency-free + wasm-clean, so
// it lives in `op-i18n` and re-exports cleanly into the state layer.
pub use op_i18n::Locale;

/// Light / dark UI theme switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn flipped(self) -> Self {
        match self {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        }
    }
}

pub use crate::property_panel_state::{
    BooleanOp, ExportFormat, FillType, FlexLayout, ImageAdjustmentField, ImageFillMode,
    PaddingEditMode, PropertyTab,
};

/// File-menu actions the host runner has to handle (rfd dialogs +
/// serde live host-side, not here). `ExportImage` opens the picker;
/// `ExportImageConfirm` commits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    New,
    Open,
    Save,
    SaveAs,
    ExportImage,
    ExportImageConfirm,
    ImportFigma,
    /// User chose `Import image or SVG…` in the toolbar shape picker
    /// — host opens a file dialog, then inserts the raster image as a
    /// new Image node (or parses the SVG into nodes; SVG path lands
    /// as a follow-up).
    ImportImageOrSvg,
    /// User clicked the `图片` fill body row — host opens a file
    /// dialog and writes the chosen image into the selected node's
    /// primary fill as `PenFill::Image { url: <data-url> }`.
    PickFillImage,
    /// User clicked the image-section warning row's Relink button —
    /// host opens a file dialog and rewrites the selected image
    /// node's `src` (relative to the document path when possible,
    /// TS `toStoredAssetPath`).
    RelinkImage,
    OpenRecent(usize),
    ClearRecent,
}

/// Theme-preset file IO the host must run (rfd dialog + fs IO live
/// host-side). Raised by the preset dropdown's Import / Export rows
/// (TS `variable-theme-manager.tsx:153-164`); drained by the desktop
/// host (`theme_preset_host.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePresetIo {
    /// `variables.importPreset` — pick a `.optheme` file and merge
    /// its themes + variables into the document.
    Import,
    /// `variables.exportPreset` — save the current document themes +
    /// variables as `theme-preset.optheme`.
    Export,
}

/// Auto-update status surfaced in the settings modal's System tab.
///
/// The desktop host runs a background probe against the GitHub
/// releases API and writes the outcome here; the System tab paints
/// from it. `Idle` is the pre-probe state, `Checking` while the
/// request is in flight. `Available` carries the newer release tag
/// so the tab can name the version the user can upgrade to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    /// No check has run yet (also the web build's permanent state).
    #[default]
    Idle,
    /// A release-API request is in flight.
    Checking,
    /// The running build is the latest published release.
    UpToDate,
    /// A newer release exists — carries its version string.
    Available { version: String },
    /// The probe failed (offline, rate-limited, parse error).
    Error,
}

/// One commit row shown in the Git panel — plain data snapshotted
/// by the desktop host from its git session. The platform-free
/// widget layer only paints it; it never calls git itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitSummary {
    /// Abbreviated commit hash.
    pub short_hash: String,
    /// First line of the commit message.
    pub summary: String,
    /// Author display name.
    pub author: String,
    /// Pre-formatted relative-time label (`now` / `5m` / `2h` / …),
    /// computed host-side against the wall clock when the snapshot is
    /// taken (TS `formatCompactTime`). The widget layer is platform-free
    /// and has no wall clock, so it cannot derive this itself.
    pub time_label: String,
    /// `true` for the root commit (no parent). The expanded detail card
    /// shows the "initial commit — nothing to diff" line for it (TS
    /// `git.history.diff.initialCommit`).
    pub is_initial: bool,
}

/// One `.op` candidate in the tracked-file picker (TS `GitCandidateFileInfo`).
/// Plain data the host enumerates from the repo; the widget only paints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCandidateFile {
    /// Absolute path — the bind argument.
    pub path: String,
    /// Repo-relative path — the row title.
    pub relative_path: String,
    /// Number of commits that touched this file (the "N milestones" label).
    pub milestone_count: u32,
    /// Pre-formatted relative time of the last commit touching it, or empty.
    pub last_commit_time: String,
    /// First line of the last commit's message, if any.
    pub last_commit_message: Option<String>,
}

/// One node-level change in a commit's semantic diff (TS `NodePatch`,
/// rendered as `<op> <nodeId>` in the inline detail card).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDiffPatch {
    /// `add` / `remove` / `modify` / `move`.
    pub op: String,
    /// The affected node's id.
    pub node_id: String,
}

/// Aggregated semantic diff of one commit against its parent — the TS
/// `engineDiff` result that drives `GitPanelHistoryDiff`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommitDiffSummary {
    /// Distinct parent ids touched by any patch.
    pub frames_changed: u32,
    pub nodes_added: u32,
    pub nodes_removed: u32,
    pub nodes_modified: u32,
    /// Per-node patch list (newest-first walk order).
    pub patches: Vec<CommitDiffPatch>,
}

/// Lazy state of the expanded commit's inline diff (TS `DiffState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitDiffView {
    /// The host is computing the diff on a worker / this frame.
    Loading,
    /// The root commit — no parent to diff against.
    Initial,
    /// Diff computed, but no node changed (rare; e.g. metadata-only).
    NoChanges,
    /// The diff could not be computed (parse / git error). Carries the message.
    Error(String),
    /// Computed diff ready to render.
    Ready(CommitDiffSummary),
}

/// One changed file in the Git panel's staging list — plain data
/// snapshotted by the desktop host from `git status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFileEntry {
    /// Repo-relative path.
    pub path: String,
    /// Whether the change is staged in the index.
    pub staged: bool,
    /// Single-char status: `M` / `A` / `D` / `R` / `?` / `U`.
    pub status: char,
}

/// What a Git-panel diff request should diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitDiffTarget {
    /// The whole working tree's unstaged changes (`git diff`).
    WorkingTree,
    /// One repo-relative path's working-tree changes (`git diff -- <path>`).
    Path(String),
    /// The full patch a commit introduced (`git show <rev>`).
    Commit(String),
}

/// A unified-diff view open inside the Git panel — filled by the
/// desktop host from a background `git diff` / `git show` job.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitDiffView {
    /// Human label for the diff (a path, or a commit summary).
    pub title: String,
    /// The diff text split into lines for per-line colouring.
    pub lines: Vec<String>,
    /// Index of the first visible line — paged by the ▲ / ▼ buttons
    /// and the mouse wheel.
    pub scroll: usize,
    /// First visible character column — long lines scroll sideways
    /// with the ◀ / ▶ buttons.
    pub h_scroll: usize,
    /// Repo-relative path when this diff is a single working-tree
    /// file that supports per-hunk staging — `None` for a commit
    /// diff or the whole-tree diff.
    pub stage_path: Option<String>,
}

/// One node conflict in the interactive merge-resolution view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeConflictRow {
    /// The conflicting node's `id`.
    pub id: String,
    /// Display label — the node's name / type.
    pub label: String,
    /// Human label for the conflict kind (e.g. "both modified").
    pub kind: String,
    /// Whether "theirs" is a selectable resolution — `false` for a
    /// structural conflict, which can only be resolved to "ours".
    pub theirs_allowed: bool,
    /// The chosen side: `false` = keep ours, `true` = take theirs.
    pub take_theirs: bool,
}

/// One conflicted `.op` file in the merge-resolution view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeResolveFile {
    /// Repo-relative path.
    pub path: String,
    /// The three merge-stage blobs — kept so "Apply" can re-run the
    /// structured merge with the user's per-node choices.
    pub base: String,
    pub ours: String,
    pub theirs: String,
    /// The file's node conflicts.
    pub conflicts: Vec<MergeConflictRow>,
}

/// Interactive merge-conflict-resolution state — set when a branch
/// merge conflicts entirely in structured `.op` files. The panel
/// shows each conflicting node with an ours/theirs choice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeResolveState {
    /// The branch being merged in.
    pub branch: String,
    /// The conflicted `.op` files.
    pub files: Vec<MergeResolveFile>,
}

impl MergeResolveState {
    /// Total conflict count across every file.
    pub fn total(&self) -> usize {
        self.files.iter().map(|f| f.conflicts.len()).sum()
    }

    /// Every conflict row, flattened in file order — the order the
    /// panel paints and hit-tests.
    pub fn rows(&self) -> Vec<&MergeConflictRow> {
        self.files.iter().flat_map(|f| &f.conflicts).collect()
    }

    /// Set the choice of the flat-indexed conflict row. A `theirs`
    /// choice on a structural conflict falls back to "ours".
    pub fn set_choice(&mut self, flat_index: usize, take_theirs: bool) {
        let mut i = 0;
        for file in &mut self.files {
            for row in &mut file.conflicts {
                if i == flat_index {
                    row.take_theirs = take_theirs && row.theirs_allowed;
                    return;
                }
                i += 1;
            }
        }
    }
}

/// Which view the ready-state overflow `…` popover is showing. The
/// top-level menu opens subviews in place (mirrors the TS header's
/// `overflowView` state machine), resetting to `Menu` on close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitOverflowView {
    /// The top-level action list.
    #[default]
    Menu,
    /// The remote-settings subview — origin URL + HTTPS credential.
    RemoteSettings,
    /// The tracked-file picker subview — pick which `.op` the panel tracks.
    TrackedPicker,
    /// The SSH-keys subview — list keys + import / generate.
    SshKeys,
}

/// Which sub-mode the branch-picker dropdown is showing (mirrors the
/// TS `GitPanelBranchPicker` `mode` state machine). Resets to `List`
/// when the dropdown closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitBranchPickerMode {
    /// Branch list + the `新建分支` / `合并分支` footer actions.
    #[default]
    List,
    /// Inline `新建分支` form — a branch-name text input.
    Create,
    /// `合并分支` mode — pick a non-current branch to merge into HEAD.
    Merge,
}

/// An interactive action requested from the Git panel. The desktop
/// host drains it from [`GitPanelState::pending_action`] and runs it
/// against its `GitSession` (the widget layer never calls git).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitPanelAction {
    /// Empty-state Init card — create a local repo for the saved doc.
    InitRepo,
    /// Empty-state Open card — pick + bind an existing repo folder.
    OpenRepo,
    /// Empty-state Clone card — clone a remote into a chosen folder.
    CloneRepo,
    /// Re-read repository state into the panel.
    Refresh,
    /// Pull the current branch's upstream.
    Pull,
    /// Push the current branch to its upstream.
    Push,
    /// Stage + commit the tracked document with the panel's
    /// `commit_input`.
    Commit,
    /// Ready-view "Save milestone": save the current design to the
    /// tracked `.op`, stage it, and commit with the panel's
    /// `commit_input` — the TS `commitMilestone` flow. Unlike
    /// [`GitPanelAction::Commit`] (which commits a pre-assembled staged
    /// index) this snapshots the live editor state in one click.
    CommitMilestone,
    /// Switch the working tree to the named branch.
    SwitchBranch(String),
    /// Create a new branch with the given name (from the inline
    /// `新建分支` form) and switch to it.
    CreateBranch(String),
    /// Add / re-point the `origin` remote to the given URL.
    SetRemote(String),
    /// Generate (or reuse) an SSH key for the `origin` host and bind
    /// it as that host's stored credential.
    SetupSshAuth,
    /// Store an HTTPS credential for the `origin` host — the payload
    /// is the `username:token` text typed into the Remotes section.
    SetHttpsAuth(String),
    /// Merge the named branch into the current one through an
    /// isolated worktree (the live tree is never marked up).
    MergeBranch(String),
    /// Abort an in-progress merge, restoring the pre-merge state.
    AbortMerge,
    /// Finalize an in-progress merge once its conflicts are resolved.
    CompleteMerge,
    /// Compute a unified diff and open it in the panel's diff view.
    ShowDiff(GitDiffTarget),
    /// Toggle whether the named changed file is staged in the index.
    ToggleStageFile(String),
    /// Stage a single hunk of the open diff — `(path, hunk_index)`.
    StageHunk(String, usize),
    /// Re-run the branch merge applying the per-node ours/theirs
    /// choices the user picked in the merge-resolution view.
    ApplyMergeResolution,
    /// Clone-form "选择…" — open a native folder picker and write the
    /// chosen path into the form's `dest` field.
    PickCloneDest,
    /// Clone-form submit — `git clone <url> <dest>` on a worker thread,
    /// then bind the cloned repo. Reads url / dest from `clone_form`.
    SubmitClone,
    /// Roll the tracked document back to the given commit (hash) and
    /// reload the editor — the TS `restoreCommit`. The payload is the
    /// commit's (short) hash from the expanded detail card.
    RestoreCommit(String),
    /// Copy the given commit hash to the OS clipboard (TS copy-hash).
    CopyHash(String),
    /// Compute the semantic diff of `recent_commits[index]` against its
    /// parent and store it in `expanded_commit_diff` (TS `computeDiff`,
    /// triggered when a commit row's detail card is expanded).
    LoadCommitDiff(usize),
    /// Overflow "切换跟踪文件" — enumerate the repo's `.op` candidates into
    /// `candidate_files` and open the tracked-file picker subview.
    EnterTrackedPicker,
    /// Bind the panel to the given `.op` path (TS `bindTrackedFile`). The
    /// `bool` is "also load it into the editor" (TS "track and open").
    BindTrackedFile(String, bool),
    /// Overflow "清除提交作者" — clear the stored commit-author identity.
    ClearAuthor,
    /// Overflow "关闭仓库" — unbind the repository and reset to empty state.
    CloseRepo,
    /// Overflow "SSH 密钥" — enumerate stored SSH keys + open the subview.
    EnterSshKeys,
    /// SSH subview "导入现有密钥" — pick a private key file and import it.
    ImportSshKey,
    /// Remote-settings "获取" — run `git fetch` on the origin remote.
    FetchRemote,
    /// Commit-signature form "保存" — write the name/email drafts into the
    /// repo identity, then re-fire the pending milestone commit.
    SaveAuthor,
}

/// Which clone-form text field has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneField {
    /// The remote URL (`https://…` / `git@…`).
    Url,
    /// The local destination folder.
    Dest,
}

/// Inline clone-wizard state — `Some` on `GitPanelState.clone_form`
/// puts the panel into the clone view (a port of the TS
/// `GitPanelCloneForm`). Reached from the empty-state Clone card. Plain
/// data so the widget layer stays wasm-clean; the desktop host owns the
/// folder picker + the `git clone` job.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CloneFormState {
    /// Remote URL draft.
    pub url_input: jian_core::text_input::TextInputState,
    /// Local destination-folder draft.
    pub dest_input: jian_core::text_input::TextInputState,
    /// Which field has keyboard focus (`None` = no caret).
    pub focus: Option<CloneField>,
    /// `true` while the `git clone` worker runs — disables the form.
    pub cloning: bool,
    /// Last clone error (validation or a failed `git clone`), shown
    /// under the fields.
    pub error: Option<String>,
}

/// Git panel state — a plain-data snapshot the desktop host fills
/// from its `GitSession`. The widget layer reads it to paint the
/// floating Git panel; it carries no git handles, so it stays
/// wasm-clean. Refreshed whenever the panel is opened.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GitPanelState {
    /// Whether the floating Git panel is currently shown.
    pub open: bool,
    /// Whether the open document lives inside a git repository.
    pub in_repo: bool,
    /// Whether the open document has an on-disk path — gates the
    /// empty-state "Init" card (can't create local history for an
    /// unsaved doc). Set by the host on each panel refresh.
    pub has_saved_file: bool,
    /// Which empty-state onboarding card the cursor is over, if any
    /// (`0` = Init, `1` = Open, `2` = Clone). Drives the per-card hover
    /// effect and the disabled-Init hint pill (shown only while card
    /// `0` is hovered with no saved file). Updated by the host on
    /// cursor-move.
    pub empty_hovered_card: Option<u8>,
    /// Ready-state header: whether the cursor is over the `⎇ <branch> ▾`
    /// button. Drives its `hover:bg-accent` wash. Updated by the host on
    /// cursor-move.
    pub branch_button_hovered: bool,
    /// Which plain Git action button (pull / push / overflow / commit /
    /// milestone / refresh) the cursor is over — drives its
    /// `theme.button_hover` wash. Updated by the host on cursor-move.
    pub button_hover: Option<crate::git_button_state::GitButton>,
    /// Ready-state header: whether the `…` overflow popover is open
    /// (switch-tracked / clear-author / remote-settings › / SSH-keys › /
    /// close-repo). Mirrors the TS header's local `overflowOpen`.
    pub overflow_open: bool,
    /// Shared interaction state for the top-level overflow menu rows.
    pub overflow_menu: jian_widgets::components::menu::MenuState,
    /// Which view the overflow popover is showing — the top-level menu
    /// or one of its subviews (remote settings). Resets to `Menu` each
    /// time the popover closes. Mirrors the TS header's `overflowView`.
    pub overflow_view: GitOverflowView,
    /// Ready-state header: whether the branch-picker dropdown (opened
    /// from the `⎇ <branch> ▾` button) is open.
    pub branch_picker_open: bool,
    /// Shared interaction state for branch-picker dropdown rows.
    pub branch_picker_menu: jian_widgets::components::menu::MenuState,
    /// Current branch name of that repository.
    pub branch: Option<String>,
    /// All local branch names, sorted — the panel lists them for
    /// one-click switching.
    pub branches: Vec<String>,
    /// Which branch-picker sub-mode is showing (list / create / merge).
    pub branch_picker_mode: GitBranchPickerMode,
    /// Draft branch name typed into the inline `新建分支` form.
    pub branch_create_input: jian_core::text_input::TextInputState,
    /// Whether the `新建分支` name input holds keyboard focus.
    pub branch_create_focused: bool,
    /// Number of changed (dirty) files in the working tree.
    pub dirty_count: usize,
    /// Commits the current branch is ahead of its upstream — gates the
    /// Push button (TS disables Push when `ahead === 0`).
    pub ahead: u32,
    /// Commits the local branch is behind its upstream (remote-settings row).
    pub behind: u32,
    /// The `origin` remote's host (e.g. `github.com`), parsed host-side.
    /// Drives the remote-settings credentials row; `None` = no host detected.
    pub remote_host: Option<String>,
    /// Stored-credential kind for `remote_host`: `"token"` / `"ssh"` /
    /// `"none"` (empty when there's no host). Host-filled.
    pub stored_auth: String,
    /// Number of files with unresolved merge conflicts.
    pub conflicted_count: usize,
    /// Whether a merge is in progress — drives the panel's conflict
    /// mode (conflicted-file list + Abort / Complete actions).
    pub merging: bool,
    /// Repo-relative paths with unresolved merge conflicts.
    pub conflicted_files: Vec<String>,
    /// Changed files in the working tree — the per-file staging list.
    pub changed_files: Vec<GitFileEntry>,
    /// Configured remotes as display strings — `name → url`.
    pub remotes: Vec<String>,
    /// Draft URL typed into the Remotes section's input box.
    pub remote_input: jian_core::text_input::TextInputState,
    /// Whether the remote-URL input holds keyboard focus.
    pub remote_focused: bool,
    /// Draft `username:token` typed into the HTTPS-credential input.
    pub https_input: jian_core::text_input::TextInputState,
    /// Whether the HTTPS-credential input holds keyboard focus.
    pub https_focused: bool,
    /// Most-recent commits, newest first.
    pub recent_commits: Vec<GitCommitSummary>,
    /// Index into `recent_commits` of the row whose inline detail card
    /// (里程碑详情 — restore + copy-hash) is expanded, if any. Pure UI
    /// state, toggled by clicking a commit row. Cleared host-side when
    /// the commit list changes so it can't point at a stale commit
    /// (TS keys the card by hash; the widget layer keys by index).
    pub expanded_commit: Option<usize>,
    /// Lazy semantic diff for the expanded commit (TS `GitPanelHistoryDiff`).
    /// `None` when no card is open; otherwise loading / initial / ready /
    /// error. The host fills it after a `LoadCommitDiff` action.
    pub expanded_commit_diff: Option<CommitDiffView>,
    /// Candidate `.op` files for the tracked-file picker subview, host-filled
    /// when the picker opens (TS `RepoMeta.candidateFiles`).
    pub candidate_files: Vec<GitCandidateFile>,
    /// The picker's currently-selected candidate index, if any.
    pub tracked_picker_selected: Option<usize>,
    /// Shared select state for the tracked-file picker row list.
    pub tracked_picker: jian_widgets::components::select::SelectState,
    /// SSH key names for the SSH-keys subview (host-filled on open).
    pub ssh_keys: Vec<String>,
    /// Commit-message draft typed into the panel's input box.
    pub commit_input: jian_core::text_input::TextInputState,
    /// Whether the commit-message input holds keyboard focus.
    pub commit_focused: bool,
    /// Set when a milestone "save" was skipped because the saved design
    /// matched the last commit — the ready view shows a "未检测到变更" hint
    /// under the commit box. Cleared when the user re-engages the input.
    pub commit_no_changes: bool,
    /// Whether the commit-signature form (`提交署名`) is showing in place of
    /// the commit box — raised when a commit is attempted with no committer
    /// identity (TS `authorPromptVisible`). The pending message stays in
    /// `commit_input` and the commit re-fires after a successful save.
    pub author_prompt: bool,
    /// Name / email drafts typed into the commit-signature form.
    pub author_name_input: jian_core::text_input::TextInputState,
    pub author_email_input: jian_core::text_input::TextInputState,
    /// Which signature-form field holds keyboard focus.
    pub author_name_focused: bool,
    pub author_email_focused: bool,
    /// Interactive action requested by a panel click / Enter —
    /// drained and executed by the desktop host.
    pub pending_action: Option<GitPanelAction>,
    /// Whether a background `git pull` is currently in flight — the
    /// panel shows a "Pulling…" status and disables the Pull button.
    pub pulling: bool,
    /// Whether a background `git push` is currently in flight — the
    /// panel shows a "Pushing…" status and disables the Push button.
    pub pushing: bool,
    /// Whether the panel is awaiting its first repository snapshot
    /// after opening / a repo switch. While `true` the panel shows a
    /// "Loading…" state instead of the (possibly stale) prior data.
    pub loading: bool,
    /// Open diff view — `Some` puts the panel into diff mode, showing
    /// a scrollable unified diff instead of the status / action area.
    /// Closed by the diff view's ✕ button.
    pub diff: Option<GitDiffView>,
    /// Interactive merge-conflict-resolution view — `Some` puts the
    /// panel into resolution mode, listing each conflicting node with
    /// an ours/theirs choice. Cleared on Apply / Cancel.
    pub merge_resolve: Option<MergeResolveState>,
    /// Inline clone wizard — `Some` puts the panel into the clone view
    /// (URL + destination + Clone / Cancel), reached from the empty-state
    /// Clone card. Cleared on Cancel or a successful clone.
    pub clone_form: Option<CloneFormState>,
}

impl GitPanelState {
    /// Whether the ready-view header popovers (the branch picker and the
    /// `…` overflow menu) may be open in this state. They live only in
    /// the bound, non-merging ready view. A dirty working tree still
    /// shows that view (TS parity — the ready view no longer gates on a
    /// clean tree), so dirtiness does NOT disqualify them; only an
    /// unbound repo or an in-progress merge does. A background status
    /// refresh that lands a non-ready state uses this to force-close the
    /// popovers so they can't go stale and dead-end input.
    pub fn header_popovers_allowed(&self) -> bool {
        self.in_repo && !self.merging
    }

    pub fn defocus_commit_input(&mut self, now_ms: u64) -> bool {
        let was_focused = self.commit_focused;
        self.commit_focused = false;
        let commit_caret = self.commit_input.caret();
        self.commit_input.set_caret(commit_caret, now_ms);
        was_focused
    }

    /// Drop keyboard focus from every git-panel text input — commit
    /// message, remote URL, HTTPS credential, branch-create name, the
    /// author signature pair, and the clone form's URL / destination.
    /// Drafts persist (focus flags only) so re-engaging an input shows
    /// what was typed. Returns `true` when any input was focused so
    /// blank-press / panel-close callers know to repaint.
    pub fn defocus_text_inputs(&mut self) -> bool {
        let clone_focused = self
            .clone_form
            .as_mut()
            .map(|form| {
                let url_caret = form.url_input.caret();
                form.url_input.set_caret(url_caret, 0);
                let dest_caret = form.dest_input.caret();
                form.dest_input.set_caret(dest_caret, 0);
                form.focus.take().is_some()
            })
            .unwrap_or(false);
        let commit_focused = self.defocus_commit_input(0);
        let was_focused = clone_focused
            || commit_focused
            || self.remote_focused
            || self.https_focused
            || self.branch_create_focused
            || self.author_name_focused
            || self.author_email_focused;
        self.remote_focused = false;
        self.https_focused = false;
        self.branch_create_focused = false;
        self.author_name_focused = false;
        self.author_email_focused = false;
        let remote_caret = self.remote_input.caret();
        self.remote_input.set_caret(remote_caret, 0);
        let https_caret = self.https_input.caret();
        self.https_input.set_caret(https_caret, 0);
        let branch_caret = self.branch_create_input.caret();
        self.branch_create_input.set_caret(branch_caret, 0);
        let author_name_caret = self.author_name_input.caret();
        self.author_name_input.set_caret(author_name_caret, 0);
        let author_email_caret = self.author_email_input.caret();
        self.author_email_input.set_caret(author_email_caret, 0);
        was_focused
    }

    /// Open the tracked-file picker row list and reset transient select
    /// interaction state for a fresh candidate set.
    pub fn open_tracked_picker(&mut self) {
        self.tracked_picker.open = true;
        self.tracked_picker.hover = None;
        self.tracked_picker.pressed = None;
        self.tracked_picker.scroll.offset = 0.0;
        self.tracked_picker_selected = None;
    }

    /// Close the tracked-file picker and clear its transient selection.
    pub fn close_tracked_picker(&mut self) -> bool {
        let changed = self.tracked_picker.open
            || self.tracked_picker.hover.is_some()
            || self.tracked_picker.pressed.is_some()
            || self.tracked_picker.scroll.offset != 0.0
            || self.tracked_picker_selected.is_some();
        self.tracked_picker.open = false;
        self.tracked_picker.hover = None;
        self.tracked_picker.pressed = None;
        self.tracked_picker.scroll.offset = 0.0;
        self.tracked_picker_selected = None;
        changed
    }
}

/// File-menu "Recent files" entry — host persists via settings IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFile {
    pub path: String,
    /// Unix seconds when last touched.
    pub modified_at: u64,
}

/// Maximum number of recent files shown in the File menu.
pub const RECENT_FILE_CAP: usize = 10;

// What the LayerPanel right-click context menu is acting on — the
// canonical definition is `ui_draft::LayerContextTarget` (it backs
// the inline-rename draft too). Re-exported so UI code that
// references a context target has one import path.
pub use crate::ui_draft::LayerContextTarget;

/// Right-click context-menu state.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerContextMenuState {
    pub target: LayerContextTarget,
    pub anchor_x: f32,
    pub anchor_y: f32,
    /// Hovered row index for the menu paint; `None` = no row hovered.
    pub menu: jian_widgets::components::menu::MenuState,
}

/// Inline-rename state for a page row (double-click → rename).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRenameState {
    pub page_index: usize,
    pub draft: String,
}

/// Editor focus for a variable row cell in the VariablesPanel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRowFocus {
    Name(usize),
    Number(usize),
    String(usize),
    NumberCell {
        row: usize,
        variant: usize,
    },
    StringCell {
        row: usize,
        variant: usize,
    },
    /// Inline hex editing of a Color cell under one variant column
    /// (TS `variable-row.tsx` ColorCell hex `<input>`). The draft is
    /// committed only when it parses as a full `#rrggbb`, mirroring
    /// TS's `/^#[0-9a-fA-F]{6}$/` gate.
    ColorCell {
        row: usize,
        variant: usize,
    },
}

/// Keyboard focus on an effect-parameter value (the Effects
/// section's editable X / Y / Blur / Spread / Radius numbers).
/// `effect` is the index of the effect on the selected node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectParamFocus {
    pub effect: usize,
    pub field: crate::EffectField,
}

/// Editor-UI overlay + panel state — the widget-layer toggles, hover
/// targets, menu / modal open flags and panel metrics that the ~30
/// editor widgets paint from. Faithful superset of the UI subset
/// of shell-core's `UiState`.
///
/// The *editor-state* subset of `UiState` (focused property field +
/// drafts, pen-tool path, color picker, text-edit drafts, variable
/// caches, active page index) lives on [`crate::ui_draft::UiDraftState`]
/// and is not duplicated here.
#[derive(Debug, Clone)]
pub struct EditorUiState {
    // --- Sidebar + panel metrics -----------------------------------
    pub sidebar_open: bool,
    pub layer_panel_width: f32,
    pub property_panel_width: f32,

    // --- Theme + locale --------------------------------------------
    /// Active UI theme — TopBar Sun icon flips it.
    pub theme_mode: ThemeMode,
    /// UI locale — TopBar Globe cycles.
    pub locale: Locale,
    /// TopBar Globe dropdown state.
    pub locale_picker: jian_widgets::components::select::SelectState,

    // --- File menu --------------------------------------------------
    /// File-menu dropdown open (anchored under folder + chevron).
    pub file_menu_open: bool,
    /// Shared file-menu interaction state; `hover = None` means no
    /// actionable row hovered.
    pub file_menu: jian_widgets::components::menu::MenuState,
    /// Pending file-menu action for the host runner to handle.
    pub pending_file_action: Option<FileAction>,
    /// Recent files (head = newest, cap 10).
    pub recent_files: Vec<RecentFile>,
    /// TopBar display name; `None` = "Untitled".
    pub file_name_display: Option<String>,

    // --- Modals -----------------------------------------------------
    /// Raster export scale (1.0 / 2.0 / 3.0). Default 2.0.
    pub export_scale: f32,
    pub export_dialog_open: bool,
    /// Which modal export-dialog button the cursor is over — drives the
    /// per-button `theme.button_hover` wash. Updated by the host on
    /// cursor-move while the dialog is open.
    pub export_dialog_hover: Option<crate::export_dialog_state::ExportDialogButton>,
    pub export_format: ExportFormat,
    /// Property-panel Export section: the scale dropdown's inline
    /// select popup is open. Mutually exclusive with
    /// `export_format_picker_open` — opening one closes the other.
    pub export_scale_picker_open: bool,
    /// Property-panel Export section: the format dropdown's inline
    /// select popup is open.
    pub export_format_picker_open: bool,
    /// Row index the cursor is over in the open Export select popup
    /// (drives the row hover highlight). `None` when no popup is
    /// open or the cursor is off every row.
    pub export_picker_hover: Option<usize>,
    /// Index into the PropertyPanel's `action_button_rects_with_fill_picker`
    /// walker of the action button the cursor is over — drives its
    /// `theme.button_hover` wash. Stable per frame (same VisibleSections
    /// + fill-picker state feed paint + the host hover update).
    pub property_action_hover: Option<usize>,
    /// PropertyPanel Design / Code tab currently under the cursor. Drives a
    /// visible pill on inactive tabs.
    pub property_tab_hover: Option<PropertyTab>,
    /// Vertical scroll offset of the right-rail PropertyPanel, in px
    /// (≥ 0). A wheel / trackpad pan over the inspector advances it;
    /// paint + hit-test shift the section content up by this amount
    /// so a tall inspector (many effects, etc.) stays reachable.
    pub property_panel_scroll: jian_core::scroll::ScrollState,
    /// Vertical scroll offset (px, ≥ 0) of the LayerPanel's 页面
    /// (Pages) section — that section has a bounded height, so a
    /// long page list scrolls within it.
    pub layer_pages_scroll: jian_core::scroll::ScrollState,
    /// Vertical scroll offset (px, ≥ 0) of the LayerPanel's 图层
    /// (Layers) section row viewport.
    pub layer_layers_scroll: jian_core::scroll::ScrollState,
    /// Horizontal scroll offset (px, ≥ 0) of the LayerPanel's 页面
    /// row content. The row chrome stays fixed; only tree content shifts.
    pub layer_pages_h_scroll: jian_core::scroll::ScrollState,
    /// Horizontal scroll offset (px, ≥ 0) of the LayerPanel's 图层
    /// tree content. Needed for deeply nested layer trees.
    pub layer_layers_h_scroll: jian_core::scroll::ScrollState,
    /// "Import from Figma" modal.
    pub figma_import_open: bool,
    /// Which Figma-import target the cursor is over (close / drop-zone)
    /// — drives the `theme.button_hover` wash. Host updates on cursor-move.
    pub figma_import_hover: Option<crate::figma_import_state::FigmaImportButton>,
    /// True while a `.fig` is being parsed on a worker thread. Paint
    /// uses this to show a "正在解析 Figma 文件…" overlay so the user
    /// gets feedback during the multi-second parse (a 2-3 MB .fig with
    /// hundreds of nodes can take a couple of seconds to walk the
    /// Kiwi schema, build the tree, and convert every node). The
    /// desktop runner sets it when spawning the worker and clears it
    /// when the result lands.
    pub figma_import_in_progress: bool,
    /// True while a file is being dragged over the window (between the
    /// platform's `HoveredFile` and `HoveredFileCancelled` / drop). Drives
    /// the full-canvas drop overlay so the user sees a clear drop target.
    pub file_drop_active: bool,
    /// Imported Figma documents parsed in Preserve mode already carry
    /// authored parent-local geometry. The scene builder can use this
    /// flag to skip the expensive flex/text layout pass.
    pub preserve_authored_geometry: bool,
    // --- Canvas preview (Play) mode -------------------------------
    /// Whether the canvas is in **Preview** (Play) mode. When `true`,
    /// the editor stops painting selection/handles + editor chrome
    /// affordances over the canvas and instead drives a live jian
    /// runtime (host-local — the runtime is `!Send` so it lives on the
    /// native host, NOT on this wasm32-clean state). Entering does NOT
    /// mutate `doc`; exiting drops the runtime and leaves `doc`
    /// byte-identical. The TopBar Play/Stop button + `Esc` toggle it.
    pub preview_mode: bool,
    /// Non-fatal warnings raised when the runtime was last built from
    /// `doc` for Preview — e.g. legacy role-frames promoted to widget
    /// nodes (`LegacyRolePromoted`). Surfaced for diagnostics; cleared
    /// on exit. Never serialized.
    pub preview_warnings: Vec<String>,
    /// Floating `Cmd+,` agent-settings modal open.
    pub agent_settings_open: bool,
    pub agent_settings: crate::agent_settings::AgentSettings,
    pub agent_settings_drag: Option<crate::agent_settings::AgentSettingsDrag>,
    /// Draft, caret, selection, and blink state for the focused
    /// settings-modal input.
    pub settings_input: jian_core::text_input::TextInputState,

    // --- Toolbar shape slot ----------------------------------------
    /// Toolbar shape-tool dropdown state.
    pub shape_picker: jian_widgets::components::select::SelectState,
    /// Toolbar button currently hovered — drives the per-button
    /// `theme.button_hover` wash on the vertical tool column.
    pub toolbar_hover: Option<crate::toolbar_state::ToolbarHover>,
    /// Last-selected shape tool — drives the toolbar shape slot's
    /// icon. Always one of Rect / Ellipse / Polygon / Line / Pen.
    pub shape_tool: Tool,
    /// Toolbar/property icon picker interaction state.
    pub icon_picker: jian_widgets::components::select::SelectState,
    /// True when the icon picker should replace the selected icon
    /// instead of inserting a new icon at the canvas centre.
    pub icon_picker_replace_selection: bool,
    /// Top-left corner of the floating Icon picker in logical px.
    /// `None` until first dragged/opened, then reused across opens.
    pub icon_picker_panel_pos: Option<(f32, f32)>,
    /// Live text filter for the native Lucide icon picker.
    pub icon_picker_search: String,
    /// True after Cmd/Ctrl+A in the icon search box. The next edit
    /// replaces the whole search query.
    pub icon_picker_select_all: bool,
    /// Remote Iconify search results appended by the desktop host.
    pub icon_picker_remote: crate::icon_picker_state::IconPickerRemoteState,
    /// Queued "load more" request drained asynchronously by desktop.
    pub icon_picker_load_more_request: Option<crate::icon_picker_state::IconifyLoadMoreRequest>,

    // --- AI chat model picker --------------------------------------
    /// AI chat model-picker dropdown interaction state.
    pub chat_model_picker: jian_widgets::components::select::SelectState,
    /// Text filter, caret, selection, and blink state for the chat
    /// model-picker search box.
    pub chat_model_picker_input: jian_core::text_input::TextInputState,
    /// Hovered chat design JSON card `(message_index, block_index)`;
    /// drives the TS-style hover reveal of the card's copy affordance.
    pub chat_design_block_hover: Option<(usize, usize)>,
    /// Index of the empty-state quick action card under the cursor.
    pub chat_example_hover: Option<usize>,
    /// Which bare chat header button (chevron / maximize / new-chat)
    /// the cursor is over — drives their `theme.button_hover` wash.
    pub chat_header_hover: Option<crate::chat_button_state::ChatHeaderButton>,
    /// Which bottom-toolbar chat control the cursor is over.
    pub chat_footer_hover: Option<crate::chat_button_state::ChatFooterButton>,
    /// Index into `AgentProvider::ALL` of the agent driving the chat.
    pub chat_selected_agent: usize,

    /// Primary-pointer pressed button target. Button feedback is exclusive
    /// across chrome families, so one field covers toolbar / topbar /
    /// statusbar / chat buttons without duplicating every hover field.
    pub pressed_button: Option<crate::button_press_state::ButtonPressTarget>,

    /// True after Cmd/Ctrl+A in the component-browser search box. The
    /// next edit replaces the whole search query.
    pub component_browser_select_all: bool,

    // --- Window chrome ---------------------------------------------
    /// Cursor is over the TopBar's window-control (traffic-light)
    /// cluster — reveals the close / minimise / maximise glyphs.
    pub topbar_traffic_hover: bool,
    /// Which TopBar chrome button the cursor is over — drives the
    /// `theme.button_hover` wash on the sidebar / file-menu / figma /
    /// theme / locale / fullscreen / git / agent-chip buttons.
    pub topbar_button_hover: Option<crate::topbar_state::TopBarButton>,
    /// Which floating status-bar control the cursor is over — drives
    /// the `theme.button_hover` wash on the search / zoom-out / zoom-in
    /// controls.
    pub statusbar_hover: Option<crate::statusbar_state::StatusBarButton>,
    /// Window is in fullscreen. macOS hides the native traffic
    /// lights then, so the TopBar drops its left-edge reservation.
    pub window_fullscreen: bool,
    /// Raised when the TopBar fullscreen button is clicked. The host
    /// runner (which owns the window) consumes it next frame to toggle
    /// the actual window fullscreen, then clears it.
    pub pending_fullscreen_toggle: bool,

    // --- Alignment toolbar -----------------------------------------
    /// Align-toolbar button currently hovered.
    pub align_toolbar_hover: Option<crate::align::AlignAction>,

    // --- Property panel: tabs + layout toggles ---------------------
    /// Active PropertyPanel tab — toggled by `Cmd+Shift+C`.
    pub property_tab: PropertyTab,
    /// Active flex-layout mode for the property panel's row.
    pub flex_layout: FlexLayout,
    /// Padding-section edit mode pinned via the gear popover. `None`
    /// re-derives the mode from the node's values each frame (TS
    /// default); `Some(_)` keeps the user's pick. The pin is scoped to
    /// [`Self::padding_edit_mode_anchor`] so it never leaks into the
    /// next selection — the panel ignores it once the anchor differs.
    pub padding_edit_mode: Option<PaddingEditMode>,
    /// Node id (anchor) the [`Self::padding_edit_mode`] pin was set for.
    /// Empty when unset. The property panel honours the pin only while
    /// the current selection anchor still matches, so selecting another
    /// node falls back to deriving the mode from that node's values.
    pub padding_edit_mode_anchor: String,
    /// Whether the padding-mode gear popover is open.
    pub padding_mode_popover_open: bool,
    /// Index (into `PaddingEditMode::ALL`) of the popover row under the
    /// cursor while the gear popover is open — drives the hover wash.
    pub padding_mode_popover_hover: Option<usize>,
    /// Stroke-width edit mode pinned via the stroke-section gear.
    /// Scoped to [`Self::stroke_edit_mode_anchor`] like padding.
    pub stroke_edit_mode: Option<PaddingEditMode>,
    /// Node id (anchor) the stroke edit-mode pin was set for.
    pub stroke_edit_mode_anchor: String,
    /// Whether the stroke-mode gear popover is open.
    pub stroke_mode_popover_open: bool,
    /// Index (into `PaddingEditMode::ALL`) of the stroke popover row
    /// under the cursor while the gear popover is open.
    pub stroke_mode_popover_hover: Option<usize>,
    pub size_fill_width: bool,
    pub size_fill_height: bool,
    pub size_hug_width: bool,
    pub size_hug_height: bool,
    pub size_clip_content: bool,
    /// Fill-type dropdown state in the PropertyPanel.
    pub fill_type_picker: jian_widgets::components::select::SelectState,
    /// Which fill row the open fill-type dropdown targets. The Fill
    /// section stacks one row per fill, each with its own type
    /// dropdown, so `fill_type_picker.open` plus this index identify
    /// the row whose picker is showing. Meaningless when the picker is
    /// closed; defaults to `0`.
    pub fill_type_picker_index: usize,
    /// Fill/stroke colour-variable dropdown currently open in the
    /// PropertyPanel; `None` means closed.
    pub property_color_variable_picker_open: Option<crate::ui_draft::ColorTarget>,
    /// Whether the image-fill editor popover is open.
    pub image_fill_popover_open: bool,
    /// Text font-family picker select state.
    pub font_picker: jian_widgets::components::select::SelectState,
    /// Live type-ahead filter for the font-family picker (TS
    /// FontPicker search input).
    pub font_picker_search: String,
    /// Host-enumerated system font families (sorted, deduped against
    /// the bundled set). Empty until a host enumerates; the picker
    /// then falls back to the TS `FALLBACK_SYSTEM_FONTS` list.
    pub system_font_families: std::sync::Arc<Vec<String>>,
    /// Whether a host already ran font enumeration (so an empty list
    /// is "machine has none" rather than "not loaded yet").
    pub system_fonts_loaded: bool,
    /// Image-node section Search / Generate popover state.
    pub image_panel: crate::image_panel_state::ImagePanelState,
    /// Whether the typography font-weight dropdown is open.
    pub font_weight_picker_open: bool,
    /// Index (into `FontWeightChoice::ALL`) of the weight-dropdown row
    /// under the cursor while it's open — drives the hover wash.
    pub font_weight_picker_hover: Option<usize>,
    /// Active-theme axis whose value picker is open; `None` = closed.
    pub axis_dropdown_open: Option<String>,
    /// Whether the Variables right-rail panel is explicitly open.
    pub variables_panel_open: bool,
    pub variables_preset_menu_open: bool,
    /// Which row inside the open theme-preset dropdown the cursor is over —
    /// drives the per-row hover wash.
    pub variables_preset_menu_hover: Option<crate::variables_panel_state::PresetMenuButton>,
    pub variables_add_menu_open: bool,
    /// Whether the preset dropdown's save-as-name input is showing
    /// (TS `showPresetNameInput`). This legacy preset-name draft still
    /// lives in `UiDraftState::property_input_draft`; variables panel
    /// row/header edits use the text-input states below.
    pub variables_preset_name_focus: bool,
    /// Pending `.optheme` import / export the desktop host must run.
    pub pending_theme_preset_io: Option<ThemePresetIo>,
    /// Theme axis currently shown in the floating variables panel.
    /// Separate from `ui.variables.active_theme`, which stores the
    /// concrete value selected for each axis.
    pub variables_current_axis: Option<String>,
    /// Theme-axis tab whose rename/delete menu is open.
    pub variables_theme_menu_axis: Option<String>,
    /// Variant column whose rename/delete menu is open.
    pub variables_variant_menu_value: Option<String>,
    /// Theme-axis name currently being edited in the VariablesPanel.
    pub variables_theme_rename_axis: Option<String>,
    /// Variant column value currently being edited in the VariablesPanel.
    pub variables_variant_rename_value: Option<String>,
    /// Shared text state for VariablesPanel theme-axis / variant header renames.
    pub variables_header_input: jian_core::text_input::TextInputState,
    /// Which variables-panel target the cursor is over (row / axis chip
    /// / dropdown item) — drives the `theme.button_hover` wash.
    pub variables_panel_hover: Option<crate::variables_panel_state::VariablesPanelButton>,
    /// Editor focus for a non-color variable row (Number / String).
    pub variable_row_focus: Option<VariableRowFocus>,
    /// Text state for focused VariablesPanel row/value cells.
    pub variable_row_input: jian_core::text_input::TextInputState,
    /// Live search filter for the variables panel rows (TS
    /// `variables-panel.tsx` search box). Case-insensitive substring
    /// match on the variable name; transient, never serialized.
    pub variables_search: String,
    /// Whether the variables-panel search box owns the keyboard.
    pub variables_search_focus: bool,
    /// Vertical scroll offset (px) of the variables row list.
    pub variables_scroll: jian_core::scroll::ScrollState,
    /// Variable row whose `⋯` overflow menu (Rename / Delete) is open.
    /// Indexes the UNFILTERED `doc.variables` order.
    pub variables_row_menu: Option<usize>,
    /// User-resized panel size; `None` = the 820x480 default. Mirrors
    /// TS's transient React state (resets each session, not persisted).
    pub variables_panel_size: Option<(f32, f32)>,
    /// Editor focus on an effect-parameter value (Effects section).
    /// Shares `UiDraftState.property_input` with property-panel fields.
    pub effect_param_focus: Option<EffectParamFocus>,

    /// Legacy in-flight IME composition cache. The native host clears
    /// this defensively but no longer paints a separate preedit bubble.
    pub ime_preedit: Option<crate::ime_state::ImePreedit>,

    // --- Layer / page hover + context menu -------------------------
    /// Currently-hovered LayerPanel row, or `None`.
    pub hovered_layer_id: Option<NodeId>,
    /// Page-row hover state for the Pages section, or `None`.
    pub hovered_page_index: Option<usize>,
    /// Open layer-row context menu, or `None`.
    pub layer_context_menu: Option<LayerContextMenuState>,
    /// Node ids whose children are collapsed in the LayerPanel.
    /// Editor-only UI state — the canonical `PenNodeBase` has no
    /// `collapsed` field, so the collapse flag lives here rather than
    /// on the node.
    ///
    /// Layer-collapse is view-only UI state: deliberately excluded from
    /// the undo snapshot and from file persistence. It is transient —
    /// rebuilt on load, never serialized, and toggling it never pushes
    /// a history entry.
    pub collapsed_layers: HashSet<NodeId>,
    /// Last LayerPanel click target + ms; 400 ms re-press → rename.
    pub last_layer_click: Option<(LayerContextTarget, u64)>,
    /// Last canvas left-click target + ms; 400 ms same-node re-press
    /// on a Text node promotes to inline text edit.
    pub last_canvas_click: Option<(NodeId, u64)>,
    /// Last VariablesPanel name-cell click + ms; 400 ms same-row
    /// re-press promotes to variable rename.
    pub last_variable_name_click: Option<(usize, u64)>,
    /// Smart-guide lines to paint during the current node drag —
    /// computed each `apply_cursor_move` by `align_guides`, cleared on
    /// drag release. View-only transient state: never serialized,
    /// never part of the undo snapshot.
    pub active_guides: Vec<crate::align_guides::AlignmentGuide>,

    // --- Auto-update ------------------------------------------------
    /// Latest result of the desktop host's background update probe.
    /// Transient: never serialized, rebuilt each launch.
    pub update_status: UpdateStatus,

    // --- In-app Git -------------------------------------------------
    /// Floating Git panel snapshot — filled by the desktop host from
    /// its `GitSession`. Transient: never serialized.
    pub git_panel: GitPanelState,

    // --- Design-MD panel --------------------------------------------
    /// Whether the floating Design-MD panel is shown.
    pub design_md_panel_open: bool,
    /// Which design-md-panel button the cursor is over (close / import
    /// / export / remove / section header) — drives the hover wash.
    pub design_md_hover: Option<crate::design_md_button_state::DesignMdButton>,
    /// Top-left corner of the Design-MD panel in logical px. `None`
    /// until first opened — the host then centres it on the viewport.
    pub design_md_panel_pos: Option<(f32, f32)>,
    /// Bitmask of expanded Design-MD sections (bit 0 = theme, 1 =
    /// colors, 2 = typography, 3 = components, 4 = layout, 5 = notes).
    /// Defaults to theme + colors + typography expanded.
    pub design_md_expanded: u8,
    /// Vertical scroll offset (px) of the Design-MD panel body.
    pub design_md_scroll: jian_core::scroll::ScrollState,
    /// True while the desktop host is waiting for an AI-generated
    /// design.md brief. Transient: never serialized.
    pub design_md_generating: bool,
    /// A queued Design-MD import / export request — set by a panel
    /// click, drained by the desktop host (which owns the native file
    /// dialog). Transient: never serialized.
    pub design_md_request: Option<DesignMdRequest>,

    // --- Component browser ------------------------------------------
    /// Whether the floating Component-Browser panel is shown.
    pub component_browser_open: bool,
    /// Canvas node under the cursor (Select tool, no drag) — drives
    /// the dashed hover outline (TS `hoveredNodeId`).
    pub canvas_hover_node: Option<NodeId>,
    /// Frame/group the user "entered" by double-clicking it while
    /// selected (TS `enteredFrameId`). While set, canvas-click parent
    /// promotion stops at this container's children instead of
    /// promoting to the page-root level. Escape and selecting outside
    /// the container exit it. Transient: never serialized.
    pub entered_container: Option<NodeId>,
    /// Top-left corner of the Component-Browser panel in logical px;
    /// `None` until first opened — the host then centres it.
    pub component_browser_pos: Option<(f32, f32)>,
    /// Live search filter — names + tags substring-match against this.
    pub component_browser_search: String,
    /// Active category pill (`None` = all categories).
    pub component_browser_category: Option<crate::uikit::ComponentCategory>,
    /// Which component-browser target the cursor is over (close /
    /// category pill / card) — drives the `theme.button_hover` wash.
    pub component_browser_hover: Option<crate::component_browser_state::ComponentBrowserButton>,
    /// Active kit filter (`None` = every loaded kit). Mirrors the TS
    /// `uikit-store.ts` `activeKitId`.
    pub component_browser_kit_id: Option<String>,
    /// Whether the kit-filter dropdown popover is open (the TS panel
    /// uses a native `<select>`; the Rust panel paints a popover).
    pub component_browser_kit_picker_open: bool,
    /// Imported-kit id awaiting delete confirmation — the TS panel's
    /// `confirmDeleteKitId` (Trash press arms it; Delete / Cancel
    /// resolve it). Transient: never serialized.
    pub component_browser_confirm_delete_kit: Option<String>,
    /// A queued kit Import / Export request — set by a header-button
    /// press, drained by the desktop host (which owns the native file
    /// dialogs). Transient: never serialized.
    pub component_browser_kit_request: Option<crate::uikit_io::KitIoRequest>,
    /// Persistence-dirty flag: raised by `import_kit` / `remove_kit`,
    /// drained by the desktop host into `uikits.json` (the TS
    /// `uikit-store.persist()` counterpart).
    pub ui_kits_changed: bool,
    /// A queued component-instantiate request — `(kit_id, comp_id)`,
    /// set by a card click, drained by the desktop host so it can
    /// run the instantiate against the viewport's centre.
    pub component_browser_pending_insert: Option<(String, String)>,
}

/// A Design-MD panel action that needs the desktop host's native
/// file dialog — set by the widget layer, drained by the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMdRequest {
    /// Pick a `.md` file, parse it, and set `design_md`.
    Import,
    /// Ask the selected AI model to generate a fresh design.md from
    /// the current document.
    AutoGenerate,
    /// Write the current `design_md` to a `.md` file.
    Export,
}

impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            layer_panel_width: 240.0,
            property_panel_width: 256.0,
            theme_mode: ThemeMode::Dark,
            locale: Locale::ZhCn,
            locale_picker: jian_widgets::components::select::SelectState::default(),
            file_menu_open: false,
            file_menu: Default::default(),
            pending_file_action: None,
            recent_files: Vec::new(),
            file_name_display: None,
            export_scale: 2.0,
            export_dialog_open: false,
            export_dialog_hover: None,
            export_format: ExportFormat::Png,
            export_scale_picker_open: false,
            export_format_picker_open: false,
            export_picker_hover: None,
            property_action_hover: None,
            property_tab_hover: None,
            property_panel_scroll: Default::default(),
            layer_pages_scroll: Default::default(),
            layer_layers_scroll: Default::default(),
            layer_pages_h_scroll: Default::default(),
            layer_layers_h_scroll: Default::default(),
            figma_import_open: false,
            figma_import_hover: None,
            figma_import_in_progress: false,
            file_drop_active: false,
            preserve_authored_geometry: false,
            preview_mode: false,
            preview_warnings: Vec::new(),
            agent_settings_open: false,
            agent_settings: crate::agent_settings::AgentSettings::default(),
            agent_settings_drag: None,
            settings_input: jian_core::text_input::TextInputState::default(),
            shape_picker: jian_widgets::components::select::SelectState::default(),
            toolbar_hover: None,
            shape_tool: Tool::Rect,
            icon_picker: jian_widgets::components::select::SelectState::default(),
            icon_picker_replace_selection: false,
            icon_picker_panel_pos: None,
            icon_picker_search: String::new(),
            icon_picker_select_all: false,
            icon_picker_remote: crate::icon_picker_state::IconPickerRemoteState::default(),
            icon_picker_load_more_request: None,
            chat_model_picker: jian_widgets::components::select::SelectState::default(),
            chat_model_picker_input: jian_core::text_input::TextInputState::default(),
            chat_design_block_hover: None,
            chat_example_hover: None,
            chat_header_hover: None,
            chat_footer_hover: None,
            chat_selected_agent: 0,
            pressed_button: None,
            component_browser_select_all: false,
            topbar_traffic_hover: false,
            topbar_button_hover: None,
            statusbar_hover: None,
            window_fullscreen: false,
            pending_fullscreen_toggle: false,
            align_toolbar_hover: None,
            property_tab: PropertyTab::Design,
            flex_layout: FlexLayout::Free,
            padding_edit_mode: None,
            padding_edit_mode_anchor: String::new(),
            padding_mode_popover_open: false,
            padding_mode_popover_hover: None,
            stroke_edit_mode: None,
            stroke_edit_mode_anchor: String::new(),
            stroke_mode_popover_open: false,
            stroke_mode_popover_hover: None,
            size_fill_width: false,
            size_fill_height: false,
            size_hug_width: false,
            size_hug_height: false,
            size_clip_content: false,
            fill_type_picker: jian_widgets::components::select::SelectState::default(),
            fill_type_picker_index: 0,
            property_color_variable_picker_open: None,
            image_fill_popover_open: false,
            font_picker: jian_widgets::components::select::SelectState::default(),
            font_picker_search: String::new(),
            system_font_families: std::sync::Arc::new(Vec::new()),
            system_fonts_loaded: false,
            image_panel: crate::image_panel_state::ImagePanelState::default(),
            font_weight_picker_open: false,
            font_weight_picker_hover: None,
            axis_dropdown_open: None,
            variables_panel_open: false,
            variables_preset_menu_open: false,
            variables_preset_menu_hover: None,
            variables_add_menu_open: false,
            variables_preset_name_focus: false,
            pending_theme_preset_io: None,
            variables_current_axis: None,
            variables_theme_menu_axis: None,
            variables_variant_menu_value: None,
            variables_theme_rename_axis: None,
            variables_variant_rename_value: None,
            variables_header_input: jian_core::text_input::TextInputState::default(),
            variables_panel_hover: None,
            variable_row_focus: None,
            variable_row_input: jian_core::text_input::TextInputState::default(),
            variables_search: String::new(),
            variables_search_focus: false,
            variables_scroll: Default::default(),
            variables_row_menu: None,
            variables_panel_size: None,
            effect_param_focus: None,
            ime_preedit: None,
            hovered_layer_id: None,
            hovered_page_index: None,
            layer_context_menu: None,
            collapsed_layers: HashSet::new(),
            last_layer_click: None,
            last_canvas_click: None,
            last_variable_name_click: None,
            active_guides: Vec::new(),
            update_status: UpdateStatus::Idle,
            git_panel: GitPanelState::default(),
            design_md_panel_open: false,
            design_md_hover: None,
            design_md_panel_pos: None,
            design_md_expanded: 0b0000_0111,
            design_md_scroll: Default::default(),
            design_md_generating: false,
            design_md_request: None,
            component_browser_open: false,
            canvas_hover_node: None,
            entered_container: None,
            component_browser_pos: None,
            component_browser_search: String::new(),
            component_browser_category: None,
            component_browser_hover: None,
            component_browser_kit_id: None,
            component_browser_kit_picker_open: false,
            component_browser_confirm_delete_kit: None,
            component_browser_kit_request: None,
            ui_kits_changed: false,
            component_browser_pending_insert: None,
        }
    }
}

impl EditorUiState {
    /// A fresh UI state — sidebar open, dark theme, no menus open.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_button_press_target(&mut self) {
        self.pressed_button = None;
    }

    pub fn touch_recent_file(&mut self, path: String, modified_at: u64) {
        self.recent_files.retain(|recent| recent.path != path);
        self.recent_files
            .insert(0, RecentFile { path, modified_at });
        self.recent_files.truncate(RECENT_FILE_CAP);
    }

    pub fn remove_recent_file(&mut self, path: &str) -> bool {
        let before = self.recent_files.len();
        self.recent_files.retain(|recent| recent.path != path);
        self.recent_files.len() != before
    }

    /// Enter canvas Preview (Play) mode. The host follows up by
    /// building the jian runtime from the (unchanged) document; this
    /// only flips the flag + clears any stale warnings so the host
    /// knows to build a fresh runtime. Idempotent.
    pub fn enter_preview(&mut self) {
        self.preview_mode = true;
        self.preview_warnings.clear();
    }

    /// Exit Preview mode. The host follows up by dropping the runtime.
    /// Clears the warning list. Idempotent.
    pub fn exit_preview(&mut self) {
        self.preview_mode = false;
        self.preview_warnings.clear();
    }

    /// Flip Preview mode on/off. Returns the new state (`true` =
    /// now in Preview).
    pub fn toggle_preview(&mut self) -> bool {
        if self.preview_mode {
            self.exit_preview();
        } else {
            self.enter_preview();
        }
        self.preview_mode
    }

    pub fn button_pressed(&self, target: crate::button_press_state::ButtonPressTarget) -> bool {
        self.pressed_button == Some(target)
    }

    pub fn toggle_fill_type_picker(&mut self) {
        self.toggle_fill_type_picker_for(0);
    }

    /// Toggle the fill-type dropdown for fill `index`. Opening on a
    /// different row than the one currently open re-targets the picker
    /// (stays open, new index); toggling the same row closes it.
    pub fn toggle_fill_type_picker_for(&mut self, index: usize) {
        let opening = !(self.fill_type_picker.open && self.fill_type_picker_index == index);
        self.fill_type_picker.open = opening;
        self.fill_type_picker_index = index;
        self.fill_type_picker.hover = None;
        self.fill_type_picker.pressed = None;
        if opening {
            self.fill_type_picker.scroll.offset = 0.0;
        }
    }

    pub fn close_fill_type_picker(&mut self) -> bool {
        let changed = self.fill_type_picker.open
            || self.fill_type_picker.hover.is_some()
            || self.fill_type_picker.pressed.is_some()
            || self.fill_type_picker.scroll.offset != 0.0;
        self.fill_type_picker.open = false;
        self.fill_type_picker.hover = None;
        self.fill_type_picker.pressed = None;
        self.fill_type_picker.scroll.offset = 0.0;
        changed
    }

    pub fn open_icon_picker(&mut self, replace_selection: bool) {
        self.close_icon_picker();
        self.icon_picker.open = true;
        self.icon_picker_replace_selection = replace_selection;
    }

    pub fn close_icon_picker(&mut self) -> bool {
        let changed = self.icon_picker.open
            || self.icon_picker.hover.is_some()
            || self.icon_picker.pressed.is_some()
            || self.icon_picker.scroll.offset != 0.0
            || self.icon_picker_replace_selection
            || !self.icon_picker_search.is_empty()
            || self.icon_picker_select_all;
        self.icon_picker.open = false;
        self.icon_picker.hover = None;
        self.icon_picker.pressed = None;
        self.icon_picker.scroll.offset = 0.0;
        self.icon_picker_replace_selection = false;
        self.icon_picker_search.clear();
        self.icon_picker_select_all = false;
        changed
    }

    pub fn toggle_font_picker(&mut self) {
        let opening = !self.font_picker.open;
        self.close_font_picker();
        if opening {
            self.font_picker.open = true;
        }
    }

    pub fn close_font_picker(&mut self) -> bool {
        let changed = self.font_picker.open
            || self.font_picker.hover.is_some()
            || self.font_picker.pressed.is_some()
            || self.font_picker.scroll.offset != 0.0
            || !self.font_picker_search.is_empty();
        self.font_picker.open = false;
        self.font_picker.hover = None;
        self.font_picker.pressed = None;
        self.font_picker.scroll.offset = 0.0;
        self.font_picker_search.clear();
        changed
    }

    pub fn toggle_chat_model_picker(&mut self) -> bool {
        let opening = !self.chat_model_picker.open;
        self.close_chat_model_picker();
        if opening {
            self.chat_model_picker.open = true;
        }
        opening
    }

    pub fn close_chat_model_picker(&mut self) -> bool {
        let changed = self.chat_model_picker.open
            || self.chat_model_picker.hover.is_some()
            || self.chat_model_picker.pressed.is_some()
            || self.chat_model_picker.scroll.offset != 0.0
            || !self.chat_model_picker_input.text().is_empty();
        self.chat_model_picker.open = false;
        self.chat_model_picker.hover = None;
        self.chat_model_picker.pressed = None;
        self.chat_model_picker.scroll.offset = 0.0;
        self.chat_model_picker_input.set_text("");
        changed
    }

    /// Whether the preset dropdown's save-as-name input owns the
    /// keyboard. Gated on the menu being open so a stale focus flag
    /// (e.g. the menu closed by a panel-side `close_variable_menus`)
    /// can never eat keystrokes.
    pub fn preset_name_input_active(&self) -> bool {
        self.variables_preset_menu_open && self.variables_preset_name_focus
    }

    pub fn builtin_agent_draft_ready(&self) -> bool {
        use crate::agent_settings::BuiltinAgentField;

        let Some(name) = self.builtin_agent_draft_field_text(BuiltinAgentField::DisplayName) else {
            return false;
        };
        let Some(api_key) = self.builtin_agent_draft_field_text(BuiltinAgentField::ApiKey) else {
            return false;
        };
        let Some(model) = self.builtin_agent_draft_field_text(BuiltinAgentField::Model) else {
            return false;
        };
        !name.trim().is_empty() && !api_key.trim().is_empty() && !model.trim().is_empty()
    }

    pub fn acp_agent_draft_ready(&self) -> bool {
        use crate::agent_settings::{AcpAgentField, AcpConnectionType};

        let Some(draft) = self.agent_settings.acp_agent_draft.as_ref() else {
            return false;
        };
        let Some(name) = self.acp_agent_draft_field_text(AcpAgentField::DisplayName) else {
            return false;
        };
        let endpoint_field = match draft.connection_type {
            AcpConnectionType::Local => AcpAgentField::Command,
            AcpConnectionType::Remote => AcpAgentField::Url,
        };
        let Some(endpoint) = self.acp_agent_draft_field_text(endpoint_field) else {
            return false;
        };
        !name.trim().is_empty() && !endpoint.trim().is_empty()
    }

    pub fn builtin_agent_draft_field_text(
        &self,
        field: crate::agent_settings::BuiltinAgentField,
    ) -> Option<&str> {
        use crate::agent_settings::{BuiltinAgentField, SettingsFocus};

        let draft = self.agent_settings.builtin_agent_draft.as_ref()?;
        if self.agent_settings.focus == Some(SettingsFocus::BuiltinAgentDraft(field)) {
            return Some(self.settings_input.text());
        }
        Some(match field {
            BuiltinAgentField::DisplayName => draft.display_name.as_str(),
            BuiltinAgentField::ApiKey => draft.api_key.as_str(),
            BuiltinAgentField::Model => draft.model.as_str(),
            BuiltinAgentField::BaseUrl => draft.base_url.as_str(),
        })
    }

    pub fn acp_agent_draft_field_text(
        &self,
        field: crate::agent_settings::AcpAgentField,
    ) -> Option<std::borrow::Cow<'_, str>> {
        use crate::agent_settings::{AcpAgentField, SettingsFocus};

        let draft = self.agent_settings.acp_agent_draft.as_ref()?;
        if self.agent_settings.focus == Some(SettingsFocus::AcpAgentDraft(field)) {
            return Some(std::borrow::Cow::Borrowed(self.settings_input.text()));
        }
        Some(match field {
            AcpAgentField::DisplayName => std::borrow::Cow::Borrowed(draft.display_name.as_str()),
            AcpAgentField::Command => std::borrow::Cow::Borrowed(draft.command.as_str()),
            AcpAgentField::Args => std::borrow::Cow::Owned(draft.args_text()),
            AcpAgentField::Env => std::borrow::Cow::Owned(draft.env_text()),
            AcpAgentField::Url => std::borrow::Cow::Borrowed(draft.url.as_deref().unwrap_or("")),
        })
    }

    /// Clear transient UI state that references specific document nodes/pages
    /// or the (now-cleared) selection, so a wholesale document replacement
    /// ([`crate::EditorState::replace_document`]) can't leave hover highlights,
    /// an open layer context menu, collapsed-layer entries, alignment guides,
    /// or in-progress property edits pointing at nodes that no longer exist.
    /// Settings, theme/locale, panel sizes, recent files, the Git panel, and
    /// agent/MCP config are PRESERVED — they are not document-derived.
    pub fn clear_document_derived(&mut self) {
        self.hovered_layer_id = None;
        self.hovered_page_index = None;
        self.layer_context_menu = None;
        self.layer_pages_scroll.offset = 0.0;
        self.layer_layers_scroll.offset = 0.0;
        self.layer_pages_h_scroll.offset = 0.0;
        self.layer_layers_h_scroll.offset = 0.0;
        self.collapsed_layers.clear();
        self.last_layer_click = None;
        self.last_canvas_click = None;
        self.entered_container = None;
        self.active_guides.clear();
        self.padding_edit_mode = None;
        self.padding_edit_mode_anchor = String::new();
        self.padding_mode_popover_open = false;
        self.stroke_edit_mode = None;
        self.stroke_edit_mode_anchor = String::new();
        self.stroke_mode_popover_open = false;
        self.stroke_mode_popover_hover = None;
        self.property_color_variable_picker_open = None;
        self.axis_dropdown_open = None;
        self.variables_theme_rename_axis = None;
        self.variables_variant_rename_value = None;
        self.variables_header_input.set_text("");
        self.variable_row_focus = None;
        self.variable_row_input.set_text("");
        // Document-derived variables-panel transients: the search
        // filter, scroll offset and open row menu all reference the
        // replaced document's variable list. The panel SIZE is a panel
        // metric and is preserved.
        self.variables_search.clear();
        self.variables_search_focus = false;
        self.variables_scroll.offset = 0.0;
        self.variables_row_menu = None;
        self.design_md_scroll.offset = 0.0;
        self.design_md_generating = false;
        self.effect_param_focus = None;
        // Document-derived: set true only by a Figma import to keep that
        // document's authored absolute geometry. A replacement document must
        // not inherit it (it would force the new tree through the
        // preserve-geometry layout path) — matches file-open, which resets it
        // via a fresh `editor_ui`.
        self.preserve_authored_geometry = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_editor_ui_is_quiescent() {
        let c = EditorUiState::new();
        assert!(c.sidebar_open);
        assert_eq!(c.theme_mode, ThemeMode::Dark);
        assert_eq!(c.locale, Locale::ZhCn);
        assert!(!c.file_menu_open);
        assert!(!c.export_dialog_open);
        assert!(!c.agent_settings_open);
        assert_eq!(c.shape_tool, Tool::Rect);
        assert_eq!(c.property_tab, PropertyTab::Design);
        assert_eq!(c.flex_layout, FlexLayout::Free);
        assert!(c.recent_files.is_empty());
        assert!(c.collapsed_layers.is_empty());
    }

    #[test]
    fn editor_pixel_scroll_fields_use_scroll_state() {
        let mut s = EditorUiState::default();

        s.property_panel_scroll.offset = 12.0;
        s.layer_pages_scroll.offset = 24.0;
        s.layer_layers_scroll.offset = 36.0;
        s.layer_pages_h_scroll.offset = 48.0;
        s.layer_layers_h_scroll.offset = 60.0;
        s.variables_scroll.offset = 72.0;
        s.design_md_scroll.offset = 84.0;

        assert_eq!(s.property_panel_scroll.offset, 12.0);
        assert_eq!(s.layer_pages_scroll.offset, 24.0);
        assert_eq!(s.layer_layers_scroll.offset, 36.0);
        assert_eq!(s.layer_pages_h_scroll.offset, 48.0);
        assert_eq!(s.layer_layers_h_scroll.offset, 60.0);
        assert_eq!(s.variables_scroll.offset, 72.0);
        assert_eq!(s.design_md_scroll.offset, 84.0);
    }

    #[test]
    fn button_press_target_clears_chrome_button_families() {
        let mut ui = EditorUiState {
            pressed_button: Some(crate::button_press_state::ButtonPressTarget::FigmaImport(
                crate::FigmaImportButton::DropZone,
            )),
            ..Default::default()
        };

        assert!(
            ui.button_pressed(crate::button_press_state::ButtonPressTarget::FigmaImport(
                crate::FigmaImportButton::DropZone,
            ))
        );

        ui.clear_button_press_target();

        assert_eq!(ui.pressed_button, None);
    }

    #[test]
    fn theme_mode_flips() {
        assert_eq!(ThemeMode::Dark.flipped(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.flipped(), ThemeMode::Dark);
    }

    #[test]
    fn preview_mode_defaults_off() {
        let ui = EditorUiState::new();
        assert!(!ui.preview_mode);
        assert!(ui.preview_warnings.is_empty());
    }

    #[test]
    fn preview_toggle_round_trips_and_clears_warnings() {
        let mut ui = EditorUiState::new();
        // Enter — flips on + clears stale warnings.
        ui.preview_warnings.push("stale".to_string());
        assert!(ui.toggle_preview());
        assert!(ui.preview_mode);
        assert!(ui.preview_warnings.is_empty());

        // A warning recorded during preview is dropped on exit.
        ui.preview_warnings
            .push("LegacyRolePromoted: button → text_input".to_string());
        assert!(!ui.toggle_preview());
        assert!(!ui.preview_mode);
        assert!(ui.preview_warnings.is_empty());
    }

    #[test]
    fn preview_enter_exit_are_idempotent() {
        let mut ui = EditorUiState::new();
        ui.enter_preview();
        ui.enter_preview();
        assert!(ui.preview_mode);
        ui.exit_preview();
        ui.exit_preview();
        assert!(!ui.preview_mode);
    }

    #[test]
    fn export_format_metadata() {
        assert_eq!(ExportFormat::ALL.len(), 5);
        assert_eq!(ExportFormat::Png.extension(), "png");
        assert_eq!(ExportFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn builtin_agent_draft_ready_reads_focused_settings_input() {
        use crate::agent_settings::{BuiltinAgentField, SettingsFocus};

        let mut ui = EditorUiState::new();
        ui.agent_settings.begin_builtin_agent_draft();
        assert!(!ui.builtin_agent_draft_ready());

        ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
        ui.settings_input.set_text("sk-test");

        assert!(ui.builtin_agent_draft_ready());
    }

    #[test]
    fn acp_agent_draft_ready_reads_focused_settings_input() {
        use crate::agent_settings::{AcpAgentField, SettingsFocus};

        let mut ui = EditorUiState::new();
        ui.agent_settings.begin_acp_agent_draft();
        assert!(!ui.acp_agent_draft_ready());

        ui.agent_settings.focus = Some(SettingsFocus::AcpAgentDraft(AcpAgentField::Command));
        ui.settings_input.set_text("op-agent");

        assert!(ui.acp_agent_draft_ready());
    }

    #[test]
    fn dirty_ready_repo_keeps_header_popovers_allowed() {
        // TS parity (the ready view now shows for dirty trees too): a
        // bound, non-merging repo shows the branch-picker / overflow
        // popovers whether the working tree is clean OR dirty. A periodic
        // status refresh must NOT force-close them just because files
        // changed — that was the pre-parity behaviour.
        let mut s = GitPanelState {
            in_repo: true,
            merging: false,
            ..Default::default()
        };
        assert!(s.header_popovers_allowed(), "clean bound repo");
        s.changed_files = vec![GitFileEntry {
            path: "a.op".into(),
            staged: false,
            status: 'M',
        }];
        assert!(
            s.header_popovers_allowed(),
            "dirty bound repo still shows the ready view → popovers stay"
        );
    }

    #[test]
    fn non_ready_states_disallow_header_popovers() {
        // No repo, or a merge in progress → not the ready view → the
        // header popovers can't exist, so a refresh clears them.
        let mut s = GitPanelState {
            in_repo: false,
            ..Default::default()
        };
        assert!(!s.header_popovers_allowed(), "unbound repo");
        s.in_repo = true;
        s.merging = true;
        assert!(!s.header_popovers_allowed(), "merge in progress");
    }

    #[test]
    fn tracked_picker_helpers_reset_select_interaction_state() {
        let mut s = GitPanelState {
            tracked_picker_selected: Some(2),
            ..Default::default()
        };
        s.tracked_picker.hover = Some(1);
        s.tracked_picker.pressed = Some(1);
        s.tracked_picker.scroll.offset = 44.0;

        s.open_tracked_picker();
        assert!(s.tracked_picker.open);
        assert_eq!(s.tracked_picker_selected, None);
        assert_eq!(s.tracked_picker.hover, None);
        assert_eq!(s.tracked_picker.pressed, None);
        assert_eq!(s.tracked_picker.scroll.offset, 0.0);

        s.tracked_picker_selected = Some(0);
        s.tracked_picker.hover = Some(0);
        s.tracked_picker.pressed = Some(0);
        s.tracked_picker.scroll.offset = 44.0;
        assert!(s.close_tracked_picker());
        assert!(!s.tracked_picker.open);
        assert_eq!(s.tracked_picker_selected, None);
        assert_eq!(s.tracked_picker.hover, None);
        assert_eq!(s.tracked_picker.pressed, None);
        assert_eq!(s.tracked_picker.scroll.offset, 0.0);
    }

    #[test]
    fn font_picker_helpers_reset_select_interaction_state_and_search() {
        let mut ui = EditorUiState::new();
        ui.font_picker_search = "inter".to_string();
        ui.font_picker.hover = Some(1);
        ui.font_picker.pressed = Some(1);
        ui.font_picker.scroll.offset = 24.0;

        ui.toggle_font_picker();
        assert!(ui.font_picker.open);
        assert!(ui.font_picker_search.is_empty());
        assert_eq!(ui.font_picker.hover, None);
        assert_eq!(ui.font_picker.pressed, None);
        assert_eq!(ui.font_picker.scroll.offset, 0.0);

        ui.font_picker_search = "roboto".to_string();
        ui.font_picker.hover = Some(0);
        ui.font_picker.pressed = Some(0);
        ui.font_picker.scroll.offset = 24.0;
        assert!(ui.close_font_picker());
        assert!(!ui.font_picker.open);
        assert!(ui.font_picker_search.is_empty());
        assert_eq!(ui.font_picker.hover, None);
        assert_eq!(ui.font_picker.pressed, None);
        assert_eq!(ui.font_picker.scroll.offset, 0.0);
    }

    #[test]
    fn chat_model_picker_helpers_reset_select_interaction_state_and_search() {
        let mut ui = EditorUiState::new();
        ui.chat_model_picker_input.set_text("gpt");
        ui.chat_model_picker.hover = Some(1);
        ui.chat_model_picker.pressed = Some(1);
        ui.chat_model_picker.scroll.offset = 28.0;

        assert!(ui.toggle_chat_model_picker());
        assert!(ui.chat_model_picker.open);
        assert!(ui.chat_model_picker_input.text().is_empty());
        assert_eq!(ui.chat_model_picker.hover, None);
        assert_eq!(ui.chat_model_picker.pressed, None);
        assert_eq!(ui.chat_model_picker.scroll.offset, 0.0);

        ui.chat_model_picker_input.set_text("claude");
        ui.chat_model_picker.hover = Some(0);
        ui.chat_model_picker.pressed = Some(0);
        ui.chat_model_picker.scroll.offset = 28.0;
        assert!(!ui.toggle_chat_model_picker());
        assert!(!ui.chat_model_picker.open);
        assert!(ui.chat_model_picker_input.text().is_empty());
        assert_eq!(ui.chat_model_picker.hover, None);
        assert_eq!(ui.chat_model_picker.pressed, None);
        assert_eq!(ui.chat_model_picker.scroll.offset, 0.0);
    }

    #[test]
    fn icon_picker_helpers_reset_select_interaction_state_and_search() {
        let mut ui = EditorUiState::new();
        ui.icon_picker_replace_selection = true;
        ui.icon_picker_search = "home".to_string();
        ui.icon_picker_select_all = true;
        ui.icon_picker.hover = Some(1);
        ui.icon_picker.pressed = Some(1);

        ui.open_icon_picker(false);
        assert!(ui.icon_picker.open);
        assert!(!ui.icon_picker_replace_selection);
        assert!(ui.icon_picker_search.is_empty());
        assert!(!ui.icon_picker_select_all);
        assert_eq!(ui.icon_picker.hover, None);
        assert_eq!(ui.icon_picker.pressed, None);

        ui.icon_picker_replace_selection = true;
        ui.icon_picker_search = "settings".to_string();
        ui.icon_picker_select_all = true;
        ui.icon_picker.hover = Some(0);
        ui.icon_picker.pressed = Some(0);
        assert!(ui.close_icon_picker());
        assert!(!ui.icon_picker.open);
        assert!(!ui.icon_picker_replace_selection);
        assert!(ui.icon_picker_search.is_empty());
        assert!(!ui.icon_picker_select_all);
        assert_eq!(ui.icon_picker.hover, None);
        assert_eq!(ui.icon_picker.pressed, None);
    }
}
