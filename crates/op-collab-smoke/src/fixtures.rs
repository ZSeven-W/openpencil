use anyhow::{Context, Result};
use jian_ops_schema::PenDocument;
use op_collab::{
    AdmissionGrant, ConnectionPrincipal, Epoch, ParticipantId, PeerId, PeerNamespace, Role,
    SessionId, VerifiedAuthMetadata,
};

pub const DISCOVERY_ID: &str = "00112233445566778899aabbccddeeff";
pub const SESSION_ID: &str = "smoke-session";
pub const EPOCH: Epoch = Epoch(1);
pub const OWNER_PEER: &str = "smoke-owner";
pub const OWNER_PARTICIPANT: &str = "smoke-participant-owner";
pub const OWNER_NAMESPACE: &str = "owner";
pub const GUEST_PEER: &str = "smoke-guest";
pub const GUEST_PARTICIPANT: &str = "smoke-participant-guest";
pub const GUEST_NAMESPACE: &str = "guest";

pub fn session_id() -> SessionId {
    SessionId::from(SESSION_ID)
}

pub fn initial_document() -> Result<PenDocument> {
    serde_json::from_str(r#"{"version":"1.0","children":[]}"#).context("decode smoke document")
}

pub fn desired_guest_document(namespace: &str) -> Result<PenDocument> {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "rectangle",
            "id": format!("c_{namespace}_0"),
            "name": "P2P smoke",
            "x": 16,
            "y": 24,
            "width": 120,
            "height": 80,
            "fills": [{
                "type": "solid",
                "color": "#3366FF"
            }]
        }]
    }))
    .context("build smoke edit")
}

pub fn desired_owner_document(document: &PenDocument) -> Result<PenDocument> {
    let mut value = serde_json::to_value(document).context("encode owner smoke edit")?;
    value["children"][0]["name"] = serde_json::json!("Owner confirmed");
    value["children"][0]["x"] = serde_json::json!(64);
    serde_json::from_value(value).context("build owner smoke edit")
}

pub fn desired_guest_followup(document: &PenDocument) -> Result<PenDocument> {
    let mut value = serde_json::to_value(document).context("encode guest follow-up edit")?;
    value["children"][0]["y"] = serde_json::json!(48);
    value["children"][0]["opacity"] = serde_json::json!(0.75);
    serde_json::from_value(value).context("build guest follow-up edit")
}

pub fn expected_alternating_document() -> Result<PenDocument> {
    let guest = desired_guest_document(GUEST_NAMESPACE)?;
    let owner = desired_owner_document(&guest)?;
    desired_guest_followup(&owner)
}

pub fn grant(
    auth: VerifiedAuthMetadata,
    role: Role,
    participant: &str,
    peer: &str,
    namespace: &str,
) -> Result<AdmissionGrant> {
    Ok(AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            auth,
            ParticipantId::from(participant),
            PeerId::from(peer),
            role,
        ),
        PeerNamespace::try_from(namespace)?,
    ))
}
