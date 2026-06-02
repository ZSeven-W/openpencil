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
//! module — `ExportFormat` in `widgets/export_dialog.rs`,
//! `FileMenuChoice` in `widgets/file_menu.rs`, `ShapeChoice` in
//! `widgets/shape_picker.rs`. They are data/state enums, not rendering
//! code, so their type definitions belong in the state layer. The
//! widget *painting / hit-test* code stays in shell-core untouched.
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

/// File-menu choices. State enum ported from shell-core's
/// `widgets/file_menu::FileMenuChoice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMenuChoice {
    NewFile,
    OpenFile,
    Save,
    SaveAs,
    ExportImage,
    OpenRecent(usize),
    ClearRecent,
}

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
    OpenRecent(usize),
    ClearRecent,
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
    /// `commit_message`.
    Commit,
    /// Ready-view "Save milestone": save the current design to the
    /// tracked `.op`, stage it, and commit with the panel's
    /// `commit_message` — the TS `commitMilestone` flow. Unlike
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloneFormState {
    /// Remote URL draft.
    pub url: String,
    /// Local destination-folder draft.
    pub dest: String,
    /// Which field has keyboard focus (`None` = no caret).
    pub focus: Option<CloneField>,
    /// `true` while the `git clone` worker runs — disables the form.
    pub cloning: bool,
    /// Last clone error (validation or a failed `git clone`), shown
    /// under the fields.
    pub error: Option<String>,
    /// Caret-blink anchor for the focused field — same cadence as the
    /// commit input.
    pub caret_anchor_ms: u64,
}

/// Git panel state — a plain-data snapshot the desktop host fills
/// from its `GitSession`. The widget layer reads it to paint the
/// floating Git panel; it carries no git handles, so it stays
/// wasm-clean. Refreshed whenever the panel is opened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    /// Ready-state header: whether the `…` overflow popover is open
    /// (switch-tracked / clear-author / remote-settings › / SSH-keys › /
    /// close-repo). Mirrors the TS header's local `overflowOpen`.
    pub overflow_open: bool,
    /// Which view the overflow popover is showing — the top-level menu
    /// or one of its subviews (remote settings). Resets to `Menu` each
    /// time the popover closes. Mirrors the TS header's `overflowView`.
    pub overflow_view: GitOverflowView,
    /// Ready-state header: whether the branch-picker dropdown (opened
    /// from the `⎇ <branch> ▾` button) is open.
    pub branch_picker_open: bool,
    /// Current branch name of that repository.
    pub branch: Option<String>,
    /// All local branch names, sorted — the panel lists them for
    /// one-click switching.
    pub branches: Vec<String>,
    /// Which branch-picker sub-mode is showing (list / create / merge).
    pub branch_picker_mode: GitBranchPickerMode,
    /// Draft branch name typed into the inline `新建分支` form.
    pub branch_create_draft: String,
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
    pub remote_draft: String,
    /// Whether the remote-URL input holds keyboard focus.
    pub remote_focused: bool,
    /// Draft `username:token` typed into the HTTPS-credential input.
    pub https_draft: String,
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
    /// SSH key names for the SSH-keys subview (host-filled on open).
    pub ssh_keys: Vec<String>,
    /// Commit-message draft typed into the panel's input box.
    pub commit_message: String,
    /// Whether the commit-message input holds keyboard focus.
    pub commit_focused: bool,
    /// Set when a milestone "save" was skipped because the saved design
    /// matched the last commit — the ready view shows a "未检测到变更" hint
    /// under the commit box. Cleared when the user re-engages the input.
    pub commit_no_changes: bool,
    /// Whether the commit-signature form (`提交署名`) is showing in place of
    /// the commit box — raised when a commit is attempted with no committer
    /// identity (TS `authorPromptVisible`). The pending message stays in
    /// `commit_message` and the commit re-fires after a successful save.
    pub author_prompt: bool,
    /// Name / email drafts typed into the commit-signature form.
    pub author_name_draft: String,
    pub author_email_draft: String,
    /// Which signature-form field holds keyboard focus.
    pub author_name_focused: bool,
    pub author_email_focused: bool,
    /// Caret-blink anchor (ms) for the commit input — reset on focus +
    /// each keystroke so the caret stays solid while typing, then
    /// blinks (same cadence as the chat / property inputs).
    pub commit_caret_anchor_ms: u64,
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
}

/// File-menu "Recent files" entry — host persists via settings IO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentFile {
    pub path: String,
    /// Unix seconds when last touched.
    pub modified_at: u64,
}

/// Toolbar shape-slot dropdown choice. State enum ported from
/// shell-core's `widgets/shape_picker::ShapeChoice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeChoice {
    /// Pick a shape tool (Rect / Ellipse / Polygon / Line / Pen).
    Tool(Tool),
    /// Open the icon picker (host concern; this only reports intent).
    OpenIconPicker,
    /// Open a file dialog to import an image / SVG.
    ImportImageOrSvg,
}

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
    pub hovered_row: Option<u8>,
}

/// Inline-rename state for a page row (double-click → rename).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRenameState {
    pub page_index: usize,
    pub draft: String,
}

/// Editor focus for a non-color variable row in the VariablesPanel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRowFocus {
    Number(usize),
    String(usize),
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
    /// TopBar Globe dropdown open.
    pub locale_picker_open: bool,
    /// Locale row currently hovered while the locale picker is open.
    pub locale_picker_hover: Option<Locale>,

    // --- File menu --------------------------------------------------
    /// File-menu dropdown open (anchored under folder + chevron).
    pub file_menu_open: bool,
    /// File-menu row currently hovered — drives the per-row tint.
    pub file_menu_hover: Option<FileMenuChoice>,
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
    /// Vertical scroll offset of the right-rail PropertyPanel, in px
    /// (≥ 0). A wheel / trackpad pan over the inspector advances it;
    /// paint + hit-test shift the section content up by this amount
    /// so a tall inspector (many effects, etc.) stays reachable.
    pub property_panel_scroll: f32,
    /// Vertical scroll offset (px, ≥ 0) of the LayerPanel's 页面
    /// (Pages) section — that section has a bounded height, so a
    /// long page list scrolls within it.
    pub layer_pages_scroll: f32,
    /// Vertical scroll offset (px, ≥ 0) of the LayerPanel's 图层
    /// (Layers) section row viewport.
    pub layer_layers_scroll: f32,
    /// "Import from Figma" modal.
    pub figma_import_open: bool,
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
    /// Floating `Cmd+,` agent-settings modal open.
    pub agent_settings_open: bool,
    pub agent_settings: crate::agent_settings::AgentSettings,
    pub agent_settings_drag: Option<crate::agent_settings::AgentSettingsDrag>,
    /// Draft for the focused settings-modal input (e.g. MCP port).
    pub settings_input_draft: String,
    /// Byte caret for the focused settings-modal input.
    pub settings_input_caret: Option<usize>,
    /// Focus identity that owns [`Self::settings_input_caret`].
    pub settings_input_caret_focus: Option<crate::agent_settings::SettingsFocus>,
    /// Last focus / edit timestamp for focused settings-modal inputs.
    pub settings_input_caret_anchor_ms: u64,

    // --- Toolbar shape slot ----------------------------------------
    /// Whether the Toolbar shape-tool dropdown is open.
    pub shape_picker_open: bool,
    /// Shape-picker row currently hovered.
    pub shape_picker_hover: Option<ShapeChoice>,
    /// Toolbar button currently hovered — drives the per-button
    /// `theme.button_hover` wash on the vertical tool column.
    pub toolbar_hover: Option<crate::toolbar_state::ToolbarHover>,
    /// Last-selected shape tool — drives the toolbar shape slot's
    /// icon. Always one of Rect / Ellipse / Polygon / Line / Pen.
    pub shape_tool: Tool,
    /// Whether the Toolbar's Icon action picker is open.
    pub icon_picker_open: bool,
    /// True when the icon picker should replace the selected icon
    /// instead of inserting a new icon at the canvas centre.
    pub icon_picker_replace_selection: bool,
    /// Top-left corner of the floating Icon picker in logical px.
    /// `None` until first dragged/opened, then reused across opens.
    pub icon_picker_panel_pos: Option<(f32, f32)>,
    /// Live text filter for the native Lucide icon picker.
    pub icon_picker_search: String,
    /// Remote Iconify search results appended by the desktop host.
    pub icon_picker_remote: crate::icon_picker_state::IconPickerRemoteState,
    /// Queued "load more" request drained asynchronously by desktop.
    pub icon_picker_load_more_request: Option<crate::icon_picker_state::IconifyLoadMoreRequest>,

    // --- AI chat model picker --------------------------------------
    /// AI chat model-picker dropdown open.
    pub chat_model_picker_open: bool,
    /// Vertical scroll offset of the model-picker dropdown, in px.
    /// Non-zero only when the connected catalog is taller than the
    /// picker's capped height; the host clamps it on wheel input.
    pub chat_model_picker_scroll: f32,
    /// Live text filter for the chat model picker. While the picker
    /// is open it owns typed characters, matching the TS search box.
    pub chat_model_picker_search: String,
    /// Byte caret for the chat model-picker search box.
    pub chat_model_picker_caret: Option<usize>,
    /// Last focus / edit timestamp for the chat model-picker search
    /// caret blink cycle.
    pub chat_model_picker_caret_anchor_ms: u64,
    /// Index into `chat.available_models` of the model row the cursor
    /// is over, or `None`. Drives the picker's hover-row tint.
    pub chat_model_picker_hover: Option<usize>,
    /// Hovered chat design JSON card `(message_index, block_index)`;
    /// drives the TS-style hover reveal of the card's copy affordance.
    pub chat_design_block_hover: Option<(usize, usize)>,
    /// Index into `AgentProvider::ALL` of the agent driving the chat.
    pub chat_selected_agent: usize,

    // --- Window chrome ---------------------------------------------
    /// Cursor is over the TopBar's window-control (traffic-light)
    /// cluster — reveals the close / minimise / maximise glyphs.
    pub topbar_traffic_hover: bool,
    /// Window is in fullscreen. macOS hides the native traffic
    /// lights then, so the TopBar drops its left-edge reservation.
    pub window_fullscreen: bool,

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
    pub size_fill_width: bool,
    pub size_fill_height: bool,
    pub size_hug_width: bool,
    pub size_hug_height: bool,
    pub size_clip_content: bool,
    /// Whether the fill-type dropdown is open.
    pub fill_type_picker_open: bool,
    /// Whether the image-fill editor popover is open.
    pub image_fill_popover_open: bool,
    /// Whether the text font-family picker is open.
    pub font_family_picker_open: bool,
    /// Whether the typography font-weight dropdown is open.
    pub font_weight_picker_open: bool,
    /// Index (into `FontWeightChoice::ALL`) of the weight-dropdown row
    /// under the cursor while it's open — drives the hover wash.
    pub font_weight_picker_hover: Option<usize>,
    /// Active-theme axis whose value picker is open; `None` = closed.
    pub axis_dropdown_open: Option<String>,
    /// Editor focus for a non-color variable row (Number / String).
    pub variable_row_focus: Option<VariableRowFocus>,
    /// Editor focus on an effect-parameter value (Effects section).
    /// Shares `UiDraftState.property_input_draft` + caret like the
    /// variable-row focus does.
    pub effect_param_focus: Option<EffectParamFocus>,

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
    /// Caret-blink anchor (ms) for an inline layer / page rename.
    pub rename_caret_anchor_ms: u64,
    /// Last LayerPanel click target + ms; 400 ms re-press → rename.
    pub last_layer_click: Option<(LayerContextTarget, u64)>,
    /// Last canvas left-click target + ms; 400 ms same-node re-press
    /// on a Text node promotes to inline text edit.
    pub last_canvas_click: Option<(NodeId, u64)>,
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
    /// Top-left corner of the Design-MD panel in logical px. `None`
    /// until first opened — the host then centres it on the viewport.
    pub design_md_panel_pos: Option<(f32, f32)>,
    /// Bitmask of expanded Design-MD sections (bit 0 = theme, 1 =
    /// colors, 2 = typography, 3 = components, 4 = layout, 5 = notes).
    /// Defaults to theme + colors + typography expanded.
    pub design_md_expanded: u8,
    /// A queued Design-MD import / export request — set by a panel
    /// click, drained by the desktop host (which owns the native file
    /// dialog). Transient: never serialized.
    pub design_md_request: Option<DesignMdRequest>,

    // --- Component browser ------------------------------------------
    /// Whether the floating Component-Browser panel is shown.
    pub component_browser_open: bool,
    /// Top-left corner of the Component-Browser panel in logical px;
    /// `None` until first opened — the host then centres it.
    pub component_browser_pos: Option<(f32, f32)>,
    /// Live search filter — names + tags substring-match against this.
    pub component_browser_search: String,
    /// Active category pill (`None` = all categories).
    pub component_browser_category: Option<crate::uikit::ComponentCategory>,
    /// Active kit filter (`None` = every loaded kit). Kept for the
    /// future imported-kits surface; v1 ships one built-in kit.
    pub component_browser_kit_id: Option<String>,
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
            locale_picker_open: false,
            locale_picker_hover: None,
            file_menu_open: false,
            file_menu_hover: None,
            pending_file_action: None,
            recent_files: Vec::new(),
            file_name_display: None,
            export_scale: 2.0,
            export_dialog_open: false,
            export_format: ExportFormat::Png,
            export_scale_picker_open: false,
            export_format_picker_open: false,
            export_picker_hover: None,
            property_panel_scroll: 0.0,
            layer_pages_scroll: 0.0,
            layer_layers_scroll: 0.0,
            figma_import_open: false,
            figma_import_in_progress: false,
            file_drop_active: false,
            preserve_authored_geometry: false,
            agent_settings_open: false,
            agent_settings: crate::agent_settings::AgentSettings::default(),
            agent_settings_drag: None,
            settings_input_draft: String::new(),
            settings_input_caret: None,
            settings_input_caret_focus: None,
            settings_input_caret_anchor_ms: 0,
            shape_picker_open: false,
            shape_picker_hover: None,
            toolbar_hover: None,
            shape_tool: Tool::Rect,
            icon_picker_open: false,
            icon_picker_replace_selection: false,
            icon_picker_panel_pos: None,
            icon_picker_search: String::new(),
            icon_picker_remote: crate::icon_picker_state::IconPickerRemoteState::default(),
            icon_picker_load_more_request: None,
            chat_model_picker_open: false,
            chat_model_picker_scroll: 0.0,
            chat_model_picker_search: String::new(),
            chat_model_picker_caret: None,
            chat_model_picker_caret_anchor_ms: 0,
            chat_model_picker_hover: None,
            chat_design_block_hover: None,
            chat_selected_agent: 0,
            topbar_traffic_hover: false,
            window_fullscreen: false,
            align_toolbar_hover: None,
            property_tab: PropertyTab::Design,
            flex_layout: FlexLayout::Free,
            padding_edit_mode: None,
            padding_edit_mode_anchor: String::new(),
            padding_mode_popover_open: false,
            padding_mode_popover_hover: None,
            size_fill_width: false,
            size_fill_height: false,
            size_hug_width: false,
            size_hug_height: false,
            size_clip_content: false,
            fill_type_picker_open: false,
            image_fill_popover_open: false,
            font_family_picker_open: false,
            font_weight_picker_open: false,
            font_weight_picker_hover: None,
            axis_dropdown_open: None,
            variable_row_focus: None,
            effect_param_focus: None,
            hovered_layer_id: None,
            hovered_page_index: None,
            layer_context_menu: None,
            collapsed_layers: HashSet::new(),
            rename_caret_anchor_ms: 0,
            last_layer_click: None,
            last_canvas_click: None,
            active_guides: Vec::new(),
            update_status: UpdateStatus::Idle,
            git_panel: GitPanelState::default(),
            design_md_panel_open: false,
            design_md_panel_pos: None,
            design_md_expanded: 0b0000_0111,
            design_md_request: None,
            component_browser_open: false,
            component_browser_pos: None,
            component_browser_search: String::new(),
            component_browser_category: None,
            component_browser_kit_id: None,
            component_browser_pending_insert: None,
        }
    }
}

impl EditorUiState {
    /// A fresh UI state — sidebar open, dark theme, no menus open.
    pub fn new() -> Self {
        Self::default()
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
    fn theme_mode_flips() {
        assert_eq!(ThemeMode::Dark.flipped(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.flipped(), ThemeMode::Dark);
    }

    #[test]
    fn export_format_metadata() {
        assert_eq!(ExportFormat::ALL.len(), 5);
        assert_eq!(ExportFormat::Png.extension(), "png");
        assert_eq!(ExportFormat::Jpeg.extension(), "jpg");
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
}
