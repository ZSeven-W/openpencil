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
        "collab.action.start" => "Create session",
        "collab.action.join" => "Join session",
        "collab.home.hint" => "Create a new session or join with an invite.",
        "collab.create.choose" => "Choose how other people can connect.",
        "collab.action.leave" => "Leave session",
        "collab.action.retry" => "Retry",
        "collab.action.cancel" => "Cancel",
        "collab.action.connect" => "Connect",
        "collab.action.copyInviteCode" => "Copy invite code",
        "collab.action.copyInviteLink" => "Copy invite link",
        "collab.action.findNearby" => "Find nearby",
        "collab.join.code" => "Invite code or IP address",
        "collab.join.codePlaceholder" => "A2C4E6G8J0 or 192.168.1.8:43120",
        "collab.join.publicHint" => "Invite codes connect securely over the internet.",
        "collab.join.nearby" => "Nearby sessions",
        "collab.session.invite" => "Public invite code",
        "collab.session.preparingInvite" => "Preparing a secure invite…",
        "collab.session.publicReady" => "Public relay is ready",
        "collab.session.region" => "Home relay region",
        "collab.connection.lan" => "Local network",
        "collab.connection.relay" => "Public relay",
        "collab.region.china" => "China",
        "collab.region.global" => "Global",
        "collab.error.inviteUnavailable" => "This invite is unavailable or has expired.",
        "collab.error.relayUnavailable" => "The public relay is temporarily unavailable.",
        "collab.error.inviteInvalid" => "This invite code is invalid. Check it and try again.",
        "collab.error.inviteExpired" => "This invite code has expired. Ask the owner for a new one.",
        "collab.error.relayNotConfigured" => "No public relay is configured on this device.",
        "collab.error.regionUnavailable" => "The invite's home relay region is unavailable.",
        "collab.error.secureKeyUnavailable" => "The device's secure key is unavailable. Check system keychain access and try again.",
        "collab.error.rateLimited" => "Too many connection attempts. Try again shortly.",
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
        "collab.reject.conflictDetail" => "Discarded: {{fields}} on “{{node}}”.",
        "collab.action.reapply" => "Reapply my edit",
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
        "collab.status.localEditPreserved" => {
            "The remote version was applied. Use Undo to restore your local changes."
        }
        "collab.a11y.participant" => "{{name}}, {{role}}",
        "collab.a11y.remoteCursor" => "{{name}}'s cursor",
        "icon.catalogLoading" => "Icon catalog is still loading…",
        "sceneTemplate.documentUnavailable" => "That template's document could not be loaded. Try again.",
        "sceneTemplate.deleteFailed" => "That template could not be deleted. Try again.",
        "sceneTemplate.item.soundingNavyDeck.title" => "Sounding Chart · Strategy Deck",
        "sceneTemplate.item.soundingNavyDeck.summary" => "Chart-paper white against steel and ink blue: every slide leads with a conclusion and backs it with one sourced reading. Depth-profile bars and a sounding-track agenda carry all seven pages, for consulting deliverables and strategy reviews.",
        "sceneTemplate.item.tidemarkSlateDeck.title" => "Tidemark Slate · Data Review Deck",
        "sceneTemplate.item.tidemarkSlateDeck.summary" => "Overview tiles, a trend line, a detail table, a risk list and a swimlane roadmap on slate grey — seven pages that walk one full data review, for quarterly retrospectives and metric meetings.",
        "sceneTemplate.item.banxinRuleDeck.title" => "Banxin Rule · Chinese Typography Deck",
        "sceneTemplate.item.banxinRuleDeck.summary" => "Built on the classical Chinese page: type block, column rules, marginal notes and a fishtail folio. Body lines stay under thirty characters, with facing columns and a quotation page that reward reading, for lectures and seminars.",
        "sceneTemplate.item.gridpaperGraphiteDeck.title" => "Graphite Grid · Academic Defence Deck",
        "sceneTemplate.item.gridpaperGraphiteDeck.summary" => "Graphite type on grid paper: problem and gap, method, two pages of results, limitations, conclusion and references — eight pages in defence order, for proposals and thesis defences.",
        "sceneTemplate.item.dossierLinenDeck.title" => "Linen Dossier · Memo Deck",
        "sceneTemplate.item.dossierLinenDeck.summary" => "A linen-paper dossier from cover sheet through background, current data, analysis and option comparison to the resolution — eight pages that read as a standalone memo, for decision reviews.",
        "sceneTemplate.item.ledgerTickDeck.title" => "Ledger Tick · Competitive Matrix Deck",
        "sceneTemplate.item.ledgerTickDeck.summary" => "Scoring criteria, the main matrix, quantile scales and a gap-versus-strength read-out on ledger ruling — seven pages that tell a competitive comparison like a balanced account, for vendor selection and market analysis.",
        "sceneTemplate.item.brandConceptSheet.title" => "Brand Concept Sheet",
        "sceneTemplate.item.brandConceptSheet.summary" => "One horizontal review board for the primary lockup, construction rationale, monochrome reversals and minimum-size survival — built for first-round concept handoff.",
        "sceneTemplate.item.logoQaBoard.title" => "Logo Fusion QA Board",
        "sceneTemplate.item.logoQaBoard.summary" => "Four review cells check shared semantic load, structural dependence, silhouette unity and whether the secondary reading feels discovered rather than attached.",
        "account.mcpToken" => "MCP Tokens",
        "menu.saveAsTemplate" => "Save As Template…",
        "menu.saveAsTemplate.saved" => "Saved as a template",
        "menu.saveAsTemplate.failed" => "Could not save the template",
        "assetCenter.template.mine" => "My templates",
        "assetCenter.template.builtIn" => "Built-in templates",
        "ai.designProgress.detail.failureReason" => "Reason: {{reason}}",
        "ai.designProgress.detail.noDiagnostic" => "The agent failed without returning an error description.",
        "ai.designProgress.detail.noResult" => "The agent stopped before returning a result for this section.",
        "ai.designProgress.detail.connectionClosed" => "The agent connection closed before this section returned a result.",
        "ai.designProgress.detail.stoppedByUser" => "Stopped by the user before this section completed.",
        _ => return None,
    })
}
