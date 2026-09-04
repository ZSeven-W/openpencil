//! Guest-side "who am I joining?" presentation model.
//!
//! Unlike every other collaboration model, this one deliberately carries the
//! peer's account subject and device id: it exists precisely so a human can
//! read them before any session data is accepted. The split into
//! `authoritative` rows and a separate `claimed_name` field is the security
//! contract, not styling. Subject and device id come from the verified ticket
//! and the issuer controls them; the profile name and avatar are chosen by the
//! peer's own account and can say anything, so they never occupy an
//! authoritative row and are always rendered under their own "claimed" label.

use super::*;

/// One labelled identity row shown above the confirm/decline decision.
#[derive(Clone, PartialEq, Eq)]
pub struct CollabOwnerIdentityRow {
    pub label: String,
    pub value: String,
}

impl std::fmt::Debug for CollabOwnerIdentityRow {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CollabOwnerIdentityRow")
            .field("label", &self.label)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

/// Everything the guest is shown before it may accept the session.
#[derive(Clone, PartialEq, Eq)]
pub struct CollabOwnerConfirmModel {
    pub request_key: CollabAdmissionRequestKey,
    pub title: String,
    pub hint: String,
    /// Issuer-verified identity. This is what the decision is about.
    pub authoritative: Vec<CollabOwnerIdentityRow>,
    /// Peer-chosen profile name, already labelled as a claim. `None` when the
    /// peer set none or set one that survived sanitization as empty.
    pub claimed_name: Option<CollabOwnerIdentityRow>,
    /// Peer-chosen avatar. Carried for hosts that render one; it is never a
    /// substitute for an authoritative row.
    pub claimed_avatar_url: Option<String>,
    pub actions: Vec<CollabPanelActionModel>,
}

impl std::fmt::Debug for CollabOwnerConfirmModel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CollabOwnerConfirmModel")
            .field("request_key", &self.request_key)
            .field("title", &self.title)
            .field("hint", &self.hint)
            .field("authoritative", &self.authoritative)
            .field("claimed_name", &self.claimed_name)
            .field(
                "claimed_avatar_url",
                &self.claimed_avatar_url.as_ref().map(|_| "[REDACTED]"),
            )
            .field("actions", &self.actions)
            .finish()
    }
}

/// Build the confirmation model, or `None` when no decision is pending.
pub(super) fn owner_confirm_model(ui: &EditorUiState) -> Option<CollabOwnerConfirmModel> {
    let pending = ui.collab.pending_owner_confirmation()?;
    let identity = pending.identity();
    let request_key = pending.request_key().clone();
    Some(CollabOwnerConfirmModel {
        title: op_i18n::translate(ui.effective_locale(), "collab.ownerConfirm.title").to_string(),
        hint: op_i18n::translate(ui.effective_locale(), "collab.ownerConfirm.hint").to_string(),
        authoritative: vec![
            CollabOwnerIdentityRow {
                label: op_i18n::translate(ui.effective_locale(), "collab.ownerConfirm.account")
                    .to_string(),
                value: identity.subject().to_string(),
            },
            CollabOwnerIdentityRow {
                label: op_i18n::translate(ui.effective_locale(), "collab.ownerConfirm.device")
                    .to_string(),
                value: identity.device_id().to_string(),
            },
        ],
        claimed_name: identity
            .claimed_display_name()
            .map(|name| CollabOwnerIdentityRow {
                label: op_i18n::translate(ui.effective_locale(), "collab.ownerConfirm.claimedName")
                    .to_string(),
                value: name.to_string(),
            }),
        claimed_avatar_url: identity.claimed_avatar_url().map(str::to_string),
        actions: vec![
            action_model(
                ui,
                CollabUiAction::ConfirmOwnerIdentity {
                    request_key: request_key.clone(),
                },
                true,
            ),
            action_model(
                ui,
                CollabUiAction::RejectOwnerIdentity {
                    request_key: request_key.clone(),
                },
                false,
            ),
        ],
        request_key,
    })
}
