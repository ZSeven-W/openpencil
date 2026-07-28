use std::collections::VecDeque;

use op_collab::{
    canonical_document_hash, diff_supported, AdmissionGrant, CanonicalHashError, ClientOpId,
    CollabMessage, CommitSeq, ConnectionKey, DiffContext, DiffError, Epoch, FrameEnvelope,
    OwnerEffect, OwnerSessionConfig, OwnerSessionCore, PeerActivation, PeerId, PeerNamespace,
    ProtocolError, Role, SessionError, SessionId, Submit, UndoRequest, VerifiedAuthMetadata,
};
use op_editor_core::{
    DocumentInstallError, EditOrigin, LocalEditCapture, LocalEditError, LocalEditOutcome,
};

use super::CollaborationEditorHost;

pub const DEFAULT_MAX_QUEUED_DOCUMENT_FRAMES: usize = 128;
pub const DEFAULT_MAX_QUEUED_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;

/// Bounds frames held while a local pointer gesture owns the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerEditorLimits {
    pub max_queued_document_frames: usize,
    pub max_queued_document_bytes: usize,
}

impl Default for OwnerEditorLimits {
    fn default() -> Self {
        Self {
            max_queued_document_frames: DEFAULT_MAX_QUEUED_DOCUMENT_FRAMES,
            max_queued_document_bytes: DEFAULT_MAX_QUEUED_DOCUMENT_BYTES,
        }
    }
}

impl OwnerEditorLimits {
    fn validate(self) -> Result<Self, OwnerEditorError> {
        if self.max_queued_document_frames == 0 || self.max_queued_document_bytes == 0 {
            return Err(OwnerEditorError::InvalidQueueLimits);
        }
        Ok(self)
    }
}

#[derive(Debug)]
struct QueuedFrame {
    connection: ConnectionKey,
    frame: FrameEnvelope,
    encoded_bytes: usize,
}

/// Non-fatal result for the local gesture that just ended.
#[derive(Debug)]
pub enum LocalEditResolution {
    NoChange,
    Committed(ClientOpId),
    Rejected(LocalEditRejection),
}

#[derive(Debug)]
pub enum LocalEditRejection {
    Unsupported(DiffError),
    OwnerRejected,
    CandidateMismatch,
}

/// Effects ready for the transport after one owner-actor turn.
#[derive(Debug)]
pub struct OwnerEditorOutput {
    pub effects: Vec<OwnerEffect>,
    pub local_edit: Option<LocalEditResolution>,
    /// Connections whose queued frames failed owner-session validation.
    ///
    /// The host must close and disconnect only these peers before routing the
    /// successful effects from the same turn.
    pub failed_connections: Vec<ConnectionKey>,
}

impl OwnerEditorOutput {
    fn remote(effects: Vec<OwnerEffect>) -> Self {
        Self {
            effects,
            local_edit: None,
            failed_connections: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
struct QueuedFlush {
    effects: Vec<OwnerEffect>,
    failed_connections: Vec<ConnectionKey>,
}

#[derive(Debug, thiserror::Error)]
pub enum OwnerEditorError {
    #[error("owner editor queue limits must be non-zero")]
    InvalidQueueLimits,
    #[error("a local collaboration edit is already active")]
    LocalEditAlreadyActive,
    #[error("no local collaboration edit is active")]
    NoLocalEdit,
    #[error("owner document frame queue exceeded its bounded capacity")]
    QueueFull,
    #[error("owner collaboration actor is unusable after a post-install failure")]
    Poisoned,
    #[error("owner peer sequencing state is unavailable")]
    MissingOwnerProgress,
    #[error("owner submit did not produce exactly one install candidate")]
    MissingInstallCandidate,
    #[error("owner editor transaction failed: {0}")]
    LocalEdit(#[from] LocalEditError),
    #[error("owner session failed: {0}")]
    Session(#[from] SessionError),
    #[error("collaboration frame failed validation: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("collaboration document install failed: {0}")]
    Install(#[from] DocumentInstallError),
    #[error("collaboration document hashing failed: {0}")]
    Hash(#[from] CanonicalHashError),
}

/// Owner sequencer attached to one live editor actor.
pub struct OwnerEditorSession {
    core: OwnerSessionCore,
    owner_connection: ConnectionKey,
    owner_peer_id: PeerId,
    owner_namespace: PeerNamespace,
    local_capture: Option<LocalEditCapture>,
    queued_document_frames: VecDeque<QueuedFrame>,
    queued_document_bytes: usize,
    limits: OwnerEditorLimits,
    poisoned: bool,
}

impl OwnerEditorSession {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: SessionId,
        epoch: Epoch,
        seq: CommitSeq,
        owner_connection: ConnectionKey,
        owner_grant: AdmissionGrant,
        host: &impl CollaborationEditorHost,
        session_config: OwnerSessionConfig,
        limits: OwnerEditorLimits,
    ) -> Result<Self, OwnerEditorError> {
        let limits = limits.validate()?;
        let owner_peer_id = owner_grant.principal().peer_id().clone();
        let owner_namespace = owner_grant.peer_namespace().clone();
        let core = OwnerSessionCore::new(
            session_id,
            epoch,
            seq,
            owner_connection,
            owner_grant,
            &host.editor_state().doc,
            session_config,
        )?;
        Ok(Self {
            core,
            owner_connection,
            owner_peer_id,
            owner_namespace,
            local_capture: None,
            queued_document_frames: VecDeque::new(),
            queued_document_bytes: 0,
            limits,
            poisoned: false,
        })
    }

    pub fn core(&self) -> &OwnerSessionCore {
        &self.core
    }

    /// Successfully installed owner-local property commits retained for M1
    /// conditional selective undo, ordered oldest to newest.
    pub fn own_undo_targets(&self) -> Vec<ClientOpId> {
        self.core.own_undo_targets()
    }

    pub fn latest_own_undo_target(&self) -> Option<ClientOpId> {
        self.core.latest_own_undo_target()
    }

    /// Allocate, but do not consume, an idempotent owner-local undo request.
    ///
    /// The runtime should retain this token until the install succeeds or is
    /// definitively aborted. Replaying the exact token returns the cached
    /// result without applying the compensation twice.
    pub fn next_own_undo_request(
        &self,
        target_client_op_id: ClientOpId,
    ) -> Result<UndoRequest, OwnerEditorError> {
        self.ensure_idle_document_actor()?;
        Ok(self.core.next_own_undo_request(target_client_op_id)?)
    }

    /// Execute one owner-local conditional undo through the same synchronous
    /// document installation and finalize path used by remote submissions.
    pub fn request_own_undo(
        &mut self,
        request: UndoRequest,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<OwnerEditorOutput, OwnerEditorError> {
        self.ensure_idle_document_actor()?;
        let effects = self
            .core
            .request_own_undo(request, &host.editor_state().doc)?;
        let mut effects = self.install_prepared_effects(host, effects)?;
        let queued = self.flush_queued(host)?;
        effects.extend(queued.effects);
        Ok(OwnerEditorOutput {
            effects,
            local_edit: None,
            failed_connections: queued.failed_connections,
        })
    }

    pub fn activate_peer(
        &mut self,
        connection: ConnectionKey,
        grant: AdmissionGrant,
        host: &impl CollaborationEditorHost,
    ) -> Result<PeerActivation, OwnerEditorError> {
        self.ensure_idle_document_actor()?;
        Ok(self
            .core
            .activate_peer(connection, grant, &host.editor_state().doc)?)
    }

    pub fn resume_peer(
        &mut self,
        connection: ConnectionKey,
        grant: AdmissionGrant,
    ) -> Result<PeerActivation, OwnerEditorError> {
        self.ensure_idle_document_actor()?;
        Ok(self.core.resume_peer(connection, grant)?)
    }

    pub fn complete_renewal(
        &mut self,
        connection: ConnectionKey,
        auth: VerifiedAuthMetadata,
    ) -> Result<(), OwnerEditorError> {
        self.ensure_usable()?;
        Ok(self.core.complete_renewal(connection, auth)?)
    }

    pub fn disconnect(
        &mut self,
        connection: ConnectionKey,
    ) -> Result<OwnerEditorOutput, OwnerEditorError> {
        self.ensure_usable()?;
        self.purge_queued_connection(connection);
        Ok(OwnerEditorOutput::remote(self.core.disconnect(connection)?))
    }

    pub fn begin_local_edit(
        &mut self,
        host: &impl CollaborationEditorHost,
    ) -> Result<(), OwnerEditorError> {
        self.ensure_usable()?;
        if self.local_capture.is_some() {
            return Err(OwnerEditorError::LocalEditAlreadyActive);
        }
        if self.core.install_pending() {
            return Err(OwnerEditorError::MissingInstallCandidate);
        }
        self.local_capture = Some(host.editor_state().begin_local_edit());
        Ok(())
    }

    pub fn accept_frame(
        &mut self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<OwnerEditorOutput, OwnerEditorError> {
        self.ensure_usable()?;
        if self.local_capture.is_some() && document_ordered_message(frame.body()) {
            self.enqueue_document_frame(connection, frame)?;
            return Ok(OwnerEditorOutput::remote(Vec::new()));
        }
        let effects = self
            .core
            .accept_frame(connection, frame, &host.editor_state().doc)?;
        let effects = self.install_prepared_effects(host, effects)?;
        Ok(OwnerEditorOutput::remote(effects))
    }

    pub fn finish_local_edit(
        &mut self,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<OwnerEditorOutput, OwnerEditorError> {
        self.ensure_usable()?;
        let capture = self
            .local_capture
            .take()
            .ok_or(OwnerEditorError::NoLocalEdit)?;
        let outcome = host.editor_state_mut().end_local_edit(capture)?;
        let local_edit = match outcome {
            LocalEditOutcome::NoChange => LocalEditResolution::NoChange,
            LocalEditOutcome::Changed(completed) => {
                let progress = self
                    .core
                    .peer_progress(&self.owner_peer_id)
                    .ok_or(OwnerEditorError::MissingOwnerProgress)?;
                let Some(counter) = progress.next_counter else {
                    host.editor_state_mut().rollback_local_edit(completed)?;
                    return Err(OwnerEditorError::MissingOwnerProgress);
                };
                let diff_context = DiffContext::new(
                    self.owner_namespace.clone(),
                    Role::Owner,
                    progress.next_id_counter,
                );
                let supported =
                    match diff_supported(completed.before(), completed.after(), &diff_context) {
                        Ok(supported) => supported,
                        Err(error) => {
                            host.editor_state_mut().rollback_local_edit(completed)?;
                            let queued = self.flush_queued(host)?;
                            return Ok(OwnerEditorOutput {
                                effects: queued.effects,
                                local_edit: Some(LocalEditResolution::Rejected(
                                    LocalEditRejection::Unsupported(error),
                                )),
                                failed_connections: queued.failed_connections,
                            });
                        }
                    };
                let client_op_id = ClientOpId {
                    peer_id: self.owner_peer_id.clone(),
                    local_counter: counter,
                };
                let submit = Submit {
                    client_op_id: client_op_id.clone(),
                    base_seq: self.core.seq(),
                    txn: supported.txn,
                };
                let frame = FrameEnvelope::new(
                    self.core.session_id().clone(),
                    self.core.epoch(),
                    CollabMessage::Submit(submit),
                );
                let effects =
                    self.core
                        .accept_frame(self.owner_connection, frame, completed.before())?;
                let (mut passthrough, prepared) = take_install_candidate(effects)?;
                let Some(prepared) = prepared else {
                    host.editor_state_mut().rollback_local_edit(completed)?;
                    let queued = self.flush_queued(host)?;
                    passthrough.extend(queued.effects);
                    return Ok(OwnerEditorOutput {
                        effects: passthrough,
                        local_edit: Some(LocalEditResolution::Rejected(
                            LocalEditRejection::OwnerRejected,
                        )),
                        failed_connections: queued.failed_connections,
                    });
                };
                let live_hash = canonical_document_hash(&host.editor_state().doc)?;
                if live_hash != prepared.candidate_hash() {
                    self.core.abort_prepare(prepared)?;
                    host.editor_state_mut().rollback_local_edit(completed)?;
                    let queued = self.flush_queued(host)?;
                    passthrough.extend(queued.effects);
                    return Ok(OwnerEditorOutput {
                        effects: passthrough,
                        local_edit: Some(LocalEditResolution::Rejected(
                            LocalEditRejection::CandidateMismatch,
                        )),
                        failed_connections: queued.failed_connections,
                    });
                }
                completed.accept();
                passthrough.push(self.core.finalize_install(prepared, live_hash)?);
                let queued = self.flush_queued(host)?;
                passthrough.extend(queued.effects);
                return Ok(OwnerEditorOutput {
                    effects: passthrough,
                    local_edit: Some(LocalEditResolution::Committed(client_op_id)),
                    failed_connections: queued.failed_connections,
                });
            }
        };
        let queued = self.flush_queued(host)?;
        Ok(OwnerEditorOutput {
            effects: queued.effects,
            local_edit: Some(local_edit),
            failed_connections: queued.failed_connections,
        })
    }

    fn install_prepared_effects(
        &mut self,
        host: &mut impl CollaborationEditorHost,
        effects: Vec<OwnerEffect>,
    ) -> Result<Vec<OwnerEffect>, OwnerEditorError> {
        let (mut output, prepared) = take_install_candidate(effects)?;
        let Some(mut prepared) = prepared else {
            return Ok(output);
        };
        let candidate = prepared
            .take_candidate_document()
            .ok_or(OwnerEditorError::MissingInstallCandidate)?;
        if let Err(error) = host.install_collaboration_document(candidate, EditOrigin::RemoteCommit)
        {
            self.core.abort_prepare(prepared)?;
            return Err(error.into());
        }
        let installed_hash = match canonical_document_hash(&host.editor_state().doc) {
            Ok(hash) => hash,
            Err(error) => {
                self.poisoned = true;
                return Err(error.into());
            }
        };
        output.push(self.core.finalize_install(prepared, installed_hash)?);
        Ok(output)
    }

    fn enqueue_document_frame(
        &mut self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
    ) -> Result<(), OwnerEditorError> {
        let encoded_bytes = frame.to_json_vec()?.len();
        let next_bytes = self
            .queued_document_bytes
            .checked_add(encoded_bytes)
            .ok_or(OwnerEditorError::QueueFull)?;
        if self.queued_document_frames.len() >= self.limits.max_queued_document_frames
            || next_bytes > self.limits.max_queued_document_bytes
        {
            return Err(OwnerEditorError::QueueFull);
        }
        self.queued_document_bytes = next_bytes;
        self.queued_document_frames.push_back(QueuedFrame {
            connection,
            frame,
            encoded_bytes,
        });
        Ok(())
    }

    fn flush_queued(
        &mut self,
        host: &mut impl CollaborationEditorHost,
    ) -> Result<QueuedFlush, OwnerEditorError> {
        let mut output = QueuedFlush::default();
        while let Some(queued) = self.queued_document_frames.pop_front() {
            self.queued_document_bytes = self
                .queued_document_bytes
                .checked_sub(queued.encoded_bytes)
                .expect("queued document byte accounting must remain balanced");
            let effects = match self.core.accept_frame(
                queued.connection,
                queued.frame,
                &host.editor_state().doc,
            ) {
                Ok(effects) => effects,
                Err(_) => {
                    output.failed_connections.push(queued.connection);
                    self.purge_queued_connection(queued.connection);
                    continue;
                }
            };
            output
                .effects
                .extend(self.install_prepared_effects(host, effects)?);
        }
        Ok(output)
    }

    fn purge_queued_connection(&mut self, connection: ConnectionKey) {
        let mut removed_bytes = 0usize;
        self.queued_document_frames.retain(|queued| {
            if queued.connection == connection {
                removed_bytes = removed_bytes
                    .checked_add(queued.encoded_bytes)
                    .expect("queued document byte accounting must not overflow");
                false
            } else {
                true
            }
        });
        self.queued_document_bytes = self
            .queued_document_bytes
            .checked_sub(removed_bytes)
            .expect("queued document byte accounting must remain balanced");
    }

    fn ensure_usable(&self) -> Result<(), OwnerEditorError> {
        if self.poisoned {
            Err(OwnerEditorError::Poisoned)
        } else {
            Ok(())
        }
    }

    fn ensure_idle_document_actor(&self) -> Result<(), OwnerEditorError> {
        self.ensure_usable()?;
        if self.local_capture.is_some() {
            Err(OwnerEditorError::LocalEditAlreadyActive)
        } else if self.core.install_pending() {
            Err(OwnerEditorError::MissingInstallCandidate)
        } else {
            Ok(())
        }
    }
}

fn take_install_candidate(
    effects: Vec<OwnerEffect>,
) -> Result<(Vec<OwnerEffect>, Option<op_collab::PreparedCommit>), OwnerEditorError> {
    let mut output = Vec::with_capacity(effects.len());
    let mut prepared = None;
    for effect in effects {
        match effect {
            OwnerEffect::PrepareInstall(candidate) if prepared.is_none() => {
                prepared = Some(*candidate);
            }
            OwnerEffect::PrepareInstall(_) => {
                return Err(OwnerEditorError::MissingInstallCandidate);
            }
            effect => output.push(effect),
        }
    }
    Ok((output, prepared))
}

fn document_ordered_message(message: &CollabMessage) -> bool {
    matches!(
        message,
        CollabMessage::Submit(_) | CollabMessage::CatchUp(_) | CollabMessage::UndoRequest(_)
    )
}
