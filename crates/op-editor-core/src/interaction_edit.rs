//! Structured model behind the property panel's Interactions section.
//!
//! The Preview runtime (jian) executes a node's `events` block — a map of
//! trigger keys (`onTap`, `onHoverEnter`, …) to action lists, each action
//! a one-key JSON object (`{"push": "\"/home\""}`). This module is the
//! wasm32-clean bridge between that wire shape and a panel a person can
//! drive with clicks: it classifies authored actions into a closed set of
//! editable kinds, renders their single parameter for display, and
//! applies structured [`InteractionEdit`]s back onto the `events` JSON.
//!
//! Anything the panel does not understand (an unknown trigger, a `fetch`
//! action, a `set` with several pairs) survives untouched: unknown
//! triggers show as read-only rows and unknown actions keep their raw
//! body, so hand-written and AI-written documents never lose behaviour by
//! being opened in the panel.

use jian_ops_schema::events::EventHandlers;
use serde_json::{Map, Value};

/// Triggers the panel can author, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractionTrigger {
    Tap,
    DoubleTap,
    LongPress,
    HoverEnter,
    HoverLeave,
    PressStart,
    PressEnd,
    Swipe,
    Change,
    Submit,
    Focus,
    Blur,
    ReachEnd,
}

impl InteractionTrigger {
    pub const ALL: [InteractionTrigger; 13] = [
        Self::Tap,
        Self::DoubleTap,
        Self::LongPress,
        Self::HoverEnter,
        Self::HoverLeave,
        Self::PressStart,
        Self::PressEnd,
        Self::Swipe,
        Self::Change,
        Self::Submit,
        Self::Focus,
        Self::Blur,
        Self::ReachEnd,
    ];

    /// The `events` wire key.
    pub fn key(self) -> &'static str {
        match self {
            Self::Tap => "onTap",
            Self::DoubleTap => "onDoubleTap",
            Self::LongPress => "onLongPress",
            Self::HoverEnter => "onHoverEnter",
            Self::HoverLeave => "onHoverLeave",
            Self::PressStart => "onPressStart",
            Self::PressEnd => "onPressEnd",
            Self::Swipe => "onSwipe",
            Self::Change => "onChange",
            Self::Submit => "onSubmit",
            Self::Focus => "onFocus",
            Self::Blur => "onBlur",
            Self::ReachEnd => "onReachEnd",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.key() == key)
    }

    /// i18n key + English fallback for the trigger label.
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::Tap => ("interactions.trigger.tap", "On tap"),
            Self::DoubleTap => ("interactions.trigger.doubleTap", "On double tap"),
            Self::LongPress => ("interactions.trigger.longPress", "On long press"),
            Self::HoverEnter => ("interactions.trigger.hoverEnter", "On hover"),
            Self::HoverLeave => ("interactions.trigger.hoverLeave", "On hover end"),
            Self::PressStart => ("interactions.trigger.pressStart", "On press"),
            Self::PressEnd => ("interactions.trigger.pressEnd", "On release"),
            Self::Swipe => ("interactions.trigger.swipe", "On swipe"),
            Self::Change => ("interactions.trigger.change", "On change"),
            Self::Submit => ("interactions.trigger.submit", "On submit"),
            Self::Focus => ("interactions.trigger.focus", "On focus"),
            Self::Blur => ("interactions.trigger.blur", "On blur"),
            Self::ReachEnd => ("interactions.trigger.reachEnd", "On scroll end"),
        }
    }
}

/// Action kinds the panel can author, in menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractionActionKind {
    Navigate,
    Back,
    Show,
    Hide,
    ToggleVisibility,
    ScrollTo,
    ToggleState,
    Toast,
    OpenUrl,
    Copy,
    Delay,
    Haptic,
    DismissKeyboard,
}

/// What the single parameter of an action is, which decides the editor
/// the panel shows for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionParamKind {
    None,
    /// A `screen` route path, picked from the page's screens.
    Screen,
    /// A node id, picked from the current screen's layers.
    Node,
    /// Free text (toast message, URL, clipboard text).
    Text,
    /// A `$app` state key name (free text).
    StateKey,
    /// Milliseconds (free text, digits).
    Duration,
}

impl InteractionActionKind {
    pub const ALL: [InteractionActionKind; 13] = [
        Self::Navigate,
        Self::Back,
        Self::Show,
        Self::Hide,
        Self::ToggleVisibility,
        Self::ScrollTo,
        Self::ToggleState,
        Self::Toast,
        Self::OpenUrl,
        Self::Copy,
        Self::Delay,
        Self::Haptic,
        Self::DismissKeyboard,
    ];

    pub fn param_kind(self) -> InteractionParamKind {
        use InteractionParamKind as P;
        match self {
            Self::Navigate => P::Screen,
            Self::Show | Self::Hide | Self::ToggleVisibility | Self::ScrollTo => P::Node,
            Self::Toast | Self::OpenUrl | Self::Copy => P::Text,
            Self::ToggleState => P::StateKey,
            Self::Delay => P::Duration,
            Self::Back | Self::Haptic | Self::DismissKeyboard => P::None,
        }
    }

    /// i18n key + English fallback for the action label.
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::Navigate => ("interactions.action.navigate", "Navigate to"),
            Self::Back => ("interactions.action.back", "Go back"),
            Self::Show => ("interactions.action.show", "Show layer"),
            Self::Hide => ("interactions.action.hide", "Hide layer"),
            Self::ToggleVisibility => ("interactions.action.toggleVisibility", "Toggle layer"),
            Self::ScrollTo => ("interactions.action.scrollTo", "Scroll to"),
            Self::ToggleState => ("interactions.action.toggleState", "Toggle state"),
            Self::Toast => ("interactions.action.toast", "Show toast"),
            Self::OpenUrl => ("interactions.action.openUrl", "Open link"),
            Self::Copy => ("interactions.action.copy", "Copy text"),
            Self::Delay => ("interactions.action.delay", "Wait"),
            Self::Haptic => ("interactions.action.haptic", "Haptic feedback"),
            Self::DismissKeyboard => ("interactions.action.dismissKeyboard", "Dismiss keyboard"),
        }
    }

    /// Classify an authored one-key action. `None` for anything the panel
    /// cannot round-trip (it is then shown read-only by its raw key).
    pub fn classify(key: &str, body: &Value) -> Option<Self> {
        Some(match key {
            "push" | "replace" => Self::Navigate,
            "pop" => Self::Back,
            "show" => Self::Show,
            "hide" => Self::Hide,
            "toggle_visibility" => Self::ToggleVisibility,
            "scroll_to" => Self::ScrollTo,
            "toggle" if body.is_string() => Self::ToggleState,
            "toast" => Self::Toast,
            "open_url" => Self::OpenUrl,
            "copy" => Self::Copy,
            "delay" => Self::Delay,
            "haptic" => Self::Haptic,
            "dismiss_keyboard" => Self::DismissKeyboard,
            _ => return None,
        })
    }
}

/// One authored action, summarized for the panel.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionActionView {
    /// `None` = unrecognised action (read-only, shown by `raw_key`).
    pub kind: Option<InteractionActionKind>,
    pub raw_key: String,
    /// Display/edit value of the single parameter (route path, node id,
    /// text, state key, or milliseconds). Empty when the kind has none.
    pub param: String,
}

/// One trigger with its action list.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionTriggerView {
    /// `None` = a trigger the panel cannot author (pan/scale/key/…):
    /// shown read-only and removable as a whole.
    pub trigger: Option<InteractionTrigger>,
    pub raw_key: String,
    pub actions: Vec<InteractionActionView>,
}

/// Summarize a node's `events` block. Known triggers come first in
/// [`InteractionTrigger::ALL`] order, then unknown keys alphabetically.
pub fn summarize_events(events: Option<&EventHandlers>) -> Vec<InteractionTriggerView> {
    let map = events_map(events);
    let mut out = Vec::new();
    for trigger in InteractionTrigger::ALL {
        if let Some(list) = map.get(trigger.key()) {
            out.push(InteractionTriggerView {
                trigger: Some(trigger),
                raw_key: trigger.key().to_string(),
                actions: action_views(list),
            });
        }
    }
    for (key, list) in &map {
        if InteractionTrigger::from_key(key).is_none() {
            out.push(InteractionTriggerView {
                trigger: None,
                raw_key: key.clone(),
                actions: action_views(list),
            });
        }
    }
    out
}

fn action_views(list: &Value) -> Vec<InteractionActionView> {
    list.as_array()
        .map(|items| items.iter().map(action_view).collect())
        .unwrap_or_default()
}

fn action_view(action: &Value) -> InteractionActionView {
    let Some((key, body)) = action
        .as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.iter().next())
    else {
        return InteractionActionView {
            kind: None,
            raw_key: String::new(),
            param: String::new(),
        };
    };
    let kind = InteractionActionKind::classify(key, body);
    InteractionActionView {
        kind,
        raw_key: key.clone(),
        param: kind.map(|k| read_param(k, body)).unwrap_or_default(),
    }
}

/// Serialize the typed handlers to a key → action-list JSON map.
pub(crate) fn events_map(events: Option<&EventHandlers>) -> Map<String, Value> {
    events
        .and_then(|e| serde_json::to_value(e).ok())
        .and_then(|v| match v {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

// ── Body codecs ─────────────────────────────────────────────────────

/// Strip a Tier-1 string-literal expression (`"x"` / `'x'`) back to its
/// text; any other expression is shown verbatim.
pub fn unquote_literal(src: &str) -> String {
    let s = src.trim();
    if s.len() >= 2 {
        if s.starts_with('"') && s.ends_with('"') {
            if let Ok(Value::String(text)) = serde_json::from_str::<Value>(s) {
                return text;
            }
        }
        if s.starts_with('\'') && s.ends_with('\'') && !s[1..s.len() - 1].contains('\'') {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Encode text as a Tier-1 string-literal expression source.
pub fn quote_literal(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
}

fn str_or_field<'a>(body: &'a Value, field: &str) -> &'a str {
    body.as_str()
        .or_else(|| body.get(field).and_then(Value::as_str))
        .unwrap_or("")
}

fn read_param(kind: InteractionActionKind, body: &Value) -> String {
    use InteractionActionKind as K;
    match kind {
        K::Navigate => unquote_literal(body.as_str().unwrap_or("")),
        K::Show | K::Hide | K::ToggleVisibility | K::ScrollTo => {
            str_or_field(body, "target").trim().to_string()
        }
        K::Toast => unquote_literal(str_or_field(body, "message")),
        K::OpenUrl => unquote_literal(str_or_field(body, "url")),
        K::Copy => unquote_literal(str_or_field(body, "text")),
        K::ToggleState => body
            .as_str()
            .map(strip_state_prefix)
            .unwrap_or_default()
            .to_string(),
        K::Delay => body
            .get("ms")
            .and_then(Value::as_u64)
            .map(|ms| ms.to_string())
            .unwrap_or_default(),
        K::Back | K::Haptic | K::DismissKeyboard => String::new(),
    }
}

/// `$app.open` / `$state.open` → `open`.
pub fn strip_state_prefix(path: &str) -> &str {
    let path = path.trim();
    path.strip_prefix("$app.")
        .or_else(|| path.strip_prefix("$state."))
        .unwrap_or(path)
}

/// Sanitize a free-text state key to the `[A-Za-z0-9_]` subset a
/// single-segment `$app` path accepts. Empty when nothing usable is left.
pub fn sanitize_state_key(raw: &str) -> String {
    let key: String = strip_state_prefix(raw)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if key.starts_with(|c: char| c.is_ascii_digit()) {
        format!("_{key}")
    } else {
        key
    }
}

/// Build the one-key action JSON for `kind` with `param`. `verb` keeps an
/// authored `replace` when only the route changes.
pub fn build_action(kind: InteractionActionKind, param: &str, verb: Option<&str>) -> Value {
    use serde_json::json;
    use InteractionActionKind as K;
    let param = param.trim();
    match kind {
        K::Navigate => {
            let verb = match verb {
                Some("replace") => "replace",
                _ => "push",
            };
            json!({ verb: quote_literal(param) })
        }
        K::Back => json!({ "pop": null }),
        K::Show => json!({ "show": param }),
        K::Hide => json!({ "hide": param }),
        K::ToggleVisibility => json!({ "toggle_visibility": param }),
        K::ScrollTo => json!({ "scroll_to": { "target": param, "alignment": "start" } }),
        K::ToggleState => json!({ "toggle": format!("$app.{}", sanitize_state_key(param)) }),
        K::Toast => json!({ "toast": quote_literal(param) }),
        K::OpenUrl => json!({ "open_url": { "url": quote_literal(param) } }),
        K::Copy => json!({ "copy": quote_literal(param) }),
        K::Delay => json!({ "delay": { "ms": param.parse::<u64>().unwrap_or(300).min(60_000) } }),
        K::Haptic => json!({ "haptic": { "style": "medium" } }),
        K::DismissKeyboard => json!({ "dismiss_keyboard": null }),
    }
}

#[cfg(test)]
#[path = "interaction_edit_tests.rs"]
mod tests;
