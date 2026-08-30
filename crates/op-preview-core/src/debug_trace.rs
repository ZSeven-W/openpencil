//! Bounded Preview trace, action provenance, and debugger clock controls.

use jian_core::action::context::ActionContext;
use jian_core::action::error::ActionResult;
use jian_core::action::services::ActionObserver;
use op_preview_contracts::{
    PreviewDebugSnapshot, PreviewDiagnostic, PreviewQueueCounts, PreviewRunState,
    PreviewStateProvenance, PreviewStateRow, PreviewStateScope, PreviewTraceEntry,
    PreviewTraceKind,
};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

const TRACE_CAPACITY: usize = 256;
const DIAGNOSTIC_CAPACITY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ActualScope {
    App,
    Page,
    SelfNode,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StateKey {
    scope: ActualScope,
    owner: Option<String>,
    key: String,
}

#[derive(Default)]
struct StateSnapshot {
    values: BTreeMap<StateKey, Value>,
}

impl StateSnapshot {
    fn capture(context: &ActionContext) -> Self {
        let mut values = BTreeMap::new();
        for (key, value) in context.state.app_snapshot() {
            values.insert(
                StateKey {
                    scope: ActualScope::App,
                    owner: None,
                    key,
                },
                value,
            );
        }
        for page in context.state.page_keys() {
            for (key, value) in context.state.page_snapshot(&page) {
                values.insert(
                    StateKey {
                        scope: ActualScope::Page,
                        owner: Some(page.clone()),
                        key,
                    },
                    value,
                );
            }
        }
        for (page, node) in context.state.self_keys() {
            for (key, value) in context.state.self_snapshot(&page, &node) {
                values.insert(
                    StateKey {
                        scope: ActualScope::SelfNode,
                        owner: Some(format!("{page}/{node}")),
                        key,
                    },
                    value,
                );
            }
        }
        Self { values }
    }
}

struct ActionFrame {
    action: &'static str,
    before: StateSnapshot,
    route_before: jian_core::action::services::RouteState,
    node_id: Option<String>,
    event: Option<String>,
}

struct PendingTrace {
    at_ms: u64,
    kind: PreviewTraceKind,
    name: String,
    node_id: Option<String>,
    event: Option<String>,
    action: Option<String>,
    detail: Value,
    provenance: Vec<StateKey>,
}

#[derive(Default)]
struct TraceInner {
    entries: VecDeque<PreviewTraceEntry>,
    diagnostics: VecDeque<PreviewDiagnostic>,
    provenance: BTreeMap<StateKey, PreviewStateProvenance>,
    action_frames: BTreeMap<u64, ActionFrame>,
    semantic_payloads: BTreeMap<String, Value>,
    pending: Vec<PendingTrace>,
    buffering_input: bool,
    next_sequence: u64,
    next_action_token: u64,
}

#[derive(Clone, Default)]
pub(crate) struct PreviewDebugTrace {
    inner: Rc<RefCell<TraceInner>>,
}

impl PreviewDebugTrace {
    pub(crate) fn begin_input(&self, name: &str, at_ms: u64, detail: Value) {
        let mut inner = self.inner.borrow_mut();
        inner.buffering_input = true;
        inner.pending.clear();
        push_direct(
            &mut inner,
            PendingTrace {
                at_ms,
                kind: PreviewTraceKind::Input,
                name: name.to_owned(),
                node_id: None,
                event: None,
                action: None,
                detail: redact_value(None, detail),
                provenance: Vec::new(),
            },
        );
    }

    pub(crate) fn record_semantics(&self, handlers: &[&'static str], at_ms: u64) {
        let mut inner = self.inner.borrow_mut();
        for handler in handlers {
            let detail = inner
                .semantic_payloads
                .remove(*handler)
                .unwrap_or_else(|| serde_json::json!({ "handler": handler }));
            push_direct(
                &mut inner,
                PendingTrace {
                    at_ms,
                    kind: PreviewTraceKind::SemanticEvent,
                    name: (*handler).to_owned(),
                    node_id: None,
                    event: Some((*handler).to_owned()),
                    action: None,
                    detail,
                    provenance: Vec::new(),
                },
            );
        }
    }

    pub(crate) fn finish_input(&self) {
        let mut inner = self.inner.borrow_mut();
        let pending = std::mem::take(&mut inner.pending);
        inner.buffering_input = false;
        for entry in pending {
            push_direct(&mut inner, entry);
        }
    }

    pub(crate) fn record_effect(
        &self,
        name: &str,
        node_id: Option<String>,
        event: Option<String>,
        at_ms: u64,
    ) {
        self.record(PendingTrace {
            at_ms,
            kind: PreviewTraceKind::Effect,
            name: name.to_owned(),
            node_id,
            event,
            action: Some(name.to_owned()),
            detail: serde_json::json!({ "payload": "<redacted>" }),
            provenance: Vec::new(),
        });
    }

    pub(crate) fn record_effect_result(&self, id: u64, result: &str, at_ms: u64) {
        self.record(PendingTrace {
            at_ms,
            kind: PreviewTraceKind::EffectResult,
            name: result.to_owned(),
            node_id: None,
            event: None,
            action: None,
            detail: serde_json::json!({ "effectId": id, "result": result }),
            provenance: Vec::new(),
        });
    }

    pub(crate) fn record_animation(&self, target: &str, property: &str, at_ms: u64, phase: &str) {
        self.record(PendingTrace {
            at_ms,
            kind: PreviewTraceKind::Animation,
            name: property.to_owned(),
            node_id: Some(target.to_owned()),
            event: None,
            action: Some("animate".to_owned()),
            detail: serde_json::json!({ "phase": phase }),
            provenance: Vec::new(),
        });
    }

    pub(crate) fn record_route(&self, path: &str, stack: &[String], at_ms: u64) {
        self.record(PendingTrace {
            at_ms,
            kind: PreviewTraceKind::Route,
            name: path.to_owned(),
            node_id: None,
            event: None,
            action: None,
            detail: serde_json::json!({ "path": path, "stack": stack }),
            provenance: Vec::new(),
        });
    }

    pub(crate) fn record_control(&self, name: &str, at_ms: u64) {
        self.record(PendingTrace {
            at_ms,
            kind: PreviewTraceKind::Control,
            name: name.to_owned(),
            node_id: None,
            event: None,
            action: None,
            detail: Value::Null,
            provenance: Vec::new(),
        });
    }

    pub(crate) fn record_diagnostic(
        &self,
        code: &str,
        message: &str,
        node_id: Option<String>,
        event: Option<String>,
        action: Option<String>,
        at_ms: u64,
    ) {
        self.record(PendingTrace {
            at_ms,
            kind: PreviewTraceKind::Diagnostic,
            name: code.to_owned(),
            node_id,
            event,
            action,
            detail: serde_json::json!({ "message": redact_text(message) }),
            provenance: Vec::new(),
        });
    }

    fn record(&self, entry: PendingTrace) {
        let mut inner = self.inner.borrow_mut();
        if inner.buffering_input {
            inner.pending.push(entry);
        } else {
            push_direct(&mut inner, entry);
        }
    }

    pub(crate) fn entries(&self) -> Vec<PreviewTraceEntry> {
        self.inner.borrow().entries.iter().cloned().collect()
    }

    pub(crate) fn diagnostics(&self) -> Vec<PreviewDiagnostic> {
        self.inner.borrow().diagnostics.iter().cloned().collect()
    }

    fn provenance(&self) -> BTreeMap<StateKey, PreviewStateProvenance> {
        self.inner.borrow().provenance.clone()
    }
}

impl ActionObserver for PreviewDebugTrace {
    fn action_started(&self, action: &'static str, context: &ActionContext) -> u64 {
        if let (Some(handler), Some(event)) = (&context.handler, &context.event) {
            self.inner
                .borrow_mut()
                .semantic_payloads
                .entry(handler.clone())
                .or_insert_with(|| redact_value(None, event.0.clone()));
        }
        self.record(PendingTrace {
            at_ms: context.now_ms(),
            kind: PreviewTraceKind::Action,
            name: action.to_owned(),
            node_id: context.node_id.clone(),
            event: context.handler.clone(),
            action: Some(action.to_owned()),
            detail: serde_json::json!({ "phase": "start" }),
            provenance: Vec::new(),
        });
        let mut inner = self.inner.borrow_mut();
        inner.next_action_token = inner.next_action_token.wrapping_add(1).max(1);
        let token = inner.next_action_token;
        inner.action_frames.insert(
            token,
            ActionFrame {
                action,
                before: StateSnapshot::capture(context),
                route_before: context.router.current(),
                node_id: context.node_id.clone(),
                event: context.handler.clone(),
            },
        );
        token
    }

    fn action_finished(
        &self,
        token: u64,
        action: &'static str,
        context: &ActionContext,
        result: &ActionResult,
    ) {
        let frame = self.inner.borrow_mut().action_frames.remove(&token);
        self.record(PendingTrace {
            at_ms: context.now_ms(),
            kind: PreviewTraceKind::Action,
            name: action.to_owned(),
            node_id: context.node_id.clone(),
            event: context.handler.clone(),
            action: Some(action.to_owned()),
            detail: serde_json::json!({
                "phase": "result",
                "ok": result.is_ok(),
                "error": result
                    .as_ref()
                    .err()
                    .map(ToString::to_string)
                    .map(|message| redact_text(&message)),
            }),
            provenance: Vec::new(),
        });
        let Some(frame) = frame else {
            return;
        };
        debug_assert_eq!(frame.action, action);
        let after = StateSnapshot::capture(context);
        let keys: BTreeSet<_> = frame
            .before
            .values
            .keys()
            .chain(after.values.keys())
            .cloned()
            .collect();
        for key in keys {
            let before = frame.before.values.get(&key);
            let after = after.values.get(&key);
            if before == after {
                continue;
            }
            self.record(PendingTrace {
                at_ms: context.now_ms(),
                kind: PreviewTraceKind::StateDiff,
                name: key.key.clone(),
                node_id: frame.node_id.clone(),
                event: frame.event.clone(),
                action: Some(action.to_owned()),
                detail: serde_json::json!({
                    "scope": scope_name(&key.scope),
                    "owner": key.owner,
                    "before": redact_value(Some(&key.key), before.cloned().unwrap_or(Value::Null)),
                    "after": redact_value(Some(&key.key), after.cloned().unwrap_or(Value::Null)),
                }),
                provenance: vec![key],
            });
        }
        let route_after = context.router.current();
        if route_after != frame.route_before {
            self.record_route(&route_after.path, &route_after.stack, context.now_ms());
        }
        if let Err(error) = result {
            self.record_diagnostic(
                "ActionError",
                &error.to_string(),
                frame.node_id,
                frame.event,
                Some(action.to_owned()),
                context.now_ms(),
            );
        }
    }
}

fn push_direct(inner: &mut TraceInner, pending: PendingTrace) {
    inner.next_sequence = inner.next_sequence.wrapping_add(1).max(1);
    let sequence = inner.next_sequence;
    let entry = PreviewTraceEntry {
        sequence,
        at_ms: pending.at_ms,
        kind: pending.kind,
        name: pending.name,
        node_id: pending.node_id,
        event: pending.event,
        action: pending.action,
        detail: pending.detail,
    };
    for key in pending.provenance {
        inner.provenance.insert(
            key,
            PreviewStateProvenance {
                node_id: entry.node_id.clone(),
                event: entry.event.clone(),
                action: entry.action.clone(),
                sequence,
            },
        );
    }
    if entry.kind == PreviewTraceKind::Diagnostic {
        let message = entry
            .detail
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if inner.diagnostics.len() >= DIAGNOSTIC_CAPACITY {
            inner.diagnostics.pop_front();
        }
        inner.diagnostics.push_back(PreviewDiagnostic {
            sequence,
            code: entry.name.clone(),
            message,
            node_id: entry.node_id.clone(),
            event: entry.event.clone(),
            action: entry.action.clone(),
        });
    }
    if inner.entries.len() >= TRACE_CAPACITY {
        inner.entries.pop_front();
    }
    inner.entries.push_back(entry);
}

fn scope_name(scope: &ActualScope) -> &'static str {
    match scope {
        ActualScope::App => "$app",
        ActualScope::Page => "$page",
        ActualScope::SelfNode => "$self",
    }
}

fn is_private_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "credential",
        "authorization",
        "apikey",
        "api_key",
        "activation",
        "clipboard",
        "payload",
    ]
    .iter()
    .any(|private| lower.contains(private))
}

fn redact_value(key: Option<&str>, value: Value) -> Value {
    if key.is_some_and(is_private_key) {
        return Value::String("<redacted>".to_owned());
    }
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    let value = if is_private_key(&key) {
                        Value::String("<redacted>".to_owned())
                    } else {
                        redact_value(Some(&key), value)
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| redact_value(None, value))
                .collect(),
        ),
        other => other,
    }
}

fn redact_text(message: &str) -> String {
    if is_private_key(message) {
        "<redacted>".to_owned()
    } else {
        message.to_owned()
    }
}

#[derive(Default)]
struct ControlClock {
    run_state: PreviewRunState,
    last_host_ms: u64,
    paused_at_host_ms: u64,
    clock_offset_ms: u64,
}

#[derive(Clone, Default)]
pub(crate) struct PreviewDebugState {
    pub(crate) trace: PreviewDebugTrace,
    control: Rc<RefCell<ControlClock>>,
}

impl PreviewDebugState {
    pub(crate) fn run_state(&self) -> PreviewRunState {
        self.control.borrow().run_state
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.run_state() == PreviewRunState::Paused
    }

    pub(crate) fn note_host_time(&self, host_ms: u64) {
        let mut control = self.control.borrow_mut();
        control.last_host_ms = control.last_host_ms.max(host_ms);
    }

    pub(crate) fn last_host_time(&self) -> u64 {
        self.control.borrow().last_host_ms
    }

    pub(crate) fn logical_time(&self, host_ms: u64) -> u64 {
        host_ms.saturating_sub(self.control.borrow().clock_offset_ms)
    }

    pub(crate) fn host_deadline(&self, logical_ms: u64) -> u64 {
        logical_ms.saturating_add(self.control.borrow().clock_offset_ms)
    }

    pub(crate) fn pause(&self, logical_now: u64) {
        let mut control = self.control.borrow_mut();
        if control.run_state == PreviewRunState::Paused {
            return;
        }
        control.run_state = PreviewRunState::Paused;
        control.paused_at_host_ms = control.last_host_ms;
        drop(control);
        self.trace.record_control("pause", logical_now);
    }

    pub(crate) fn resume(&self, logical_now: u64) {
        let mut control = self.control.borrow_mut();
        if control.run_state == PreviewRunState::Running {
            return;
        }
        let paused_duration = control
            .last_host_ms
            .saturating_sub(control.paused_at_host_ms);
        control.clock_offset_ms = control.clock_offset_ms.saturating_add(paused_duration);
        control.run_state = PreviewRunState::Running;
        drop(control);
        self.trace.record_control("resume", logical_now);
    }
}

impl crate::session::PreviewSession {
    pub fn pause(&mut self) {
        self.debug.pause(self.last_now_ms);
        self.runtime.set_debug_paused(true);
    }

    pub fn resume(&mut self) {
        self.debug.resume(self.last_now_ms);
        self.runtime.set_debug_paused(false);
    }

    pub fn reset(&mut self) -> Result<(), crate::PreviewEnterError> {
        let seed = self.reset_seed.clone();
        let rebuilt = crate::PreviewSession::enter_with_capabilities(
            &seed.document,
            seed.canvas_size,
            &seed.active_theme,
            seed.active_page_index,
            seed.preserve_authored_geometry,
            seed.presenting,
            seed.measure,
            seed.host_capabilities,
        )?;
        *self = rebuilt;
        self.debug.trace.record_control("reset", 0);
        Ok(())
    }

    pub fn trace_entries(&self) -> Vec<PreviewTraceEntry> {
        self.debug.trace.entries()
    }

    pub fn debug_snapshot(&self) -> PreviewDebugSnapshot {
        let declared_state: BTreeSet<String> = self
            .reset_seed
            .document
            .state
            .as_ref()
            .map(|state| state.keys().cloned().collect())
            .unwrap_or_default();
        let provenance = self.debug.trace.provenance();
        let mut state = Vec::new();
        for (key, value) in self.runtime.state.app_snapshot() {
            let scope = if declared_state.contains(&key) {
                PreviewStateScope::State
            } else {
                PreviewStateScope::App
            };
            let provenance_key = StateKey {
                scope: ActualScope::App,
                owner: None,
                key: key.clone(),
            };
            state.push(PreviewStateRow {
                scope,
                owner: None,
                key: key.clone(),
                value: redact_value(Some(&key), value),
                provenance: provenance.get(&provenance_key).cloned(),
            });
        }
        for page in self.runtime.state.page_keys() {
            for (key, value) in self.runtime.state.page_snapshot(&page) {
                let provenance_key = StateKey {
                    scope: ActualScope::Page,
                    owner: Some(page.clone()),
                    key: key.clone(),
                };
                state.push(PreviewStateRow {
                    scope: PreviewStateScope::Page,
                    owner: Some(page.clone()),
                    key: key.clone(),
                    value: redact_value(Some(&key), value),
                    provenance: provenance.get(&provenance_key).cloned(),
                });
            }
        }
        for (page, node) in self.runtime.state.self_keys() {
            let owner = format!("{page}/{node}");
            for (key, value) in self.runtime.state.self_snapshot(&page, &node) {
                let provenance_key = StateKey {
                    scope: ActualScope::SelfNode,
                    owner: Some(owner.clone()),
                    key: key.clone(),
                };
                state.push(PreviewStateRow {
                    scope: PreviewStateScope::SelfNode,
                    owner: Some(owner.clone()),
                    key: key.clone(),
                    value: redact_value(Some(&key), value),
                    provenance: provenance.get(&provenance_key).cloned(),
                });
            }
        }
        state.sort_by(|left, right| {
            (left.scope, &left.owner, &left.key).cmp(&(right.scope, &right.owner, &right.key))
        });
        let route = self.runtime.nav.current();
        let mut captured_pointers: Vec<u32> = self.gesture_mappings.keys().copied().collect();
        captured_pointers.sort_unstable();
        let mut diagnostics = self.debug.trace.diagnostics();
        for warning in &self.warnings {
            diagnostics.push(PreviewDiagnostic {
                sequence: 0,
                code: "PreviewWarning".to_owned(),
                message: redact_text(warning),
                node_id: None,
                event: None,
                action: None,
            });
        }
        for warning in self.effects.diagnostics() {
            diagnostics.push(PreviewDiagnostic {
                sequence: 0,
                code: "EffectDiagnostic".to_owned(),
                message: redact_text(&warning),
                node_id: None,
                event: None,
                action: None,
            });
        }
        PreviewDebugSnapshot {
            run_state: self.debug.run_state(),
            route_stack: route.stack,
            current_screen: Some(route.path),
            focused_node: self.focused_schema_id(),
            captured_pointers,
            active_gestures: self.runtime.debug_active_gesture_count(),
            state,
            queues: PreviewQueueCounts {
                action_tasks: self.runtime.debug_action_task_count(),
                effects: self.effects.outstanding(),
                animations: self.active_animation_track_count(),
            },
            diagnostics,
        }
    }
}
