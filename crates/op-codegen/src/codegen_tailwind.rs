//! React + Tailwind CSS + shadcn/ui generator (`react-tailwind`).
//!
//! Deterministic: the same document always yields byte-identical output
//! (ordered maps/sets only, no clocks, no model). Emits one TSX
//! component whose styling is Tailwind utilities — design variables map
//! to token classes (`bg-background`, `text-foreground`,
//! `border-border`), auto layout to `flex`/`gap-*`/`justify-*`/`items-*`,
//! sizes and padding to the spacing scale with `[13px]` arbitrary values
//! only when off-scale, text styles to `text-*`/`font-*`/`leading-*` —
//! and instances of the built-in shadcn UIKit to real shadcn/ui
//! component usages imported from `@/components/ui/*`. Alongside it,
//! `globals.css` (Tailwind v4) and `tailwind.config.ts` (Tailwind v3)
//! bind the variables per theme, light and dark.
//!
//! Module layout: this spine owns the public surface, the emission
//! context and the ref pre-pass; siblings own class mapping (`style`,
//! `scale`), JSX emission (`jsx`, `widgets`), kit recognition
//! (`shadcn`, `shadcn_blocks`) and the theme files (`theme`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::ref_resolve::{resolve_refs_for_canvas_roots, roots_have_refs};

use crate::{root_nodes, Codegen};

#[path = "codegen_tailwind_jsx.rs"]
mod jsx;
#[path = "codegen_tailwind_scale.rs"]
mod scale;
#[path = "codegen_tailwind_shadcn.rs"]
mod shadcn;
#[path = "codegen_tailwind_shadcn_blocks.rs"]
mod shadcn_blocks;
#[path = "codegen_tailwind_style.rs"]
mod style;
#[path = "codegen_tailwind_theme.rs"]
mod theme;
#[path = "codegen_tailwind_widgets.rs"]
mod widgets;

use style::Parent;

/// Module path shadcn/ui components are imported from.
const UI_ALIAS: &str = "@/components/ui";

/// Tokens the stock shadcn/ui components style themselves with. Bound
/// whenever a shadcn component is emitted so a `<Button>` has its
/// `bg-primary` even when the design itself never referenced `primary`.
const SHADCN_CORE_TOKENS: &[&str] = &[
    "background",
    "foreground",
    "card",
    "card-foreground",
    "popover",
    "popover-foreground",
    "primary",
    "primary-foreground",
    "secondary",
    "secondary-foreground",
    "muted",
    "muted-foreground",
    "accent",
    "accent-foreground",
    "destructive",
    "destructive-foreground",
    "border",
    "input",
    "ring",
];

/// React + Tailwind CSS + shadcn/ui generator.
pub struct ReactTailwind;

/// The generator's three artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailwindFiles {
    /// `Page.tsx`-style component source.
    pub component: String,
    /// `app/globals.css` for Tailwind v4 + shadcn/ui.
    pub globals_css: String,
    /// `tailwind.config.ts` for Tailwind v3 projects.
    pub tailwind_config: String,
}

impl TailwindFiles {
    /// One paste-able file: the component, then the two theme files as
    /// trailing line-comment sections. Line comments (not a block
    /// comment) because the config's `**/*` globs would close one.
    pub fn to_single_file(&self) -> String {
        let mut out = self.component.clone();
        push_commented(
            &mut out,
            "app/globals.css (Tailwind CSS v4 + shadcn/ui)",
            &self.globals_css,
        );
        push_commented(
            &mut out,
            "tailwind.config.ts (Tailwind CSS v3 projects; start globals.css with the @tailwind directives instead of @import)",
            &self.tailwind_config,
        );
        out
    }
}

fn push_commented(out: &mut String, title: &str, body: &str) {
    out.push_str(&format!("\n// ───────── {title} ─────────\n"));
    for line in body.lines() {
        if line.is_empty() {
            out.push_str("//\n");
        } else {
            out.push_str("// ");
            out.push_str(line);
            out.push('\n');
        }
    }
}

/// Mutable state threaded through one emission.
pub(crate) struct Ctx {
    radius: scale::RadiusScale,
    /// Design tokens referenced by emitted classes (bare names).
    tokens: BTreeSet<String>,
    /// module → imported names.
    imports: BTreeMap<String, BTreeSet<String>>,
}

impl Ctx {
    /// Import a shadcn/ui component (`Button` → `@/components/ui/button`).
    fn ui(&mut self, component: &str) {
        let module = ui_module(component);
        self.import(&format!("{UI_ALIAS}/{module}"), component);
    }

    fn import(&mut self, module: &str, name: &str) {
        self.imports
            .entry(module.to_string())
            .or_default()
            .insert(name.to_string());
    }
}

/// shadcn/ui file a component is exported from.
fn ui_module(component: &str) -> &'static str {
    const FAMILIES: &[(&str, &str)] = &[
        ("AlertDialog", "alert-dialog"),
        ("Alert", "alert"),
        ("Avatar", "avatar"),
        ("Badge", "badge"),
        ("Breadcrumb", "breadcrumb"),
        ("Button", "button"),
        ("Card", "card"),
        ("Checkbox", "checkbox"),
        ("Input", "input"),
        ("Label", "label"),
        ("Progress", "progress"),
        ("RadioGroup", "radio-group"),
        ("Select", "select"),
        ("Separator", "separator"),
        ("Skeleton", "skeleton"),
        ("Slider", "slider"),
        ("Switch", "switch"),
        ("Table", "table"),
        ("Tabs", "tabs"),
        ("Textarea", "textarea"),
    ];
    FAMILIES
        .iter()
        .find(|(prefix, _)| component.starts_with(prefix))
        .map(|(_, module)| *module)
        .unwrap_or("button")
}

impl Codegen for ReactTailwind {
    fn target_label(&self) -> &'static str {
        "react-tailwind"
    }

    fn generate(&self, doc: &PenDocument) -> String {
        self.generate_files(doc).to_single_file()
    }
}

impl ReactTailwind {
    /// Generate the component and theme files for `doc`'s first page
    /// (or bare children).
    pub fn generate_files(&self, doc: &PenDocument) -> TailwindFiles {
        self.generate_files_with_components(doc, &[])
    }

    /// Like [`Self::generate_files`], with extra component definitions
    /// available to resolve `ref` instances whose target lives outside
    /// `doc` (a codegen selection carries only the selected subtrees).
    pub fn generate_files_with_components(
        &self,
        doc: &PenDocument,
        components: &[PenNode],
    ) -> TailwindFiles {
        let roots = expand_refs(doc, components);
        let mut ctx = Ctx {
            radius: scale::RadiusScale::new(theme::document_radius_px(doc)),
            tokens: BTreeSet::new(),
            imports: BTreeMap::new(),
        };
        let visible: Vec<&PenNode> = roots
            .iter()
            .filter(|n| n.base().visible != Some(false))
            .collect();
        let mut body = String::new();
        let single = visible.len() == 1 && !matches!(visible[0], PenNode::Ref(_));
        if visible.is_empty() {
            body.push_str("  return null\n");
        } else if single {
            body.push_str("  return (\n");
            jsx::emit_node(&mut ctx, &mut body, visible[0], Parent::Root, 2);
            body.push_str("  )\n");
        } else {
            body.push_str("  return (\n    <>\n");
            for node in &visible {
                jsx::emit_node(&mut ctx, &mut body, node, Parent::Root, 3);
            }
            body.push_str("    </>\n  )\n");
        }

        let mut component = String::from("// Generated by OpenPencil — codegen::ReactTailwind\n");
        for (module, names) in &ctx.imports {
            let names = names.iter().cloned().collect::<Vec<_>>().join(", ");
            component.push_str(&format!("import {{ {names} }} from \"{module}\"\n"));
        }
        if !ctx.imports.is_empty() {
            component.push('\n');
        }
        let name = component_name(&visible);
        component.push_str(&format!("export default function {name}() {{\n"));
        component.push_str(&body);
        component.push_str("}\n");

        if ctx
            .imports
            .keys()
            .any(|module| module.starts_with(UI_ALIAS))
        {
            ctx.tokens
                .extend(SHADCN_CORE_TOKENS.iter().map(|t| (*t).to_string()));
        }
        let tables = theme::ThemeTables::build(doc, &ctx.tokens);
        TailwindFiles {
            component,
            globals_css: tables.globals_css(),
            tailwind_config: tables.tailwind_config(),
        }
    }
}

/// PascalCase component name from a single root's layer name, else `Page`.
fn component_name(roots: &[&PenNode]) -> String {
    let [root] = roots else {
        return "Page".into();
    };
    let name = root.base().name.as_deref().unwrap_or_default();
    let mut out = String::new();
    for word in name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.starts_with(|c: char| c.is_ascii_alphabetic()) {
        out
    } else {
        "Page".into()
    }
}

// --- Component instances ---------------------------------------------

fn index<'a>(nodes: &'a [PenNode], by_id: &mut HashMap<&'a str, &'a PenNode>) {
    for node in nodes {
        by_id.entry(node.base().id.as_str()).or_insert(node);
        if let Some(children) = node.children() {
            index(children, by_id);
        }
    }
}

fn collect_ref_targets(nodes: &[PenNode], out: &mut BTreeSet<String>) {
    for node in nodes {
        if let PenNode::Ref(reference) = node {
            out.insert(reference.target.clone());
        }
        if let Some(children) = node.children() {
            collect_ref_targets(children, out);
        }
    }
}

/// Mark every ref whose target is a shadcn kit component so the kit
/// identity survives expansion (the instance's `role` overrides the
/// component's in `ref_resolve`).
fn tag_kit_refs(nodes: &mut [PenNode], kits: &BTreeMap<String, &'static str>) {
    for node in nodes {
        if let PenNode::Ref(reference) = node {
            if let Some(kit_id) = kits.get(&reference.target) {
                reference.base.role = Some(format!("{}{kit_id}", shadcn::KIT_ROLE_PREFIX));
            }
        }
        if let Some(children) = node.children_mut() {
            tag_kit_refs(children, kits);
        }
    }
}

/// Top-level nodes with every component instance expanded; kit
/// instances keep a role marker for [`shadcn::detect`].
fn expand_refs(doc: &PenDocument, components: &[PenNode]) -> Vec<PenNode> {
    let mut roots = root_nodes(doc).to_vec();
    if !roots_have_refs(&roots) {
        return roots;
    }
    let mut lookup = doc.clone();
    lookup.children.extend(components.iter().cloned());
    let mut targets = BTreeSet::new();
    collect_ref_targets(&roots, &mut targets);
    for page in lookup.pages.iter().flatten() {
        collect_ref_targets(&page.children, &mut targets);
    }
    collect_ref_targets(&lookup.children, &mut targets);

    // Kit templates stand in for kit targets the document lacks.
    let mut known = HashMap::new();
    for page in lookup.pages.iter().flatten() {
        index(&page.children, &mut known);
    }
    index(&lookup.children, &mut known);
    let missing: Vec<String> = targets
        .iter()
        .filter(|t| !known.contains_key(t.as_str()) && shadcn::kit_for_id(t).is_some())
        .cloned()
        .collect();
    let kit_ids: BTreeMap<String, &'static str> = targets
        .iter()
        .filter_map(|target| {
            let by_id = shadcn::KIT_COMPONENTS
                .iter()
                .find(|(id, _, _)| id == target)
                .map(|(id, _, _)| *id);
            let by_component = known.get(target.as_str()).and_then(|component| {
                let base = component.base();
                shadcn::KIT_COMPONENTS
                    .iter()
                    .find(|(id, _, _)| *id == base.id)
                    .map(|(id, _, _)| *id)
                    .or_else(|| base.name.as_deref().and_then(shadcn::kit_id_for_name))
            });
            by_id.or(by_component).map(|id| (target.clone(), id))
        })
        .collect();
    drop(known);
    if !missing.is_empty() {
        let kit = op_editor_core::uikit_shadcn::shadcn_kit();
        for id in &missing {
            if let Some(component) = kit.components.iter().find(|c| &c.id == id) {
                lookup.children.push(component.template.clone());
            }
        }
    }

    tag_kit_refs(&mut roots, &kit_ids);
    for page in lookup.pages.iter_mut().flatten() {
        tag_kit_refs(&mut page.children, &kit_ids);
        clear_master_origins(&mut page.children, &targets);
    }
    tag_kit_refs(&mut lookup.children, &kit_ids);
    clear_master_origins(&mut lookup.children, &targets);
    resolve_refs_for_canvas_roots(&roots, &lookup)
}

/// A master's `x`/`y` is its spot on the component sheet; an instance
/// inherits it through the prop merge and would otherwise be emitted as
/// absolutely positioned inside a flex parent. Only the lookup copy is
/// touched, never the emitted roots.
fn clear_master_origins(nodes: &mut [PenNode], targets: &BTreeSet<String>) {
    for node in nodes {
        if targets.contains(&node.base().id) {
            let base = node.base_mut();
            base.x = None;
            base.y = None;
        }
        if let Some(children) = node.children_mut() {
            clear_master_origins(children, targets);
        }
    }
}

/// `(kit id, kit name)` pairs the recognizer knows — lets tests pin the
/// table to the embedded kit.
#[cfg(test)]
pub(crate) fn kit_component_names() -> Vec<(&'static str, &'static str)> {
    shadcn::KIT_COMPONENTS
        .iter()
        .map(|(id, name, _)| (*id, *name))
        .collect()
}

/// Component definitions the `ref` instances inside `roots` (and inside
/// those definitions, transitively) point at, looked up anywhere in
/// `doc`. Hosts pass these along with a selection so the generator can
/// resolve instances whose masters were not selected.
pub fn referenced_components(roots: &[&PenNode], doc: &PenDocument) -> Vec<PenNode> {
    let mut pending = BTreeSet::new();
    for root in roots {
        collect_ref_targets(std::slice::from_ref(*root), &mut pending);
    }
    if pending.is_empty() {
        // Instance-free selections (the common case) skip the
        // whole-document index.
        return Vec::new();
    }
    let mut by_id = HashMap::new();
    for page in doc.pages.iter().flatten() {
        index(&page.children, &mut by_id);
    }
    index(&doc.children, &mut by_id);
    let mut selected = HashMap::new();
    for root in roots {
        index(std::slice::from_ref(*root), &mut selected);
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    while let Some(target) = pending.pop_first() {
        if !seen.insert(target.clone()) || selected.contains_key(target.as_str()) {
            continue;
        }
        if let Some(component) = by_id.get(target.as_str()) {
            collect_ref_targets(std::slice::from_ref(*component), &mut pending);
            out.push((*component).clone());
        }
    }
    out
}
