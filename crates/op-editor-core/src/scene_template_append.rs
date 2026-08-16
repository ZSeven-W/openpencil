//! Bringing a shipped scene template into the document the user already has.
//!
//! The Asset Center's other door replaces the document; this one adds to it.
//! That difference is the whole module: a template that arrives as a new file
//! can bring its own variable table and start at the origin, while a template
//! landing beside existing work must not collide with either.
//!
//! Two hazards, both handled here rather than at the call site so the native
//! and web hosts cannot solve them differently:
//!
//! 1. **Variable collisions.** Every shipped template paints through the same
//!    seven names (`c-bg`, `c-accent`, …) with different values, so merging
//!    two templates' tables would silently restyle whichever arrived first.
//!    Each template's variables are namespaced by its id on the way in, and
//!    every `$ref` inside its boards is rewritten to match — so appending the
//!    same template twice is idempotent and appending two different ones
//!    leaves both looking like themselves.
//! 2. **Placement.** Boards keep their relative layout and move as a block to
//!    the right of whatever is already on the page.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use base64::Engine as _;
use jian_ops_schema::compat::{load_str_with_preprocess, LoadOptions};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::variable::VariableDefinition;
use jian_ops_schema::PenDocument;

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;

/// Document-space gap between the existing content's right edge and the
/// first appended board. Roughly a fifth of a 1920 slide — wide enough that
/// the seam reads as "a different thing starts here" at fit-to-screen zoom.
pub const TEMPLATE_APPEND_GAP: f64 = 400.0;

/// A template's top-level boards plus the variables they reference.
///
/// Produced by [`template_boards`], consumed by
/// [`EditorState::append_template_boards`]. The two are separate so a caller
/// can parse without mutating — the parse is the step that can fail on a
/// malformed asset, and it must fail before anything touches the document.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateBoards {
    pub nodes: Vec<PenNode>,
    pub variables: BTreeMap<String, VariableDefinition>,
    image_thumbnails: Vec<(u64, Arc<[u8]>)>,
}

/// Parse a template's canonical `.op` source into namespaced boards.
///
/// Canonical saved documents may carry their boards under `pages`, their
/// selected page under `editorMeta.activePageIndex`, and externalized image
/// payloads under `images`. Running the schema compat loader here keeps this
/// path aligned with normal document loads instead of maintaining a partial
/// template-only parser.
///
/// `None` means the asset did not parse. No nodes, variables, or thumbnails
/// are published until the returned value is successfully inserted.
pub fn template_boards(source: &str, template_id: &str) -> Option<TemplateBoards> {
    let mut active_page_index = 0;
    let mut image_thumbnails = BTreeMap::new();
    let mut renames = BTreeMap::new();
    let loaded = load_str_with_preprocess(source, LoadOptions::default(), |document| {
        active_page_index = template_active_page_index(document);
        image_thumbnails = take_raw_template_thumbnails(document);
        renames = variable_renames(document, template_id);
        if let Some(children) = active_raw_children_mut(document, active_page_index) {
            rewrite_variable_refs(children, &renames);
        }
    })
    .ok()?;

    let mut document = loaded.value;
    let nodes = take_active_children(&mut document, active_page_index);
    let referenced_thumbnail_ids = referenced_thumbnail_ids(&nodes);
    image_thumbnails.retain(|id, _| referenced_thumbnail_ids.contains(id));
    let variables = document
        .variables
        .take()
        .unwrap_or_default()
        .into_iter()
        .map(|(name, definition)| (renames.get(&name).cloned().unwrap_or(name), definition))
        .collect();

    Some(TemplateBoards {
        nodes,
        variables,
        image_thumbnails: image_thumbnails.into_iter().collect(),
    })
}

/// Saved-document page selection, matching the editor metadata loader's
/// camel-case wire field plus its former snake-case alias.
fn template_active_page_index(document: &serde_json::Value) -> usize {
    document
        .get("editorMeta")
        .and_then(serde_json::Value::as_object)
        .and_then(|meta| {
            meta.get("activePageIndex")
                .or_else(|| meta.get("active_page_index"))
        })
        .and_then(serde_json::Value::as_u64)
        .and_then(|index| usize::try_from(index).ok())
        .unwrap_or(0)
}

/// The raw active-page children used by the variable-reference preprocessor.
/// Empty page lists follow the editor's root-children fallback.
fn active_raw_children_mut(
    document: &mut serde_json::Value,
    active_page_index: usize,
) -> Option<&mut serde_json::Value> {
    let page_count = document
        .get("pages")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    if page_count == 0 {
        return document.get_mut("children");
    }
    document
        .get_mut("pages")?
        .as_array_mut()?
        .get_mut(active_page_index.min(page_count - 1))?
        .get_mut("children")
}

/// Typed counterpart to [`active_raw_children_mut`].
fn take_active_children(document: &mut PenDocument, active_page_index: usize) -> Vec<PenNode> {
    match document.pages.as_mut() {
        Some(pages) if !pages.is_empty() => {
            let index = active_page_index.min(pages.len() - 1);
            std::mem::take(&mut pages[index].children)
        }
        _ => std::mem::take(&mut document.children),
    }
}

/// Remove and decode the canonical `imageThumbs` side table before the schema
/// loader sees it.
///
/// A template is parsed for extraction, not installed as the active document,
/// so its thumbnail seed must never enter the schema loader's pending-document
/// registry. Keeping the decoded bytes local also means parsing cannot touch
/// the active thumbnail registry while a desktop decode worker writes to it.
/// The bounds and validation match `jian_ops_schema::image_thumbs` exactly.
fn take_raw_template_thumbnails(document: &mut serde_json::Value) -> BTreeMap<u64, Arc<[u8]>> {
    let Some(serde_json::Value::Object(table)) = document
        .as_object_mut()
        .and_then(|root| root.remove("imageThumbs"))
    else {
        return BTreeMap::new();
    };
    let max_bytes = jian_ops_schema::image_thumbs::MAX_THUMB_BYTES;
    let max_encoded_bytes = max_bytes.div_ceil(3) * 4;
    table
        .into_iter()
        .filter_map(|(id, encoded)| {
            let id = id.parse::<u64>().ok()?;
            let encoded = encoded.as_str()?;
            if encoded.len() > max_encoded_bytes {
                return None;
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()?;
            (bytes.len() <= max_bytes).then(|| (id, Arc::from(bytes)))
        })
        .collect()
}

/// Paint ids actually referenced by the adopted page. A full saved document
/// may carry thumbnails for images on every other page; those do not belong
/// in the destination document's runtime cache.
fn referenced_thumbnail_ids(nodes: &[PenNode]) -> BTreeSet<u64> {
    let mut ids = BTreeSet::new();
    jian_ops_schema::image_table::visit_legacy_node_roots(nodes, &mut |source| {
        ids.insert(jian_ops_schema::node::image_src::paint_image_id(
            source.as_str(),
        ));
    });
    ids
}

/// The document for a template id from either half of the catalogue.
///
/// Bare ids resolve to the embedded shipped asset; ids carrying the `user:`
/// prefix resolve to the runtime registry's saved document. `None` means the
/// id names nothing — for a shipped id a corrupt or renamed asset, for a
/// `user:` id a template that was deleted since the card was painted.
pub fn template_document_for(template_id: &str) -> Option<String> {
    if template_id.starts_with(crate::user_scene_templates::USER_TEMPLATE_ID_PREFIX) {
        return crate::user_scene_templates::user_scene_templates()
            .into_iter()
            .find(|template| template.id == template_id)
            .map(|template| template.document.clone());
    }
    crate::scene_template_catalog::scene_template_document(template_id).map(str::to_string)
}

/// The namespaced name for each variable the template declares.
///
/// Namespacing unconditionally rather than only on collision is what makes
/// this idempotent: the second append of the same template produces the same
/// names with the same values, so the merge is a no-op instead of a rename
/// chain that grows a suffix every time.
fn variable_renames(document: &serde_json::Value, template_id: &str) -> BTreeMap<String, String> {
    let Some(variables) = document.get("variables").and_then(|v| v.as_object()) else {
        return BTreeMap::new();
    };
    variables
        .keys()
        .map(|name| (name.clone(), format!("{template_id}--{name}")))
        .collect()
}

/// Rewrite every `$name` reference under `value` through `renames`.
///
/// Works on the JSON rather than the typed tree because a reference can sit
/// in any string-valued field of any of the twelve node variants — fills,
/// strokes, text colour, sizing expressions — and a typed walk would have to
/// enumerate all of them and then keep up as the schema grows.
fn rewrite_variable_refs(value: &mut serde_json::Value, renames: &BTreeMap<String, String>) {
    if renames.is_empty() {
        return;
    }
    match value {
        serde_json::Value::String(text) => {
            if let Some(rewritten) = rewrite_refs_in_text(text, renames) {
                *text = rewritten;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                rewrite_variable_refs(item, renames);
            }
        }
        serde_json::Value::Object(fields) => {
            for field in fields.values_mut() {
                rewrite_variable_refs(field, renames);
            }
        }
        _ => {}
    }
}

/// `None` when the text holds no reference this rename map knows.
///
/// The identifier after `$` is taken at maximal length and looked up whole,
/// so `$c-accent-soft` resolves to that variable rather than to `c-accent`
/// followed by a stray `-soft`. A `$` that is not followed by a known name —
/// a price in body copy, say — is left exactly as written.
fn rewrite_refs_in_text(text: &str, renames: &BTreeMap<String, String>) -> Option<String> {
    if !text.contains('$') {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut rest = text;
    while let Some(offset) = rest.find('$') {
        out.push_str(&rest[..offset]);
        let after = &rest[offset + 1..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(after.len());
        match renames.get(&after[..end]) {
            Some(renamed) => {
                out.push('$');
                out.push_str(renamed);
                changed = true;
            }
            None => {
                out.push('$');
                out.push_str(&after[..end]);
            }
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    changed.then_some(out)
}

impl EditorState {
    /// Append a template's boards to the active page, to the right of what
    /// is already there.
    ///
    /// One transaction, one undo entry: every board goes in through a single
    /// insert, which is also what keeps the empty-root swap from firing (it
    /// only ever considers a lone incoming frame). Returns whether the
    /// document changed; a rejected insert leaves the variable table alone,
    /// so a failure cannot deposit orphan variables.
    pub fn append_template_boards(&mut self, boards: TemplateBoards) -> bool {
        self.insert_template_boards(boards, false)
    }

    /// Bring a template in the way the user meant, given what is open.
    ///
    /// On an untouched starter the template takes the page over — keeping the
    /// blank frame beside it would leave the user to delete a placeholder
    /// before they could use what they asked for. Anywhere else it appends.
    ///
    /// This is the whole decision a host without a document loader needs; the
    /// desktop takes a longer road for the starter case because it also has a
    /// file path to unbind and preferences to carry across, neither of which
    /// exists here.
    pub fn adopt_template_boards(&mut self, boards: TemplateBoards) -> bool {
        let onto_starter = crate::blank_starter::active_page_is_blank_starter(self);
        self.insert_template_boards(boards, onto_starter)
    }

    /// One transaction either way: snapshot, place, insert, merge, commit.
    ///
    /// The order matters at one point — the insert runs before the variable
    /// merge, so a rejected insert cannot leave the document carrying a
    /// palette for boards that never arrived.
    fn insert_template_boards(&mut self, boards: TemplateBoards, clear_page: bool) -> bool {
        let TemplateBoards {
            mut nodes,
            variables,
            image_thumbnails,
        } = boards;
        if nodes.is_empty() {
            return false;
        }
        let snapshot = self.snapshot_for_history();

        if clear_page {
            self.active_children_mut().clear();
            self.deselect_all();
        } else {
            let (dx, dy) = append_offset(self.active_children(), &nodes);
            for node in &mut nodes {
                let base = node.base_mut();
                base.x = Some(base.x.unwrap_or(0.0) + dx);
                base.y = Some(base.y.unwrap_or(0.0) + dy);
            }
        }

        if self
            .insert_subtree_preserving_roots(nodes, &NodeId::NONE)
            .is_none()
        {
            // The page was already emptied on the `clear_page` road, so the
            // snapshot is the only way back — restoring it is what keeps a
            // failed adopt from being a delete.
            if clear_page {
                self.restore(snapshot);
            }
            return false;
        }

        let table = self.doc.variables.get_or_insert_with(BTreeMap::new);
        for (name, definition) in variables {
            table.entry(name).or_insert(definition);
        }
        for (paint_id, jpeg_bytes) in image_thumbnails {
            if jian_ops_schema::image_thumbs::thumb_for(paint_id).is_none() {
                jian_ops_schema::image_thumbs::store_thumb(paint_id, jpeg_bytes);
            }
        }

        self.history_push_past(snapshot);
        true
    }
}

/// How far the incoming boards move so they sit right of `existing`.
///
/// Vertically they align with the top of the existing content rather than
/// keeping their own y: a deck authored at y=0 dropped beside work that
/// starts at y=900 would otherwise land off-screen above it.
fn append_offset(existing: &[PenNode], incoming: &[PenNode]) -> (f64, f64) {
    let Some((incoming_min_x, incoming_min_y, _, _)) = board_bounds(incoming) else {
        return (0.0, 0.0);
    };
    let Some((_, existing_min_y, existing_max_x, _)) = board_bounds(existing) else {
        return (0.0, 0.0);
    };
    (
        existing_max_x + TEMPLATE_APPEND_GAP - incoming_min_x,
        existing_min_y - incoming_min_y,
    )
}

/// `(min_x, min_y, max_x, max_y)` over top-level boards, from their authored
/// geometry rather than a layout pass — this crate has no layout engine, and
/// every page-level board carries an explicit position and size. A board
/// sized by its content contributes its origin only, which is the honest
/// answer when its width is not knowable here.
fn board_bounds(nodes: &[PenNode]) -> Option<(f64, f64, f64, f64)> {
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    for node in nodes {
        let x = node.base().x.unwrap_or(0.0);
        let y = node.base().y.unwrap_or(0.0);
        let right = x + node.width_px().unwrap_or(0.0);
        let bottom = y + node.height_px().unwrap_or(0.0);
        bounds = Some(match bounds {
            None => (x, y, right, bottom),
            Some((min_x, min_y, max_x, max_y)) => (
                min_x.min(x),
                min_y.min(y),
                max_x.max(right),
                max_y.max(bottom),
            ),
        });
    }
    bounds
}

#[cfg(test)]
#[path = "scene_template_append_tests.rs"]
mod scene_template_append_tests;
