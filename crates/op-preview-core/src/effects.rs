//! R3 Preview effect queue — the bounded FIFO between the jian action
//! runtime and the host.
//!
//! Effect-producing actions hand their requests to the engine's
//! platform-neutral `EffectSink`; this module adapts that sink onto the
//! frozen `op-preview-contracts` DTOs: every accepted request becomes a
//! `PreviewEffect` with a monotonically increasing id, its factual
//! [`EffectSource`], and the capability class the host declared. The
//! host drains (`PreviewSession::drain_effects`), performs, and
//! completes (`PreviewSession::complete_effect`) — exactly once per
//! effect.
//!
//! Fail-closed rules enforced at ENQUEUE time:
//! - a capability the host did not declare → `Unsupported` (never
//!   queued, never silently allowed);
//! - an `open_url` target whose scheme is not `http`/`https`/`mailto`/
//!   `tel` → rejected with a structured diagnostic;
//! - a full queue → rejected (bounded memory, no unbounded host work).

use jian_core::action::context::EffectRequestContext;
use jian_core::action::services::effect_sink::{EffectOutcome, EffectRequest, EffectSink};
use op_preview_contracts::{
    EffectSource, HapticStyle, PreviewCapability, PreviewEffect, PreviewEffectResult,
    PreviewHostCapabilities, SharePayload, UserActivationId,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

/// Default queue capacity: generous for one interaction burst, bounded
/// so a misbehaving document cannot queue host work without end.
const DEFAULT_CAPACITY: usize = 64;
/// Bounded diagnostics: the newest rejection reasons, never unbounded.
const DIAGNOSTIC_CAPACITY: usize = 32;

struct QueueInner {
    capacity: usize,
    effects: VecDeque<PreviewEffect>,
    next_id: u64,
    total_enqueued: usize,
    completed: std::collections::HashMap<u64, PreviewEffectResult>,
    diagnostics: VecDeque<String>,
}

/// The queue shared between the session and its sink adapter.
#[derive(Clone)]
pub struct PreviewEffectQueue {
    inner: Rc<RefCell<QueueInner>>,
}

impl PreviewEffectQueue {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Rc::new(RefCell::new(QueueInner {
                capacity,
                effects: VecDeque::with_capacity(capacity),
                next_id: 1,
                total_enqueued: 0,
                completed: std::collections::HashMap::new(),
                diagnostics: VecDeque::new(),
            })),
        }
    }

    fn allocate_id(&self) -> u64 {
        let id = self.inner.borrow().next_id;
        self.inner.borrow_mut().next_id = id.wrapping_add(1).max(1);
        id
    }

    fn enqueue(&self, effect: PreviewEffect) -> bool {
        let mut inner = self.inner.borrow_mut();
        // Bounded FIFO: a full queue rejects instead of growing without
        // end (host work stays bounded by what was drained).
        if inner.effects.len() >= inner.capacity {
            return false;
        }
        inner.total_enqueued += 1;
        inner.effects.push_back(effect);
        true
    }

    fn reject(&self, reason: String) {
        let mut inner = self.inner.borrow_mut();
        if inner.diagnostics.len() >= DIAGNOSTIC_CAPACITY {
            inner.diagnostics.pop_front();
        }
        inner.diagnostics.push_back(reason);
    }

    /// Total effects ever enqueued (the `effects_enqueued` delta source).
    pub fn total_enqueued(&self) -> usize {
        self.inner.borrow().total_enqueued
    }

    /// Outstanding (drained but not completed) effect count.
    pub fn outstanding(&self) -> usize {
        let inner = self.inner.borrow();
        inner.total_enqueued - inner.completed.len()
    }

    /// Structured rejection diagnostics (bounded, newest last).
    pub fn diagnostics(&self) -> Vec<String> {
        self.inner.borrow().diagnostics.iter().cloned().collect()
    }
}

impl Default for PreviewEffectQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// The engine-facing adapter: maps platform-neutral requests onto
/// `PreviewEffect` DTOs under the host's declared capability set.
struct QueueEffectSink {
    queue: PreviewEffectQueue,
    capabilities: PreviewHostCapabilities,
}

impl QueueEffectSink {
    fn source(&self, ctx: &EffectRequestContext, capability: PreviewCapability) -> EffectSource {
        EffectSource {
            node_id: ctx.node_id.clone().unwrap_or_default(),
            event: ctx.handler.clone().unwrap_or_default(),
            activation: ctx.activation.map(UserActivationId::from_raw),
            required_capability: capability,
        }
    }

    /// The approved URL schemes (R3 Step 4). Anything else is rejected
    /// before it can reach the host.
    fn validate_url(url: &str) -> Result<(), &'static str> {
        let lower = url.to_ascii_lowercase();
        for scheme in ["http://", "https://", "mailto:", "tel:"] {
            if lower.starts_with(scheme) {
                return Ok(());
            }
        }
        Err("invalid url scheme (allowed: http, https, mailto, tel)")
    }
}

impl EffectSink for QueueEffectSink {
    fn request(&self, ctx: &EffectRequestContext, request: &EffectRequest) -> EffectOutcome {
        let (capability, effect): (PreviewCapability, Option<PreviewEffect>) = match request {
            EffectRequest::OpenUrl { url } => {
                if let Err(reason) = Self::validate_url(url) {
                    self.queue.reject(format!("open_url `{url}`: {reason}"));
                    return EffectOutcome::Rejected(reason.to_owned());
                }
                let capability = PreviewCapability::OpenUrl;
                (
                    capability,
                    Some(PreviewEffect::OpenUrl {
                        id: self.queue.allocate_id(),
                        url: url.clone(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::Copy { text } => {
                let capability = PreviewCapability::Clipboard;
                (
                    capability,
                    Some(PreviewEffect::Copy {
                        id: self.queue.allocate_id(),
                        text: text.clone(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::Share { payload } => {
                let capability = PreviewCapability::Share;
                let parsed: SharePayload =
                    serde_json::from_value(payload.clone()).unwrap_or_default();
                (
                    capability,
                    Some(PreviewEffect::Share {
                        id: self.queue.allocate_id(),
                        payload: parsed,
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::Haptic { style } => {
                let capability = PreviewCapability::Haptics;
                (
                    capability,
                    Some(PreviewEffect::Haptic {
                        id: self.queue.allocate_id(),
                        style: HapticStyle::from_authored(style),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::FocusNode { node_id } => {
                let capability = PreviewCapability::Focus;
                (
                    capability,
                    Some(PreviewEffect::FocusNode {
                        id: self.queue.allocate_id(),
                        node_id: node_id.clone(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::BlurFocus => {
                let capability = PreviewCapability::Focus;
                (
                    capability,
                    Some(PreviewEffect::BlurFocus {
                        id: self.queue.allocate_id(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::DismissKeyboard => {
                let capability = PreviewCapability::DismissKeyboard;
                (
                    capability,
                    Some(PreviewEffect::DismissKeyboard {
                        id: self.queue.allocate_id(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::Toast { message } => {
                let capability = PreviewCapability::Notifications;
                (
                    capability,
                    Some(PreviewEffect::Toast {
                        id: self.queue.allocate_id(),
                        message: message.clone(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::Alert { title, message } => {
                let capability = PreviewCapability::Notifications;
                (
                    capability,
                    Some(PreviewEffect::Alert {
                        id: self.queue.allocate_id(),
                        title: title.clone(),
                        message: message.clone(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
            EffectRequest::Confirm { title, message } => {
                let capability = PreviewCapability::Notifications;
                (
                    capability,
                    Some(PreviewEffect::Confirm {
                        id: self.queue.allocate_id(),
                        title: title.clone(),
                        message: message.clone(),
                        source: self.source(ctx, capability),
                    }),
                )
            }
        };
        // `confirm` asks a question and the authored `on_confirm` /
        // `on_cancel` branches are the answer's two destinations. This
        // queue can carry the request to the host but has no way to carry
        // the reply back yet, and `Accepted` would tell the action "it is
        // handled" — which silently drops both branches. Declining is the
        // honest answer: the action falls through to the synchronous
        // feedback service, which does run them. R9's completion-resume
        // mechanism is what lets the queue take this over.
        if matches!(request, EffectRequest::Confirm { .. }) {
            return EffectOutcome::Unsupported;
        }
        // Fail-closed: an undeclared host capability means the effect
        // class is Unsupported — nothing is queued, nothing is allowed.
        if !self.capabilities.supports(capability) {
            return EffectOutcome::Unsupported;
        }
        let Some(effect) = effect else {
            return EffectOutcome::Unsupported;
        };
        if self.queue.enqueue(effect) {
            EffectOutcome::Accepted
        } else {
            self.queue.reject("effect queue full".to_owned());
            EffectOutcome::Rejected("effect queue full".to_owned())
        }
    }
}

impl PreviewEffectQueue {
    /// Drain every queued effect in FIFO order (the host's per-frame
    /// pull). Drained effects are still completed via
    /// [`PreviewSession::complete_effect`].
    pub fn drain(&self) -> Vec<PreviewEffect> {
        std::mem::take(&mut self.inner.borrow_mut().effects)
            .into_iter()
            .collect()
    }

    /// Complete one effect EXACTLY ONCE. A second completion for the
    /// same id is rejected with a diagnostic — a host double-completing
    /// is a bug, and resuming a continuation twice would double-run
    /// authored actions.
    pub fn complete(&self, id: u64, result: PreviewEffectResult) -> Result<(), String> {
        let mut inner = self.inner.borrow_mut();
        if inner.completed.contains_key(&id) {
            let reason = format!("effect {id} completed more than once");
            if inner.diagnostics.len() >= DIAGNOSTIC_CAPACITY {
                inner.diagnostics.pop_front();
            }
            inner.diagnostics.push_back(reason.clone());
            return Err(reason);
        }
        inner.completed.insert(id, result);
        Ok(())
    }

    /// The recorded completion result for `id`, when completed.
    pub fn result_of(&self, id: u64) -> Option<PreviewEffectResult> {
        self.inner.borrow().completed.get(&id).cloned()
    }
}

/// Install the queue + policy pair on a preview runtime: the engine's
/// sink becomes the queue adapter and the fixed Preview allowlist
/// becomes the action policy.
pub(crate) fn install_on_runtime(
    runtime: &mut jian_core::Runtime,
    queue: &PreviewEffectQueue,
    capabilities: &PreviewHostCapabilities,
) {
    runtime.set_effect_sink(Rc::new(QueueEffectSink {
        queue: queue.clone(),
        capabilities: *capabilities,
    }));
    runtime.set_policy(Some(Rc::new(
        jian_core::action::policy::PreviewActionPolicy::policy(),
    )));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bounded FIFO never hands a completed effect out twice and
    /// rejects double completion with a diagnostic.
    #[test]
    fn complete_is_exactly_once() {
        let queue = PreviewEffectQueue::new();
        queue.enqueue(PreviewEffect::BlurFocus {
            id: queue.allocate_id(),
            source: EffectSource {
                node_id: "n".to_owned(),
                event: "onTap".to_owned(),
                activation: None,
                required_capability: PreviewCapability::Focus,
            },
        });
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert!(queue
            .complete(drained[0].id(), PreviewEffectResult::Success)
            .is_ok());
        assert!(queue
            .complete(drained[0].id(), PreviewEffectResult::Success)
            .is_err());
        assert_eq!(
            queue.result_of(drained[0].id()),
            Some(PreviewEffectResult::Success)
        );
    }
}
