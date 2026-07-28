use std::sync::Arc;

use crate::{
    session::OwnerSessionCore, session_state::CommitRecord, Applied, CatchUp, CollabMessage,
    ConnectionKey, OwnerEffect, ParticipantPresence, PeerId, Presence, SessionError,
};

impl OwnerSessionCore {
    pub(crate) fn handle_catch_up(
        &self,
        connection: ConnectionKey,
        catch_up: CatchUp,
        document: &jian_ops_schema::PenDocument,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        if catch_up.after_seq.0 > self.seq.0 {
            return Err(SessionError::FutureCatchUpSequence);
        }
        if catch_up.after_seq == self.seq {
            return Ok(vec![OwnerEffect::CommitBatch {
                to: connection,
                commits: Vec::new(),
            }]);
        }
        let expected_first = catch_up.after_seq.0.saturating_add(1);
        let can_replay = self
            .commit_log
            .front()
            .is_some_and(|record| record.commit.seq.0 <= expected_first);
        if can_replay {
            let commits = self
                .commit_log
                .iter()
                .filter(|record| record.commit.seq.0 > catch_up.after_seq.0)
                .map(|record| record.commit.clone())
                .collect();
            return Ok(vec![OwnerEffect::CommitBatch {
                to: connection,
                commits,
            }]);
        }
        let snapshot = self.snapshot_for(document)?;
        let (_, snapshot_message) =
            self.checked_outbound(CollabMessage::Snapshot(Box::new(snapshot)))?;
        let CollabMessage::Snapshot(snapshot) = snapshot_message else {
            unreachable!("checked Snapshot must retain its message kind");
        };
        Ok(vec![OwnerEffect::Snapshot {
            to: connection,
            snapshot,
        }])
    }

    pub(crate) fn handle_applied(
        &mut self,
        peer_id: &PeerId,
        applied: Applied,
    ) -> Result<(), SessionError> {
        if applied.through_seq.0 > self.seq.0 {
            return Err(SessionError::FutureAppliedSequence);
        }
        let peer = self
            .peers
            .get_mut(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        if applied.through_seq.0 < peer.applied_through.0 {
            return Err(SessionError::AppliedSequenceRegressed);
        }
        peer.applied_through = applied.through_seq;
        self.prune_commit_log_from_acks();
        Ok(())
    }

    pub(crate) fn handle_presence(
        &self,
        peer_id: &PeerId,
        presence: Presence,
    ) -> Result<Vec<OwnerEffect>, SessionError> {
        let peer = self
            .peers
            .get(peer_id.as_ref())
            .ok_or(SessionError::UnknownPeer)?;
        let (_, message) =
            self.checked_outbound(CollabMessage::PresenceChanged(ParticipantPresence {
                participant_id: peer.principal.participant_id().clone(),
                peer_id: peer.principal.peer_id().clone(),
                presence,
            }))?;
        Ok(vec![OwnerEffect::Broadcast { message }])
    }

    pub(crate) fn push_commit(&mut self, record: Arc<CommitRecord>) {
        let limits = self.config.session_limits;
        self.commit_log_bytes = self.commit_log_bytes.saturating_add(record.weight);
        self.commit_log.push_back(record);
        while self.commit_log.len() > limits.commit_log_entries
            || self.commit_log_bytes > limits.commit_log_bytes
        {
            let Some(removed) = self.commit_log.pop_front() else {
                break;
            };
            self.commit_log_bytes = self.commit_log_bytes.saturating_sub(removed.weight);
        }
    }

    fn prune_commit_log_from_acks(&mut self) {
        let mut remote_acks = self
            .peers
            .values()
            .filter(|peer| {
                peer.connection.is_some() && peer.principal.peer_id() != &self.owner_peer_id
            })
            .map(|peer| peer.applied_through.0);
        let Some(mut through) = remote_acks.next() else {
            return;
        };
        for ack in remote_acks {
            through = through.min(ack);
        }
        while self
            .commit_log
            .front()
            .is_some_and(|record| record.commit.seq.0 <= through)
        {
            let removed = self.commit_log.pop_front().expect("front was checked");
            self.commit_log_bytes = self.commit_log_bytes.saturating_sub(removed.weight);
        }
    }
}
