//! Hidden ARIA DOM mirror for the CanvasKit web shell (#57).
//!
//! The editor renders to an opaque `<canvas>`, so a screen reader sees
//! nothing. This module builds a parallel, visually-hidden DOM tree —
//! one focusable element per accessibility node — from the same
//! `accesskit::TreeUpdate` the platform-free assembler
//! (`op_editor_ui::accessibility`) produces. Each mirror element carries
//! an ARIA `role`, an `aria-label`, and a `tabindex` so VoiceOver /
//! Narrator / Orca can read + tab through the editor's regions.
//!
//! The mirror is NOT `display:none` (which removes it from the
//! accessibility tree); it uses the standard `.sr-only` clip technique so
//! it stays screen-reader-visible while being invisible + zero-area on
//! screen.
//!
//! ## Why this module is platform-only (no `WidgetHost`)
//!
//! It depends solely on `accesskit` + `web-sys`, so it compile-checks
//! under BOTH the production `canvaskit` feature and the wasm32-clean
//! `web` stub feature. The host-side enumeration that *produces* the
//! `TreeUpdate` (and routes DOM events back into editor state) lives in
//! `widget_host/a11y_bridge.rs`, which is `canvaskit`-only because it
//! pulls `op-editor-core`.
//!
//! ## Update strategy (v1)
//!
//! Rebuild-on-change: each refresh clears the container and re-creates one
//! element per node. The tree is tiny (~8 always-present regions), so a
//! full rebuild per dirty frame is cheaper than diffing and keeps the
//! NodeId→element mapping trivially consistent. The map is retained so the
//! event-routing layer can resolve a focused/clicked element back to its
//! `accesskit::NodeId`.
//!
//! Under the `web` stub feature the mirror compiles for CI coverage but
//! has no caller (the host enumeration is `canvaskit`-only), so its public
//! surface reads as dead code there — silenced below.
#![cfg_attr(not(feature = "canvaskit"), allow(dead_code))]

use std::collections::HashMap;

use accesskit::{NodeId, Role, TreeUpdate};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// `data-*` attribute holding the stable `accesskit::NodeId` of a mirror
/// element. The event-routing layer reads it back to map a DOM event to a
/// host action.
pub(crate) const NODE_ID_ATTR: &str = "data-op-a11y-id";

/// The id of the mirror container element appended next to the canvas.
const CONTAINER_ID: &str = "op-a11y-mirror";

/// Standard screen-reader-only style: zero-area, clipped, off-screen, but
/// NOT `display:none` (which would drop it from the accessibility tree).
/// Applied to the container; children inherit visibility via the same
/// clip on the container box.
const SR_ONLY_STYLE: &str = "position:absolute;width:1px;height:1px;\
padding:0;margin:-1px;overflow:hidden;clip:rect(0 0 0 0);\
clip-path:inset(50%);white-space:nowrap;border:0;left:0;top:0;";

/// The hidden ARIA DOM mirror. Owns the container element and the live
/// `NodeId → element` map rebuilt on every `update`.
pub(crate) struct A11yDomMirror {
    document: Document,
    container: HtmlElement,
    /// NodeId → focusable mirror element. Rebuilt each `update`; retained
    /// so event routing can resolve a DOM target back to its NodeId.
    elements: HashMap<NodeId, HtmlElement>,
    /// Hash of the last tree we rendered. The CanvasKit mount calls `update`
    /// after EVERY repaint (caret blink, hover…); skipping the rebuild when
    /// the tree is unchanged avoids destroying + recreating the focused
    /// element each frame (which would churn screen-reader / keyboard focus).
    /// `None` until the first build.
    signature: Option<u64>,
}

impl A11yDomMirror {
    /// Create the mirror container and append it as a sibling of `canvas`
    /// (or, failing that, to `<body>`). Returns `None` only if the DOM is
    /// unreachable (non-browser host).
    pub(crate) fn create(canvas: &web_sys::HtmlCanvasElement) -> Option<Self> {
        let document = canvas
            .owner_document()
            .or_else(|| web_sys::window().and_then(|w| w.document()))?;

        // Reuse an existing container across re-mounts so we don't leak a
        // second hidden tree onto the page.
        let container: HtmlElement = match document.get_element_by_id(CONTAINER_ID) {
            Some(el) => el.dyn_into::<HtmlElement>().ok()?,
            None => {
                let el = document.create_element("div").ok()?;
                el.set_id(CONTAINER_ID);
                let el: HtmlElement = el.dyn_into::<HtmlElement>().ok()?;
                let _ = el.set_attribute("style", SR_ONLY_STYLE);
                // `role=application` tells the screen reader this subtree is
                // an interactive app surface, not a document to be read
                // linearly — so arrow keys reach the canvas shortcuts.
                let _ = el.set_attribute("role", "application");
                let _ = el.set_attribute("aria-label", "OpenPencil editor");
                // Insert as a sibling right after the canvas when possible
                // (keeps it adjacent in source order); else append to body.
                if let Some(parent) = canvas.parent_node() {
                    let _ = parent.append_child(&el);
                } else if let Some(body) = document.body() {
                    let _ = body.append_child(&el);
                }
                el
            }
        };

        Some(Self {
            document,
            container,
            elements: HashMap::new(),
            signature: None,
        })
    }

    /// Rebuild the mirror from a freshly assembled `TreeUpdate`.
    ///
    /// Clears the container and re-creates one focusable element per node
    /// (skipping the synthetic root `Window`, which the container already
    /// represents as `role=application`). The map is repopulated so the
    /// event-routing layer can resolve targets.
    pub(crate) fn update(&mut self, tree: &TreeUpdate) {
        // Skip the rebuild entirely when the tree is unchanged — the mount
        // calls this every repaint, and a full rebuild would destroy the
        // focused element each frame, churning focus (see `signature`).
        let signature = tree_signature(tree);
        if self.signature == Some(signature) {
            return;
        }
        self.signature = Some(signature);

        // Remember whether focus currently lives inside the mirror, so we can
        // restore it after the rebuild without stealing focus from elsewhere
        // on the page.
        let focus_was_in_mirror = self
            .document
            .active_element()
            .map(|active| self.container.contains(active.dyn_ref::<web_sys::Node>()))
            .unwrap_or(false);

        // Clear previous mirror.
        self.container.set_inner_html("");
        self.elements.clear();

        // The first node is the root host frame (`ROOT_WIDGET_ID`,
        // Role::Window) — the container stands in for it, so mirror only
        // its descendants. Falling back to mirroring all nodes is safe if
        // the assembler ever changes the ordering.
        let root_id = tree.tree.as_ref().map(|t| t.root);

        for (id, node) in &tree.nodes {
            if Some(*id) == root_id {
                continue;
            }
            if let Some(el) = self.build_node_element(*id, node) {
                let _ = self.container.append_child(&el);
                self.elements.insert(*id, el);
            }
        }

        // Restore focus to the tree's focus target if focus had been in the
        // mirror — otherwise the rebuild silently drops it.
        if focus_was_in_mirror {
            if let Some(el) = self.elements.get(&tree.focus) {
                let _ = el.focus();
            }
        }
    }

    /// Build one focusable mirror element for an accessibility node.
    fn build_node_element(&self, id: NodeId, node: &accesskit::Node) -> Option<HtmlElement> {
        let el = self.document.create_element("div").ok()?;
        let el: HtmlElement = el.dyn_into::<HtmlElement>().ok()?;

        let _ = el.set_attribute("role", aria_role_for(node.role()));
        // Label precedence: explicit value (e.g. property panel kind) wins
        // over the static region label, mirroring how a sighted user reads
        // the selected node's kind in the property panel header.
        let label = node.value().or_else(|| node.label()).unwrap_or("");
        if !label.is_empty() {
            let _ = el.set_attribute("aria-label", label);
        }
        // Every mirror node is tab-focusable so a keyboard / screen-reader
        // user can move through the editor's regions in reading order.
        let _ = el.set_attribute("tabindex", "0");
        let _ = el.set_attribute(NODE_ID_ATTR, &id.0.to_string());

        Some(el)
    }

    /// The mirror container element — event-routing attaches delegated
    /// `focus` / `click` listeners here.
    pub(crate) fn container(&self) -> &HtmlElement {
        &self.container
    }

    /// Resolve a DOM `EventTarget` (the focused / clicked element) back to
    /// its `accesskit::NodeId` by reading the `data-*` id attribute.
    /// Returns `None` for targets outside the mirror.
    pub(crate) fn node_id_for_target(target: &Element) -> Option<NodeId> {
        let raw = target.get_attribute(NODE_ID_ATTR)?;
        raw.parse::<u64>().ok().map(NodeId)
    }
}

/// Map an `accesskit::Role` to the closest standard ARIA role token.
///
/// Only the roles the editor's always-present regions actually emit are
/// mapped to specific ARIA roles; everything else falls back to `group`
/// (a neutral, valid landmark-less container) so an unmapped role never
/// produces an invalid `role=` attribute.
///
/// | accesskit Role     | region            | ARIA role     |
/// | ------------------ | ----------------- | ------------- |
/// | `Window`           | host root frame   | `application` |
/// | `Header`           | TopBar            | `banner`      |
/// | `Tree`             | LayerPanel        | `tree`        |
/// | `Canvas`           | CanvasViewport    | `img`         |
/// | `Toolbar`          | Toolbar           | `toolbar`     |
/// | `Group`            | PropertyPanel /   | `group`       |
/// |                    | AIChat / StatusBar|               |
/// | `GenericContainer` | misc container    | `group`       |
/// | `Dialog`           | modal overlay     | `dialog`      |
/// | `ListBox`          | dropdown list     | `listbox`     |
/// | `Menu`             | context menu      | `menu`        |
fn aria_role_for(role: Role) -> &'static str {
    match role {
        // The host root frame; the container already carries this, but map
        // it for completeness if a caller mirrors the root directly.
        Role::Window => "application",
        // The TopBar is the editor's banner region.
        Role::Header => "banner",
        Role::Tree => "tree",
        // A canvas has no DOM-native semantic; `img` with a label is the
        // conventional way to expose an opaque graphic to a screen reader.
        Role::Canvas => "img",
        Role::Toolbar => "toolbar",
        Role::Dialog => "dialog",
        Role::ListBox => "listbox",
        Role::Menu => "menu",
        // PropertyPanel / AIChat / StatusBar all advertise `Group`; a plain
        // labelled `group` is the right neutral container.
        Role::Group | Role::GenericContainer => "group",
        // Any role not surfaced by the v1 region set: a valid neutral
        // container so the attribute is never malformed.
        _ => "group",
    }
}

/// Hash of the parts of a tree the mirror renders (node ids + roles +
/// labels/values + focus). Used to skip a DOM rebuild when the tree is
/// unchanged between repaints, which keeps screen-reader focus stable.
///
/// Per-node sub-hashes are combined with XOR so the signature depends only on
/// the SET of nodes + their content + focus, NOT on the `nodes` vec iteration
/// order — a future reorder (or a map-backed source) can't spuriously force a
/// rebuild. Node ids are unique, so distinct nodes never cancel.
fn tree_signature(tree: &TreeUpdate) -> u64 {
    use std::hash::{Hash, Hasher};

    fn node_hash(id: NodeId, node: &accesskit::Node) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        id.0.hash(&mut h);
        // `Role` has no `Hash` impl; its `Debug` form is stable + cheap for
        // the ~8-node region tree.
        format!("{:?}", node.role()).hash(&mut h);
        node.label().unwrap_or("").hash(&mut h);
        node.value().unwrap_or("").hash(&mut h);
        h.finish()
    }

    let mut acc: u64 = 0;
    for (id, node) in &tree.nodes {
        acc ^= node_hash(*id, node);
    }
    let mut fh = std::collections::hash_map::DefaultHasher::new();
    tree.focus.0.hash(&mut fh);
    acc ^ fh.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_is_stable_and_label_sensitive() {
        fn node(role: Role, label: &str) -> accesskit::Node {
            let mut n = accesskit::Node::new(role);
            n.set_label(label);
            n
        }
        let mk = |label: &str| TreeUpdate {
            nodes: vec![(NodeId(1), node(Role::Toolbar, label))],
            tree: Some(accesskit::Tree::new(NodeId(0))),
            tree_id: accesskit::TreeId::ROOT,
            focus: NodeId(1),
        };
        // Same tree → same signature (no rebuild); changed label → different.
        assert_eq!(tree_signature(&mk("A")), tree_signature(&mk("A")));
        assert_ne!(tree_signature(&mk("A")), tree_signature(&mk("B")));
    }

    #[test]
    fn signature_is_independent_of_node_order() {
        fn node(role: Role, label: &str) -> accesskit::Node {
            let mut n = accesskit::Node::new(role);
            n.set_label(label);
            n
        }
        let a = (NodeId(1), node(Role::Toolbar, "Toolbar"));
        let b = (NodeId(2), node(Role::Tree, "Layers"));
        let forward = TreeUpdate {
            nodes: vec![a.clone(), b.clone()],
            tree: Some(accesskit::Tree::new(NodeId(0))),
            tree_id: accesskit::TreeId::ROOT,
            focus: NodeId(1),
        };
        let reversed = TreeUpdate {
            nodes: vec![b, a],
            tree: Some(accesskit::Tree::new(NodeId(0))),
            tree_id: accesskit::TreeId::ROOT,
            focus: NodeId(1),
        };
        // Same node set + focus, different vec order → identical signature.
        assert_eq!(tree_signature(&forward), tree_signature(&reversed));
    }

    #[test]
    fn known_roles_map_to_expected_aria_tokens() {
        assert_eq!(aria_role_for(Role::Window), "application");
        assert_eq!(aria_role_for(Role::Header), "banner");
        assert_eq!(aria_role_for(Role::Tree), "tree");
        assert_eq!(aria_role_for(Role::Canvas), "img");
        assert_eq!(aria_role_for(Role::Toolbar), "toolbar");
        assert_eq!(aria_role_for(Role::Group), "group");
        assert_eq!(aria_role_for(Role::GenericContainer), "group");
        assert_eq!(aria_role_for(Role::Dialog), "dialog");
        assert_eq!(aria_role_for(Role::ListBox), "listbox");
        assert_eq!(aria_role_for(Role::Menu), "menu");
    }

    #[test]
    fn unmapped_role_falls_back_to_group() {
        // A role outside the v1 region set must still yield a valid token.
        assert_eq!(aria_role_for(Role::Button), "group");
    }
}
