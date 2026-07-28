use std::collections::VecDeque;

use op_collab::{
    canonical_document_hash, ClientOpId, CollabMessage, Epoch, FrameEnvelope, GuestEffect,
    GuestError, GuestInstallReason, GuestSessionConfig, GuestSessionCore, ProtocolError, SessionId,
    Welcome,
};
use op_editor_core::{
    DocumentInstallError, EditOrigin, LocalEditCapture, LocalEditError, LocalEditOutcome,
};

use super::CollaborationEditorHost;

pub const DEFAULT_MAX_QUEUED_GUEST_FRAMES: usize = 128;
pub const DEFAULT_MAX_QUEUED_GUEST_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestEditorLimits {
    pub max_queued_document_frames: usize,
    pub max_queued_document_bytes: usize,
}

impl Default for GuestEditorLimits {
    fn default() -> Self {
        Self {
            max_queued_document_frames: DEFAULT_MAX_QUEUED_GUEST_FRAMES,
            max_queued_document_bytes: DEFAULT_MAX_QUEUED_GUEST_BYTES,
        }
    }
}

impl GuestEditorLimits {
    fn validate(self) -> Result<Self, GuestEditorError> {
        if self.max_queued_document_frames == 0 || self.max_queued_document_bytes == 0 {
            return Err(GuestEditorError::InvalidQueueLimits);
        }
        Ok(self)
    }
}

#[derive(Debug)]
struct QueuedFrame {
    frame: FrameEnvelope,
    encoded_bytes: usize,
}

#[derive(Debug)]
pub enum GuestLocalEditResolution {
    NoChange,
    Submitted,
    Rejected(GuestLocalEditRejection),
}

#[derive(Debug)]
pub enum GuestLocalEditRejection {
    Unsupported(GuestError),
}

#[derive(Debug)]
pub struct GuestEditorOutput {
    pub effects: Vec<GuestEffect>,
    pub local_edit: Option<GuestLocalEditResolution>,
}

impl GuestEditorOutput {
    fn remote(effects: Vec<GuestEffect>) -> Self {
        Self {
            effects,
            local_edit: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GuestEditorError {
    #[error("guest editor queue limits must be non-zero")]
    InvalidQueueLimits,
    #[error("a local guest edit is already active")]
    LocalEditAlreadyActive,
    #[error("no local guest edit is active")]
    NoLocalEdit,
    #[error("guest is not currently allowed to mutate the document")]
    MutationNotAllowed,
    #[error("guest document frame queue exceeded its bounded capacity")]
    QueueFull,
    #[error("guest collaboration actor is unusable after a post-install failure")]
    Poisoned,
    #[error("guest stream terminal reconciliation requires a typed Bye frame")]
    TerminalFrameRequired,
    #[error("guest editor transaction failed: {0}")]
    LocalEdit(#[from] LocalEditError),
    #[error("guest collaboration state failed: {0}")]
    Guest(#[from] GuestError),
    #[error("collaboration frame failed validation: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("collaboration document install failed: {0}")]
    Install(#[from] DocumentInstallError),
}

pub struct GuestEditorSession {
    core: GuestSessionCore,
    local_capture: Option<LocalEditCapture>,
    queued_document_frames: VecDeque<QueuedFrame>,
    queued_document_bytes: usize,
    limits: GuestEditorLimits,
    poisoned: bool,
}

impl GuestEditorSession {
    pub fn new(
        session_id: SessionId,
        epoch: Epoch,
        welcome: Welcome,
        config: GuestSessionConfig,
        limits: GuestEditorLimits,
    ) -> Result<Self, GuestEditorError> {
        Ok(Self {
            core: GuestSessionCore::new(session_id, epoch, welcome, config)?,
            local_capture: None,
            queued_document_frames: VecDeque::new(),
            queued_document_bytes: 0,
            limits: limits.validate()?,
            poisoned: false,
        })
    }

    pub fn core(&self) -> &GuestSessionCore {
        &self.core
    }

    /// Successfully installed property commits authored by this guest,
    /// ordered oldest to newest.
    pub fn undo_targets(&self) -> Vec<ClientOpId> {
        self.core.undo_targets()
    }

    pub fn latest_undo_target(&self) -> Option<ClientOpId> {
        self.core.latest_undo_target().cloned()
    }

    /// Emit an idempotent selective-undo request for runtime routing.
    ///
    /// The resulting `GuestEditorOutput.effects` contains the outbound
    /// `UndoRequest`. `UndoResult` and the compensation Commit return through
    /// `accept_frame`, which performs any required atomic document install.
    pub fn request_undo(
        &mut self,
        target_client_op_id: ClientOpId,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<GuestEditorOutput, GuestEditorError> {
        self.ensure_usable()?;
        if self.local_capture.is_some() {
            return Err(GuestEditorError::LocalEditAlreadyActive);
        }
        let effect = self.core.request_undo(target_client_op_id)?;
        let effects = self.install_prepared_effects(host, vec![effect])?;
        Ok(GuestEditorOutput::remote(effects))
    }

    pub fn resume(
        &mut self,
        session_id: SessionId,
        epoch: Epoch,
        welcome: Welcome,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<GuestEditorOutput, GuestEditorError> {
        self.ensure_usable()?;
        if self.local_capture.is_some() {
            return Err(GuestEditorError::LocalEditAlreadyActive);
        }
        let effects = self.core.resume(session_id, epoch, welcome)?;
        let effects = self.install_prepared_effects(host, effects)?;
        Ok(GuestEditorOutput::remote(effects))
    }

    pub fn disconnect(
        &mut self,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<(), GuestEditorError> {
        self.cancel_local_edit(host)?;
        self.queued_document_frames.clear();
        self.queued_document_bytes = 0;
        self.core.disconnect();
        Ok(())
    }

    pub fn begin_local_edit(
        &mut self,
        host: &impl CollaborationEditorHost,
    ) -> Result<(), GuestEditorError> {
        self.ensure_usable()?;
        if self.local_capture.is_some() {
            return Err(GuestEditorError::LocalEditAlreadyActive);
        }
        if !self.core.document_mutation_allowed() {
            return Err(GuestEditorError::MutationNotAllowed);
        }
        self.local_capture = Some(host.editor_state().begin_local_edit());
        Ok(())
    }

    pub fn cancel_local_edit(
        &mut self,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<(), GuestEditorError> {
        let Some(capture) = self.local_capture.take() else {
            return Ok(());
        };
        match host.editor_state_mut().end_local_edit(capture)? {
            LocalEditOutcome::NoChange => {}
            LocalEditOutcome::Changed(completed) => {
                host.editor_state_mut().rollback_local_edit(completed)?;
            }
        }
        Ok(())
    }

    pub fn accept_frame(
        &mut self,
        frame: FrameEnvelope,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<GuestEditorOutput, GuestEditorError> {
        self.ensure_usable()?;
        if self.local_capture.is_some() && document_ordered_message(frame.body()) {
            self.enqueue_document_frame(frame)?;
            return Ok(GuestEditorOutput::remote(Vec::new()));
        }
        let effects = self.core.accept_frame(frame)?;
        let effects = self.install_prepared_effects(host, effects)?;
        Ok(GuestEditorOutput::remote(effects))
    }

    /// Reconciles every already-received authoritative frame before transport
    /// EOF is committed to the guest state.
    ///
    /// A terminal frame travels on the desktop's independent lifecycle lane,
    /// so it can arrive while a GUI gesture still owns an optimistic capture.
    /// Roll that capture back, install queued frames in wire order, and only
    /// then apply the optional typed terminal frame. Any resulting acknowledgments
    /// are returned for observation, but the caller must not require delivery
    /// after the remote stream has closed.
    pub fn finish_inbound_stream(
        &mut self,
        terminal_frame: Option<FrameEnvelope>,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<GuestEditorOutput, GuestEditorError> {
        self.ensure_usable()?;
        if terminal_frame
            .as_ref()
            .is_some_and(|frame| !matches!(frame.body(), CollabMessage::Bye(_)))
        {
            return Err(GuestEditorError::TerminalFrameRequired);
        }
        self.cancel_local_edit(host)?;
        let mut effects = self.flush_queued(host)?;
        if let Some(frame) = terminal_frame {
            let terminal = self.core.accept_frame(frame)?;
            effects.extend(self.install_prepared_effects(host, terminal)?);
        }
        Ok(GuestEditorOutput::remote(effects))
    }

    pub fn finish_local_edit(
        &mut self,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<GuestEditorOutput, GuestEditorError> {
        self.ensure_usable()?;
        let capture = self
            .local_capture
            .take()
            .ok_or(GuestEditorError::NoLocalEdit)?;
        let outcome = host.editor_state_mut().end_local_edit(capture)?;
        let local_edit = match outcome {
            LocalEditOutcome::NoChange => GuestLocalEditResolution::NoChange,
            LocalEditOutcome::Changed(completed) => {
                let effect = match self.core.begin_local_edit(completed.after()) {
                    Ok(effect) => effect,
                    Err(error) => {
                        host.editor_state_mut().rollback_local_edit(completed)?;
                        let effects = self.flush_queued(host)?;
                        return Ok(GuestEditorOutput {
                            effects,
                            local_edit: Some(GuestLocalEditResolution::Rejected(
                                GuestLocalEditRejection::Unsupported(error),
                            )),
                        });
                    }
                };
                completed.accept();
                let mut effects = self.install_prepared_effects(host, vec![effect])?;
                effects.extend(self.flush_queued(host)?);
                return Ok(GuestEditorOutput {
                    effects,
                    local_edit: Some(GuestLocalEditResolution::Submitted),
                });
            }
        };
        let effects = self.flush_queued(host)?;
        Ok(GuestEditorOutput {
            effects,
            local_edit: Some(local_edit),
        })
    }

    fn install_prepared_effects(
        &mut self,
        host: &mut impl CollaborationEditorHost,
        effects: Vec<GuestEffect>,
    ) -> Result<Vec<GuestEffect>, GuestEditorError> {
        let mut pending: VecDeque<_> = effects.into();
        let mut output = Vec::new();
        while let Some(effect) = pending.pop_front() {
            let GuestEffect::PrepareInstall(mut prepared) = effect else {
                output.push(effect);
                continue;
            };
            let candidate = prepared
                .take_candidate_document()
                .ok_or(GuestError::PreparedInstallMismatch)?;
            let origin = match prepared.reason() {
                GuestInstallReason::Snapshot => EditOrigin::Snapshot,
                GuestInstallReason::RemoteCommit => EditOrigin::RemoteCommit,
                GuestInstallReason::OwnCommit
                | GuestInstallReason::UndoCompensation
                | GuestInstallReason::RejectRollback => EditOrigin::Replay,
            };
            if let Err(error) = host.install_collaboration_document(candidate, origin) {
                self.core.abort_install(*prepared)?;
                return Err(error.into());
            }
            let installed_hash = match canonical_document_hash(&host.editor_state().doc) {
                Ok(hash) => hash,
                Err(error) => {
                    self.poisoned = true;
                    return Err(GuestError::CanonicalHash(error).into());
                }
            };
            let continued = self.core.finalize_install(*prepared, installed_hash)?;
            pending.extend(continued);
        }
        Ok(output)
    }

    fn enqueue_document_frame(&mut self, frame: FrameEnvelope) -> Result<(), GuestEditorError> {
        let encoded_bytes = frame.to_json_vec()?.len();
        let next_bytes = self
            .queued_document_bytes
            .checked_add(encoded_bytes)
            .ok_or(GuestEditorError::QueueFull)?;
        if self.queued_document_frames.len() >= self.limits.max_queued_document_frames
            || next_bytes > self.limits.max_queued_document_bytes
        {
            return Err(GuestEditorError::QueueFull);
        }
        self.queued_document_bytes = next_bytes;
        self.queued_document_frames.push_back(QueuedFrame {
            frame,
            encoded_bytes,
        });
        Ok(())
    }

    fn flush_queued(
        &mut self,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<Vec<GuestEffect>, GuestEditorError> {
        let mut output = Vec::new();
        while let Some(queued) = self.queued_document_frames.pop_front() {
            self.queued_document_bytes = self
                .queued_document_bytes
                .saturating_sub(queued.encoded_bytes);
            let effects = self.core.accept_frame(queued.frame)?;
            output.extend(self.install_prepared_effects(host, effects)?);
        }
        Ok(output)
    }

    fn ensure_usable(&self) -> Result<(), GuestEditorError> {
        if self.poisoned {
            Err(GuestEditorError::Poisoned)
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    pub(super) fn has_pending_host_work(&self) -> bool {
        self.local_capture.is_some()
            || !self.queued_document_frames.is_empty()
            || self.queued_document_bytes != 0
    }
}

fn document_ordered_message(message: &CollabMessage) -> bool {
    matches!(
        message,
        CollabMessage::Welcome(_)
            | CollabMessage::Commit(_)
            | CollabMessage::Reject(_)
            | CollabMessage::Snapshot(_)
            | CollabMessage::UndoResult(_)
            | CollabMessage::Bye(_)
    )
}
