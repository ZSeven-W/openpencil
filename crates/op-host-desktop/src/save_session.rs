//! Background `.op` save serialization for the desktop host.
//!
//! The UI thread captures only the canonical document, active-page index, and
//! revision identity. JSON encoding and the atomic sibling-temp write run on a
//! worker. Requests are serialized and the latest extra request is coalesced,
//! so an older snapshot can never rename over a newer one.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::Instant;

use op_editor_core::EditorState;
use winit::event_loop::EventLoopProxy;

use crate::DesktopEvent;

#[path = "save_session/error.rs"]
mod error;
pub(crate) use error::SaveError;

#[must_use = "the enqueue outcome determines whether a Save As acquired fork authority"]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnqueueOutcome {
    Started,
    Queued,
    AlreadyPending,
}

/// Immutable save payload captured on the UI thread.
enum SavePayload {
    Canonical(Box<op_host_services::doc_io::CanonicalSaveSnapshot>),
    CleanBoundOp {
        source_path: PathBuf,
        editor_meta: op_pen_loader::EditorMeta,
    },
}

struct SaveSnapshot {
    payload: SavePayload,
    document_epoch: u64,
    generation: u64,
    revision: u64,
}

impl SaveSnapshot {
    fn capture(state: &EditorState, document_epoch: u64, previous: Option<&SaveSnapshot>) -> Self {
        Self {
            payload: SavePayload::Canonical(Box::new(
                op_host_services::doc_io::CanonicalSaveSnapshot::capture_reusing(
                    state,
                    previous.and_then(SaveSnapshot::canonical),
                ),
            )),
            document_epoch,
            generation: state.document_generation(),
            revision: state.document_revision(),
        }
    }

    fn capture_clean_bound_op(
        state: &EditorState,
        document_epoch: u64,
        source_path: PathBuf,
    ) -> Self {
        Self {
            payload: SavePayload::CleanBoundOp {
                source_path,
                editor_meta: op_pen_loader::EditorMeta::from_state(state),
            },
            document_epoch,
            generation: state.document_generation(),
            revision: state.document_revision(),
        }
    }

    fn canonical(&self) -> Option<&op_host_services::doc_io::CanonicalSaveSnapshot> {
        match &self.payload {
            SavePayload::Canonical(snapshot) => Some(snapshot.as_ref()),
            SavePayload::CleanBoundOp { .. } => None,
        }
    }
}

struct SaveRequest {
    snapshot: Arc<SaveSnapshot>,
    path: PathBuf,
    set_current_path: bool,
    wake: Option<EventLoopProxy<DesktopEvent>>,
}

struct RunningSave {
    snapshot: Arc<SaveSnapshot>,
    path: PathBuf,
    set_current_path: bool,
    document_epoch: u64,
    generation: u64,
    revision: u64,
    rx: Receiver<Result<(), SaveError>>,
}

/// Result delivered back to the UI thread. A successful disk write is only a
/// saved-state acknowledgement when `(host epoch, generation, revision)`
/// still belongs to the live document.
pub(crate) struct SaveCompletion {
    pub(crate) path: PathBuf,
    pub(crate) set_current_path: bool,
    pub(crate) document_epoch: u64,
    pub(crate) generation: u64,
    pub(crate) revision: u64,
    pub(crate) result: Result<(), SaveError>,
}

pub(crate) struct SaveSession {
    running: Option<RunningSave>,
    /// One latest-wins queued request. The running job always commits first,
    /// then this request starts, preventing rename-order inversions.
    queued: Option<SaveRequest>,
}

impl SaveSession {
    pub(crate) fn new() -> Self {
        Self {
            running: None,
            queued: None,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.running.is_some() || self.queued.is_some()
    }

    /// Most recently requested target for the live document, including a
    /// Save-As that has not yet completed. Cmd+S pressed behind that Save-As
    /// should coalesce onto the same new path. A stale Save-As left behind by
    /// Open/New must never capture the replacement document's Cmd+S.
    pub(crate) fn latest_target(&self, document_epoch: u64) -> Option<&Path> {
        self.queued
            .as_ref()
            .filter(|request| request.snapshot.document_epoch == document_epoch)
            .map(|request| request.path.as_path())
            .or_else(|| {
                self.running
                    .as_ref()
                    .filter(|job| job.document_epoch == document_epoch)
                    .map(|job| job.path.as_path())
            })
    }

    pub(crate) fn enqueue(
        &mut self,
        state: &EditorState,
        document_epoch: u64,
        path: PathBuf,
        set_current_path: bool,
        wake: Option<EventLoopProxy<DesktopEvent>>,
    ) -> EnqueueOutcome {
        self.enqueue_impl(state, document_epoch, path, set_current_path, wake, None)
    }

    /// Queue a clean, bound `.op` Save As without cloning the live document.
    /// If any eligibility condition changed, fall back to the canonical
    /// snapshot path instead of risking a stale source-file copy.
    pub(crate) fn enqueue_clean_bound_op_save_as(
        &mut self,
        state: &EditorState,
        document_epoch: u64,
        source_path: PathBuf,
        path: PathBuf,
        set_current_path: bool,
        wake: Option<EventLoopProxy<DesktopEvent>>,
    ) -> EnqueueOutcome {
        let clean_source =
            can_copy_clean_bound_op(state, &source_path, &path).then_some(source_path);
        self.enqueue_impl(
            state,
            document_epoch,
            path,
            set_current_path,
            wake,
            clean_source,
        )
    }

    fn enqueue_impl(
        &mut self,
        state: &EditorState,
        document_epoch: u64,
        path: PathBuf,
        set_current_path: bool,
        wake: Option<EventLoopProxy<DesktopEvent>>,
        clean_source: Option<PathBuf>,
    ) -> EnqueueOutcome {
        let generation = state.document_generation();
        let revision = state.document_revision();
        if self.matches_pending(
            &path,
            set_current_path,
            document_epoch,
            generation,
            revision,
        ) {
            return EnqueueOutcome::AlreadyPending;
        }

        let started = Instant::now();
        let previous = self.capture_anchor(document_epoch, generation);
        let snapshot = match clean_source {
            Some(source_path) => {
                SaveSnapshot::capture_clean_bound_op(state, document_epoch, source_path)
            }
            None => SaveSnapshot::capture(state, document_epoch, previous),
        };
        let request = SaveRequest {
            snapshot: Arc::new(snapshot),
            path,
            set_current_path,
            wake,
        };
        eprintln!(
            "[save] captured revision {generation}:{revision} in {:.1} ms",
            started.elapsed().as_secs_f64() * 1_000.0
        );
        if self.running.is_none() {
            self.start(request);
            EnqueueOutcome::Started
        } else {
            self.queued = Some(request);
            EnqueueOutcome::Queued
        }
    }

    pub(crate) fn poll(&mut self) -> Option<SaveCompletion> {
        let result = match self.running.as_ref()?.rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(SaveError::WorkerStopped),
        };
        self.finish_running(result)
    }

    /// Block only for close/reload confirmation. Ordinary Save never calls
    /// this path; it exists so exiting cannot abandon a Save-As worker or race
    /// a second synchronous write against its sibling temp file.
    pub(crate) fn wait_next(&mut self) -> Option<SaveCompletion> {
        let result = self
            .running
            .as_ref()?
            .rx
            .recv()
            .unwrap_or(Err(SaveError::WorkerStopped));
        self.finish_running(result)
    }

    fn matches_pending(
        &self,
        path: &Path,
        set_current_path: bool,
        document_epoch: u64,
        generation: u64,
        revision: u64,
    ) -> bool {
        let same = |job_path: &Path,
                    job_sets_current_path: bool,
                    job_epoch: u64,
                    job_generation: u64,
                    job_revision: u64| {
            job_path == path
                && job_sets_current_path == set_current_path
                && job_epoch == document_epoch
                && job_generation == generation
                && job_revision == revision
        };
        self.running.as_ref().is_some_and(|job| {
            same(
                &job.path,
                job.set_current_path,
                job.document_epoch,
                job.generation,
                job.revision,
            )
        }) || self.queued.as_ref().is_some_and(|request| {
            same(
                &request.path,
                request.set_current_path,
                request.snapshot.document_epoch,
                request.snapshot.generation,
                request.snapshot.revision,
            )
        })
    }

    /// Pick only an in-flight snapshot that belongs to the live document.
    /// The queued snapshot is newer than the running one and therefore the
    /// closest structural-sharing anchor when both exist.
    fn capture_anchor(&self, document_epoch: u64, generation: u64) -> Option<&SaveSnapshot> {
        let is_live = |snapshot: &&SaveSnapshot| {
            snapshot.document_epoch == document_epoch && snapshot.generation == generation
        };
        self.queued
            .as_ref()
            .map(|request| request.snapshot.as_ref())
            .filter(is_live)
            .or_else(|| {
                self.running
                    .as_ref()
                    .map(|job| job.snapshot.as_ref())
                    .filter(is_live)
            })
    }

    fn start(&mut self, request: SaveRequest) {
        let SaveRequest {
            snapshot,
            path,
            set_current_path,
            wake,
        } = request;
        let document_epoch = snapshot.document_epoch;
        let generation = snapshot.generation;
        let revision = snapshot.revision;
        let worker_path = path.clone();
        let worker_snapshot = Arc::clone(&snapshot);
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("op-document-save".to_string())
            .spawn(move || {
                let started = Instant::now();
                // Both payloads stream directly: a dirty document serializes
                // its shared snapshot, while a clean bound OP copies source
                // bytes and rewrites only editorMeta.
                let result = match &worker_snapshot.payload {
                    SavePayload::Canonical(snapshot) => {
                        op_host_services::doc_io::save_snapshot_to_path(
                            snapshot.as_ref(),
                            &worker_path,
                        )
                    }
                    SavePayload::CleanBoundOp {
                        source_path,
                        editor_meta,
                    } => op_host_services::doc_io::copy_clean_document_with_editor_meta_to_path(
                        source_path,
                        &worker_path,
                        editor_meta.clone(),
                    ),
                }
                // `doc_io` belongs to `op-host-services`, a crate this pass
                // does not own; carry its message so the dialog text and the
                // stderr line are unchanged.
                .map_err(|error| SaveError::Write(error.to_string()));
                eprintln!(
                    "[save] {} revision {generation}:{revision} in {:.1} ms",
                    if result.is_ok() { "wrote" } else { "failed" },
                    started.elapsed().as_secs_f64() * 1_000.0
                );
                let _ = tx.send(result);
                if let Some(proxy) = wake {
                    let _ = proxy.send_event(DesktopEvent::SaveReady);
                }
            });
        if let Err(err) = spawned {
            // The worker never started, so `tx` is gone and the rx below is
            // disconnected — `poll`/`wait_next` surface that as a failed
            // completion instead of the UI thread crashing mid-save.
            eprintln!("[save] failed to spawn save worker: {err}");
        }
        self.running = Some(RunningSave {
            snapshot,
            path,
            set_current_path,
            document_epoch,
            generation,
            revision,
            rx,
        });
    }

    fn finish_running(&mut self, result: Result<(), SaveError>) -> Option<SaveCompletion> {
        let Some(job) = self.running.take() else {
            // Both callers check `running` before calling; an ordering bug
            // here must not panic mid-save. Drop the stray result instead.
            eprintln!("[save] finish_running called with no running job; result dropped");
            return None;
        };
        let completion = SaveCompletion {
            path: job.path,
            set_current_path: job.set_current_path,
            document_epoch: job.document_epoch,
            generation: job.generation,
            revision: job.revision,
            result,
        };
        if let Some(next) = self.queued.take() {
            self.start(next);
        }
        Some(completion)
    }
}

impl crate::DesktopApp {
    /// Complete every standalone document writer before Start/Join/Retry may
    /// bind the document to collaboration. This is a phase-transition barrier:
    /// no old Save/Save As or adjacent Figma publication can land after the
    /// shared identity becomes active.
    pub(crate) fn settle_document_io_before_collaboration(&mut self) {
        let _ = self.finish_background_saves();
        crate::figma_import_session::cancel_and_wait_before_collaboration(
            &mut self.host,
            &mut self.current_figma_import,
        );
        crate::html_import_session::cancel(&mut self.host, &mut self.current_html_import);
    }

    /// Queue Cmd+S without running JSON serialization on the winit thread.
    pub(crate) fn request_background_save(&mut self) -> bool {
        let gate = if self.current_path.is_some() {
            op_editor_core::CollabGateAction::SaveShared
        } else {
            op_editor_core::CollabGateAction::SaveFork
        };
        if !self
            .host
            .gate_collaboration_action(gate, op_editor_core::CollabEditSource::User)
        {
            return false;
        }
        let document_epoch = self.host.document_epoch();
        let path = self
            .save_session
            .latest_target(document_epoch)
            .map(Path::to_path_buf)
            .or_else(|| self.current_path.clone());
        let Some(path) = path else {
            return self.request_background_save_as();
        };
        if can_skip_unchanged_current_save(
            self.host.editor_state(),
            self.current_path.as_deref(),
            &path,
            self.save_session.is_active(),
        ) {
            eprintln!("[save] skipped unchanged document");
            return true;
        }
        let set_current_path = self.current_path.as_deref() != Some(path.as_path());
        let _ = self.enqueue_background_save(path, set_current_path);
        true
    }

    /// Show the native picker synchronously, then queue only the expensive
    /// snapshot serialization and disk write.
    pub(crate) fn request_background_save_as(&mut self) -> bool {
        if !self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::SaveFork,
            op_editor_core::CollabEditSource::User,
        ) {
            return false;
        }
        let collaboration_bound =
            crate::persistence::requires_background_save_as_fork(self.host.editor_state());
        let Some(path) = crate::persistence::pick_save_as_path(self.host.editor_state()) else {
            return false;
        };
        if collaboration_bound
            && !collaboration_fork_target_is_safe(self.current_path.as_deref(), &path)
        {
            let detail =
                "Save As must use a different, resolvable path while collaboration is active.";
            eprintln!(
                "[save] rejected collaboration fork target {}: {detail}",
                path.display()
            );
            crate::persistence::show_error_dialog_public(
                &self.host,
                op_host_services::doc_io::ErrorKind::Save,
                Some(&path),
                detail,
            );
            return false;
        }
        let identity = (
            self.host.document_epoch(),
            self.host.editor_state().document_generation(),
            self.host.editor_state().document_revision(),
            path.clone(),
        );
        let outcome = self.enqueue_background_save(path, true);
        if collaboration_bound {
            return self.confirm_collaboration_fork_enqueue(identity, outcome);
        }
        true
    }

    fn enqueue_background_save(&mut self, path: PathBuf, set_current_path: bool) -> EnqueueOutcome {
        let outcome = match (set_current_path, self.current_path.clone()) {
            (true, Some(source_path)) => self.save_session.enqueue_clean_bound_op_save_as(
                self.host.editor_state(),
                self.host.document_epoch(),
                source_path,
                path,
                set_current_path,
                self.mcp_wake_proxy.clone(),
            ),
            _ => self.save_session.enqueue(
                self.host.editor_state(),
                self.host.document_epoch(),
                path,
                set_current_path,
                self.mcp_wake_proxy.clone(),
            ),
        };
        eprintln!("[save] request {outcome:?}");
        outcome
    }

    /// Register leave authority only for the exact Save-As request that was
    /// accepted by `SaveSession`. `AlreadyPending` is not fresh authority: it
    /// is valid only when the same fork was registered by its original
    /// accepted enqueue.
    fn confirm_collaboration_fork_enqueue(
        &mut self,
        identity: (u64, u64, u64, PathBuf),
        outcome: EnqueueOutcome,
    ) -> bool {
        match outcome {
            EnqueueOutcome::Started | EnqueueOutcome::Queued => {
                if !self.collab_fork_saves.contains(&identity) {
                    self.collab_fork_saves.push(identity);
                }
                true
            }
            EnqueueOutcome::AlreadyPending => {
                let registered = self.collab_fork_saves.contains(&identity);
                if !registered {
                    eprintln!(
                        "[save] refused unregistered pending save as collaboration fork: {}",
                        identity.3.display()
                    );
                }
                registered
            }
        }
    }

    /// Drain one non-blocking completion after `DesktopEvent::SaveReady`.
    pub(crate) fn poll_background_save(&mut self) -> bool {
        let Some(completion) = self.save_session.poll() else {
            return false;
        };
        self.apply_save_completion(completion);
        true
    }

    /// Close/reload stop-gate: wait for all already-requested saves and apply
    /// their acks before deciding whether another synchronous save is needed.
    pub(crate) fn finish_background_saves(&mut self) -> bool {
        let mut latest_ok = true;
        while self.save_session.is_active() {
            let Some(completion) = self.save_session.wait_next() else {
                break;
            };
            latest_ok = self.apply_save_completion(completion);
        }
        latest_ok
    }

    fn apply_save_completion(&mut self, completion: SaveCompletion) -> bool {
        // The worker just dropped its save snapshot or source mapping. Ask the
        // allocator-relief worker to return retained free pages after its
        // trailing debounce; this never scans on the UI thread.
        crate::heap_pressure::schedule_relief("document save snapshot drop");
        let SaveCompletion {
            path,
            set_current_path,
            document_epoch,
            generation,
            revision,
            result,
        } = completion;
        let fork = self
            .collab_fork_saves
            .iter()
            .position(|candidate| {
                candidate.0 == document_epoch
                    && candidate.1 == generation
                    && candidate.2 == revision
                    && candidate.3 == path
            })
            .map(|index| {
                self.collab_fork_saves.remove(index);
            })
            .is_some();
        if let Err(error) = result {
            eprintln!("[save] {}: {error}", path.display());
            crate::persistence::show_error_dialog_public(
                &self.host,
                op_host_services::doc_io::ErrorKind::Save,
                Some(&path),
                &error.to_string(),
            );
            return false;
        }

        let live_epoch = self.host.document_epoch();
        let live_generation = self.host.editor_state().document_generation();
        if live_epoch != document_epoch || live_generation != generation {
            eprintln!(
                "[save] ignored stale ack epoch {document_epoch} revision {generation}:{revision}; live identity is epoch {live_epoch} revision {live_generation}:{}",
                self.host.editor_state().document_revision()
            );
            // The old snapshot did reach its requested path. It simply has no
            // authority over the document that is now open, so treat the job
            // as completed without rebinding or changing the live baseline.
            return true;
        }

        // Preserve the synchronous Save contract: once the live document was
        // persisted successfully, an import started from that document must
        // not land afterward and replace what the user just saved. Stale save
        // acknowledgements return above and cannot cancel the replacement
        // document's own import.
        crate::figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
        crate::html_import_session::cancel(&mut self.host, &mut self.current_html_import);
        if set_current_path {
            self.current_path = Some(path.clone());
        }
        crate::settings_io::touch_recent(&mut self.host, &path);
        op_host_services::doc_io::set_file_name_display(self.host.editor_state_mut(), Some(&path));
        // A user may keep editing while the worker writes. Acknowledging the
        // captured revision leaves a newer live revision dirty by design.
        if !self
            .host
            .editor_state_mut()
            .mark_saved_revision_at(generation, revision)
        {
            return false;
        }
        self.host.mark_editor_state_dirty();
        self.image_search.reset();
        self.rebind_git_session_for_current_path();
        if fork {
            // Leaving rotates the collab avatar generation (op-collab-host's
            // `advance_generation`), which evicts every cached avatar byte
            // from the process-global registry in op-editor-ui. op-collab-host
            // guards its own tests against that, but its `#[cfg(test)]` guard
            // is compiled OUT when this binary links it as a dependency —
            // `cfg(test)` is only active for a crate's own test build. So the
            // guard must live here, in the crate whose test binary actually
            // runs this path: without it, a fork-save test's rotation landed
            // between the image_decode_host avatar test's
            // `note_pending_decode` and its first `pump`, evicted the avatar
            // bytes, and the pump discarded the queued id as byte-less
            // (linux-aarch64 CI, reproduced deterministically in a container
            // by widening that window). Reentrant per thread; compiled out of
            // production builds.
            #[cfg(test)]
            let _avatar_guard = crate::collab_avatar_host::lock_avatar_test_registry();
            self.collab_runtime.leave(&mut self.host);
        }
        true
    }
}

fn can_skip_unchanged_current_save(
    state: &EditorState,
    current_path: Option<&Path>,
    target: &Path,
    save_active: bool,
) -> bool {
    !save_active && !state.is_dirty() && current_path == Some(target) && target.is_file()
}

fn can_copy_clean_bound_op(state: &EditorState, source: &Path, target: &Path) -> bool {
    !state.is_dirty()
        && paths_provably_distinct(source, target)
        && source.is_file()
        && source
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("op"))
}

fn collaboration_fork_target_is_safe(current_path: Option<&Path>, target: &Path) -> bool {
    current_path.is_none_or(|current| paths_provably_distinct(current, target))
}

/// Resolve an existing path, or a not-yet-created leaf below an existing
/// directory, to an absolute comparison identity. Ambiguous paths fail closed:
/// a collaboration fork must prove that it cannot overwrite the shared path.
fn path_comparison_identity(path: &Path) -> Option<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(canonical) => return Some(canonical),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return None,
    }
    let file_name = path.file_name()?;
    let parent = path.parent()?;
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    std::fs::canonicalize(parent)
        .ok()
        .map(|canonical_parent| canonical_parent.join(file_name))
}

fn paths_provably_distinct(left: &Path, right: &Path) -> bool {
    let Some(left) = path_comparison_identity(left) else {
        return false;
    };
    let Some(right) = path_comparison_identity(right) else {
        return false;
    };
    paths_differ_on_platform(&left, &right)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn paths_differ_on_platform(left: &Path, right: &Path) -> bool {
    // Conservatively reject case-only changes on the platforms commonly
    // backed by case-insensitive filesystems. Existing case-sensitive paths
    // still canonicalize above; this may reject a valid fork but cannot let an
    // equivalent shared path through.
    !left
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn paths_differ_on_platform(left: &Path, right: &Path) -> bool {
    left != right
}

#[cfg(test)]
#[path = "save_session/clean_copy_tests.rs"]
mod clean_copy_tests;

#[cfg(test)]
#[path = "save_session/tests.rs"]
mod tests;
