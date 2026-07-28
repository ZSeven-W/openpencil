use std::collections::VecDeque;

use jian_ops_schema::PenDocument;
use op_collab::{
    canonical_document_hash, AdmissionGrant, Applied, CanonicalHash, CollabMessage, CommitSeq,
    ConnectionKey, ConnectionPrincipal, Epoch, FrameEnvelope, GuestConnectionState, GuestEffect,
    GuestSessionConfig, GuestSessionCore, OwnerEffect, OwnerSessionConfig, OwnerSessionCore,
    ParticipantId, PeerId, PeerNamespace, Role, SessionId, VerifiedAuthMetadata,
};

const SESSION: &str = "deterministic-simulation";
const EPOCH: u64 = 37;
const GUEST_COUNT: usize = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Stats {
    commits: usize,
    rejects: usize,
    duplicate_peer_frames: usize,
    reordered_owner_frames: usize,
    retained_catchups: usize,
    snapshots: usize,
    reconnects: usize,
}

impl Stats {
    fn add(&mut self, other: Self) {
        self.commits += other.commits;
        self.rejects += other.rejects;
        self.duplicate_peer_frames += other.duplicate_peer_frames;
        self.reordered_owner_frames += other.reordered_owner_frames;
        self.retained_catchups += other.retained_catchups;
        self.snapshots += other.snapshots;
        self.reconnects += other.reconnects;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Outcome {
    hash: CanonicalHash,
    seq: CommitSeq,
    stats: Stats,
}

struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1) ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn range(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next() % upper as u64) as usize
    }
}

struct SimGuest {
    core: GuestSessionCore,
    document: PenDocument,
    connection: ConnectionKey,
    connected: bool,
    inbound: Vec<CollabMessage>,
    outbound: VecDeque<CollabMessage>,
    withheld_applied: Option<CommitSeq>,
}

struct Simulation {
    seed: u64,
    rng: DeterministicRng,
    owner: OwnerSessionCore,
    owner_document: PenDocument,
    guests: Vec<SimGuest>,
    suppress_applied: bool,
    edit_serial: u64,
    stats: Stats,
}

impl Simulation {
    fn new(seed: u64) -> Self {
        let owner_document = initial_document();
        let mut config = OwnerSessionConfig::default();
        config.session_limits.commit_log_entries = 3;
        let mut owner = OwnerSessionCore::new(
            SessionId::from(SESSION),
            Epoch(EPOCH),
            CommitSeq(0),
            connection(1),
            grant("owner", Role::Owner),
            &owner_document,
            config,
        )
        .unwrap();

        let first = owner
            .activate_peer(connection(2), grant("a", Role::Editor), &owner_document)
            .unwrap();
        assert_eq!(first.joined.display_name.as_deref(), Some("Simulation a"));
        assert_eq!(
            first.joined.avatar_url.as_deref(),
            Some("https://profiles.example/a.png")
        );
        let mut guest_a = guest_from_activation(first);
        let second = owner
            .activate_peer(connection(3), grant("b", Role::Editor), &owner_document)
            .unwrap();
        assert_eq!(second.joined.display_name.as_deref(), Some("Simulation b"));
        let joined = second.joined.clone();
        let guest_b = guest_from_activation(second);
        guest_a
            .core
            .accept_frame(frame(CollabMessage::ParticipantJoined(joined)))
            .unwrap();

        Self {
            seed,
            rng: DeterministicRng::new(seed),
            owner,
            owner_document,
            guests: vec![guest_a, guest_b],
            suppress_applied: false,
            edit_serial: 0,
            stats: Stats::default(),
        }
    }

    fn run(mut self) -> Outcome {
        for round in 0..12_u64 {
            if round == 6 {
                self.exercise_reconnect();
            }
            if round % 3 == 0 {
                self.begin_edit(0);
                self.begin_edit(1);
            } else {
                let guest = self.rng.range(GUEST_COUNT);
                self.begin_edit(guest);
            }
            self.drain();
            self.assert_stable();
        }
        self.release_applied();
        self.drain();
        self.assert_converged();
        Outcome {
            hash: self.owner.document_hash(),
            seq: self.owner.seq(),
            stats: self.stats,
        }
    }

    fn begin_edit(&mut self, guest: usize) {
        assert!(self.guests[guest].connected);
        assert!(self.guests[guest].core.document_mutation_allowed());
        self.edit_serial += 1;
        let field = self.rng.range(3);
        let value = self
            .seed
            .saturating_mul(10_000)
            .saturating_add(self.edit_serial);
        let desired = mutate_document(&self.guests[guest].document, field, value);
        let effect = self.guests[guest].core.begin_local_edit(&desired).unwrap();
        self.guests[guest].document = desired;
        self.handle_guest_effects(guest, vec![effect]);
    }

    fn exercise_reconnect(&mut self) {
        self.drain();
        self.assert_stable();
        let disconnected = (self.seed as usize) % GUEST_COUNT;
        let active = 1 - disconnected;
        let old_connection = self.guests[disconnected].connection;
        self.guests[disconnected].core.disconnect();
        self.guests[disconnected].connected = false;
        let effects = self.owner.disconnect(old_connection).unwrap();
        self.handle_owner_effects(effects);
        self.drain();

        let use_retained_log = self.seed.is_multiple_of(2);
        self.suppress_applied = use_retained_log;
        let offline_commits = if use_retained_log { 2 } else { 5 };
        for _ in 0..offline_commits {
            self.begin_edit(active);
            self.drain();
            assert!(self.guests[active].core.pending_edit().is_none());
        }

        let new_connection = connection(100 + self.seed * 4 + disconnected as u64);
        let peer = peer_name(disconnected);
        let activation = self
            .owner
            .resume_peer(new_connection, grant(peer, Role::Editor))
            .unwrap();
        self.guests[disconnected].connection = new_connection;
        self.guests[disconnected].connected = true;
        let effects = self.guests[disconnected]
            .core
            .resume(SessionId::from(SESSION), Epoch(EPOCH), activation.welcome)
            .unwrap();
        self.handle_guest_effects(disconnected, effects);
        for guest in 0..GUEST_COUNT {
            if self.guests[guest].connected {
                self.enqueue_owner_message(
                    guest,
                    CollabMessage::ParticipantJoined(activation.joined.clone()),
                );
            }
        }
        self.stats.reconnects += 1;
        self.drain();
        self.suppress_applied = false;
        self.release_applied();
        self.drain();
        self.assert_stable();
    }

    fn drain(&mut self) {
        const STEP_LIMIT: usize = 50_000;
        for _ in 0..STEP_LIMIT {
            let outbound: Vec<_> = (0..GUEST_COUNT)
                .filter(|guest| {
                    self.guests[*guest].connected && !self.guests[*guest].outbound.is_empty()
                })
                .collect();
            let inbound: Vec<_> = (0..GUEST_COUNT)
                .filter(|guest| {
                    self.guests[*guest].connected && !self.guests[*guest].inbound.is_empty()
                })
                .collect();
            if outbound.is_empty() && inbound.is_empty() {
                return;
            }
            let deliver_outbound =
                !outbound.is_empty() && (inbound.is_empty() || self.rng.range(2) == 0);
            if deliver_outbound {
                let guest = outbound[self.rng.range(outbound.len())];
                let message = self.guests[guest]
                    .outbound
                    .pop_front()
                    .expect("selected outbound queue is non-empty");
                self.deliver_to_owner(guest, message);
            } else {
                let guest = inbound[self.rng.range(inbound.len())];
                let message_index = self.rng.range(self.guests[guest].inbound.len());
                if message_index != 0 {
                    self.stats.reordered_owner_frames += 1;
                }
                let message = self.guests[guest].inbound.remove(message_index);
                self.deliver_to_guest(guest, message);
            }
        }
        panic!("seed {} exceeded deterministic drain bound", self.seed);
    }

    fn deliver_to_owner(&mut self, guest: usize, message: CollabMessage) {
        let effects = self
            .owner
            .accept_frame(
                self.guests[guest].connection,
                frame(message),
                &self.owner_document,
            )
            .unwrap_or_else(|error| panic!("seed {} owner input failed: {error}", self.seed));
        self.handle_owner_effects(effects);
    }

    fn handle_owner_effects(&mut self, effects: Vec<OwnerEffect>) {
        let mut pending: VecDeque<_> = effects.into();
        while let Some(effect) = pending.pop_front() {
            match effect {
                OwnerEffect::PrepareInstall(mut prepared) => {
                    self.owner_document = prepared.take_candidate_document().unwrap();
                    let hash = canonical_document_hash(&self.owner_document).unwrap();
                    let finalized = self.owner.finalize_install(*prepared, hash).unwrap();
                    pending.push_back(finalized);
                }
                OwnerEffect::Broadcast { message } => {
                    for guest in 0..GUEST_COUNT {
                        if self.guests[guest].connected {
                            self.enqueue_owner_message(
                                guest,
                                message
                                    .try_clone_non_sensitive()
                                    .expect("owner broadcasts never carry renewal credentials"),
                            );
                        }
                    }
                }
                OwnerEffect::BroadcastCommit { commit } => {
                    self.stats.commits += 1;
                    for guest in 0..GUEST_COUNT {
                        if self.guests[guest].connected {
                            self.enqueue_owner_message(
                                guest,
                                CollabMessage::Commit(commit.as_ref().clone()),
                            );
                        }
                    }
                }
                OwnerEffect::Reply { to, message } => {
                    if matches!(message, CollabMessage::Reject(_)) {
                        self.stats.rejects += 1;
                    }
                    let guest = self
                        .guest_for_connection(to)
                        .expect("reply targets an active simulated guest");
                    self.enqueue_owner_message(guest, message);
                }
                OwnerEffect::ReplyCommit { to, commit } => {
                    let guest = self
                        .guest_for_connection(to)
                        .expect("commit reply targets an active simulated guest");
                    self.enqueue_owner_message(guest, CollabMessage::Commit((*commit).clone()));
                }
                OwnerEffect::CommitBatch { to, mut commits } => {
                    let guest = self
                        .guest_for_connection(to)
                        .expect("catch-up targets an active simulated guest");
                    if !commits.is_empty() {
                        self.stats.retained_catchups += 1;
                        commits.reverse();
                    }
                    for commit in commits {
                        self.enqueue_owner_message(
                            guest,
                            CollabMessage::Commit(commit.as_ref().clone()),
                        );
                    }
                }
                OwnerEffect::Snapshot { to, snapshot } => {
                    self.stats.snapshots += 1;
                    let guest = self
                        .guest_for_connection(to)
                        .expect("snapshot targets an active simulated guest");
                    self.enqueue_owner_message(guest, CollabMessage::Snapshot(snapshot));
                }
                OwnerEffect::VerifyRenewal { .. }
                | OwnerEffect::UndoRequested(_)
                | OwnerEffect::UndoCommitted { .. }
                | OwnerEffect::Close { .. } => {
                    panic!("unexpected owner effect in deterministic simulation")
                }
            }
        }
    }

    fn enqueue_owner_message(&mut self, guest: usize, message: CollabMessage) {
        self.guests[guest].inbound.push(message);
    }

    fn deliver_to_guest(&mut self, guest: usize, message: CollabMessage) {
        let effects = self.guests[guest]
            .core
            .accept_frame(frame(message))
            .unwrap_or_else(|error| panic!("seed {} guest {guest} failed: {error}", self.seed));
        self.handle_guest_effects(guest, effects);
    }

    fn handle_guest_effects(&mut self, guest: usize, effects: Vec<GuestEffect>) {
        let mut pending: VecDeque<_> = effects.into();
        while let Some(effect) = pending.pop_front() {
            match effect {
                GuestEffect::Send(message) => self.enqueue_peer_message(guest, message),
                GuestEffect::PrepareInstall(mut prepared) => {
                    self.guests[guest].document = prepared
                        .take_candidate_document()
                        .expect("candidate exists");
                    let hash = canonical_document_hash(&self.guests[guest].document).unwrap();
                    let continued = self.guests[guest]
                        .core
                        .finalize_install(*prepared, hash)
                        .unwrap();
                    pending.extend(continued);
                }
                GuestEffect::ParticipantJoined(_)
                | GuestEffect::ParticipantLeft(_)
                | GuestEffect::PresenceChanged(_)
                | GuestEffect::PendingCancelled { .. }
                | GuestEffect::VerifyRenewal { .. } => {}
                GuestEffect::UndoResult(_) | GuestEffect::SessionEnded { .. } => {
                    panic!("unexpected guest effect in deterministic simulation")
                }
            }
        }
    }

    fn enqueue_peer_message(&mut self, guest: usize, message: CollabMessage) {
        if let CollabMessage::Applied(applied) = &message {
            if self.suppress_applied {
                let retained = &mut self.guests[guest].withheld_applied;
                if retained.is_none_or(|through| through < applied.through_seq) {
                    *retained = Some(applied.through_seq);
                }
                return;
            }
        }
        let duplicate = matches!(message, CollabMessage::Submit(_));
        if duplicate {
            self.guests[guest].outbound.push_back(
                message
                    .try_clone_non_sensitive()
                    .expect("only a Submit is duplicated"),
            );
        }
        self.guests[guest].outbound.push_back(message);
        if duplicate {
            self.stats.duplicate_peer_frames += 1;
        }
    }

    fn release_applied(&mut self) {
        for guest in 0..GUEST_COUNT {
            if let Some(through_seq) = self.guests[guest].withheld_applied.take() {
                self.guests[guest]
                    .outbound
                    .push_back(CollabMessage::Applied(Applied { through_seq }));
            }
        }
    }

    fn guest_for_connection(&self, connection: ConnectionKey) -> Option<usize> {
        self.guests
            .iter()
            .position(|guest| guest.connected && guest.connection == connection)
    }

    fn assert_stable(&self) {
        assert!(!self.owner.install_pending(), "seed {}", self.seed);
        for (index, guest) in self.guests.iter().enumerate() {
            if guest.connected {
                assert!(
                    guest.core.pending_edit().is_none(),
                    "seed {} guest {index} retained a pending edit",
                    self.seed
                );
                assert!(
                    !guest.core.install_pending(),
                    "seed {} guest {index} retained an install",
                    self.seed
                );
            }
        }
    }

    fn assert_converged(&self) {
        self.assert_stable();
        let expected = canonical_document_hash(&self.owner_document).unwrap();
        assert_eq!(expected, self.owner.document_hash(), "seed {}", self.seed);
        for (index, guest) in self.guests.iter().enumerate() {
            assert_eq!(
                guest.core.state(),
                GuestConnectionState::Active,
                "seed {} guest {index}",
                self.seed
            );
            assert_eq!(
                guest.core.confirmed_seq(),
                Some(self.owner.seq()),
                "seed {} guest {index}",
                self.seed
            );
            assert_eq!(
                canonical_document_hash(&guest.document).unwrap(),
                expected,
                "seed {} guest {index}",
                self.seed
            );
            assert_eq!(
                guest.core.confirmed_hash(),
                Some(expected),
                "seed {} guest {index}",
                self.seed
            );
        }
    }
}

fn guest_from_activation(activation: op_collab::PeerActivation) -> SimGuest {
    let mut core = GuestSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        activation.welcome,
        GuestSessionConfig::default(),
    )
    .unwrap();
    let snapshot = activation.snapshot.expect("new guest receives snapshot");
    let effects = core
        .accept_frame(frame(CollabMessage::Snapshot(Box::new(snapshot))))
        .unwrap();
    let mut prepared = effects
        .into_iter()
        .find_map(|effect| match effect {
            GuestEffect::PrepareInstall(prepared) => Some(prepared),
            _ => None,
        })
        .expect("initial snapshot prepares install");
    let document = prepared.take_candidate_document().unwrap();
    let hash = canonical_document_hash(&document).unwrap();
    core.finalize_install(*prepared, hash).unwrap();
    SimGuest {
        core,
        document,
        connection: activation.connection,
        connected: true,
        inbound: Vec::new(),
        outbound: VecDeque::new(),
        withheld_applied: None,
    }
}

fn initial_document() -> PenDocument {
    serde_json::from_str(
        r#"{"version":"1.0","children":[{
            "type":"rectangle",
            "id":"base",
            "name":"initial",
            "x":0,
            "y":0,
            "width":20,
            "height":20
        }]}"#,
    )
    .unwrap()
}

fn mutate_document(document: &PenDocument, field: usize, value: u64) -> PenDocument {
    let mut json = serde_json::to_value(document).unwrap();
    match field {
        0 => json["children"][0]["x"] = serde_json::json!(value as f64),
        1 => json["children"][0]["y"] = serde_json::json!(value as f64),
        _ => json["children"][0]["name"] = serde_json::json!(format!("edit-{value}")),
    }
    serde_json::from_value(json).unwrap()
}

fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).unwrap()
}

fn peer_name(index: usize) -> &'static str {
    match index {
        0 => "a",
        1 => "b",
        _ => unreachable!(),
    }
}

fn grant(peer: &str, role: Role) -> AdmissionGrant {
    AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            VerifiedAuthMetadata {
                issuer: "simulation-issuer".into(),
                subject: format!("subject-{peer}"),
                device_id: format!("device-{peer}"),
                proof_binding: format!("binding-{peer}"),
                expires_at_unix_ms: 1_000_000,
                display_name: Some(format!("Simulation {peer}")),
                avatar_url: Some(format!("https://profiles.example/{peer}.png")),
            },
            ParticipantId::from(format!("participant-{peer}")),
            PeerId::from(peer),
            role,
        ),
        PeerNamespace::try_from(format!("{peer}-ns")).unwrap(),
    )
}

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from(SESSION), Epoch(EPOCH), message)
}

#[test]
fn deterministic_multi_seed_protocol_simulation_converges() {
    let mut aggregate = Stats::default();
    for seed in 1..=12_u64 {
        let outcome = Simulation::new(seed).run();
        if seed <= 3 {
            assert_eq!(
                outcome,
                Simulation::new(seed).run(),
                "seed {seed} must be reproducible"
            );
        }
        aggregate.add(outcome.stats);
    }
    assert!(aggregate.commits > 0);
    assert!(aggregate.rejects > 0);
    assert!(aggregate.duplicate_peer_frames > 0);
    assert!(aggregate.reordered_owner_frames > 0);
    assert!(aggregate.retained_catchups > 0);
    assert!(aggregate.snapshots > 0);
    assert_eq!(aggregate.reconnects, 12);
}
