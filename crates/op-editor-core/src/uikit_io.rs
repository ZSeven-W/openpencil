//! Kit import / export — the Rust counterpart of the TS
//! `apps/web/src/uikit/kit-import-export.ts` + `kit-utils.ts` +
//! `uikit-store.ts` action surface.
//!
//! The kit FILE format is just a canonical `.op` `PenDocument` whose
//! top-level `children` are the reusable component roots (plus the
//! `$variable` definitions + themes they reference) — exactly what TS
//! `exportKit` writes and `importKitFromFile` reads, so kits round-trip
//! between the TS and Rust editors unchanged.
//!
//! This module is platform-free: the desktop host owns the file
//! dialogs (`op-host-desktop/src/kit_io.rs`) and persistence
//! (`kit_persistence.rs`); presses queue a [`KitIoRequest`] on
//! `EditorUiState::component_browser_kit_request` the host drains.
//!
//! Documented divergence: the canonical Rust schema carries the
//! `reusable` flag on `Frame` nodes only (TS also persists it on
//! group / rectangle) — a kit exported from TS with non-frame
//! components imports only its frame components here.

use std::collections::BTreeSet;

use jian_ops_schema::node::base::{BoolOrExpression, NumberOrExpression};
use jian_ops_schema::node::container::{ContainerProps, Padding};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenFill, StrokeThickness};
use jian_ops_schema::PenDocument;

use crate::fills::{node_fills, node_stroke_fills};
use crate::pen_node_ext::PenNodeExt;
use crate::uikit::{ComponentCategory, KitComponent, UIKit};
use crate::variables_resolve::is_variable_ref;
use crate::EditorState;

/// A queued kit import / export request — set by a Component-Browser
/// header-button press, drained by the desktop host (which owns the
/// native file dialogs). Mirrors `DesignMdRequest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitIoRequest {
    /// Pick a `.op` / `.pen` / `.json` file and import its reusable
    /// components as a new kit (TS `importKitFromFile`).
    Import,
    /// Export the current document's reusable components as a kit
    /// file (TS `exportKit`).
    Export,
}

// ---------------------------------------------------------------------------
// Import — TS `kit-utils.ts::extractComponentsFromDocument` +
// `kit-import-export.ts::importKitFromFile`.
// ---------------------------------------------------------------------------

/// Root nodes per TS `getAllChildren` (`pen-core/tree-utils.ts:64`):
/// non-empty `pages` flat-map their children, else the top-level
/// `children` array.
fn all_children(doc: &PenDocument) -> Vec<&PenNode> {
    match doc.pages.as_ref() {
        Some(pages) if !pages.is_empty() => pages.iter().flat_map(|p| p.children.iter()).collect(),
        _ => doc.children.iter().collect(),
    }
}

/// Walk the document tree and extract reusable nodes as
/// [`KitComponent`]s, the node itself becoming the instantiate
/// template. Mirrors TS `extractComponentsFromDocument` (no meta
/// overrides — imports infer category + tags from the name).
pub fn extract_components_from_document(doc: &PenDocument) -> Vec<KitComponent> {
    fn walk(node: &PenNode, out: &mut Vec<KitComponent>) {
        if is_reusable(node) {
            let id = node.base().id.clone();
            let name = node.base().name.clone().unwrap_or_else(|| id.clone());
            out.push(KitComponent {
                id,
                category: infer_category(&name),
                tags: infer_tags(&name),
                // TS: literal number or the 100-px fallback.
                width: node.width_px().unwrap_or(100.0) as f32,
                height: node.height_px().unwrap_or(100.0) as f32,
                template: node.clone(),
                name,
            });
        }
        if let Some(children) = node.children() {
            for child in children {
                walk(child, out);
            }
        }
    }
    let mut out = Vec::new();
    for root in all_children(doc) {
        walk(root, &mut out);
    }
    out
}

/// Build a [`UIKit`] from a parsed kit document. `None` when the file
/// carries no reusable components (TS `importKitFromFile` returns
/// `null`). `kit_id` is host-minted (TS uses `nanoid()`).
pub fn import_kit_from_document(doc: &PenDocument, kit_id: String) -> Option<UIKit> {
    let components = extract_components_from_document(doc);
    if components.is_empty() {
        return None;
    }
    Some(UIKit {
        id: kit_id,
        name: doc
            .name
            .clone()
            .unwrap_or_else(|| "Imported Kit".to_string()),
        version: doc.version.clone(),
        built_in: false,
        components,
        variables: doc.variables.clone(),
    })
}

// ---------------------------------------------------------------------------
// Export — TS `kit-import-export.ts::exportKit`.
// ---------------------------------------------------------------------------

/// Build the kit `PenDocument` TS `exportKit` writes: the selected
/// reusable nodes as top-level children, plus the `$variable`
/// definitions they reference and (when variables were copied) the
/// source document's themes. `component_ids` empty = all reusable
/// components. `None` when nothing reusable matches.
pub fn build_kit_document(
    source: &PenDocument,
    component_ids: &[String],
    kit_name: &str,
) -> Option<PenDocument> {
    // TS: `doc.pages ? pages.flatMap(children) : doc.children` — note
    // the truthiness check: an empty `pages: []` still routes through
    // pages (yielding zero roots), unlike `getAllChildren`.
    let roots: Vec<&PenNode> = match source.pages.as_ref() {
        Some(pages) => pages.iter().flat_map(|p| p.children.iter()).collect(),
        None => source.children.iter().collect(),
    };
    let mut reusable = Vec::new();
    for root in roots {
        collect_reusable_nodes(root, &mut reusable);
    }
    let selected: Vec<&PenNode> = if component_ids.is_empty() {
        reusable
    } else {
        reusable
            .into_iter()
            .filter(|n| component_ids.iter().any(|id| id == &n.base().id))
            .collect()
    };
    if selected.is_empty() {
        return None;
    }

    let mut kit = PenDocument {
        version: "1.0.0".to_string(),
        name: Some(kit_name.to_string()),
        themes: None,
        variables: None,
        pages: None,
        children: selected.iter().map(|n| (*n).clone()).collect(),
        format_version: None,
        id: None,
        app: None,
        routes: None,
        state: None,
        lifecycle: None,
        logic_modules: None,
        design_md: None,
        conversion: None,
    };

    // Copy referenced variables (TS export-side `collectNodeRefs` —
    // opacity / enabled / gap / padding / solid fill colors; the
    // export collector deliberately omits strokes, unlike the
    // instantiate-side collector below).
    if let Some(source_vars) = source.variables.as_ref() {
        let mut refs = BTreeSet::new();
        for node in &selected {
            collect_export_refs(node, &mut refs);
        }
        let mut vars = std::collections::BTreeMap::new();
        for r in refs {
            let name = r.strip_prefix('$').unwrap_or(&r);
            if let Some(def) = source_vars.get(name) {
                vars.insert(name.to_string(), def.clone());
            }
        }
        if !vars.is_empty() {
            kit.variables = Some(vars);
        }
    }
    // Copy themes if variables were included.
    if kit.variables.is_some() {
        kit.themes = source.themes.clone();
    }
    Some(kit)
}

/// Default export file name — TS:
/// `` `${kitName.replace(/[^a-zA-Z0-9-_ ]/g, '').trim() || 'kit'}.op` ``.
pub fn kit_export_file_name(kit_name: &str) -> String {
    let cleaned: String = kit_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .collect();
    let cleaned = cleaned.trim();
    let stem = if cleaned.is_empty() { "kit" } else { cleaned };
    format!("{stem}.op")
}

// ---------------------------------------------------------------------------
// Store actions — TS `uikit-store.ts::importKit / removeKit`.
// ---------------------------------------------------------------------------

impl EditorState {
    /// Append an imported kit (TS `importKit`). Raises the
    /// `ui_kits_changed` persistence-dirty flag the desktop host
    /// drains into `uikits.json`.
    pub fn import_kit(&mut self, kit: UIKit) {
        self.ui_kits.push(kit);
        self.editor_ui.ui_kits_changed = true;
    }

    /// Remove an imported kit by id (TS `removeKit`): built-in kits
    /// are never removed; the kit filter resets when it pointed at
    /// the removed kit. Returns `true` when a kit was removed.
    pub fn remove_kit(&mut self, kit_id: &str) -> bool {
        let before = self.ui_kits.len();
        self.ui_kits.retain(|k| k.id != kit_id || k.built_in);
        let removed = self.ui_kits.len() != before;
        if self.editor_ui.component_browser_kit_id.as_deref() == Some(kit_id) && removed {
            self.editor_ui.component_browser_kit_id = None;
        }
        self.editor_ui.component_browser_confirm_delete_kit = None;
        if removed {
            self.editor_ui.ui_kits_changed = true;
        }
        removed
    }
}

// ---------------------------------------------------------------------------
// Reusable walk + `$variable` collectors.
// ---------------------------------------------------------------------------

/// The canonical schema persists `reusable` on Frame nodes only (see
/// module docs for the TS divergence).
fn is_reusable(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(f) if f.reusable == Some(true))
}

/// TS `collectReusableNodes` — depth-first, reusable parents AND any
/// reusable descendants are all collected.
fn collect_reusable_nodes<'a>(node: &'a PenNode, out: &mut Vec<&'a PenNode>) {
    if is_reusable(node) {
        out.push(node);
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_reusable_nodes(child, out);
        }
    }
}

fn expr_ref(value: &NumberOrExpression, refs: &mut BTreeSet<String>) {
    if let NumberOrExpression::Expression(e) = value {
        if is_variable_ref(e) {
            refs.insert(e.clone());
        }
    }
}

fn container_refs(container: &ContainerProps, refs: &mut BTreeSet<String>) {
    if let Some(gap) = container.gap.as_ref() {
        expr_ref(gap, refs);
    }
    if let Some(Padding::Expression(e)) = container.padding.as_ref() {
        if is_variable_ref(e) {
            refs.insert(e.clone());
        }
    }
}

fn fill_refs(fills: &[PenFill], refs: &mut BTreeSet<String>) {
    for fill in fills {
        if let PenFill::Solid(body) = fill {
            if is_variable_ref(&body.color) {
                refs.insert(body.color.clone());
            }
        }
    }
}

fn base_refs(node: &PenNode, refs: &mut BTreeSet<String>) {
    let base = node.base();
    if let Some(opacity) = base.opacity.as_ref() {
        expr_ref(opacity, refs);
    }
    if let Some(BoolOrExpression::Expression(e)) = base.enabled.as_ref() {
        if is_variable_ref(e) {
            refs.insert(e.clone());
        }
    }
}

fn node_container(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

/// Export-side collector — TS `kit-import-export.ts::collectNodeRefs`
/// (opacity / enabled / gap / padding / solid fill colors; NO stroke).
fn collect_export_refs(node: &PenNode, refs: &mut BTreeSet<String>) {
    base_refs(node, refs);
    if let Some(container) = node_container(node) {
        container_refs(container, refs);
    }
    if let Some(fills) = node_fills(node) {
        fill_refs(fills, refs);
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_export_refs(child, refs);
        }
    }
}

/// Instantiate-side collector — TS `kit-utils.ts::collectRefs`
/// (export-side set PLUS stroke solid-fill colors; the TS stroke
/// `thickness` check never fires on the Rust schema — `thickness` is
/// numeric-only here, divergence noted inline).
pub(crate) fn collect_template_variable_refs(node: &PenNode, refs: &mut BTreeSet<String>) {
    base_refs(node, refs);
    if let Some(container) = node_container(node) {
        container_refs(container, refs);
    }
    if let Some(fills) = node_fills(node) {
        fill_refs(fills, refs);
    }
    if let Some(stroke_fills) = node_stroke_fills(node) {
        fill_refs(stroke_fills, refs);
    }
    // TS also collects `stroke.thickness` when it is a `$ref` string;
    // `jian_ops_schema::style::StrokeThickness` has no expression arm,
    // so that branch is structurally unreachable here.
    let _ = StrokeThickness::Uniform(0.0);
    if let Some(children) = node.children() {
        for child in children {
            collect_template_variable_refs(child, refs);
        }
    }
}

// ---------------------------------------------------------------------------
// Category + tag inference — TS `kit-utils.ts`.
// ---------------------------------------------------------------------------

/// TS `CATEGORY_KEYWORDS`, in object-insertion (= match) order.
const CATEGORY_KEYWORDS: &[(ComponentCategory, &[&str])] = &[
    (ComponentCategory::Buttons, &["button", "btn", "cta"]),
    (
        ComponentCategory::Inputs,
        &[
            "input",
            "text field",
            "textarea",
            "checkbox",
            "radio",
            "toggle",
            "switch",
            "select",
            "form",
        ],
    ),
    (ComponentCategory::Cards, &["card", "tile"]),
    (
        ComponentCategory::Navigation,
        &["nav", "navbar", "tab", "breadcrumb", "menu", "sidebar"],
    ),
    (
        ComponentCategory::Layout,
        &["divider", "separator", "spacer", "container", "grid"],
    ),
    (
        ComponentCategory::Feedback,
        &[
            "alert", "banner", "toast", "badge", "tag", "chip", "avatar", "tooltip",
        ],
    ),
    (
        ComponentCategory::DataDisplay,
        &["table", "list", "stat", "chart", "progress"],
    ),
];

fn infer_category(name: &str) -> ComponentCategory {
    let lower = name.to_lowercase();
    for (category, keywords) in CATEGORY_KEYWORDS {
        for kw in *keywords {
            if lower.contains(kw) {
                return *category;
            }
        }
    }
    ComponentCategory::Other
}

/// TS `inferTags` — lowercase, strip `[^a-z0-9\s-]`, split on
/// whitespace / hyphens, drop empties.
fn infer_tags(name: &str) -> Vec<String> {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c.is_whitespace() || *c == '-')
        .collect::<String>()
        .split(|c: char| c.is_whitespace() || c == '-')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::style::SolidFillBody;
    use jian_ops_schema::variable::{
        VariableDefinition, VariableKind, VariableScalar, VariableValue,
    };

    fn reusable_frame(id: &str, name: &str, fill_color: Option<&str>) -> PenNode {
        let mut frame = match crate::uikit::builtin_starter_kit()
            .components
            .into_iter()
            .find(|c| c.id == "btn-primary")
            .map(|c| c.template)
        {
            Some(PenNode::Frame(f)) => f,
            _ => panic!("starter kit btn-primary must be a frame"),
        };
        frame.base.id = id.to_string();
        frame.base.name = Some(name.to_string());
        frame.reusable = Some(true);
        if let Some(color) = fill_color {
            frame.container.fill = Some(vec![PenFill::Solid(SolidFillBody {
                color: color.to_string(),
                explain: None,
                opacity: None,
                blend_mode: None,
            })]);
        }
        PenNode::Frame(frame)
    }

    fn color_var(hex: &str) -> VariableDefinition {
        VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str(hex.to_string())),
        }
    }

    fn doc_with(children: Vec<PenNode>) -> PenDocument {
        PenDocument {
            version: "1.0.0".to_string(),
            name: Some("Source".to_string()),
            themes: None,
            variables: None,
            pages: None,
            children,
            format_version: None,
            id: None,
            app: None,
            routes: None,
            state: None,
            lifecycle: None,
            logic_modules: None,
            design_md: None,
            conversion: None,
        }
    }

    #[test]
    fn build_kit_document_collects_reusable_nodes_and_variables() {
        let mut doc = doc_with(vec![
            reusable_frame("c1", "Primary Button", Some("$brand")),
            reusable_frame("c2", "Stat Card", None),
        ]);
        let mut vars = std::collections::BTreeMap::new();
        vars.insert("brand".to_string(), color_var("#3366FF"));
        vars.insert("unused".to_string(), color_var("#000000"));
        doc.variables = Some(vars);
        doc.themes = Some(std::collections::BTreeMap::from([(
            "Theme".to_string(),
            vec!["Light".to_string(), "Dark".to_string()],
        )]));

        let kit = build_kit_document(&doc, &[], "My Kit").expect("kit");
        assert_eq!(kit.version, "1.0.0");
        assert_eq!(kit.name.as_deref(), Some("My Kit"));
        assert_eq!(kit.children.len(), 2);
        // Only the REFERENCED variable is copied.
        let kit_vars = kit.variables.expect("vars");
        assert!(kit_vars.contains_key("brand"));
        assert!(!kit_vars.contains_key("unused"));
        // Themes ride along when variables were included.
        assert!(kit.themes.is_some());
    }

    #[test]
    fn build_kit_document_filters_by_component_ids() {
        let doc = doc_with(vec![
            reusable_frame("c1", "Button", None),
            reusable_frame("c2", "Card", None),
        ]);
        let kit = build_kit_document(&doc, &["c2".to_string()], "K").expect("kit");
        assert_eq!(kit.children.len(), 1);
        assert_eq!(kit.children[0].base().id, "c2");
        // No variables referenced → no themes either.
        assert!(kit.variables.is_none());
        assert!(kit.themes.is_none());
    }

    #[test]
    fn build_kit_document_without_reusable_nodes_returns_none() {
        let doc = doc_with(vec![]);
        assert!(build_kit_document(&doc, &[], "K").is_none());
    }

    #[test]
    fn kit_export_file_name_sanitizes_like_ts() {
        assert_eq!(kit_export_file_name("My Kit"), "My Kit.op");
        assert_eq!(kit_export_file_name("  Kit/!@#  "), "Kit.op");
        assert_eq!(kit_export_file_name("///"), "kit.op");
        assert_eq!(kit_export_file_name(""), "kit.op");
    }

    #[test]
    fn import_kit_from_document_extracts_components_with_inference() {
        let mut doc = doc_with(vec![
            reusable_frame("c1", "Primary Button", None),
            reusable_frame("c2", "Mystery Widget", None),
        ]);
        doc.name = Some("Design System".to_string());
        let kit = import_kit_from_document(&doc, "kit-1".to_string()).expect("kit");
        assert_eq!(kit.id, "kit-1");
        assert_eq!(kit.name, "Design System");
        assert_eq!(kit.version, "1.0.0");
        assert!(!kit.built_in);
        assert_eq!(kit.components.len(), 2);
        let button = &kit.components[0];
        assert_eq!(button.category, ComponentCategory::Buttons);
        assert_eq!(button.tags, vec!["primary", "button"]);
        assert_eq!(button.width, 120.0);
        assert_eq!(button.height, 40.0);
        assert_eq!(kit.components[1].category, ComponentCategory::Other);
    }

    #[test]
    fn import_kit_from_document_without_components_returns_none() {
        let doc = doc_with(vec![]);
        assert!(import_kit_from_document(&doc, "kit-1".to_string()).is_none());
    }

    #[test]
    fn import_uses_pages_when_present() {
        let mut doc = doc_with(vec![]);
        doc.pages = Some(vec![jian_ops_schema::page::PenPage {
            id: "p1".to_string(),
            name: "Page 1".to_string(),
            children: vec![reusable_frame("c1", "Nav Bar", None)],
            state: None,
            lifecycle: None,
        }]);
        let kit = import_kit_from_document(&doc, "kit-1".to_string()).expect("kit");
        assert_eq!(kit.components.len(), 1);
        assert_eq!(kit.components[0].category, ComponentCategory::Navigation);
    }

    #[test]
    fn remove_kit_protects_builtins_and_resets_filter() {
        let mut state = EditorState::new();
        // Built-in kits never delete.
        assert!(!state.remove_kit("openpencil-starter"));
        assert!(state.ui_kits.iter().any(|k| k.id == "openpencil-starter"));

        let doc = doc_with(vec![reusable_frame("c1", "Button", None)]);
        let kit = import_kit_from_document(&doc, "imp-1".to_string()).expect("kit");
        state.import_kit(kit);
        assert!(state.editor_ui.ui_kits_changed);
        state.editor_ui.ui_kits_changed = false;
        state.editor_ui.component_browser_kit_id = Some("imp-1".to_string());

        assert!(state.remove_kit("imp-1"));
        assert!(!state.ui_kits.iter().any(|k| k.id == "imp-1"));
        assert_eq!(state.editor_ui.component_browser_kit_id, None);
        assert!(state.editor_ui.ui_kits_changed);
    }

    #[test]
    fn instantiate_strips_the_reusable_flag_from_the_clone() {
        // TS `component-browser-card.tsx:36-40` deletes `reusable` on
        // the inserted clone so a later kit export does not re-collect
        // instantiated copies.
        let doc = doc_with(vec![reusable_frame("c1", "Button", None)]);
        let mut state = EditorState::new();
        let kit = import_kit_from_document(&doc, "imp-1".to_string()).expect("kit");
        state.import_kit(kit);
        let _ = state
            .instantiate_kit_component("imp-1", "c1", 0.0, 0.0)
            .expect("instantiated");
        let inserted = state.active_children().last().expect("inserted").clone();
        match inserted {
            PenNode::Frame(f) => assert_eq!(f.reusable, None),
            other => panic!("expected Frame, got {other:?}"),
        }
        // The next export must find nothing reusable on this document.
        assert!(build_kit_document(&state.doc, &[], "K").is_none());
    }

    #[test]
    fn instantiate_copies_referenced_kit_variables_without_overwriting() {
        let mut doc = doc_with(vec![reusable_frame("c1", "Button", Some("$brand"))]);
        let mut vars = std::collections::BTreeMap::new();
        vars.insert("brand".to_string(), color_var("#3366FF"));
        vars.insert("unreferenced".to_string(), color_var("#111111"));
        doc.variables = Some(vars);

        let mut state = EditorState::new();
        // Pre-existing target definition must NOT be overwritten (TS:
        // `varDef && !document.variables?.[name]`).
        state
            .doc
            .variables
            .get_or_insert_with(Default::default)
            .insert("brand".to_string(), color_var("#FF0000"));
        let kit = import_kit_from_document(&doc, "imp-1".to_string()).expect("kit");
        state.import_kit(kit);

        let id = state.instantiate_kit_component("imp-1", "c1", 0.0, 0.0);
        assert!(id.is_some());
        let target_vars = state.doc.variables.as_ref().expect("vars");
        assert_eq!(target_vars.get("brand"), Some(&color_var("#FF0000")));
        assert!(!target_vars.contains_key("unreferenced"));

        // A missing target definition IS copied.
        let mut state2 = EditorState::new();
        let kit2 = import_kit_from_document(&doc, "imp-2".to_string()).expect("kit");
        state2.import_kit(kit2);
        let id2 = state2.instantiate_kit_component("imp-2", "c1", 0.0, 0.0);
        assert!(id2.is_some());
        let vars2 = state2.doc.variables.as_ref().expect("vars");
        assert_eq!(vars2.get("brand"), Some(&color_var("#3366FF")));
    }
}
