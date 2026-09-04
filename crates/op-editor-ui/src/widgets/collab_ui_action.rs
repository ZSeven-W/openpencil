//! Localized collaboration action presentation.

use super::*;

pub(super) fn action_model(
    ui: &EditorUiState,
    action: CollabUiAction,
    primary: bool,
) -> CollabPanelActionModel {
    let key = match action {
        CollabUiAction::OpenCreate => "collab.action.start",
        CollabUiAction::Start => "collab.connection.relay",
        CollabUiAction::StartLan => "collab.connection.lan",
        CollabUiAction::SetRelayRegion { region } => region.i18n_key(),
        CollabUiAction::OpenJoin => "collab.action.join",
        CollabUiAction::BeginDiscovery => "collab.action.findNearby",
        CollabUiAction::JoinDiscovered { .. } | CollabUiAction::JoinAddress { .. } => {
            "collab.action.connect"
        }
        CollabUiAction::Cancel => "collab.action.cancel",
        CollabUiAction::Retry => "collab.action.retry",
        CollabUiAction::Leave => "collab.action.leave",
        CollabUiAction::DiscardPending => "collab.action.discardPending",
        CollabUiAction::ReapplyDiscarded => "collab.action.reapply",
        CollabUiAction::SaveAsFork => "collab.action.saveAsFork",
        CollabUiAction::ApproveAdmissionEditor { .. } => "collab.action.approveEditor",
        CollabUiAction::ApproveAdmissionViewer { .. } => "collab.action.approveViewer",
        CollabUiAction::RejectAdmission { .. } => "collab.action.rejectAdmission",
        CollabUiAction::ConfirmOwnerIdentity { .. } => "collab.action.confirmOwner",
        CollabUiAction::RejectOwnerIdentity { .. } => "collab.action.rejectOwner",
    };
    CollabPanelActionModel {
        action,
        label: op_i18n::translate(ui.effective_locale(), key).to_string(),
        primary,
    }
}
