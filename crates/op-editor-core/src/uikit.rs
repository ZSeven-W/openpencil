//! UIKit data model + built-in kit.
//!
//! The Rust counterpart of the TS `apps/web/src/types/uikit.ts` +
//! `apps/web/src/uikit/`: a `UIKit` is a named bundle of reusable
//! [`KitComponent`]s, each backed by a [`PenNode`] template the
//! Component-Browser panel instantiates on click. Built-in kits live
//! here as code; imported `.kit` files (TS feature) are out of scope
//! for v1 — the browser surface this enables is the missing piece.

use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::{
    FrameNode, PenNode, PenNodeBase, RectangleNode, TextContent, TextNode,
};
use jian_ops_schema::sizing::SizingBehavior;

use crate::fills::{set_primary_fill_hex, set_primary_stroke_hex};
use crate::pen_node_ext::PenNodeExt;
use crate::{walkers, EditorState, NodeId};

/// Browser-side grouping for a [`KitComponent`]. Mirrors the TS
/// `ComponentCategory` enum so locale-table keys line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentCategory {
    Buttons,
    Inputs,
    Cards,
    Navigation,
    Layout,
    Feedback,
    DataDisplay,
    Other,
}

/// One reusable component in a [`UIKit`]. The `template` is the
/// `PenNode` tree the browser clones on click — its ids carry the
/// `kit-<slug>` prefix so they never collide with the editor's
/// numeric `nXX` allocator (the deep-clone walker rewrites them).
#[derive(Debug, Clone)]
pub struct KitComponent {
    /// Stable id of the component within its kit (browser key).
    pub id: String,
    /// Display label shown on the card.
    pub name: String,
    /// Category pill the component sits under.
    pub category: ComponentCategory,
    /// Free-text tags — the browser's search filter substring-matches.
    pub tags: Vec<String>,
    /// Width hint, used to centre the instantiated node on the
    /// viewport.
    pub width: f32,
    /// Height hint.
    pub height: f32,
    /// `PenNode` template — cloned with fresh ids on instantiate.
    pub template: PenNode,
}

/// A bundle of reusable components — built-in or (future) imported.
#[derive(Debug, Clone)]
pub struct UIKit {
    pub id: String,
    pub name: String,
    pub built_in: bool,
    pub components: Vec<KitComponent>,
}

// ---------------------------------------------------------------------------
// Tree builders — small helpers so the per-component builders stay terse.
// ---------------------------------------------------------------------------

fn base(id: &str, name: &str, x: i32, y: i32) -> PenNodeBase {
    PenNodeBase {
        id: id.to_string(),
        name: Some(name.to_string()),
        x: Some(x as f64),
        y: Some(y as f64),
        ..Default::default()
    }
}

fn sizing(w: i32, h: i32) -> (SizingBehavior, SizingBehavior) {
    (
        SizingBehavior::Number(w.max(0) as f64),
        SizingBehavior::Number(h.max(0) as f64),
    )
}

/// A leaf `Rectangle` with a solid fill.
fn rect(id: &str, name: &str, x: i32, y: i32, w: i32, h: i32, fill_hex: &str) -> PenNode {
    let (sw, sh) = sizing(w, h);
    let mut node = PenNode::Rectangle(RectangleNode {
        base: base(id, name, x, y),
        container: ContainerProps {
            width: Some(sw),
            height: Some(sh),
            ..Default::default()
        },
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    });
    set_primary_fill_hex(&mut node, fill_hex);
    node
}

/// A leaf `Text` node with `content` as plain text.
fn text(id: &str, content: &str, x: i32, y: i32, w: i32, h: i32) -> PenNode {
    let (sw, sh) = sizing(w, h);
    PenNode::Text(TextNode {
        base: base(id, content, x, y),
        width: Some(sw),
        height: Some(sh),
        content: TextContent::Plain(content.to_string()),
        font_family: None,
        font_size: None,
        font_weight: None,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill: None,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// A `Frame` container with a solid fill and pre-built children.
#[allow(clippy::too_many_arguments)]
fn frame(
    id: &str,
    name: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill_hex: Option<&str>,
    children: Vec<PenNode>,
) -> PenNode {
    let (sw, sh) = sizing(w, h);
    let mut node = PenNode::Frame(FrameNode {
        base: base(id, name, x, y),
        container: ContainerProps {
            width: Some(sw),
            height: Some(sh),
            ..Default::default()
        },
        children: Some(children),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    });
    if let Some(hex) = fill_hex {
        set_primary_fill_hex(&mut node, hex);
    }
    node
}

/// A `Frame` with a primary stroke (no fill) — the input / card outline.
#[allow(clippy::too_many_arguments)]
fn outlined(
    id: &str,
    name: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    stroke_hex: &str,
    children: Vec<PenNode>,
) -> PenNode {
    let mut node = frame(id, name, x, y, w, h, Some("#FFFFFF"), children);
    set_primary_stroke_hex(&mut node, stroke_hex);
    node
}

// ---------------------------------------------------------------------------
// Built-in components — one per major category. Each template's child
// ids are unique within the template tree; the deep-clone walker
// rewrites them with fresh `nXX` ids on instantiate.
// ---------------------------------------------------------------------------

fn primary_button() -> PenNode {
    frame(
        "kit/btn-primary/root",
        "Primary Button",
        0,
        0,
        120,
        40,
        Some("#3366FF"),
        vec![text("kit/btn-primary/label", "Button", 32, 12, 56, 16)],
    )
}

fn text_input() -> PenNode {
    outlined(
        "kit/input-text/root",
        "Text Input",
        0,
        0,
        240,
        40,
        "#D0D5DD",
        vec![text(
            "kit/input-text/placeholder",
            "Placeholder",
            12,
            12,
            216,
            16,
        )],
    )
}

fn basic_card() -> PenNode {
    outlined(
        "kit/card-basic/root",
        "Card",
        0,
        0,
        280,
        160,
        "#E5E7EB",
        vec![
            text("kit/card-basic/title", "Card title", 20, 20, 240, 20),
            text(
                "kit/card-basic/body",
                "A short description of what the card contains.",
                20,
                48,
                240,
                40,
            ),
        ],
    )
}

fn nav_bar() -> PenNode {
    frame(
        "kit/nav-bar/root",
        "Nav Bar",
        0,
        0,
        480,
        56,
        Some("#FFFFFF"),
        vec![
            text("kit/nav-bar/brand", "Brand", 20, 18, 80, 20),
            text("kit/nav-bar/link-1", "Home", 200, 18, 60, 20),
            text("kit/nav-bar/link-2", "About", 270, 18, 60, 20),
            text("kit/nav-bar/link-3", "Contact", 340, 18, 80, 20),
        ],
    )
}

fn divider() -> PenNode {
    rect("kit/divider/root", "Divider", 0, 0, 240, 1, "#E5E7EB")
}

fn badge() -> PenNode {
    frame(
        "kit/badge/root",
        "Badge",
        0,
        0,
        60,
        24,
        Some("#3366FF"),
        vec![text("kit/badge/label", "New", 16, 5, 32, 14)],
    )
}

/// The built-in OpenPencil starter kit — one component per major
/// category so every browser pill has something to show.
pub fn builtin_starter_kit() -> UIKit {
    fn comp(
        id: &str,
        name: &str,
        category: ComponentCategory,
        tags: &[&str],
        width: f32,
        height: f32,
        template: PenNode,
    ) -> KitComponent {
        KitComponent {
            id: id.to_string(),
            name: name.to_string(),
            category,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            width,
            height,
            template,
        }
    }
    UIKit {
        id: "openpencil-starter".to_string(),
        name: "OpenPencil Starter".to_string(),
        built_in: true,
        components: vec![
            comp(
                "btn-primary",
                "Primary Button",
                ComponentCategory::Buttons,
                &["button", "primary", "cta"],
                120.0,
                40.0,
                primary_button(),
            ),
            comp(
                "input-text",
                "Text Input",
                ComponentCategory::Inputs,
                &["input", "text", "field"],
                240.0,
                40.0,
                text_input(),
            ),
            comp(
                "card-basic",
                "Card",
                ComponentCategory::Cards,
                &["card", "container", "surface"],
                280.0,
                160.0,
                basic_card(),
            ),
            comp(
                "nav-bar",
                "Nav Bar",
                ComponentCategory::Navigation,
                &["navbar", "header", "navigation"],
                480.0,
                56.0,
                nav_bar(),
            ),
            comp(
                "divider",
                "Divider",
                ComponentCategory::Layout,
                &["divider", "separator", "line"],
                240.0,
                1.0,
                divider(),
            ),
            comp(
                "badge",
                "Badge",
                ComponentCategory::Feedback,
                &["badge", "tag", "label"],
                60.0,
                24.0,
                badge(),
            ),
        ],
    }
}

/// All built-in kits the editor ships with. v1 has one starter kit;
/// future kits append here.
pub fn builtin_kits() -> Vec<UIKit> {
    vec![builtin_starter_kit()]
}

impl EditorState {
    /// Instantiate a UIKit component onto the active page: deep-clone
    /// the template with fresh ids, place its top-left at `(doc_x,
    /// doc_y)`, rename to the component label, push, single-select,
    /// and snapshot the document for undo. Returns the new node's id;
    /// `None` when the kit / component is missing or the id allocator
    /// is exhausted.
    pub fn instantiate_kit_component(
        &mut self,
        kit_id: &str,
        component_id: &str,
        doc_x: f64,
        doc_y: f64,
    ) -> Option<NodeId> {
        let (template, label) = {
            let comp = self
                .ui_kits
                .iter()
                .find(|k| k.id == kit_id)?
                .components
                .iter()
                .find(|c| c.id == component_id)?;
            (comp.template.clone(), comp.name.clone())
        };
        let snap = self.snapshot_for_history();
        let mut next_id = self.max_node_id().checked_add(1)?;
        let mut taken = std::collections::HashSet::new();
        let mut clone = walkers::deep_clone_with_new_ids(&template, &mut next_id, &mut taken);
        // Children carry document-absolute coords, so positioning the
        // instantiated component is a subtree translate from the
        // template's root origin to `(doc_x, doc_y)` — moving only the
        // root would leave descendants at the template coordinates.
        let dx = doc_x - clone.base().x.unwrap_or(0.0);
        let dy = doc_y - clone.base().y.unwrap_or(0.0);
        walkers::translate_subtree(&mut clone, dx, dy);
        clone.base_mut().name = Some(label);
        let new_id = NodeId::new(clone.base().id.clone());
        self.active_children_mut().push(clone);
        self.set_single_selection(new_id.clone());
        self.history_push_past(snap);
        Some(new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_kit_covers_six_categories() {
        let kit = builtin_starter_kit();
        assert_eq!(kit.id, "openpencil-starter");
        assert!(kit.built_in);
        assert_eq!(kit.components.len(), 6);
        // Every category we ship at least one of must be distinct.
        let cats: std::collections::HashSet<_> =
            kit.components.iter().map(|c| c.category).collect();
        assert_eq!(cats.len(), 6);
    }

    #[test]
    fn instantiate_clones_template_with_fresh_ids_and_selects() {
        let mut state = EditorState::new();
        let before = state.active_children_mut().len();
        let id = state
            .instantiate_kit_component("openpencil-starter", "btn-primary", 100.0, 200.0)
            .expect("instantiated");
        assert_eq!(state.active_children_mut().len(), before + 1);
        let inserted = state.active_children_mut().last().unwrap();
        assert_eq!(inserted.base().id, id.as_str());
        // Fresh `nXX` id, not the kit's template prefix.
        assert!(
            inserted.base().id.starts_with('n'),
            "expected fresh n-id, got {:?}",
            inserted.base().id
        );
        // Position written through to the canonical base.
        assert_eq!(inserted.base().x, Some(100.0));
        assert_eq!(inserted.base().y, Some(200.0));
        // Selection now points at the new node.
        assert_eq!(state.selection.anchor, id);
    }

    #[test]
    fn instantiate_translates_descendants_into_doc_space() {
        // The Primary Button template has a child label at (32, 12)
        // relative to its (0, 0) root. After instantiating at
        // (200, 300), the label's absolute position must be
        // (232, 312) — `instantiate_kit_component` translates the
        // whole subtree, not only the root.
        let mut state = EditorState::new();
        let _ = state
            .instantiate_kit_component("openpencil-starter", "btn-primary", 200.0, 300.0)
            .expect("instantiated");
        let root = state.active_children_mut().last().expect("root").clone();
        assert_eq!(root.base().x, Some(200.0));
        assert_eq!(root.base().y, Some(300.0));
        let label = match &root {
            PenNode::Frame(f) => f
                .children
                .as_ref()
                .and_then(|c| c.first())
                .cloned()
                .expect("label"),
            _ => panic!("expected Frame"),
        };
        assert_eq!(label.base().x, Some(232.0));
        assert_eq!(label.base().y, Some(312.0));
    }

    #[test]
    fn instantiate_into_empty_pages_doc_lands_visibly() {
        // A `pages: Some([])` document — legal on the wire even if
        // the page-mutator layer normally normalizes it away —
        // mustn't trap inserts somewhere `active_children` cannot
        // see. After an instantiate, `active_children` must include
        // the new node (read / write sides agree on the fallback).
        let mut state = EditorState::new();
        state.doc.pages = Some(Vec::new());
        let before = state.active_children().len();
        let id = state
            .instantiate_kit_component("openpencil-starter", "badge", 0.0, 0.0)
            .expect("instantiated");
        let after = state.active_children();
        assert_eq!(after.len(), before + 1, "insert must be visible");
        assert_eq!(after.last().unwrap().base().id, id.as_str());
    }

    #[test]
    fn empty_pages_insert_survives_subsequent_add_page() {
        // A component inserted while `pages: Some([])` lands in
        // `doc.children` (via the read/write fallback). A later
        // `add_page` must migrate those nodes into Page 1 rather than
        // mint a fresh Page 1 alongside them — otherwise the original
        // insert is stranded and disappears from view.
        let mut state = EditorState::new();
        state.doc.pages = Some(Vec::new());
        let _ = state
            .instantiate_kit_component("openpencil-starter", "badge", 0.0, 0.0)
            .expect("instantiated");
        let new_idx = state.add_page().expect("added page");
        // add_page switches to the new page; flip back to Page 1.
        state.ui.active_page_index = 0;
        let active = state.active_children();
        assert_eq!(
            active.len(),
            1,
            "Page 1 must inherit the pre-add_page insert (got {} children, new_idx={})",
            active.len(),
            new_idx,
        );
    }

    #[test]
    fn instantiate_unknown_kit_returns_none() {
        let mut state = EditorState::new();
        assert!(state
            .instantiate_kit_component("no-such-kit", "btn-primary", 0.0, 0.0)
            .is_none());
    }

    #[test]
    fn template_ids_are_unique_within_a_component() {
        for comp in builtin_starter_kit().components {
            let mut seen = std::collections::HashSet::new();
            fn walk(n: &PenNode, seen: &mut std::collections::HashSet<String>) -> bool {
                let id = match n {
                    PenNode::Frame(f) => &f.base.id,
                    PenNode::Group(g) => &g.base.id,
                    PenNode::Rectangle(r) => &r.base.id,
                    PenNode::Text(t) => &t.base.id,
                    _ => return true,
                };
                if !seen.insert(id.clone()) {
                    return false;
                }
                let children = match n {
                    PenNode::Frame(f) => f.children.as_deref(),
                    PenNode::Group(g) => g.children.as_deref(),
                    PenNode::Rectangle(r) => r.children.as_deref(),
                    _ => None,
                };
                children.into_iter().flatten().all(|c| walk(c, seen))
            }
            assert!(
                walk(&comp.template, &mut seen),
                "duplicate id in {}",
                comp.id
            );
        }
    }
}
