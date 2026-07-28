use jian_ops_schema::node::PenNode;
use op_collab::{
    Applied, ClientOpId, CollabMessage, CollabOp, CollabTxn, CommitSeq, Epoch, FrameEnvelope,
    PageRef, Participant, ParticipantId, PeerId, Role, SessionId, Submit,
};

const SEEDS: [u64; 16] = [
    1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1_597,
];

#[test]
fn deterministic_codec_mutations_never_escape_validation() {
    let corpus = corpus();
    for seed in SEEDS {
        let mut random = XorShift64::new(seed);
        for original in &corpus {
            for _ in 0..96 {
                let mutated = mutate(original, &mut random);
                assert_valid_result_is_canonical(&mutated);
            }
        }

        for _ in 0..64 {
            let length = random.usize(4_096);
            let mut arbitrary = vec![0_u8; length];
            for byte in &mut arbitrary {
                *byte = random.next() as u8;
            }
            assert_valid_result_is_canonical(&arbitrary);
        }
    }
}

#[test]
fn duplicate_fields_and_recursive_transaction_shapes_fail_closed() {
    let duplicate_version = br#"{
        "protocolVersion":1,
        "protocolVersion":1,
        "sessionId":"property-session",
        "epoch":1,
        "body":{"type":"applied","throughSeq":0}
    }"#;
    assert!(FrameEnvelope::from_json_slice(duplicate_version).is_err());

    let mut nested =
        br#"{"protocolVersion":1,"sessionId":"property-session","epoch":1,"body":"#.to_vec();
    for _ in 0..256 {
        nested.extend_from_slice(br#"{"type":"submit","clientOpId":"#);
    }
    nested.extend_from_slice(br#"null"#);
    nested.extend(std::iter::repeat_n(b'}', 256));
    nested.push(b'}');
    assert!(FrameEnvelope::from_json_slice(&nested).is_err());
}

fn corpus() -> Vec<Vec<u8>> {
    let applied = frame(CollabMessage::Applied(Applied {
        through_seq: CommitSeq(7),
    }));
    let node: PenNode =
        serde_json::from_str(r#"{"type":"rectangle","id":"c_property_0","name":"Box"}"#).unwrap();
    let submit = frame(CollabMessage::Submit(Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from("property-peer"),
            local_counter: 1,
        },
        base_seq: CommitSeq(0),
        txn: CollabTxn::new(vec![CollabOp::InsertExact {
            page: PageRef::DocumentRoot,
            parent_id: None,
            index: 0,
            node,
        }]),
    }));
    let participant = frame(CollabMessage::ParticipantJoined(Participant {
        participant_id: ParticipantId::from("participant-property"),
        peer_id: PeerId::from("property-peer"),
        role: Role::Editor,
        display_name: Some("Property 资料".into()),
        avatar_url: Some("https://profiles.example/property.png?size=80".into()),
    }));
    vec![
        applied.to_json_vec().unwrap(),
        submit.to_json_vec().unwrap(),
        participant.to_json_vec().unwrap(),
    ]
}

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from("property-session"), Epoch(1), message)
}

fn mutate(original: &[u8], random: &mut XorShift64) -> Vec<u8> {
    let mut bytes = original.to_vec();
    match random.usize(4) {
        0 if !bytes.is_empty() => {
            let index = random.usize(bytes.len());
            bytes[index] = random.next() as u8;
        }
        1 if !bytes.is_empty() => {
            let index = random.usize(bytes.len());
            bytes.remove(index);
        }
        2 => {
            let index = random.usize(bytes.len().saturating_add(1));
            bytes.insert(index, random.next() as u8);
        }
        _ if !bytes.is_empty() => {
            let start = random.usize(bytes.len());
            let remaining = bytes.len() - start;
            let length = random.usize(remaining.min(32).saturating_add(1));
            let duplicate = bytes[start..start + length].to_vec();
            bytes.splice(start..start, duplicate);
        }
        _ => bytes.push(random.next() as u8),
    }
    bytes
}

fn assert_valid_result_is_canonical(bytes: &[u8]) {
    let Ok(decoded) = FrameEnvelope::from_json_slice(bytes) else {
        return;
    };
    let encoded = decoded
        .to_json_vec()
        .expect("a validated frame must encode within default limits");
    let round_trip = FrameEnvelope::from_json_slice(&encoded)
        .expect("a validated frame's canonical encoding must decode");
    assert_eq!(round_trip, decoded);
}

struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn usize(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive == 0 {
            return 0;
        }
        (self.next() as usize) % upper_exclusive
    }
}
