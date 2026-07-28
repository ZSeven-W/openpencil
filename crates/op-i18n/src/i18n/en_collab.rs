//! Collaboration UI strings.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "collab.topbar.collaborate" => "Collaborate",
        "collab.topbar.starting" => "Starting collaboration…",
        "collab.topbar.joining" => "Joining…",
        "collab.topbar.authenticating" => "Authenticating…",
        "collab.topbar.connected" => "Connected",
        "collab.topbar.reconnecting" => "Reconnecting…",
        "collab.topbar.readOnly" => "Read-only",
        "collab.topbar.ended" => "Session ended",
        "collab.topbar.participants" => "{{count}} participants",
        "collab.topbar.unavailable" => "Collaboration is unavailable in this build",
        "collab.action.start" => "Start session",
        "collab.action.join" => "Join session",
        "collab.action.leave" => "Leave session",
        "collab.action.retry" => "Retry",
        "collab.action.cancel" => "Cancel",
        "collab.action.connect" => "Connect",
        "collab.action.discardPending" => "Discard pending edit",
        "collab.action.saveAsFork" => "Save as a fork",
        "collab.action.approveEditor" => "Approve editor",
        "collab.action.approveViewer" => "Approve viewer",
        "collab.action.rejectAdmission" => "Reject",
        "collab.admission.request" => "An authenticated peer is requesting access.",
        "collab.join.title" => "Join a collaboration session",
        "collab.join.discovering" => "Looking for sessions on your local network…",
        "collab.join.noSessions" => "No local sessions found",
        "collab.join.address" => "IP address and port",
        "collab.join.addressPlaceholder" => "192.168.1.8:43120",
        "collab.join.authenticating" => "Verifying the secure session…",
        "collab.join.incompatible" => "This session uses an incompatible version",
        "collab.join.signInRequired" => "Sign in to start or join a session",
        "collab.session.title" => "Collaboration",
        "collab.session.name" => "Session: {{name}}",
        "collab.session.shareAddress" => "Share address",
        "collab.session.role.owner" => "Owner",
        "collab.session.role.editor" => "Editor",
        "collab.session.role.viewer" => "Viewer",
        "collab.session.pending" => "Waiting for the owner to confirm your edit…",
        "collab.status.disconnectedReadOnly" => {
            "Connection lost. Editing is paused while OpenPencil reconnects."
        }
        "collab.status.ticketExpired" => "Your collaboration sign-in expired. Sign in again.",
        "collab.status.ownerLeft" => {
            "The owner left, so this session has ended. You can save a separate copy."
        }
        "collab.status.epochChanged" => {
            "The owner started a new session. Your pending edit was not submitted."
        }
        "collab.status.undoConflict" => {
            "That change cannot be undone because someone edited the same field later."
        }
        "collab.status.unsupportedEdit" => {
            "That edit is not supported in collaboration yet and was not applied."
        }
        "collab.status.profileUnavailable" => "Profile image unavailable; showing initials.",
        "collab.reject.staleBase" => "The document changed first. Catching up before retrying.",
        "collab.reject.readOnly" => "You have view-only access to this session.",
        "collab.reject.unsupported" => "The owner does not support that edit.",
        "collab.reject.conflict" => "That edit conflicts with a newer change.",
        "collab.reject.resourceLimit" => "That edit is too large for this session.",
        "collab.reject.authentication" => "Your collaboration authorization is no longer valid.",
        "collab.reject.unknown" => "The owner rejected that edit.",
        "collab.gate.pages" => "Page changes are not supported in collaboration yet.",
        "collab.gate.pageBackground" => {
            "Page background changes are not supported in collaboration yet."
        }
        "collab.gate.variablesThemes" => {
            "Variables and themes are not supported in collaboration yet."
        }
        "collab.gate.components" => {
            "Component registry changes are not supported in collaboration yet."
        }
        "collab.gate.uikit" => "UIKit changes are not supported in collaboration yet.",
        "collab.gate.externalAssets" => {
            "Images, SVG, HTML, and other external assets cannot be imported in collaboration yet."
        }
        "collab.gate.clipboardPaste" => {
            "Pasting document content is not supported in collaboration yet."
        }
        "collab.gate.duplicate" => "Duplicating nodes is not supported in collaboration yet.",
        "collab.gate.bulkWrite" => "Bulk document changes are disabled during collaboration.",
        "collab.gate.replaceDocument" => {
            "Replacing the whole document is disabled during collaboration."
        }
        "collab.gate.rootMetadata" => {
            "Document metadata changes are not supported in collaboration yet."
        }
        "collab.gate.typography" => {
            "Typography changes are not supported in collaboration yet."
        }
        "collab.gate.effects" => "Effects are not supported in collaboration yet.",
        "collab.gate.visibilityLocking" => {
            "Visibility and locking changes are not supported in collaboration yet."
        }
        "collab.gate.nodeReplacement" => {
            "Replacing a node is not supported in collaboration yet."
        }
        "collab.gate.nodeProperty" => {
            "That node property is not supported in collaboration yet."
        }
        "collab.gate.nodeKind" => "That node type is not supported in collaboration yet.",
        "collab.gate.sessionTransition" => {
            "Editing is paused while the collaboration session is being prepared."
        }
        "collab.gate.readOnly" => "This collaboration session is read-only.",
        "collab.gate.pendingEdit" => {
            "Wait for your pending edit to be confirmed before making another change."
        }
        "collab.gate.aiMcp" => "AI and MCP document writes are disabled during collaboration.",
        "collab.gate.undoUnavailable" => {
            "Global undo is disabled in collaboration. Only confirmed personal changes can be undone."
        }
        "collab.gate.redoUnavailable" => "Redo is not available in collaboration yet.",
        "collab.gate.ownerOnlySave" => "Only the owner can save the shared source file.",
        "collab.gate.leaveSessionFirst" => {
            "Leave the collaboration session before replacing or opening another document."
        }
        "collab.a11y.participant" => "{{name}}, {{role}}",
        "collab.a11y.remoteCursor" => "{{name}}'s cursor",
        _ => return None,
    })
}
