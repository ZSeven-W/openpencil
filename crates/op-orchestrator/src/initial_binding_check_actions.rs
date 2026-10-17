//! Evidence of local UI writes and possible initialization, without executing actions.

use std::collections::HashSet;

use jian_core::action::{default_registry, SharedRegistry};
use jian_core::state::{Scope, Segment, StatePath};
use serde_json::Value;

use super::expression::is_pure_app_expression;

pub(super) struct WriteEvidence {
    pub user_keys: HashSet<String>,
    pub deferred_keys: HashSet<String>,
    pub opaque_initializer: bool,
    pub uncertain_visibility: HashSet<String>,
    registry: SharedRegistry,
}

impl Default for WriteEvidence {
    fn default() -> Self {
        Self {
            user_keys: HashSet::new(),
            deferred_keys: HashSet::new(),
            opaque_initializer: false,
            uncertain_visibility: HashSet::new(),
            registry: default_registry(),
        }
    }
}

impl WriteEvidence {
    pub fn lifecycle(&mut self, node: &Value, hook: &str) {
        let Some(hooks) = node.get("lifecycle") else {
            return;
        };
        if disabled(hooks, hook) {
            return;
        }
        if let Some(list) = hooks.get(hook) {
            // An enabled initializer may be conditional, asynchronous, or ordered
            // relative to a different mount. Defer its keys rather than simulate it.
            self.defer_list(list);
            self.initial_visibility(list, node.get("id").and_then(Value::as_str));
        }
    }

    fn initial_visibility(&mut self, list: &Value, owner: Option<&str>) {
        let Some(actions) = list.as_array() else {
            return;
        };
        for action in actions {
            let Some((name, body)) = single_action(action) else {
                continue;
            };
            match name {
                "show" | "hide" | "toggle_visibility" | "animate" => {
                    if name == "animate"
                        && !["from", "to"].iter().any(|field| {
                            body.get(field)
                                .is_some_and(|value| value.get("opacity").is_some())
                        })
                    {
                        continue;
                    }
                    let target = body
                        .as_str()
                        .or_else(|| body.get("target").and_then(Value::as_str));
                    let target = match target {
                        Some("$self") => owner,
                        other => other,
                    };
                    if let Some(target) = target.filter(|target| !target.starts_with('$')) {
                        self.uncertain_visibility.insert(target.to_owned());
                    } else {
                        self.opaque_initializer = true;
                    }
                }
                "if" | "fetch" | "confirm" | "for_each" => {
                    for field in ["then", "else", "on_error", "on_confirm", "on_cancel", "do"] {
                        if let Some(list) = body.get(field) {
                            self.initial_visibility(list, owner)
                        }
                    }
                }
                "parallel" | "race" => {
                    if let Some(lists) = body.as_array() {
                        for list in lists {
                            self.initial_visibility(list, owner)
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn events(&mut self, node: &Value) {
        let Some(events) = node.get("events") else {
            return;
        };
        for hook in [
            "onTap",
            "onDoubleTap",
            "onLongPress",
            "onPressStart",
            "onPressEnd",
            "onPressCancel",
            "onChange",
            "onSubmit",
            "onKey",
            "onSwipe",
        ] {
            if !disabled(events, hook) {
                if let Some(list) = events.get(hook) {
                    // A set inside an invalid action chain is not an executable
                    // writer. Parse the real DSL, but never execute its actions.
                    if self.registry.borrow().parse_list(list).is_ok() {
                        self.user_list(list);
                    } else {
                        self.defer_list(list);
                    }
                }
            }
        }
        // Focus, scroll, hover and gesture streams may be host-generated or
        // depend on mounting/viewport state. They only defer matching keys.
        for hook in [
            "onFocus",
            "onBlur",
            "onScroll",
            "onReachEnd",
            "onHoverEnter",
            "onHoverLeave",
            "onPanStart",
            "onPanUpdate",
            "onPanEnd",
            "onScaleStart",
            "onScaleUpdate",
            "onScaleEnd",
            "onRotateStart",
            "onRotateUpdate",
            "onRotateEnd",
            "onRawPointer",
            "onContextMenu",
        ] {
            if !disabled(events, hook) {
                if let Some(list) = events.get(hook) {
                    self.defer_list(list)
                }
            }
        }
    }

    fn user_list(&mut self, list: &Value) {
        let Some(actions) = list.as_array() else {
            return;
        };
        let mut delayed = false;
        for action in actions {
            let Some((name, body)) = single_action(action) else {
                continue;
            };
            if delayed {
                self.defer_action(name, body);
                continue;
            }
            match name {
                "set" => {
                    for (target, value) in set_pairs(body) {
                        if let Some(key) = app_key(target) {
                            if value.as_str().is_some_and(is_pure_app_expression) {
                                self.user_keys.insert(key);
                            } else {
                                self.deferred_keys.insert(key);
                            }
                        }
                    }
                }
                "if" => {
                    let pure = body
                        .get("expr")
                        .and_then(Value::as_str)
                        .is_some_and(is_pure_app_expression);
                    for branch in ["then", "else"] {
                        if let Some(list) = body.get(branch) {
                            if pure {
                                self.user_list(list)
                            } else {
                                self.defer_list(list)
                            }
                        }
                    }
                }
                "delay" | "abort" => delayed = true,
                "fetch" | "paste" | "call" | "parallel" | "race" | "confirm" | "for_each"
                | "ws_connect" => {
                    self.defer_action(name, body);
                    delayed = true;
                }
                _ => self.defer_action(name, body),
            }
        }
    }

    fn defer_list(&mut self, list: &Value) {
        if let Some(actions) = list.as_array() {
            for action in actions {
                if let Some((name, body)) = single_action(action) {
                    self.defer_action(name, body);
                }
            }
        }
    }

    fn defer_action(&mut self, name: &str, body: &Value) {
        match name {
            "set" => {
                for (target, _) in set_pairs(body) {
                    self.defer_target(target);
                }
            }
            "toggle" | "delete" => {
                if let Some(target) = body
                    .as_str()
                    .or_else(|| body.get("target").and_then(Value::as_str))
                {
                    self.defer_target(target);
                }
            }
            "reset" => {
                if let Some(target) = body.as_str() {
                    self.defer_target(target)
                }
            }
            "if" => {
                for branch in ["then", "else"] {
                    if let Some(list) = body.get(branch) {
                        self.defer_list(list)
                    }
                }
            }
            "parallel" | "race" => {
                if let Some(lists) = body.as_array() {
                    for list in lists {
                        self.defer_list(list)
                    }
                }
            }
            "for_each" => {
                if let Some(list) = body.get("do") {
                    self.defer_list(list)
                }
            }
            "fetch" | "paste" => {
                for field in ["into", "loading"] {
                    if let Some(target) = body.get(field).and_then(Value::as_str) {
                        self.defer_target(target);
                    }
                }
                if let Some(list) = body.get("on_error") {
                    self.defer_list(list)
                }
            }
            "confirm" => {
                for field in ["on_confirm", "on_cancel"] {
                    if let Some(list) = body.get(field) {
                        self.defer_list(list)
                    }
                }
            }
            // Logic and websocket callbacks can mutate state outside this tree.
            "call" | "ws_connect" => self.opaque_initializer = true,
            // Known non-state actions do not provide initialization evidence.
            "delay" | "abort" | "push" | "replace" | "pop" | "open_url" | "copy"
            | "storage_set" | "storage_clear" | "toast" | "alert" | "ws_send" | "ws_close"
            | "vibrate" | "haptic" | "share" | "notify" | "focus" | "blur" | "dismiss_keyboard"
            | "show" | "hide" | "toggle_visibility" | "scroll_to" | "animate" => {}
            _ => self.opaque_initializer = true,
        }
    }

    fn defer_target(&mut self, target: &str) {
        if let Some(key) = app_key(target) {
            self.deferred_keys.insert(key);
        }
    }
}

fn disabled(hooks: &Value, hook: &str) -> bool {
    hooks
        .get("disabledEvents")
        .and_then(Value::as_array)
        .is_some_and(|keys| keys.iter().any(|key| key.as_str() == Some(hook)))
}

fn single_action(action: &Value) -> Option<(&str, &Value)> {
    let object = action.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object
        .iter()
        .next()
        .map(|(name, body)| (name.as_str(), body))
}

fn set_pairs(body: &Value) -> Vec<(&str, &Value)> {
    let Some(object) = body.as_object() else {
        return Vec::new();
    };
    if let (Some(target), Some(value)) = (object.get("target"), object.get("value")) {
        return target
            .as_str()
            .map(|target| vec![(target, value)])
            .unwrap_or_default();
    }
    object
        .iter()
        .map(|(key, value)| (key.as_str(), value))
        .collect()
}

fn app_key(target: &str) -> Option<String> {
    let path = StatePath::parse(target).ok()?;
    match (path.scope, path.segments.as_slice()) {
        (Scope::App, [Segment::Key(key)]) => Some(key.clone()),
        _ => None,
    }
}
