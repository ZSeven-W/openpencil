//! `EditorCommand::Batch` applier — atomic multi-command apply.
//!
//! The MCP `batch_design` multi-op DSL program executor validates each
//! program line against a document snapshot and emits the surviving
//! commands as ONE `EditorCommand::Batch`, so the host applier
//! discipline (one command per tool response, pre-validate-then-mutate)
//! is preserved for multi-op programs. Apply is all-or-nothing: a
//! sub-command failing mid-batch (only possible when the live document
//! diverged from the tool's snapshot — e.g. a concurrent edit on the
//! live canvas) rolls the editor state back to the pre-batch snapshot
//! and reports `false`, mirroring `InsertAuthoredSubtree`'s
//! "never silently-wrong" collision contract.
//!
//! ## Sub-command restriction
//!
//! The rollback snapshot covers exactly the history-snapshot fields —
//! document, selection, active page index, components
//! ([`crate::history::EditorSnapshot`]). A command whose primary state
//! lives OUTSIDE that snapshot (active tool, viewport, clipboard,
//! the undo/redo stacks themselves, the transient active-theme pin)
//! could not be rolled back, so [`batchable`] rejects it: the whole
//! batch fails BEFORE any sub-command executes, with zero state
//! change. The only production `Batch` producer today — the MCP
//! `batch_program` executor — emits document commands exclusively
//! (insert / update / replace / delete / move variants), so the
//! restriction loses nothing and keeps the atomicity contract honest.
//!
//! Allowed commands may still maintain doc-derived transient caches
//! (the variable ref caches, the `preserve_authored_geometry` latch,
//! the active-theme prune in `set_themes_bulk`). Those are consistency
//! maintenance over the document — rebuilt or conservative when stale
//! — and plain `undo()` of the same commands leaves them in the same
//! place, so a rolled-back batch restores exactly as much state as an
//! undo of its sub-commands would.

use crate::command::EditorCommand;
use crate::command_apply::command_marks_document_dirty;
use crate::{EditorState, IdAllocError, IdAllocator};

/// `true` when every piece of editor state `cmd` primarily mutates is
/// covered by the pre-batch rollback snapshot (document, selection,
/// active page index, components), so the all-or-nothing contract can
/// genuinely restore it.
///
/// Exhaustive on purpose: a new `EditorCommand` variant fails to
/// compile here until someone classifies it, so the Batch atomicity
/// contract stays honest as the command set grows.
fn batchable(cmd: &EditorCommand) -> bool {
    use EditorCommand as C;
    match cmd {
        // Primary state outside the rollback snapshot: the active
        // tool, the viewport, the clipboard buffer (paste also READS
        // it, making the outcome depend on un-snapshotted state), the
        // undo/redo stacks themselves, and the transient active-theme
        // pin. Nested batches are rejected so rollback stays
        // single-level.
        C::SetActiveTool { .. }
        | C::SetViewport { .. }
        | C::CopySelected
        | C::CutSelected
        | C::PasteClipboard { .. }
        | C::Undo
        | C::Redo
        | C::SetActiveAxisValue { .. }
        | C::CycleActiveAxisValue { .. }
        | C::Batch { .. } => false,
        // Document / selection / page / component mutators — fully
        // covered by the pre-batch snapshot.
        C::SetVariableColor { .. }
        | C::InsertNode { .. }
        | C::UpdateNode { .. }
        | C::PatchNodeData { .. }
        | C::DeleteNode { .. }
        | C::MoveNode { .. }
        | C::CopyNode { .. }
        | C::ReplaceNode { .. }
        | C::ReplaceSubtree { .. }
        | C::BatchInsert { .. }
        | C::InsertSubtree { .. }
        | C::InsertAuthoredSubtree { .. }
        | C::RefineDesign { .. }
        | C::SetVariableScalar { .. }
        | C::CreateVariable { .. }
        | C::DeleteVariable { .. }
        | C::RenameVariable { .. }
        | C::SetVariables { .. }
        | C::UpsertVariables { .. }
        | C::SetThemes { .. }
        | C::MergeThemePreset { .. }
        | C::SetDesignMd { .. }
        | C::UpsertComponent { .. }
        | C::UpsertScreen { .. }
        | C::InstantiateComponent { .. }
        | C::CreateComponent { .. }
        | C::DeleteComponent { .. }
        | C::RenameComponent { .. }
        | C::InstantiateKitComponent { .. }
        | C::SetActivePage { .. }
        | C::AddPage { .. }
        | C::RenamePage { .. }
        | C::DeletePage { .. }
        | C::DuplicatePage { .. }
        | C::ReorderPage { .. }
        | C::ClearSelection
        | C::SetSelection { .. }
        | C::SetSelectionSet { .. }
        | C::ToggleNodeSelection { .. }
        | C::SetNodeFlag { .. }
        | C::SetNodeFlip { .. }
        | C::SetEllipseArc { .. }
        | C::AddNodeEffect { .. }
        | C::RemoveNodeEffect { .. }
        | C::SetEffectParam { .. }
        | C::SetEffectColor { .. }
        | C::DuplicateSelected { .. }
        | C::DeleteSelected
        | C::NudgeSelected { .. }
        | C::GroupSelected
        | C::UngroupSelected
        | C::ReorderSelected { .. }
        | C::SetNodeRotation { .. }
        | C::SetNodeText { .. }
        | C::SetNodeCornerRadius { .. }
        | C::SetNodeFontSize { .. }
        | C::SetNodeFontWeight { .. }
        | C::SetNodeStrokeHex { .. }
        | C::SetNodeStrokeWidth { .. }
        | C::SetNodeStrokeSideWidth { .. }
        | C::AlignSelected { .. }
        | C::SetNodeFillHex { .. }
        | C::SetNodeName { .. }
        | C::ImportSvg { .. }
        | C::SetNodeLayoutProp { .. }
        | C::ReplaceFontFamily { .. }
        | C::ReplaceAllMatchingProperties { .. }
        | C::MergeAppState { .. }
        | C::PromoteLegacyWidgets => true,
    }
}

impl EditorState {
    /// Apply `commands` in order; roll back everything on the first
    /// failure. On success the whole batch lands as a SINGLE undo step
    /// (intermediate history entries pushed by sub-commands are
    /// collapsed into one pre-batch snapshot).
    pub(crate) fn cmd_batch_with_allocator(
        &mut self,
        commands: Vec<EditorCommand>,
        allocator: &mut dyn IdAllocator,
    ) -> Result<bool, IdAllocError> {
        if commands.is_empty() {
            return Ok(false);
        }
        // Pre-validate the whole program BEFORE executing anything: a
        // sub-command outside the snapshot-covered set (see module
        // docs) rejects the batch with ZERO state change — nothing
        // ran, so there is nothing to roll back.
        if !commands.iter().all(batchable) {
            return Ok(false);
        }
        let marks_document_dirty = commands.iter().any(command_marks_document_dirty);
        let pre = self.snapshot_for_history();
        let past_len = self.history.past.len();
        // Sub-command history pushes clear the redo stack; park it so a
        // rolled-back batch restores redo intact. O(1) move, no clone.
        let saved_future = std::mem::take(&mut self.history.future);
        for cmd in commands {
            let outcome = self.apply_with_allocator(cmd, allocator);
            if !matches!(&outcome, Ok(true)) {
                // Restore the pre-batch history bookkeeping first (this
                // is batch-specific — a plain restore does not touch the
                // undo/redo stacks): drop any sub-command undo entries
                // and hand the parked redo stack back intact.
                self.history.past.truncate(past_len);
                self.history.future = saved_future;
                // Then route the document / selection / page / component
                // / app-state-owner / revision restore through the same
                // materializing path undo/redo use. `restore` rebuilds
                // the owned doc from the shared snapshot and re-syncs the
                // dirty flag. Ownership rolls back with doc.state — a
                // stale entry would mark a key generation-owned that the
                // restored document no longer carries, silently skipping
                // later merges of that key.
                self.restore(pre);
                return outcome;
            }
        }
        // Collapse any per-sub-command history entries into one batch
        // entry, so a GUI undo reverts the whole program at once. The
        // push also clears redo — standard semantics for a new edit.
        self.history.past.truncate(past_len);
        if marks_document_dirty {
            self.revision = pre.revision;
            self.sync_dirty_flag();
            self.history_push_past(pre);
        }
        Ok(true)
    }
}
