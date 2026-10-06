//! Figma → PenDocument top-level mapping — ports `figma-node-mapper.ts`.
//! Resolves style references, builds the tree, and converts each
//! user page into a `PenDocument`.

use crate::common::{collect_image_blobs, ConversionContext, FigLayoutMode};
use crate::converters::{convert_children, convert_node};
use crate::figma_types::FigmaDecodedFile;
use crate::image_resolver::retain_referenced_image_blobs;
use crate::page_mapper::pen_page;
use crate::tree::{
    build_tree, build_tree_for_clipboard, build_tree_owned, collect_components,
    collect_symbol_tree, guid_to_string, is_user_page, TreeNode,
};
use jian_ops_schema::document::PenDocument;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;
use std::collections::HashMap;

mod style_references;

#[cfg(test)]
use crate::kiwi::FigValue;
#[cfg(test)]
use style_references::non_empty_array;
pub use style_references::resolve_style_references;

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

pub(crate) fn empty_document(name: &str) -> PenDocument {
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
        motion: None,
        // Figma import never authors the responsive schema opt-in.
        responsive: None,
    }
}

pub(crate) fn empty_import_result(
    file_name: &str,
    warning: &str,
    _blobs: Vec<crate::figma_types::BlobOrString>,
) -> FigmaImportResult {
    FigmaImportResult {
        document: empty_document(file_name),
        warnings: vec![warning.to_string()],
        image_blobs: HashMap::new(),
    }
}

fn document_with_pages(name: &str, pages: Vec<PenPage>) -> PenDocument {
    PenDocument {
        pages: Some(pages),
        ..empty_document(name)
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

    let Some(tree) = build_tree_owned(std::mem::take(&mut decoded.node_changes)) else {
        return empty_import_result(file_name, "No document root found", decoded.blobs);
    };

    convert_tree_all_pages(&tree, decoded.blobs, file_name, layout_mode)
}

pub(crate) fn convert_tree_all_pages(
    tree: &TreeNode,
    blobs: Vec<crate::figma_types::BlobOrString>,
    file_name: &str,
    layout_mode: FigLayoutMode,
) -> FigmaImportResult {
    let pages: Vec<&TreeNode> = tree.children.iter().filter(|c| is_user_page(c)).collect();
    if pages.is_empty() {
        return empty_import_result(file_name, "No pages found in Figma file", blobs);
    }

    let mut component_map: HashMap<String, String> = HashMap::new();
    let mut symbol_tree: HashMap<String, &TreeNode> = HashMap::new();
    let mut counter: u32 = 1;
    for page in &pages {
        collect_components(page, &mut component_map, &mut counter);
    }
    collect_symbol_tree(tree, &mut symbol_tree);
    let mut instance_assignments: HashMap<String, String> = HashMap::new();
    crate::instance::seed_assignments_from_instances(tree, &symbol_tree, &mut instance_assignments);

    let mut ctx = ConversionContext {
        component_map,
        symbol_tree,
        warnings: Vec::new(),
        id_counter: counter,
        blobs,
        layout_mode,
        instance_assignments,
        instance_expansions: Default::default(),
    };

    let mut pen_pages = Vec::new();
    for (i, page) in pages.iter().enumerate() {
        let children = convert_children(page, &mut ctx);
        let name = page
            .figma
            .get_str("name")
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Page {}", i + 1));
        pen_pages.push(pen_page(page, format!("figma-page-{i}"), name, children));
    }

    let document = document_with_pages(file_name, pen_pages);
    let mut image_blobs = collect_image_blobs(std::mem::take(&mut ctx.blobs));
    retain_referenced_image_blobs(&document, &mut image_blobs);
    FigmaImportResult {
        document,
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

    let Some(tree) = build_tree_owned(std::mem::take(&mut decoded.node_changes)) else {
        return empty_import_result(file_name, "No document root found", decoded.blobs);
    };

    convert_tree_page(&tree, decoded.blobs, file_name, page_index, layout_mode)
}

pub(crate) fn convert_tree_page(
    tree: &TreeNode,
    blobs: Vec<crate::figma_types::BlobOrString>,
    file_name: &str,
    page_index: usize,
    layout_mode: FigLayoutMode,
) -> FigmaImportResult {
    let pages: Vec<&TreeNode> = tree.children.iter().filter(|c| is_user_page(c)).collect();
    let Some(page) = pages.get(page_index).or_else(|| pages.first()) else {
        return empty_import_result(file_name, "No pages found in Figma file", blobs);
    };

    let mut component_map: HashMap<String, String> = HashMap::new();
    let mut symbol_tree: HashMap<String, &TreeNode> = HashMap::new();
    let mut counter: u32 = 1;
    collect_components(page, &mut component_map, &mut counter);
    collect_symbol_tree(tree, &mut symbol_tree);
    let mut instance_assignments: HashMap<String, String> = HashMap::new();
    crate::instance::seed_assignments_from_instances(tree, &symbol_tree, &mut instance_assignments);

    let mut ctx = ConversionContext {
        component_map,
        symbol_tree,
        warnings: Vec::new(),
        id_counter: counter,
        blobs,
        layout_mode,
        instance_assignments,
        instance_expansions: Default::default(),
    };
    let children = convert_children(page, &mut ctx);
    let name = page
        .figma
        .get_str("name")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Page 1".to_string());
    let pen = pen_page(page, format!("figma-page-{page_index}"), name, children);

    let document = document_with_pages(file_name, vec![pen]);
    let mut image_blobs = collect_image_blobs(std::mem::take(&mut ctx.blobs));
    retain_referenced_image_blobs(&document, &mut image_blobs);
    FigmaImportResult {
        document,
        warnings: ctx.warnings,
        image_blobs,
    }
}

/// Page summaries for a decoded file — id / name / child count.
#[derive(Debug, Clone, PartialEq, Eq)]
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
            image_blobs: collect_image_blobs(decoded.blobs),
        };
    }

    let mut component_map: HashMap<String, String> = HashMap::new();
    let mut symbol_tree: HashMap<String, &TreeNode> = HashMap::new();
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
        instance_expansions: Default::default(),
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

    let image_blobs = collect_image_blobs(std::mem::take(&mut ctx.blobs));
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
        FigValue::Object(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
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

    fn guid(session_id: u32, local_id: u32) -> FigValue {
        obj(vec![
            ("sessionID", FigValue::Uint(session_id)),
            ("localID", FigValue::Uint(local_id)),
        ])
    }

    fn text_style_ref(session_id: u32, local_id: u32) -> FigValue {
        obj(vec![("guid", guid(session_id, local_id))])
    }

    fn line_height(px: f32) -> FigValue {
        obj(vec![
            ("value", FigValue::Float(px)),
            ("units", FigValue::Str("PIXELS".into())),
        ])
    }

    fn resolved_line_height(value: &FigValue) -> Option<f64> {
        value
            .get("lineHeight")
            .and_then(|height| height.get_f64("value"))
    }

    fn derived_text_metrics(font_sizes: &[f32], line_heights: &[f32]) -> FigValue {
        obj(vec![
            (
                "glyphs",
                FigValue::Array(
                    font_sizes
                        .iter()
                        .map(|size| obj(vec![("fontSize", FigValue::Float(*size))]))
                        .collect(),
                ),
            ),
            (
                "baselines",
                FigValue::Array(
                    line_heights
                        .iter()
                        .map(|height| obj(vec![("lineHeight", FigValue::Float(*height))]))
                        .collect(),
                ),
            ),
        ])
    }

    #[test]
    fn referenced_text_style_replaces_stale_direct_node_metrics() {
        let footnote_style = obj(vec![
            ("styleType", FigValue::Str("TEXT".into())),
            ("guid", guid(1, 10)),
            ("fontSize", FigValue::Float(12.0)),
            ("lineHeight", line_height(20.0)),
            ("textAlignVertical", FigValue::Str("BOTTOM".into())),
        ]);
        let heading_style = obj(vec![
            ("styleType", FigValue::Str("TEXT".into())),
            ("guid", guid(1, 11)),
            ("fontSize", FigValue::Float(16.0)),
            ("lineHeight", line_height(24.0)),
        ]);
        let notification = obj(vec![
            ("type", FigValue::Str("TEXT".into())),
            ("styleIdForText", text_style_ref(1, 10)),
            ("fontSize", FigValue::Float(16.0)),
            ("lineHeight", line_height(24.0)),
            ("textAlignVertical", FigValue::Str("TOP".into())),
            (
                "derivedTextData",
                derived_text_metrics(&[12.0, 12.0], &[20.0, 20.0]),
            ),
        ]);
        let version_label = obj(vec![
            ("type", FigValue::Str("TEXT".into())),
            ("styleIdForText", text_style_ref(1, 11)),
            ("fontSize", FigValue::Float(12.0)),
            ("lineHeight", line_height(20.0)),
            (
                "derivedTextData",
                derived_text_metrics(&[16.0, 16.0], &[24.0]),
            ),
        ]);

        let mut changes = vec![footnote_style, heading_style, notification, version_label];
        resolve_style_references(&mut changes);

        assert_eq!(changes[2].get_f64("fontSize"), Some(12.0));
        assert_eq!(resolved_line_height(&changes[2]), Some(20.0));
        assert_eq!(changes[2].get_str("textAlignVertical"), Some("TOP"));
        assert_eq!(changes[3].get_f64("fontSize"), Some(16.0));
        assert_eq!(resolved_line_height(&changes[3]), Some(24.0));
    }

    #[test]
    fn derived_local_text_override_beats_referenced_style() {
        let heading_style = obj(vec![
            ("styleType", FigValue::Str("TEXT".into())),
            ("guid", guid(1, 11)),
            ("fontSize", FigValue::Float(16.0)),
            ("lineHeight", line_height(24.0)),
        ]);
        let locally_overridden = obj(vec![
            ("type", FigValue::Str("TEXT".into())),
            ("styleIdForText", text_style_ref(1, 11)),
            ("fontSize", FigValue::Float(38.0)),
            ("lineHeight", line_height(46.0)),
            (
                "derivedTextData",
                derived_text_metrics(&[38.0, 38.0], &[46.0]),
            ),
        ]);

        let mut changes = vec![heading_style, locally_overridden];
        resolve_style_references(&mut changes);

        assert_eq!(changes[1].get_f64("fontSize"), Some(38.0));
        assert_eq!(resolved_line_height(&changes[1]), Some(46.0));
    }

    #[test]
    fn explicit_instance_text_override_beats_referenced_style() {
        let heading_style = obj(vec![
            ("styleType", FigValue::Str("TEXT".into())),
            ("guid", guid(1, 11)),
            ("fontSize", FigValue::Float(16.0)),
            ("lineHeight", line_height(24.0)),
        ]);
        let explicit_override = obj(vec![
            ("styleIdForText", text_style_ref(1, 11)),
            ("fontSize", FigValue::Float(38.0)),
            ("lineHeight", line_height(46.0)),
        ]);
        let instance = obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            (
                "symbolData",
                obj(vec![(
                    "symbolOverrides",
                    FigValue::Array(vec![explicit_override]),
                )]),
            ),
        ]);

        let mut changes = vec![heading_style, instance];
        resolve_style_references(&mut changes);

        let resolved = changes[1]
            .get("symbolData")
            .and_then(|data| data.get_array("symbolOverrides"))
            .and_then(|overrides| overrides.first())
            .expect("resolved symbol override");
        assert_eq!(resolved.get_f64("fontSize"), Some(38.0));
        assert_eq!(resolved_line_height(resolved), Some(46.0));
    }

    #[test]
    fn text_style_fill_applies_to_direct_text_but_not_symbol_override() {
        let text_style = obj(vec![
            ("styleType", FigValue::Str("TEXT".into())),
            ("guid", guid(1, 11)),
            ("fillPaints", solid(0.0, 0.0, 0.0)),
        ]);
        let direct_text = obj(vec![
            ("type", FigValue::Str("TEXT".into())),
            ("styleIdForText", text_style_ref(1, 11)),
        ]);
        let text_only_override = obj(vec![
            ("styleIdForText", text_style_ref(1, 11)),
            (
                "textData",
                obj(vec![("characters", FigValue::Str("Overview".into()))]),
            ),
        ]);
        let instance = obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            (
                "symbolData",
                obj(vec![(
                    "symbolOverrides",
                    FigValue::Array(vec![text_only_override]),
                )]),
            ),
        ]);

        let mut changes = vec![text_style, direct_text, instance];
        resolve_style_references(&mut changes);

        assert!(
            non_empty_array(&changes[1], "fillPaints"),
            "a direct TEXT node may use its linked text style's paint fallback"
        );
        let resolved_override = changes[2]
            .get("symbolData")
            .and_then(|data| data.get_array("symbolOverrides"))
            .and_then(|overrides| overrides.first())
            .expect("resolved symbol override");
        assert!(
            resolved_override.get("fillPaints").is_none(),
            "a text-only symbol override must preserve the target variant's fill"
        );
    }

    #[test]
    fn explicit_fill_style_still_applies_to_symbol_override() {
        let text_style = obj(vec![
            ("styleType", FigValue::Str("TEXT".into())),
            ("guid", guid(1, 11)),
            ("fillPaints", solid(0.0, 0.0, 0.0)),
        ]);
        let fill_style = obj(vec![
            ("styleType", FigValue::Str("FILL".into())),
            ("guid", guid(1, 12)),
            ("fillPaints", solid(0.0, 0.5, 1.0)),
        ]);
        let explicit_fill_override = obj(vec![
            ("styleIdForText", text_style_ref(1, 11)),
            ("styleIdForFill", text_style_ref(1, 12)),
        ]);
        let instance = obj(vec![
            ("type", FigValue::Str("INSTANCE".into())),
            (
                "symbolData",
                obj(vec![(
                    "symbolOverrides",
                    FigValue::Array(vec![explicit_fill_override]),
                )]),
            ),
        ]);

        let mut changes = vec![text_style, fill_style, instance];
        resolve_style_references(&mut changes);

        let resolved_fill = changes[2]
            .get("symbolData")
            .and_then(|data| data.get_array("symbolOverrides"))
            .and_then(|overrides| overrides.first())
            .and_then(|override_entry| override_entry.get_array("fillPaints"))
            .and_then(|paints| paints.first())
            .and_then(|paint| paint.get("color"))
            .expect("explicit fill style paint");
        assert_eq!(resolved_fill.get_f64("r"), Some(0.0));
        assert_eq!(resolved_fill.get_f64("g"), Some(0.5));
        assert_eq!(resolved_fill.get_f64("b"), Some(1.0));
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
