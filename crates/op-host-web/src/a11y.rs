//! Hidden accessibility DOM mirror for the canvas-painted web shell
//! (TS-retirement finding #58, "Web accessibility / screen readers").
//!
//! The TS web app is ordinary DOM — `<button>`s with `aria-label` /
//! `aria-pressed` (`apps/web/src/components/editor/tool-button.tsx`,
//! `toolbar.tsx`), a real `<textarea>` chat input
//! (`panels/ai-chat-input.tsx`) and Radix/shadcn primitives, so
//! screen readers get a full accessibility tree for free. The Rust
//! wasm host paints everything into one `<canvas>`, which assistive
//! tech sees as a single opaque image. This module mirrors a *useful
//! subset* of editor state into a visually-hidden DOM layer inserted
//! as a sibling **before** the canvas, so the hidden controls precede
//! the canvas in tab + reading order.
//!
//! ## v1 scope (documented divergence from full DOM parity)
//!
//! Covered:
//! - **Landmarks** — `role="toolbar"` (Tools), `role="navigation"`
//!   (Layers summary), `role="region"` (Properties / AI chat), a
//!   `role="status"` selection live region, and the canvas itself
//!   labelled `role="application"` + `tabindex="0"` so it is
//!   reachable and keystrokes route to the app (the window keydown
//!   listener), not the screen reader's virtual cursor.
//! - **Operable tool buttons** — one real `<button>` per `Tool`
//!   with `aria-pressed` synced from `EditorState::tool`; activating
//!   one runs the same state transition as clicking the painted
//!   toolbar (`widget_host/press.rs` `ToolbarHit::Tool` arm).
//! - **Chat** — a "Focus AI chat input" button (mirrors
//!   `AIChatHit::FocusInput`; afterwards DOM focus moves to the
//!   canvas so typing flows into the painted input instead of
//!   re-activating the button) and a `role="log"` transcript that
//!   appends each chat message once its turn finishes streaming
//!   (`role="log"` is implicitly `aria-live="polite"`).
//! - **Focused-input mirror** — label + live value of whichever
//!   painted text input currently owns typing (settings draft,
//!   property-panel field, icon / model / component search, chat
//!   input). The label span is `aria-live="polite"` and rewritten
//!   only when the focused *field* changes; the value span is
//!   `aria-live="off"` so per-keystroke updates stay silent but
//!   remain queryable.
//! - **Selection announcements** — "Selected Hero (Frame)" /
//!   "Selected 3 layers — …" / "Selection cleared" via the status
//!   region.
//!
//! NOT covered in v1 (deferred, by design):
//! - Full layer-tree mirroring (only a page + top-level-count
//!   summary; no per-layer rows, no tree navigation).
//! - Editable DOM proxies — text still enters through the existing
//!   window keydown / hidden-IME path, not a mirrored `<input>`.
//! - Panels and overlays beyond the focused-field announcement
//!   (property panel sections, pickers, settings modal contents).
//! - i18n — a11y strings are English-only for now; the chrome's
//!   `op-i18n` tables don't carry a11y keys yet.
//! - `accesskit::TreeUpdate` integration — a native accesskit wiring
//!   item exists separately; this layer is handwritten DOM and does
//!   not consume accesskit.
//! - Re-rendering a transcript message that mutates *after* its turn
//!   completed, and same-length transcript swaps (only growth and
//!   shrink-to-reset are detected).
//!
//! ## Sync cadence
//!
//! State→DOM sync runs on a coarse [`SYNC_INTERVAL_MS`] interval
//! (started by [`start_pump`]) plus an immediate pass after each
//! hidden-button activation. Per-frame sync would be wasteful (the
//! host repaints on every mousemove) and a per-mutation hook would
//! touch every mutator; the interval keeps the layer ~4 Hz fresh —
//! plenty for `aria-live="polite"` announcements — and every DOM
//! write is diffed against a cached signature so an idle editor does
//! zero DOM work.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, HtmlCanvasElement, HtmlElement};

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::ui_draft::PropertyFocus;
use op_editor_core::{EditorState, Tool};

use crate::listener::{add_listener, now_ms_perf, Listener};
use crate::Inner;

/// Coarse state→DOM sync cadence in ms. See the module docs for the
/// interval-over-per-frame rationale.
pub(crate) const SYNC_INTERVAL_MS: i32 = 250;

/// Visually-hidden ("sr-only") container style: invisible and
/// non-interfering for sighted users, but NOT `display:none` /
/// `visibility:hidden` (both would hide the subtree from assistive
/// tech and make the buttons unfocusable).
const SR_ONLY_STYLE: &str = "position:absolute;width:1px;height:1px;margin:-1px;\
     padding:0;border:0;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;";

/// Selection-status text for an empty selection. Doubles as the
/// initial cached signature so a freshly mounted editor (nothing
/// selected yet) does not announce "Selection cleared" on page load.
const SELECTION_CLEARED: &str = "Selection cleared";

/// The mounted hidden-DOM layer: element handles + cached signatures
/// so each sync pass only writes the DOM on actual change.
pub(crate) struct A11yLayer {
    document: Document,
    canvas: HtmlCanvasElement,
    root: HtmlElement,
    tool_buttons: Vec<(Tool, HtmlElement)>,
    chat_focus_button: HtmlElement,
    chat_log: HtmlElement,
    selection_status: HtmlElement,
    layers_summary: HtmlElement,
    focus_label: HtmlElement,
    focus_value: HtmlElement,
    // ----- cached signatures (diff guards) -----
    last_tool: Option<Tool>,
    last_selection: String,
    last_layers: String,
    last_focus_key: String,
    last_focus_value: String,
    /// How many transcript messages have been mirrored into the log.
    mirrored_messages: usize,
}

fn create_el(document: &Document, tag: &str) -> Result<HtmlElement, JsValue> {
    document
        .create_element(tag)?
        .dyn_into::<HtmlElement>()
        .map_err(|_| JsValue::from_str("a11y: created element is not an HtmlElement"))
}

impl A11yLayer {
    /// Build the hidden layer + label the canvas. The whole subtree
    /// is assembled detached; the only structural DOM mutation is the
    /// final insert, so a mid-build failure leaves no orphan nodes
    /// (the canvas attribute writes are harmless leftovers).
    pub(crate) fn mount(document: &Document, canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let root = create_el(document, "div")?;
        root.set_attribute("style", SR_ONLY_STYLE)?;
        root.set_attribute("data-op-a11y", "true")?;

        // ----- toolbar landmark: one real button per tool -----
        let toolbar = create_el(document, "div")?;
        toolbar.set_attribute("role", "toolbar")?;
        toolbar.set_attribute("aria-label", "Tools")?;
        let mut tool_buttons = Vec::with_capacity(Tool::ALL.len());
        for tool in Tool::ALL {
            let btn = create_el(document, "button")?;
            btn.set_attribute("type", "button")?;
            btn.set_attribute("data-op-a11y-tool", tool.ident())?;
            btn.set_attribute("aria-pressed", "false")?;
            btn.set_text_content(Some(&format!("{} tool", tool_label(tool))));
            toolbar.append_child(&btn)?;
            tool_buttons.push((tool, btn));
        }
        root.append_child(&toolbar)?;

        // ----- layers landmark: summary only (full tree deferred) -----
        let layers_nav = create_el(document, "div")?;
        layers_nav.set_attribute("role", "navigation")?;
        layers_nav.set_attribute("aria-label", "Layers")?;
        let layers_summary = create_el(document, "p")?;
        layers_nav.append_child(&layers_summary)?;
        root.append_child(&layers_nav)?;

        // ----- properties landmark: focused-input mirror -----
        let props = create_el(document, "div")?;
        props.set_attribute("role", "region")?;
        props.set_attribute("aria-label", "Properties")?;
        let focus_label = create_el(document, "p")?;
        focus_label.set_attribute("aria-live", "polite")?;
        let focus_value = create_el(document, "p")?;
        focus_value.set_attribute("aria-live", "off")?;
        props.append_child(&focus_label)?;
        props.append_child(&focus_value)?;
        root.append_child(&props)?;

        // ----- chat landmark: focus button + transcript log -----
        let chat = create_el(document, "div")?;
        chat.set_attribute("role", "region")?;
        chat.set_attribute("aria-label", "AI chat")?;
        let chat_focus_button = create_el(document, "button")?;
        chat_focus_button.set_attribute("type", "button")?;
        chat_focus_button.set_text_content(Some("Focus AI chat input"));
        let chat_log = create_el(document, "div")?;
        chat_log.set_attribute("role", "log")?;
        chat_log.set_attribute("aria-label", "AI chat transcript")?;
        chat.append_child(&chat_focus_button)?;
        chat.append_child(&chat_log)?;
        root.append_child(&chat)?;

        // ----- selection live region -----
        let selection_status = create_el(document, "p")?;
        selection_status.set_attribute("role", "status")?;
        root.append_child(&selection_status)?;

        // ----- canvas: name it + make it tabbable -----
        // `role="application"` keeps screen readers in pass-through
        // mode over the canvas so our window keydown listener gets the
        // raw keystrokes (the same model the painted inputs rely on).
        canvas.set_attribute("role", "application")?;
        canvas.set_attribute("aria-roledescription", "design canvas")?;
        canvas.set_attribute("aria-label", "Design canvas")?;
        canvas.set_attribute("tabindex", "0")?;

        // Final structural insert: hidden layer BEFORE the canvas so
        // its controls precede the canvas in tab + reading order.
        match canvas.parent_node() {
            Some(parent) => {
                parent.insert_before(&root, Some(canvas.unchecked_ref::<web_sys::Node>()))?;
            }
            None => {
                document
                    .body()
                    .ok_or_else(|| JsValue::from_str("a11y: document.body unavailable"))?
                    .append_child(&root)?;
            }
        }

        Ok(Self {
            document: document.clone(),
            canvas: canvas.clone(),
            root,
            tool_buttons,
            chat_focus_button,
            chat_log,
            selection_status,
            layers_summary,
            focus_label,
            focus_value,
            last_tool: None,
            last_selection: SELECTION_CLEARED.to_string(),
            last_layers: String::new(),
            last_focus_key: String::new(),
            last_focus_value: String::new(),
            mirrored_messages: 0,
        })
    }

    /// Tear the hidden layer down and strip the canvas attributes we
    /// added — leaves the page as if the layer never mounted.
    pub(crate) fn unmount(&self) {
        self.root.remove();
        for attr in ["role", "aria-roledescription", "aria-label", "tabindex"] {
            let _ = self.canvas.remove_attribute(attr);
        }
    }

    /// One diff-guarded state→DOM pass. Called by the interval pump
    /// and immediately after a hidden-button activation.
    pub(crate) fn sync(&mut self, state: &EditorState) {
        // Tool pressed-states (silent attribute writes).
        if self.last_tool != Some(state.tool) {
            for (tool, btn) in &self.tool_buttons {
                let pressed = if *tool == state.tool { "true" } else { "false" };
                let _ = btn.set_attribute("aria-pressed", pressed);
            }
            self.last_tool = Some(state.tool);
        }

        // Selection announcement (role=status — polite live region).
        let selection = selection_summary(state);
        if selection != self.last_selection {
            self.selection_status.set_text_content(Some(&selection));
            self.last_selection = selection;
        }

        // Layers summary (non-live — silent update, queryable).
        let layers = layers_summary(state);
        if layers != self.last_layers {
            self.layers_summary.set_text_content(Some(&layers));
            self.last_layers = layers;
        }

        // Focused-input mirror: announce the field on identity change;
        // keep the value current but silent (aria-live="off").
        match focused_input(state) {
            Some((key, label, value)) => {
                if key != self.last_focus_key {
                    self.focus_label
                        .set_text_content(Some(&format!("{label} focused")));
                    self.last_focus_key = key;
                }
                if value != self.last_focus_value {
                    self.focus_value.set_text_content(Some(&value));
                    self.last_focus_value = value;
                }
            }
            None => {
                if !self.last_focus_key.is_empty() {
                    self.focus_label.set_text_content(None);
                    self.focus_value.set_text_content(None);
                    self.last_focus_key = String::new();
                    self.last_focus_value = String::new();
                }
            }
        }

        self.sync_chat_log(state);
    }

    /// Mirror completed transcript messages into the `role="log"`
    /// region. Streaming turns are appended once they finish (so the
    /// screen reader hears one coherent message, not 4 Hz fragments);
    /// a transcript shrink (New Chat) clears and restarts the log.
    fn sync_chat_log(&mut self, state: &EditorState) {
        let messages = &state.chat.messages;
        if messages.len() < self.mirrored_messages {
            self.chat_log.set_text_content(None);
            self.mirrored_messages = 0;
        }
        while self.mirrored_messages < messages.len() {
            let msg = &messages[self.mirrored_messages];
            if msg.streaming {
                // Turns complete in order; wait for this one.
                break;
            }
            if !msg.content.is_empty() {
                if let Ok(p) = create_el(&self.document, "p") {
                    let who = match msg.role {
                        op_editor_core::chat::ChatRole::User => "You",
                        op_editor_core::chat::ChatRole::Assistant => "Assistant",
                    };
                    p.set_text_content(Some(&format!("{who}: {}", msg.content)));
                    let _ = self.chat_log.append_child(&p);
                }
            }
            self.mirrored_messages += 1;
        }
    }
}

/// Mount the hidden layer, register its button listeners (into the
/// shared `listeners` vec so `mount()`'s unwind + `Drop for WebShell`
/// manage their lifetime), and store the layer on `Inner`. The layer
/// is stored BEFORE listener registration so the caller's error
/// unwind (`inner.a11y.take().unmount()`) covers a mid-registration
/// failure.
pub(crate) fn wire(
    document: &Document,
    canvas: &HtmlCanvasElement,
    inner: &Rc<RefCell<Inner>>,
    listeners: &mut Vec<Listener>,
) -> Result<(), JsValue> {
    let layer = A11yLayer::mount(document, canvas)?;
    let tool_buttons: Vec<(Tool, HtmlElement)> = layer.tool_buttons.clone();
    let chat_button = layer.chat_focus_button.clone();
    inner.borrow_mut().a11y = Some(layer);

    for (tool, btn) in tool_buttons {
        let inner_tool = inner.clone();
        add_listener::<web_sys::MouseEvent, _, _>(&btn, "click", listeners, move |_evt| {
            let mut guard = inner_tool.borrow_mut();
            guard.host.a11y_set_tool(tool);
            let _ = guard.repaint();
            sync_now(&mut guard);
        })?;
    }

    let inner_chat = inner.clone();
    let canvas_focus = canvas.clone();
    add_listener::<web_sys::MouseEvent, _, _>(&chat_button, "click", listeners, move |_evt| {
        {
            let mut guard = inner_chat.borrow_mut();
            guard.host.set_now_ms(now_ms_perf());
            guard.host.a11y_focus_chat_input();
            let _ = guard.repaint();
            sync_now(&mut guard);
        }
        #[cfg(feature = "codegen")]
        crate::ensure_caret_blink_pump(&inner_chat);
        // Move DOM focus onto the canvas: a focused <button> re-fires
        // its click on Space/Enter, which would double-handle the
        // user's typing. On the canvas, keystrokes flow through the
        // window keydown listener into the now-focused chat input.
        let _ = canvas_focus.focus();
    })?;

    Ok(())
}

/// Run one sync pass against the host's editor state. Split-borrows
/// `Inner` so the read-only state borrow and the layer's `&mut`
/// coexist.
fn sync_now(guard: &mut Inner) {
    let Inner { host, a11y, .. } = guard;
    if let Some(layer) = a11y.as_mut() {
        layer.sync(host.a11y_editor_state());
    }
}

/// Live interval handle for the coarse sync pump. Dropping it clears
/// the browser interval and releases the closure slot.
pub(crate) struct A11yPump {
    interval_id: i32,
    _closure: Closure<dyn FnMut()>,
}

impl Drop for A11yPump {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            window.clear_interval_with_handle(self.interval_id);
        }
    }
}

/// Start the [`SYNC_INTERVAL_MS`] sync pump. Best-effort: a missing
/// `window` / failed registration returns `None`, leaving the hidden
/// layer static (it still reflects button-activation syncs).
pub(crate) fn start_pump(inner: &Rc<RefCell<Inner>>) -> Option<A11yPump> {
    let window = web_sys::window()?;
    let inner = inner.clone();
    let closure = Closure::<dyn FnMut()>::new(move || {
        // try_borrow: if a DOM event handler is mid-dispatch (it
        // can't be — JS is single-threaded — but a re-entrant
        // dispatch through a synchronous DOM API could), skip this
        // tick instead of panicking; the next tick catches up.
        let Ok(mut guard) = inner.try_borrow_mut() else {
            return;
        };
        sync_now(&mut guard);
    });
    let interval_id = window
        .set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            SYNC_INTERVAL_MS,
        )
        .ok()?;
    Some(A11yPump {
        interval_id,
        _closure: closure,
    })
}

// ---------------------------------------------------------------------
// Pure label / summary helpers (DOM-free — unit-testable on native).
// ---------------------------------------------------------------------

/// Human-readable tool name for the hidden toolbar buttons.
fn tool_label(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "Select",
        Tool::Rect => "Rectangle",
        Tool::Ellipse => "Ellipse",
        Tool::Polygon => "Polygon",
        Tool::Line => "Line",
        Tool::Pen => "Pen",
        Tool::Text => "Text",
        Tool::Frame => "Frame",
        Tool::Hand => "Hand (pan)",
    }
}

/// Node-kind word for selection announcements.
fn kind_label(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "Frame",
        PenNode::Group(_) => "Group",
        PenNode::Rectangle(_) => "Rectangle",
        PenNode::Ellipse(_) => "Ellipse",
        PenNode::Line(_) => "Line",
        PenNode::Polygon(_) => "Polygon",
        PenNode::Path(_) => "Path",
        PenNode::Text(_) => "Text",
        PenNode::TextInput(_) => "Text input",
        PenNode::Image(_) => "Image",
        PenNode::IconFont(_) => "Icon",
        PenNode::Ref(_) => "Component instance",
    }
}

/// "Name (Kind)" when the node has an authored name, else just the
/// kind word.
fn node_label(node: &PenNode) -> String {
    match node.base().name.as_deref().filter(|n| !n.trim().is_empty()) {
        Some(name) => format!("{name} ({})", kind_label(node)),
        None => kind_label(node).to_string(),
    }
}

/// Selection-status text. The anchor (most recently added id) names
/// the selection; multi-selects append a count.
fn selection_summary(state: &EditorState) -> String {
    if state.selection.is_empty() {
        return SELECTION_CLEARED.to_string();
    }
    let anchor_label =
        op_editor_core::walkers::find_node(state.active_children(), &state.selection.anchor)
            .map(node_label)
            .unwrap_or_else(|| "layer".to_string());
    match state.selection.len() {
        1 => format!("Selected {anchor_label}"),
        n => format!("Selected {n} layers — {anchor_label} and {} more", n - 1),
    }
}

/// Layers-landmark summary: active page name + top-level layer count.
/// Full per-layer tree mirroring is deferred (see module docs).
fn layers_summary(state: &EditorState) -> String {
    let page_name = state
        .doc
        .pages
        .as_ref()
        .filter(|pages| !pages.is_empty())
        .map(|pages| {
            let i = state.ui.active_page_index.min(pages.len() - 1);
            pages[i].name.clone()
        })
        .unwrap_or_else(|| "Page 1".to_string());
    let count = state.active_children().len();
    let noun = if count == 1 { "layer" } else { "layers" };
    format!("{page_name} — {count} top-level {noun}")
}

/// Which painted text input currently owns typing, as
/// `(identity key, label, live value)`. The priority order mirrors
/// the host keyboard routing: the settings draft swallows keystrokes
/// first, then a focused property field, then whichever search-box
/// popover is open (it owns typed characters while open), then the
/// chat input.
fn focused_input(state: &EditorState) -> Option<(String, String, String)> {
    let ui = &state.editor_ui;
    if ui.agent_settings_open && ui.agent_settings.focus.is_some() {
        return Some((
            "settings".to_string(),
            "Settings field".to_string(),
            ui.settings_input.text().to_owned(),
        ));
    }
    if let Some(focus) = state.ui.property_focus {
        return Some((
            format!("property:{focus:?}"),
            property_focus_label(focus),
            state.ui.property_input.text().to_owned(),
        ));
    }
    if ui.icon_picker.open {
        return Some((
            "icon-search".to_string(),
            "Icon search".to_string(),
            ui.icon_picker_search.clone(),
        ));
    }
    if ui.chat_model_picker.open {
        return Some((
            "model-search".to_string(),
            "Model search".to_string(),
            ui.chat_model_picker_input.text().to_owned(),
        ));
    }
    if ui.component_browser_open {
        return Some((
            "component-search".to_string(),
            "Component search".to_string(),
            ui.component_browser_search.clone(),
        ));
    }
    if state.chat.focused {
        return Some((
            "chat".to_string(),
            "AI chat input".to_string(),
            state.chat.input.text().to_owned(),
        ));
    }
    None
}

/// Spoken label for a property-panel input row.
fn property_focus_label(focus: PropertyFocus) -> String {
    match focus {
        PropertyFocus::PositionX => "X position".to_string(),
        PropertyFocus::PositionY => "Y position".to_string(),
        PropertyFocus::Rotation => "Rotation".to_string(),
        PropertyFocus::PositionR => "Corner radius".to_string(),
        PropertyFocus::SizeW => "Width".to_string(),
        PropertyFocus::SizeH => "Height".to_string(),
        PropertyFocus::LayoutGap => "Layout gap".to_string(),
        PropertyFocus::PaddingTop => "Padding top".to_string(),
        PropertyFocus::PaddingRight => "Padding right".to_string(),
        PropertyFocus::PaddingBottom => "Padding bottom".to_string(),
        PropertyFocus::PaddingLeft => "Padding left".to_string(),
        PropertyFocus::Opacity => "Opacity".to_string(),
        PropertyFocus::FillHex => "Fill color".to_string(),
        PropertyFocus::FillOpacity => "Fill opacity".to_string(),
        PropertyFocus::GradientAngle => "Gradient angle".to_string(),
        PropertyFocus::GradientStopHex(i) => format!("Gradient stop {} color", i + 1),
        PropertyFocus::GradientStopOffset(i) => format!("Gradient stop {} offset", i + 1),
        PropertyFocus::PolygonSides => "Polygon sides".to_string(),
        PropertyFocus::EllipseStart => "Arc start angle".to_string(),
        PropertyFocus::EllipseSweep => "Arc sweep angle".to_string(),
        PropertyFocus::EllipseInnerRadius => "Inner radius".to_string(),
        PropertyFocus::FontSize => "Font size".to_string(),
        PropertyFocus::FontWeight => "Font weight".to_string(),
        PropertyFocus::LineHeight => "Line height".to_string(),
        PropertyFocus::LetterSpacing => "Letter spacing".to_string(),
        PropertyFocus::StrokeHex => "Stroke color".to_string(),
        PropertyFocus::StrokeWidth => "Stroke width".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::{ContainerProps, PenNodeBase, RectangleNode};
    use op_editor_core::node_id::NodeId;
    use op_editor_core::selection::SelectionState;

    fn rect(id: &str, name: Option<&str>) -> PenNode {
        PenNode::Rectangle(RectangleNode {
            base: PenNodeBase {
                id: id.to_string(),
                name: name.map(|s| s.to_string()),
                ..Default::default()
            },
            container: ContainerProps::default(),
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    #[test]
    fn tool_labels_are_nonempty_for_all_tools() {
        for tool in Tool::ALL {
            assert!(!tool_label(tool).is_empty());
        }
    }

    #[test]
    fn selection_summary_handles_empty_named_and_multi() {
        let mut state = EditorState::new();
        assert_eq!(selection_summary(&state), SELECTION_CLEARED);

        state
            .active_children_mut()
            .extend([rect("r1", Some("Hero")), rect("r2", None)]);
        state.selection = SelectionState {
            anchor: NodeId::new("r1"),
            set: vec![NodeId::new("r1")],
        };
        assert_eq!(selection_summary(&state), "Selected Hero (Rectangle)");

        state.selection = SelectionState {
            anchor: NodeId::new("r2"),
            set: vec![NodeId::new("r1"), NodeId::new("r2")],
        };
        assert_eq!(
            selection_summary(&state),
            "Selected 2 layers — Rectangle and 1 more"
        );
    }

    #[test]
    fn selection_summary_falls_back_when_anchor_is_gone() {
        let mut state = EditorState::new();
        state.selection = SelectionState {
            anchor: NodeId::new("missing"),
            set: vec![NodeId::new("missing")],
        };
        assert_eq!(selection_summary(&state), "Selected layer");
    }

    #[test]
    fn layers_summary_counts_top_level_layers() {
        let mut state = EditorState::new();
        // Fresh single-page-fallback document: no `pages`, no nodes.
        assert_eq!(layers_summary(&state), "Page 1 — 0 top-level layers");
        state.active_children_mut().push(rect("r1", None));
        assert_eq!(layers_summary(&state), "Page 1 — 1 top-level layer");
    }

    #[test]
    fn focused_input_prefers_property_focus_over_chat() {
        let mut state = EditorState::new();
        assert!(focused_input(&state).is_none());

        state.chat.focused = true;
        state.chat.set_input_text("hello");
        let (key, label, value) = focused_input(&state).expect("chat focus");
        assert_eq!(key, "chat");
        assert_eq!(label, "AI chat input");
        assert_eq!(value, "hello");

        state.ui.property_focus = Some(PropertyFocus::SizeW);
        state.ui.property_input.set_text("120");
        let (key, label, value) = focused_input(&state).expect("property focus");
        assert_eq!(key, "property:SizeW");
        assert_eq!(label, "Width");
        assert_eq!(value, "120");
    }

    #[test]
    fn focused_input_reports_open_search_popovers() {
        let mut state = EditorState::new();
        state.editor_ui.icon_picker.open = true;
        state.editor_ui.icon_picker_search = "arrow".to_string();
        let (key, _label, value) = focused_input(&state).expect("icon search");
        assert_eq!(key, "icon-search");
        assert_eq!(value, "arrow");
    }

    #[test]
    fn property_focus_labels_cover_indexed_variants() {
        assert_eq!(
            property_focus_label(PropertyFocus::GradientStopHex(0)),
            "Gradient stop 1 color"
        );
        assert_eq!(
            property_focus_label(PropertyFocus::StrokeWidth),
            "Stroke width"
        );
    }

    #[test]
    fn node_label_uses_name_then_kind() {
        assert_eq!(node_label(&rect("a", Some("Card"))), "Card (Rectangle)");
        assert_eq!(node_label(&rect("b", None)), "Rectangle");
        assert_eq!(node_label(&rect("c", Some("   "))), "Rectangle");
        let group = op_editor_core::pen_node_ext::make_group("g1".to_string(), "Nav", vec![]);
        assert_eq!(node_label(&group), "Nav (Group)");
    }
}
