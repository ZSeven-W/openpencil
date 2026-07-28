use anyhow::{bail, Context, Result};
use jian_ops_schema::{node::PenNode, PenDocument};
use op_collab::{
    CanonicalHash, ClientOpId, CollabOp, CollabTxn, CommitSeq, PageRef, PeerId, Submit,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    RetryExactlyOnce,
    StaleRebase,
    AtomicTxnFailure,
    ReconnectCatchUp,
    ReconnectSnapshot,
    EpochChange,
    OwnerLeft,
}

impl Scenario {
    pub const ALL: [Self; 7] = [
        Self::RetryExactlyOnce,
        Self::StaleRebase,
        Self::AtomicTxnFailure,
        Self::ReconnectCatchUp,
        Self::ReconnectSnapshot,
        Self::EpochChange,
        Self::OwnerLeft,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RetryExactlyOnce => "retry-exactly-once",
            Self::StaleRebase => "stale-rebase",
            Self::AtomicTxnFailure => "atomic-txn-failure",
            Self::ReconnectCatchUp => "reconnect-catch-up",
            Self::ReconnectSnapshot => "reconnect-snapshot",
            Self::EpochChange => "epoch-change",
            Self::OwnerLeft => "owner-left",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|scenario| scenario.as_str() == value)
            .ok_or_else(|| anyhow::anyhow!("unknown smoke scenario `{value}`"))
    }
}

pub fn initial_document() -> Result<PenDocument> {
    serde_json::from_str(
        r#"{
            "version": "1.0",
            "children": [{
                "type": "rectangle",
                "id": "base",
                "name": "Fault matrix",
                "x": 0,
                "y": 0,
                "width": 120,
                "height": 80
            }]
        }"#,
    )
    .context("decode fault-matrix document")
}

pub fn replacement_epoch_document() -> Result<PenDocument> {
    serde_json::from_str(
        r#"{
            "version": "1.0",
            "children": [{
                "type": "rectangle",
                "id": "new-epoch-base",
                "name": "Replacement epoch",
                "x": 40,
                "y": 60,
                "width": 180,
                "height": 90
            }]
        }"#,
    )
    .context("decode replacement-epoch document")
}

pub fn with_position(document: &PenDocument, x: f64, y: f64) -> Result<PenDocument> {
    let mut document = document.clone();
    let Some(node) = document.children.first_mut() else {
        bail!("fault-matrix document has no root node");
    };
    let PenNode::Rectangle(rectangle) = node else {
        bail!("fault-matrix root node is not a rectangle");
    };
    rectangle.base.x = Some(x);
    rectangle.base.y = Some(y);
    Ok(document)
}

pub fn with_name(document: &PenDocument, name: &str) -> Result<PenDocument> {
    let mut value = serde_json::to_value(document).context("encode fault-matrix document")?;
    value["children"][0]["name"] = serde_json::json!(name);
    serde_json::from_value(value).context("decode renamed fault-matrix document")
}

pub fn invalid_atomic_submit() -> Result<Submit> {
    let inserted = serde_json::from_value(serde_json::json!({
        "type": "rectangle",
        "id": "c_guest_0",
        "name": "must not survive",
        "x": 1,
        "y": 2,
        "width": 10,
        "height": 10
    }))
    .context("decode atomic-failure inserted node")?;
    Ok(Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from(crate::fixtures::GUEST_PEER),
            local_counter: 1,
        },
        base_seq: CommitSeq(0),
        txn: CollabTxn::new(vec![
            CollabOp::InsertExact {
                page: PageRef::DocumentRoot,
                parent_id: None,
                index: 1,
                node: inserted,
            },
            CollabOp::DeleteExact {
                page: PageRef::DocumentRoot,
                node_id: "missing-after-valid-prefix".to_owned(),
                expected_hash: CanonicalHash::from_bytes([0x55; 32]),
            },
        ]),
    })
}
