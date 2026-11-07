//! Host-shared delivery of Preview effects inside the editor.
//!
//! Actions like `toast`, `alert`, `open_url` and `copy` reach the host
//! as queued [`PreviewEffect`]s. Before this module both editor hosts
//! entered Preview with `PreviewHostCapabilities::none()` and never
//! drained the queue, so an authored "Show toast" or "Open link" did
//! nothing. Each host now declares [`EDITOR_PREVIEW_CAPABILITIES`] and
//! calls [`PreviewSession::deliver_editor_effects`] once per frame:
//! presentation effects land in the editor's own toast slot (painted by
//! both hosts), and the two that need a platform API come back as
//! [`PreviewPlatformRequest`]s for the host's thin arm to perform.

use crate::PreviewSession;
use op_editor_core::editor_toast::EditorToastLevel;
use op_editor_core::EditorUiState;
use op_preview_contracts::{PreviewEffect, PreviewEffectResult, PreviewHostCapabilities};

/// What the editor's Preview can actually do: links, clipboard and
/// toast/alert presentation. Haptics, share sheets and soft keyboards
/// do not exist on the editor hosts, so those stay fail-closed.
pub const EDITOR_PREVIEW_CAPABILITIES: PreviewHostCapabilities = PreviewHostCapabilities {
    open_url: true,
    clipboard: true,
    share: false,
    haptics: false,
    dismiss_keyboard: false,
    notifications: true,
    focus: false,
};

/// i18n key whose template is just `{{message}}` — the authored text is
/// shown verbatim in every locale.
pub const PREVIEW_TOAST_KEY: &str = "preview.toast";

/// A Preview effect the host must hand to a platform API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewPlatformRequest {
    OpenUrl(String),
    CopyText(String),
}

impl PreviewSession {
    /// Drain this frame's effects: present toast/alert in `ui`'s toast
    /// slot, return link/clipboard work for the host, and complete every
    /// effect so the runtime's completion chain keeps moving.
    pub fn deliver_editor_effects(
        &mut self,
        ui: &mut EditorUiState,
        now_ms: u64,
    ) -> Vec<PreviewPlatformRequest> {
        let mut requests = Vec::new();
        // An action list awaits each effect's completion before running
        // the next action, so completing one can queue another. Pump and
        // drain again (bounded) so a whole chain lands in one frame.
        for _ in 0..MAX_CHAIN_ROUNDS {
            let effects = self.drain_effects();
            if effects.is_empty() {
                break;
            }
            self.deliver_batch(effects, ui, now_ms, &mut requests);
            let _ = self.pump(now_ms);
        }
        requests
    }

    fn deliver_batch(
        &self,
        effects: Vec<PreviewEffect>,
        ui: &mut EditorUiState,
        now_ms: u64,
        requests: &mut Vec<PreviewPlatformRequest>,
    ) {
        for effect in effects {
            let result = match &effect {
                PreviewEffect::Toast { message, .. } => {
                    show(ui, message.clone(), EditorToastLevel::Info, now_ms);
                    PreviewEffectResult::Success
                }
                PreviewEffect::Alert { title, message, .. } => {
                    let text = match (title.trim().is_empty(), message.trim().is_empty()) {
                        (true, _) => message.clone(),
                        (false, true) => title.clone(),
                        (false, false) => format!("{title} — {message}"),
                    };
                    show(ui, text, EditorToastLevel::Info, now_ms);
                    PreviewEffectResult::Success
                }
                PreviewEffect::OpenUrl { url, .. } => {
                    requests.push(PreviewPlatformRequest::OpenUrl(url.clone()));
                    PreviewEffectResult::Success
                }
                PreviewEffect::Copy { text, .. } => {
                    requests.push(PreviewPlatformRequest::CopyText(text.clone()));
                    PreviewEffectResult::Success
                }
                // A two-branch confirm needs a modal the editor does not
                // have; report it honestly so the runtime takes no branch.
                _ => PreviewEffectResult::Unsupported,
            };
            let _ = self.complete_effect(effect.id(), result);
        }
    }
}

/// Widen the runtime's document-declared capability gate with what the
/// editor host itself backs.
///
/// A published Jian app must declare `app.capabilities` to copy or open
/// links; a design being previewed in the editor has no such block (the
/// panel never writes one), so `copy` / `open_url` / `toast` were denied
/// and aborted the rest of the action list. In Preview the safety net is
/// elsewhere — the fixed Preview action allowlist rejects `fetch` /
/// sockets / storage wipes, and every granted effect still goes through
/// the host effect queue's activation + URL-scheme checks — so granting
/// exactly the host-declared capabilities is safe.
pub(crate) fn grant_host_capabilities(
    runtime: &mut jian_core::Runtime,
    host: &PreviewHostCapabilities,
) {
    use jian_core::capability::{from_schema_capability, Capability, DeclaredCapabilityGate};
    let mut declared: Vec<Capability> = runtime
        .document
        .as_ref()
        .and_then(|d| d.schema.app.as_ref())
        .and_then(|a| a.capabilities.as_ref())
        .map(|list| list.iter().copied().map(from_schema_capability).collect())
        .unwrap_or_default();
    let granted = [
        (host.open_url, Capability::Network),
        (host.clipboard, Capability::Clipboard),
        (host.notifications, Capability::Notifications),
        (host.haptics, Capability::Haptic),
    ];
    let before = declared.len();
    for (on, capability) in granted {
        if on && !declared.contains(&capability) {
            declared.push(capability);
        }
    }
    if declared.len() != before {
        runtime.capabilities =
            std::rc::Rc::new(DeclaredCapabilityGate::new(declared, runtime.audit.clone()));
    }
}

/// Upper bound on complete → pump → drain rounds per frame, so a
/// pathological loop of effects cannot stall the host.
const MAX_CHAIN_ROUNDS: usize = 16;

fn show(ui: &mut EditorUiState, message: String, level: EditorToastLevel, now_ms: u64) {
    ui.show_toast(
        PREVIEW_TOAST_KEY,
        vec![("message".to_string(), message)],
        level,
        now_ms,
    );
}

#[cfg(all(test, not(target_os = "windows")))]
#[path = "effect_delivery_tests.rs"]
mod tests;
