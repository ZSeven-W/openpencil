//! In-place accepted image updates for a retained preview session.
//!
//! The host calls this only after its document scene actually rebuilt. Image
//! sources are data, not a new preview: routing, input, scrolling and motion
//! retain their live state. Failed searches and general design edits stay out
//! of this deliberately narrow synchronization path.

use std::collections::{BTreeMap, BTreeSet};

use jian_ops_schema::node::{ImageSrc, PenNode};
use jian_ops_schema::style::PenFill;
use jian_ops_schema::PenDocument;
use op_editor_core::PenNodeExt;
use op_editor_ui::layout_scene::SceneNode;

use crate::PreviewSession;

#[derive(Clone, PartialEq)]
enum Asset {
    Image(ImageSrc),
    Fill(Option<Vec<PenFill>>),
}

struct Patch {
    before: Asset,
    after: Asset,
    removed_fallbacks: BTreeSet<String>,
}

type Patches = BTreeMap<String, Patch>;

impl PreviewSession {
    /// Bind to the host's document lifetime, not its changing content revision.
    /// Call once when installing a new session; do not rebind a retained one.
    pub fn bind_asset_source(&mut self, document_epoch: u64, document_generation: u64) {
        if self.asset_source_identity.is_none() {
            self.asset_source_identity = Some((document_epoch, document_generation));
        }
    }

    /// Pull accepted image sources from the same editor document without
    /// resetting live state or importing unrelated edits. Returns true when
    /// at least one asset changed. Empty/failed sources never clear an image.
    pub fn sync_image_assets(
        &mut self,
        document: &PenDocument,
        document_epoch: u64,
        document_generation: u64,
    ) -> bool {
        if self.asset_source_identity != Some((document_epoch, document_generation)) {
            return false;
        }
        let patches = collect_patches(&self.reset_seed.document, document);
        if patches.is_empty() {
            return false;
        }

        // Resolve only our asset-updated source, not the incoming document's
        // other edits. This also expands changed component assets into their
        // stable instance ids before applying them to the projected preview.
        let before = resolve_assets(&self.reset_seed.document, &self.reset_seed.active_theme);
        apply_document(&mut self.reset_seed.document, &patches);
        let after = resolve_assets(&self.reset_seed.document, &self.reset_seed.active_theme);
        let expanded = collect_patches(&before, &after);
        apply_document(&mut self.layout_doc, &expanded);
        if let Some(app) = self.app.as_mut() {
            apply_document(&mut app.promoted_doc, &expanded);
            // Same routes and pages, new image data. Retain the live router and
            // stack, and refresh only the source used for future navigation.
            if let Some(table) =
                jian_core::screens::ScreenTable::from_document(app.promoted_doc.clone())
            {
                app.table = table;
            }
        }
        let mut pruned_fallbacks = false;
        if let Some(runtime_doc) = self.runtime.document.as_mut() {
            apply_document(&mut runtime_doc.schema, &expanded);
            let removed: BTreeSet<_> = expanded
                .values()
                .flat_map(|patch| patch.removed_fallbacks.iter().cloned())
                .collect();
            let removed_keys: BTreeSet<_> = removed
                .iter()
                .filter_map(|id| runtime_doc.tree.get(id))
                .collect();
            for (_, data) in runtime_doc.tree.nodes.iter_mut() {
                apply_node_tree(&mut data.schema, &expanded);
                data.children.retain(|key| !removed_keys.contains(key));
            }
            for key in removed_keys {
                if let Some(data) = runtime_doc.tree.nodes.get(key) {
                    pruned_fallbacks = true;
                    runtime_doc.tree.by_id.remove(data.schema.id_str());
                    // An in-flight gesture may still hold this leaf's NodeKey.
                    // Keep the inert node until navigation/reset drops the tree;
                    // it is detached from children, lookup and the spatial index.
                }
            }
            self.binding_overlay
                .set_runtime_document(Some(runtime_doc.schema.clone()));
        }

        if pruned_fallbacks {
            // Old hit boxes still contain the detached fallback leaves. Rebuild only the
            // spatial/focus index, keeping the solved geometry and live state.
            self.runtime.rebuild_spatial();
        }

        // Use the canonical loader for fill stacks/crop/cache keys, but copy
        // only asset paint fields into the existing scene. Removing a fallback
        // caption must not reflow the live page or move its scroll position.
        let promoted = self
            .app
            .as_ref()
            .map(|app| &app.promoted_doc)
            .or_else(|| self.runtime.document.as_ref().map(|doc| &doc.schema));
        if let Some(promoted) = promoted {
            let fresh = op_pen_loader::pen_document_to_layout_scene_for_preview(
                promoted,
                &self.layout_doc,
                self.preserve_authored_geometry,
                &self.reset_seed.active_theme,
                self.app.as_ref().map_or(0, |app| app.page_idx),
            );
            if let (Some(current), Some(fresh)) = (
                self.scene.pages.get_mut(self.scene.active_page_index),
                fresh.active_page(),
            ) {
                for node in &mut current.children {
                    sync_scene(node, fresh, &expanded);
                }
            }
        }
        true
    }
}

fn resolve_assets(document: &PenDocument, theme: &BTreeMap<String, String>) -> PenDocument {
    let mut document = if op_editor_core::ref_resolve::document_has_refs(document) {
        op_editor_core::ref_resolve::resolve_refs_for_canvas(document)
    } else {
        document.clone()
    };
    if op_editor_core::variables_resolve::document_has_tokens(&document) {
        document = op_editor_core::variables_resolve::resolve_document_for_canvas(&document, theme);
    }
    document
}

fn accepted(source: &str) -> bool {
    let source = source.trim();
    !source.is_empty() && !source.starts_with("placeholder:") && !source.starts_with('$')
}

fn asset(node: &PenNode) -> Option<Asset> {
    match node {
        PenNode::Image(image) => Some(Asset::Image(image.src.clone())),
        PenNode::Frame(frame) => Some(Asset::Fill(frame.container.fill.clone())),
        PenNode::Rectangle(rect) => Some(Asset::Fill(rect.container.fill.clone())),
        _ => None,
    }
}

fn accepted_update(before: &Asset, incoming: &Asset) -> Option<Asset> {
    match (before, incoming) {
        (Asset::Image(_), Asset::Image(src)) if accepted(src.as_ref()) => Some(incoming.clone()),
        (Asset::Fill(old), Asset::Fill(Some(new))) => {
            let old_fills = old.as_deref().unwrap_or_default();
            let had_image = old_fills
                .iter()
                .any(|fill| matches!(fill, PenFill::Image(_)));
            if !had_image {
                // An empty/solid placeholder becomes one accepted image layer.
                return matches!(new.as_slice(), [PenFill::Image(image)] if accepted(image.url.as_ref()))
                    .then(|| incoming.clone());
            }
            if old_fills.len() != new.len() {
                return None;
            }
            let mut updated = old_fills.to_vec();
            for (old, new) in updated.iter_mut().zip(new) {
                if let (PenFill::Image(old), PenFill::Image(new)) = (old, new) {
                    if accepted(new.url.as_ref()) {
                        old.url = new.url.clone();
                    }
                }
            }
            Some(Asset::Fill(Some(updated)))
        }
        _ => None,
    }
}

fn collect_nodes<'a>(nodes: &'a [PenNode], result: &mut BTreeMap<String, &'a PenNode>) {
    for node in nodes {
        result.insert(node.id_str().to_owned(), node);
        if let Some(children) = node.children_ref() {
            collect_nodes(children, result);
        }
    }
}

fn document_nodes(document: &PenDocument) -> BTreeMap<String, &PenNode> {
    let mut result = BTreeMap::new();
    collect_nodes(&document.children, &mut result);
    if let Some(pages) = &document.pages {
        for page in pages {
            collect_nodes(&page.children, &mut result);
        }
    }
    result
}

fn collect_patches(before: &PenDocument, after: &PenDocument) -> Patches {
    let after = document_nodes(after);
    document_nodes(before)
        .into_iter()
        .filter_map(|(id, old)| {
            let new = after.get(&id)?;
            if std::mem::discriminant(old) != std::mem::discriminant(*new) {
                return None;
            }
            let before = asset(old)?;
            let after = accepted_update(&before, &asset(new)?)?;
            if before == after {
                return None;
            }
            let marked = old
                .base()
                .name
                .as_deref()
                .is_some_and(|name| name.ends_with(" (image fallback)"))
                || old
                    .base()
                    .explain
                    .as_deref()
                    .is_some_and(|explain| explain.starts_with("image fallback:"));
            let mut removed_fallbacks = BTreeSet::new();
            if marked {
                for child in old.children_ref().unwrap_or_default() {
                    // Only the generated leaf icon/caption, and only after the
                    // accepted source has really removed it from the editor slot.
                    let generated = (matches!(child, PenNode::IconFont(_))
                        && (child.id_str() == format!("{id}-image-fallback-icon")
                            || (child.id_str().ends_with("-image-fallback-icon")
                                && child.base().name.as_deref() == Some("Image fallback icon"))))
                        || (matches!(child, PenNode::Text(_))
                            && (child.id_str() == format!("{id}-image-fallback-caption")
                                || (child.id_str().ends_with("-image-fallback-caption")
                                    && child.base().name.as_deref()
                                        == Some("Image fallback caption"))));
                    if generated
                        && !new
                            .children_ref()
                            .unwrap_or_default()
                            .iter()
                            .any(|n| n.id_str() == child.id_str())
                    {
                        removed_fallbacks.insert(child.id_str().to_owned());
                    }
                }
            }
            Some((
                id,
                Patch {
                    before,
                    after,
                    removed_fallbacks,
                },
            ))
        })
        .collect()
}

fn apply_node_tree(node: &mut PenNode, patches: &Patches) {
    if let Some(patch) = patches.get(node.id_str()) {
        if asset(node).as_ref() == Some(&patch.before) {
            match (&patch.after, &mut *node) {
                (Asset::Image(src), PenNode::Image(image)) => image.src = src.clone(),
                (Asset::Fill(fill), PenNode::Frame(frame)) => frame.container.fill = fill.clone(),
                (Asset::Fill(fill), PenNode::Rectangle(rect)) => rect.container.fill = fill.clone(),
                _ => {}
            }
            if !patch.removed_fallbacks.is_empty() {
                if let Some(children) = node.children_mut() {
                    children.retain(|child| !patch.removed_fallbacks.contains(child.id_str()));
                }
            }
        }
    }
    if node.children_ref().is_some() {
        if let Some(children) = node.children_mut() {
            for child in children {
                apply_node_tree(child, patches);
            }
        }
    }
}

fn apply_document(document: &mut PenDocument, patches: &Patches) {
    for node in &mut document.children {
        apply_node_tree(node, patches);
    }
    if let Some(pages) = &mut document.pages {
        for page in pages {
            for node in &mut page.children {
                apply_node_tree(node, patches);
            }
        }
    }
}

fn sync_scene(
    node: &mut SceneNode,
    fresh: &op_editor_ui::layout_scene::ScenePage,
    patches: &Patches,
) {
    if let Some(patch) = patches.get(&node.id) {
        if let Some(source) = fresh.find(&node.id) {
            node.fill = source.fill;
            node.fill_layers = source.fill_layers.clone();
            node.fill_type = source.fill_type;
            node.gradient = source.gradient.clone();
            node.shader = source.shader.clone();
            node.image_src = source.image_src.clone();
            node.image_src_id = source.image_src_id;
            node.image_fit = source.image_fit;
            node.image_blend_mode = source.image_blend_mode;
            node.image_transform = source.image_transform;
            node.image_original_size = source.image_original_size;
            node.image_tile_scale = source.image_tile_scale;
            node.image_adjustments = source.image_adjustments;
            node.children
                .retain(|child| !patch.removed_fallbacks.contains(&child.id));
        }
    }
    for child in &mut node.children {
        sync_scene(child, fresh, patches);
    }
}
