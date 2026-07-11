//! Figma → PenDocument top-level mapping — ports `figma-node-mapper.ts`.
//! Resolves style references, builds the tree, and converts each
//! user page into a `PenDocument`.

use crate::common::{collect_image_blobs, ConversionContext, FigLayoutMode};
use crate::converters::{convert_children, convert_node};
use crate::figma_types::FigmaDecodedFile;
use crate::kiwi::FigValue;
use crate::tree::{
    build_tree, build_tree_for_clipboard, collect_components, collect_symbol_tree, guid_to_string,
    is_user_page, TreeNode,
};
use jian_ops_schema::document::PenDocument;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;
use std::collections::HashMap;
use std::rc::Rc;

/// Outcome of a full-document Figma import.
pub struct FigmaImportResult {
    pub document: PenDocument,
    pub warnings: Vec<String>,
    /// In-blob image bytes keyed by blob index.
    pub image_blobs: HashMap<u32, Vec<u8>>,
}

/// Outcome of a clipboard-style Figma import — a flat `PenNode` list
/// without a document wrapper, mirroring TS
/// `figmaNodeChangesToPenNodes`'s `{ nodes, warnings, imageBlobs }`
/// return shape.
pub struct FigmaClipboardResult {
    pub nodes: Vec<PenNode>,
    pub warnings: Vec<String>,
    /// In-blob image bytes keyed by blob index.
    pub image_blobs: HashMap<u32, Vec<u8>>,
}

fn empty_document(name: &str) -> PenDocument {
    PenDocument {
        version: "1".to_string(),
        name: Some(name.to_string()),
        themes: None,
        variables: None,
        pages: Some(Vec::new()),
        children: Vec::new(),
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

fn document_with_pages(name: &str, pages: Vec<PenPage>) -> PenDocument {
    PenDocument {
        pages: Some(pages),
        ..empty_document(name)
    }
}

fn pen_page(id: String, name: String, children: Vec<PenNode>) -> PenPage {
    PenPage {
        id,
        name,
        children,
        state: None,
        lifecycle: None,
    }
}

/// Copy style-node values inline onto the nodes that reference them,
/// so downstream converters never chase a style ref. Mutates the
/// node-change list in place.
pub fn resolve_style_references(node_changes: &mut [FigValue]) {
    let mut style_map: HashMap<String, FigValue> = HashMap::new();
    for nc in node_changes.iter() {
        if nc.get("styleType").is_some() {
            if let Some(key) = nc.get("guid").and_then(guid_to_string) {
                style_map.insert(key, nc.clone());
            }
            // Library styles are referenced by `assetRef.key` — index
            // the embedded style node under its publish key too. The
            // "key:" namespace keeps hex publish keys from ever
            // colliding with "session:local" guid strings.
            if let Some(asset_key) = nc.get_str("key") {
                if !asset_key.is_empty() {
                    style_map.insert(format!("key:{asset_key}"), nc.clone());
                }
            }
        }
    }
    if style_map.is_empty() {
        return;
    }
    for nc in node_changes.iter_mut() {
        resolve_on_node(nc, &style_map);
        // Resolve style refs inside instance symbol overrides too.
        if let Some(mut symbol_data) = nc.get("symbolData").cloned() {
            if let Some(overrides) = symbol_data.get_array("symbolOverrides") {
                let mut resolved: Vec<FigValue> = overrides.to_vec();
                for ov in &mut resolved {
                    resolve_on_node(ov, &style_map);
                }
                symbol_data.set("symbolOverrides", FigValue::Array(resolved));
                nc.set("symbolData", symbol_data);
            }
        }
    }
}

fn lookup_style<'a>(
    nc: &FigValue,
    field: &str,
    style_map: &'a HashMap<String, FigValue>,
) -> Option<&'a FigValue> {
    let sid = nc.get(field)?;
    // Local style: guid reference. Library style: assetRef publish key.
    if let Some(key) = sid.get("guid").and_then(guid_to_string) {
        return style_map.get(&key);
    }
    let asset_key = sid.get("assetRef").and_then(|a| a.get_str("key"))?;
    style_map.get(&format!("key:{asset_key}"))
}

fn non_empty_array(v: &FigValue, key: &str) -> bool {
    v.get_array(key).map(|a| !a.is_empty()).unwrap_or(false)
}

fn resolve_on_node(nc: &mut FigValue, style_map: &HashMap<String, FigValue>) {
    // FILL.
    if let Some(fs) = lookup_style(nc, "styleIdForFill", style_map) {
        if non_empty_array(fs, "fillPaints") {
            if let Some(v) = fs.get("fillPaints").cloned() {
                nc.set("fillPaints", v);
            }
        }
    }
    // STROKE — reads the style node's fillPaints, writes strokePaints.
    if let Some(ss) = lookup_style(nc, "styleIdForStrokeFill", style_map) {
        if non_empty_array(ss, "fillPaints") {
            if let Some(v) = ss.get("fillPaints").cloned() {
                nc.set("strokePaints", v);
            }
        }
    }
    // TEXT — only fills a field the node lacks.
    if let Some(ts) = lookup_style(nc, "styleIdForText", style_map).cloned() {
        for key in [
            "fontName",
            "fontSize",
            "lineHeight",
            "letterSpacing",
            "textAlignHorizontal",
            "textDecoration",
            "textCase",
        ] {
            if nc.get(key).is_none() {
                if let Some(v) = ts.get(key).cloned() {
                    nc.set(key, v);
                }
            }
        }
        if !non_empty_array(nc, "fillPaints") && non_empty_array(&ts, "fillPaints") {
            if let Some(v) = ts.get("fillPaints").cloned() {
                nc.set("fillPaints", v);
            }
        }
    }
    // EFFECT.
    if let Some(es) = lookup_style(nc, "styleIdForEffect", style_map) {
        if non_empty_array(es, "effects") && !non_empty_array(nc, "effects") {
            if let Some(v) = es.get("effects").cloned() {
                nc.set("effects", v);
            }
        }
    }
}

/// Convert every user page of a decoded `.fig` into one multi-page
/// `PenDocument`.
pub fn figma_all_pages_to_pen_document(
    mut decoded: FigmaDecodedFile,
    file_name: &str,
    layout_mode: FigLayoutMode,
) -> FigmaImportResult {
    resolve_style_references(&mut decoded.node_changes);
    let image_blobs = collect_image_blobs(&decoded.blobs);

    let Some(tree) = build_tree(&decoded.node_changes) else {
        return FigmaImportResult {
            document: empty_document(file_name),
            warnings: vec!["No document root found".to_string()],
            image_blobs,
        };
    };

    let pages: Vec<&TreeNode> = tree
        .children
        .iter()
        .filter(|c| c.figma.get_str("type") == Some("CANVAS"))
        .filter(|c| is_user_page(c))
        .collect();
    if pages.is_empty() {
        return FigmaImportResult {
            document: empty_document(file_name),
            warnings: vec!["No pages found in Figma file".to_string()],
            image_blobs,
        };
    }

    let mut component_map: HashMap<String, String> = HashMap::new();
    let mut symbol_tree: HashMap<String, Rc<TreeNode>> = HashMap::new();
    let mut counter: u32 = 1;
    for page in &pages {
        collect_components(page, &mut component_map, &mut counter);
    }
    collect_symbol_tree(&tree, &mut symbol_tree);
    let mut instance_assignments: HashMap<String, String> = HashMap::new();
    crate::instance::seed_assignments_from_instances(
        &tree,
        &symbol_tree,
        &mut instance_assignments,
    );

    let mut ctx = ConversionContext {
        component_map,
        symbol_tree,
        warnings: Vec::new(),
        id_counter: counter,
        blobs: decoded.blobs,
        layout_mode,
        instance_assignments,
    };

    let mut pen_pages = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        let children = convert_children(page, &mut ctx);
        let name = page
            .figma
            .get_str("name")
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Page {}", i + 1));
        pen_pages.push(pen_page(format!("figma-page-{i}"), name, children));
    }

    FigmaImportResult {
        document: document_with_pages(file_name, pen_pages),
        warnings: ctx.warnings,
        image_blobs,
    }
}

/// Convert a single page (`page_index`) of a decoded `.fig`.
pub fn figma_to_pen_document(
    mut decoded: FigmaDecodedFile,
    file_name: &str,
    page_index: usize,
    layout_mode: FigLayoutMode,
) -> FigmaImportResult {
    resolve_style_references(&mut decoded.node_changes);
    let image_blobs = collect_image_blobs(&decoded.blobs);

    let Some(tree) = build_tree(&decoded.node_changes) else {
        return FigmaImportResult {
            document: empty_document(file_name),
            warnings: vec!["No document root found".to_string()],
            image_blobs,
        };
    };
    let pages: Vec<&TreeNode> = tree.children.iter().filter(|c| is_user_page(c)).collect();
    let Some(page) = pages.get(page_index).or_else(|| pages.first()) else {
        return FigmaImportResult {
            document: empty_document(file_name),
            warnings: vec!["No pages found in Figma file".to_string()],
            image_blobs,
        };
    };

    let mut component_map: HashMap<String, String> = HashMap::new();
    let mut symbol_tree: HashMap<String, Rc<TreeNode>> = HashMap::new();
    let mut counter: u32 = 1;
    collect_components(page, &mut component_map, &mut counter);
    collect_symbol_tree(&tree, &mut symbol_tree);
    let mut instance_assignments: HashMap<String, String> = HashMap::new();
    crate::instance::seed_assignments_from_instances(
        &tree,
        &symbol_tree,
        &mut instance_assignments,
    );

    let mut ctx = ConversionContext {
        component_map,
        symbol_tree,
        warnings: Vec::new(),
        id_counter: counter,
        blobs: decoded.blobs,
        layout_mode,
        instance_assignments,
    };
    let children = convert_children(page, &mut ctx);
    let name = page
        .figma
        .get_str("name")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Page 1".to_string());
    let pen = pen_page(format!("figma-page-{page_index}"), name, children);

    FigmaImportResult {
        document: document_with_pages(file_name, vec![pen]),
        warnings: ctx.warnings,
        image_blobs,
    }
}

/// Page summaries for a decoded file — id / name / child count.
pub struct FigmaPageInfo {
    pub id: String,
    pub name: String,
    pub child_count: usize,
}

/// List the user pages of a decoded `.fig` without converting.
pub fn get_figma_pages(decoded: &FigmaDecodedFile) -> Vec<FigmaPageInfo> {
    let Some(tree) = build_tree(&decoded.node_changes) else {
        return Vec::new();
    };
    tree.children
        .iter()
        .filter(|c| is_user_page(c))
        .map(|c| FigmaPageInfo {
            id: c
                .figma
                .get("guid")
                .and_then(guid_to_string)
                .unwrap_or_default(),
            name: c.figma.get_str("name").unwrap_or("Page").to_string(),
            child_count: c.children.len(),
        })
        .collect()
}

/// Convert clipboard node changes into a flat `PenNode` list — no
/// document wrapper, no synthesised page. Matches the TS
/// `figmaNodeChangesToPenNodes` shape so a clipboard-paste caller can
/// splice the returned nodes into the active document at the cursor.
pub fn figma_node_changes_to_pen_nodes(
    mut decoded: FigmaDecodedFile,
    layout_mode: FigLayoutMode,
) -> FigmaClipboardResult {
    resolve_style_references(&mut decoded.node_changes);
    let image_blobs = collect_image_blobs(&decoded.blobs);
    let tree = build_tree(&decoded.node_changes);

    let top_nodes: Vec<TreeNode> = if let Some(tree) = &tree {
        let pages: Vec<&TreeNode> = tree.children.iter().filter(|c| is_user_page(c)).collect();
        if let Some(page) = pages.first() {
            page.children.clone()
        } else if !tree.children.is_empty() {
            tree.children.clone()
        } else {
            Vec::new()
        }
    } else {
        build_tree_for_clipboard(&decoded.node_changes)
    };

    if top_nodes.is_empty() {
        return FigmaClipboardResult {
            nodes: Vec::new(),
            warnings: vec!["No convertible nodes found".to_string()],
            image_blobs,
        };
    }

    let mut component_map: HashMap<String, String> = HashMap::new();
    let mut symbol_tree: HashMap<String, Rc<TreeNode>> = HashMap::new();
    let mut counter: u32 = 1;
    for node in &top_nodes {
        collect_components(node, &mut component_map, &mut counter);
    }
    if let Some(tree) = &tree {
        collect_symbol_tree(tree, &mut symbol_tree);
    }
    for node in &top_nodes {
        collect_symbol_tree(node, &mut symbol_tree);
    }
    let mut instance_assignments: HashMap<String, String> = HashMap::new();
    if let Some(tree) = &tree {
        crate::instance::seed_assignments_from_instances(
            tree,
            &symbol_tree,
            &mut instance_assignments,
        );
    }
    for node in &top_nodes {
        crate::instance::seed_assignments_from_instances(
            node,
            &symbol_tree,
            &mut instance_assignments,
        );
    }

    let mut ctx = ConversionContext {
        component_map,
        symbol_tree,
        warnings: Vec::new(),
        id_counter: counter,
        blobs: decoded.blobs,
        layout_mode,
        instance_assignments,
    };
    let mut nodes = Vec::new();
    for tree_node in &top_nodes {
        if tree_node.figma.get_bool("visible") == Some(false) {
            continue;
        }
        if let Some(node) = convert_node(tree_node, None, &mut ctx) {
            nodes.push(node);
        }
    }

    FigmaClipboardResult {
        nodes,
        warnings: ctx.warnings,
        image_blobs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
        FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn solid(r: f32, g: f32, b: f32) -> FigValue {
        FigValue::Array(vec![obj(vec![
            ("type", FigValue::Str("SOLID".into())),
            (
                "color",
                obj(vec![
                    ("r", FigValue::Float(r)),
                    ("g", FigValue::Float(g)),
                    ("b", FigValue::Float(b)),
                    ("a", FigValue::Float(1.0)),
                ]),
            ),
        ])])
    }

    /// Library styles are referenced by `assetRef.key`, not a local
    /// guid — the resolver must index style nodes by their `key` too,
    /// and the resolved fill must beat an explicit placeholder fill.
    #[test]
    fn resolves_asset_ref_fill_style_by_key() {
        let style_node = obj(vec![
            ("styleType", FigValue::Str("FILL".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(1)),
                    ("localID", FigValue::Uint(4147)),
                ]),
            ),
            ("key", FigValue::Str("2298c886".into())),
            ("fillPaints", solid(0.33, 0.44, 0.95)),
        ]);
        let node = obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(1)),
                    ("localID", FigValue::Uint(6003)),
                ]),
            ),
            // Placeholder fill that the style must replace.
            ("fillPaints", solid(1.0, 1.0, 1.0)),
            (
                "styleIdForFill",
                obj(vec![(
                    "assetRef",
                    obj(vec![("key", FigValue::Str("2298c886".into()))]),
                )]),
            ),
        ]);
        let mut changes = vec![style_node, node];
        resolve_style_references(&mut changes);
        let resolved = changes[1]
            .get_array("fillPaints")
            .and_then(|a| a.first())
            .and_then(|p| p.get("color"))
            .and_then(|c| c.get_f64("b"))
            .unwrap_or(-1.0);
        assert!(
            (resolved - 0.95).abs() < 0.001,
            "assetRef fill style must resolve to the blue style, got b={resolved}"
        );
    }

    #[test]
    fn asset_ref_publish_key_does_not_collide_with_local_guid_key() {
        let asset_style = obj(vec![
            ("styleType", FigValue::Str("FILL".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(9)),
                    ("localID", FigValue::Uint(9)),
                ]),
            ),
            ("key", FigValue::Str("1:4147".into())),
            ("fillPaints", solid(0.0, 0.0, 1.0)),
        ]);
        let local_style = obj(vec![
            ("styleType", FigValue::Str("FILL".into())),
            (
                "guid",
                obj(vec![
                    ("sessionID", FigValue::Uint(1)),
                    ("localID", FigValue::Uint(4147)),
                ]),
            ),
            ("fillPaints", solid(1.0, 0.0, 0.0)),
        ]);
        let node = obj(vec![
            ("type", FigValue::Str("RECTANGLE".into())),
            (
                "styleIdForFill",
                obj(vec![(
                    "assetRef",
                    obj(vec![("key", FigValue::Str("1:4147".into()))]),
                )]),
            ),
        ]);
        let mut changes = vec![asset_style, local_style, node];
        resolve_style_references(&mut changes);
        let blue = changes[2]
            .get_array("fillPaints")
            .and_then(|a| a.first())
            .and_then(|p| p.get("color"))
            .and_then(|c| c.get_f64("b"))
            .unwrap_or(-1.0);
        assert!(
            (blue - 1.0).abs() < 0.001,
            "assetRef key must resolve through the publish-key namespace, got b={blue}"
        );
    }
}
