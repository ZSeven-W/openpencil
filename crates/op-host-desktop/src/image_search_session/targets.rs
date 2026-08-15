//! Image-slot detection: which nodes in the document want an image, what
//! query describes them, and how a turn's intent is fingerprinted. Carved
//! out of the `image_search_session.rs` spine to keep it under the
//! 800-line cap; pure code motion.

use std::collections::{HashMap, HashSet};

use jian_ops_schema::node::{PenNode, TextContent};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::PenFill;
use op_editor_core::agent_settings::ImageGenProfile;
use op_editor_core::{EditorState, NodeId, PenNodeExt as _};

use super::{search_intent_key, ImageAspectRatio, ImageRequestMode, ImageSearchTarget};

pub(super) fn intent_fingerprint(
    target: &ImageSearchTarget,
    profile: Option<&ImageGenProfile>,
) -> String {
    let generate = target.mode == ImageRequestMode::Generate
        || (target.mode == ImageRequestMode::Auto && profile.is_some());
    if generate {
        let (profile_id, model) = profile
            .map(|profile| (profile.id.as_str(), profile.model.as_str()))
            .unwrap_or(("unconfigured", "unconfigured"));
        format!(
            "generate|{profile_id}|{model}|{}|{:?}|{:?}",
            target
                .prompt
                .as_deref()
                .filter(|prompt| !prompt.trim().is_empty())
                .unwrap_or(target.query.as_str())
                .trim(),
            target.width.map(f64::to_bits),
            target.height.map(f64::to_bits)
        )
    } else {
        let key = search_intent_key(&target.query, target.aspect_ratio);
        format!("search|{}|{:?}", key.query, key.aspect_ratio)
    }
}

pub(super) fn current_intent_fingerprints(
    state: &EditorState,
    scene: Option<&op_editor_ui::layout_scene::LayoutScene>,
) -> HashMap<String, String> {
    let profile = crate::image_panel_host::active_image_gen_profile(state);
    let targets = match scene {
        Some(scene) => collect_targets_with_scene(state, &HashSet::new(), scene),
        None => collect_targets(state, &HashSet::new()),
    };
    targets
        .into_iter()
        .map(|target| {
            let fingerprint = intent_fingerprint(&target, profile);
            (target.node_id.as_str().to_string(), fingerprint)
        })
        .collect()
}

pub(crate) fn collect_targets(
    state: &EditorState,
    known_node_ids: &HashSet<String>,
) -> Vec<ImageSearchTarget> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    collect_targets_with_scene(state, known_node_ids, &scene)
}

pub(super) fn collect_targets_with_scene(
    state: &EditorState,
    known_node_ids: &HashSet<String>,
    scene: &op_editor_ui::layout_scene::LayoutScene,
) -> Vec<ImageSearchTarget> {
    let resolved_sizes = resolved_node_sizes(scene);
    let mut targets = Vec::new();
    collect_from_children(
        state.active_children(),
        known_node_ids,
        &resolved_sizes,
        &mut targets,
        &[],
    );
    targets
}

/// Resolved node dimensions from the real layout pass. This map is used only
/// after a node has independently qualified as media; geometry never turns an
/// anonymous surface into an image target. It lets a G()-created fill/fill
/// child inherit the actual slot aspect for search, generation, and stale-job
/// detection instead of losing that intent as `None`.
fn resolved_node_sizes(
    scene: &op_editor_ui::layout_scene::LayoutScene,
) -> HashMap<String, (f64, f64)> {
    let mut out = HashMap::new();
    fn walk(
        nodes: &[op_editor_ui::layout_scene::SceneNode],
        out: &mut HashMap<String, (f64, f64)>,
    ) {
        for node in nodes {
            let bounds = node.aggregate_bounds();
            if bounds.size.x > 0.0 && bounds.size.y > 0.0 {
                out.insert(
                    node.id.clone(),
                    (f64::from(bounds.size.x), f64::from(bounds.size.y)),
                );
            }
            walk(&node.children, out);
        }
    }
    if let Some(page) = scene.active_page() {
        walk(&page.children, &mut out);
    }
    out
}

fn collect_from_children(
    children: &[PenNode],
    known_node_ids: &HashSet<String>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
    targets: &mut Vec<ImageSearchTarget>,
    parent_names: &[String],
) {
    // Direct sibling text of a bare anonymous slot may name its subject
    // ("Blinding Lights" next to a nameless 120px square = that track's
    // cover). Do not search sibling container subtrees or inherit text across
    // levels: at a rail/list boundary those words belong to cousin cards, not
    // to this slot.
    for (index, node) in children.iter().enumerate() {
        let context: Vec<String> = children
            .iter()
            .enumerate()
            .filter(|(other_index, _)| *other_index != index)
            .filter_map(|(_, other)| match other {
                PenNode::Text(text) => match &text.content {
                    TextContent::Plain(value) if !value.trim().is_empty() => {
                        Some(value.trim().to_string())
                    }
                    _ => None,
                },
                _ => None,
            })
            .take(2)
            .collect();

        if let Some(target) =
            image_search_target_for(node, known_node_ids, resolved_sizes, parent_names, &context)
        {
            targets.push(target);
        }

        if is_image_placeholder_frame(node)
            || is_image_area_frame_by_heuristic(node)
            || is_image_area_rectangle_by_heuristic(node)
        {
            continue;
        }
        if let Some(grand) = node.children() {
            let mut child_parent_names = Vec::with_capacity(parent_names.len() + 1);
            child_parent_names.push(node.base().name.clone().unwrap_or_default());
            child_parent_names.extend(parent_names.iter().cloned());
            collect_from_children(
                grand,
                known_node_ids,
                resolved_sizes,
                targets,
                &child_parent_names,
            );
        }
    }
}

fn image_search_target_for(
    node: &PenNode,
    known_node_ids: &HashSet<String>,
    resolved_sizes: &HashMap<String, (f64, f64)>,
    parent_names: &[String],
    sibling_text: &[String],
) -> Option<ImageSearchTarget> {
    let id = node.base().id.as_str();
    if known_node_ids.contains(id) {
        return None;
    }

    // Anonymous EMPTY solid square (>=48px, rounded/clipping) whose card
    // carries text siblings — DeepSeek V4 builds whole album grids this
    // way with no names and no G() bindings (measured test0711-2-ds); the
    // sibling text is the only, and a good, subject source.
    let bare_slot_with_context = !sibling_text.is_empty()
        && !has_non_media_context(parent_names)
        && is_bare_anonymous_slot(node);
    let needs_image = match node {
        PenNode::Image(image) => is_placeholder_src(&image.src),
        PenNode::Frame(_) => {
            is_frame_placeholder_still_unfilled(node)
                || bare_slot_with_context
                || has_empty_image_fill(node)
        }
        // (rectangles fall through to the arm below)
        PenNode::Rectangle(_) => {
            is_image_area_rectangle_by_heuristic(node)
                || is_unnamed_media_slot_in_context(node, parent_names)
                || bare_slot_with_context
                || has_empty_image_fill(node)
        }
        _ => false,
    };
    if !needs_image {
        return None;
    }

    let mut query = extract_query_for_node(node, parent_names);
    if bare_slot_with_context {
        // The generic name-derived fallback ("placeholder") loses to the
        // card's own text; an EXPLICIT imageSearchQuery binding still wins.
        let explicit = match node {
            PenNode::Frame(f) => f
                .image_search_query
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string(),
            _ => String::new(),
        };
        // Precedence: an explicit binding, then the nearest ancestor that NAMES
        // the subject (a card called "Bali, Indonesia"), and only then the
        // words found around the slot. Context text is the last resort because
        // it can only ever come from siblings — and a card with no words of its
        // own would otherwise borrow its neighbour's ("Santorini" on the Bali
        // card, measured while fixing this).
        query = if !explicit.is_empty() {
            explicit
        } else if let Some(name) = parent_semantic_name(parent_names) {
            name
        } else {
            sibling_text.join(" ")
        };
    }
    if query.is_empty() {
        return None;
    }

    // `image_prompt` is the author's AI-gen prompt (Image nodes only); placeholder
    // frames / rectangles carry only a name-derived query.
    let prompt = match node {
        PenNode::Image(image) => image.image_prompt.clone(),
        _ => None,
    };
    let mode = image_request_mode(node);
    let (width, height) = resolved_sizes
        .get(id)
        .copied()
        .map(|(width, height)| (Some(width), Some(height)))
        .unwrap_or_else(|| (node.width_px(), node.height_px()));
    Some(ImageSearchTarget {
        node_id: NodeId::new(id),
        query,
        aspect_ratio: infer_aspect_ratio(width, height),
        prompt,
        mode,
        width,
        height,
    })
}

pub(crate) fn image_request_mode(node: &PenNode) -> ImageRequestMode {
    match node {
        // Legacy / script-generated Image nodes intentionally carry both
        // fields: generation uses the richer prompt when a profile exists,
        // otherwise stock search falls back to the query. `G("search")` and
        // `G("generate")` remain unambiguous because they emit only one field.
        PenNode::Image(image)
            if image
                .image_prompt
                .as_deref()
                .is_some_and(|prompt| !prompt.trim().is_empty())
                && image
                    .image_search_query
                    .as_deref()
                    .is_some_and(|query| !query.trim().is_empty()) =>
        {
            ImageRequestMode::Auto
        }
        PenNode::Image(image)
            if image
                .image_prompt
                .as_deref()
                .is_some_and(|prompt| !prompt.trim().is_empty()) =>
        {
            ImageRequestMode::Generate
        }
        PenNode::Image(image)
            if image
                .image_search_query
                .as_deref()
                .is_some_and(|query| !query.trim().is_empty()) =>
        {
            ImageRequestMode::Search
        }
        PenNode::Frame(frame)
            if frame
                .image_search_query
                .as_deref()
                .is_some_and(|query| !query.trim().is_empty()) =>
        {
            ImageRequestMode::Search
        }
        _ => ImageRequestMode::Auto,
    }
}

fn is_placeholder_src(src: &str) -> bool {
    src.trim().is_empty() || src.starts_with("data:image/svg+xml;charset=utf-8,%3Csvg")
}

/// A single image fill whose url is still the placeholder/empty value —
/// the author asked for an image here but none has landed yet.
pub(super) fn has_empty_image_fill(node: &PenNode) -> bool {
    let container = match node {
        PenNode::Frame(frame) => &frame.container,
        PenNode::Rectangle(rect) => &rect.container,
        _ => return false,
    };
    let Some([PenFill::Image(body)]) = container.fill.as_deref() else {
        return false;
    };
    is_placeholder_src(&body.url)
}

fn is_image_placeholder_frame(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_)) && node.base().role.as_deref() == Some("image-placeholder")
}

pub(super) fn is_frame_placeholder_still_unfilled(node: &PenNode) -> bool {
    is_unfilled_image_placeholder_frame(node) || is_image_area_frame_by_heuristic(node)
}

fn is_unfilled_image_placeholder_frame(node: &PenNode) -> bool {
    if !is_image_placeholder_frame(node) {
        return false;
    }
    let PenNode::Frame(frame) = node else {
        return false;
    };
    match frame.container.fill.as_deref() {
        None | Some([]) => true,
        // An image fill whose url is still the placeholder value means the
        // slot was never actually filled — only a landed url counts as done.
        Some([PenFill::Image(body), ..]) => is_placeholder_src(&body.url),
        Some(_) => true,
    }
}

fn is_image_area_frame_by_heuristic(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    if frame.base.role.as_deref() == Some("image-placeholder") {
        return false;
    }
    let Some(name) = frame.base.name.as_deref() else {
        return false;
    };
    if !has_image_area_keyword(name) {
        return false;
    }
    if !is_image_area_size(&frame.container.width, &frame.container.height)
        && !is_small_thumb_size(&frame.container.width, &frame.container.height)
    {
        return false;
    }
    if !matches!(frame.container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    let Some(children) = frame.children.as_ref() else {
        return true;
    };
    matches!(children.as_slice(), [] | [PenNode::IconFont(_)])
        || matches!(children.as_slice(), [only] if is_empty_unfilled_frame(only))
}

/// A bare structural stub inside a media slot (an empty fill×fill frame the
/// model left as "where the picture goes") must not disqualify the slot.
fn is_empty_unfilled_frame(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    frame.container.fill.is_none()
        && frame
            .children
            .as_ref()
            .is_none_or(|children| children.is_empty())
}

/// An UNNAMED small square solid rectangle inside a media-named ancestor
/// ("Mini Player" > bare 44×44 rectangle, measured test0711-2-ds) — the
/// name-keyword gate lives on the ANCESTOR chain, so the artwork slot the
/// model left anonymous still enriches. The query is derived from the
/// surrounding names/labels as usual.
/// Nameless, childless, solid, rounded/clipping, roughly-square slot of at
/// least thumbnail size — the shape signature of a cover box. Only ever
/// consulted when TEXT SIBLINGS exist to supply the subject.
fn is_bare_anonymous_slot(node: &PenNode) -> bool {
    let (base, container) = match node {
        PenNode::Frame(f) => (&f.base, &f.container),
        PenNode::Rectangle(r) => (&r.base, &r.container),
        _ => return false,
    };
    if base.name.as_deref().is_some_and(|n| !n.trim().is_empty()) {
        return false;
    }
    let rounded = container.corner_radius.is_some() || container.clip_content == Some(true);
    if !rounded {
        return false;
    }
    let (Some(w), Some(h)) = (
        dimension_number(&container.width),
        dimension_number(&container.height),
    ) else {
        return false;
    };
    if w < 48.0 || h < 48.0 || w / h > 1.6 || h / w > 1.6 {
        return false;
    }
    if !matches!(container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    node.children().is_none_or(|c| c.is_empty())
}

fn is_unnamed_media_slot_in_context(node: &PenNode, parent_names: &[String]) -> bool {
    let PenNode::Rectangle(rect) = node else {
        return false;
    };
    if has_non_media_context(parent_names) {
        return false;
    }
    if rect
        .base
        .name
        .as_deref()
        .is_some_and(|name| !name.trim().is_empty())
    {
        return false;
    }
    if !is_small_thumb_size(&rect.container.width, &rect.container.height) {
        return false;
    }
    if !matches!(rect.container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    if rect
        .children
        .as_ref()
        .is_some_and(|children| !children.is_empty())
    {
        return false;
    }
    const CONTEXT_WORDS: [&str; 6] = ["player", "art", "cover", "album", "media", "track"];
    parent_names.iter().any(|name| {
        let lowered = name.to_ascii_lowercase();
        CONTEXT_WORDS.iter().any(|word| {
            lowered
                .split(|c: char| !c.is_ascii_alphanumeric())
                .any(|token| token == *word)
        })
    })
}

/// Explicit structural/control vocabulary vetoes the anonymous-slot fallback.
/// These surfaces often have the same small rounded solid geometry as cover
/// art, but filling a KPI, swatch, badge, or button with a photo is always a
/// worse failure than leaving an ambiguous box untouched. Explicit image
/// nodes, placeholder roles, and media-named slots do not depend on this
/// fallback and remain eligible.
fn has_non_media_context(parent_names: &[String]) -> bool {
    const NON_MEDIA_WORDS: [&str; 18] = [
        "kpi",
        "metric",
        "stat",
        "stats",
        "analytics",
        "chart",
        "graph",
        "swatch",
        "palette",
        "button",
        "control",
        "badge",
        "indicator",
        "progress",
        "separator",
        "divider",
        "toggle",
        "status",
    ];
    parent_names.iter().any(|name| {
        name.to_ascii_lowercase()
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| NON_MEDIA_WORDS.contains(&token))
    })
}

pub(super) fn is_image_area_rectangle_by_heuristic(node: &PenNode) -> bool {
    let PenNode::Rectangle(rect) = node else {
        return false;
    };
    let Some(name) = rect.base.name.as_deref() else {
        return false;
    };
    if !has_image_area_keyword(name) {
        return false;
    }
    if !is_image_area_size(&rect.container.width, &rect.container.height) {
        return false;
    }
    if !matches!(rect.container.fill.as_deref(), Some([PenFill::Solid(_)])) {
        return false;
    }
    let Some(children) = rect.children.as_ref() else {
        return true;
    };
    matches!(children.as_slice(), [] | [PenNode::IconFont(_)])
}

fn is_image_area_size(width: &Option<SizingBehavior>, height: &Option<SizingBehavior>) -> bool {
    let (width_ok, width_concrete) = image_area_dimension_ok(width, 80.0);
    let (height_ok, height_concrete) = image_area_dimension_ok(height, 60.0);
    width_ok && height_ok && (width_concrete || height_concrete)
}

/// Small keyword-named media slots — a 44×44 mini-player "Art" square sits
/// well below the generic 80×60 floor but is unmistakably an image slot
/// (measured: the mini-player artwork routinely shipped as an empty grey
/// square, test0711-22). Keyword gating keeps random small frames out.
fn is_small_thumb_size(width: &Option<SizingBehavior>, height: &Option<SizingBehavior>) -> bool {
    let (width_ok, width_concrete) = image_area_dimension_ok(width, 32.0);
    let (height_ok, height_concrete) = image_area_dimension_ok(height, 32.0);
    width_ok && height_ok && width_concrete && height_concrete
}

fn image_area_dimension_ok(size: &Option<SizingBehavior>, min_px: f64) -> (bool, bool) {
    match size {
        Some(SizingBehavior::Number(px)) if *px >= min_px => (true, true),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => (true, false),
        _ => (false, false),
    }
}

fn infer_aspect_ratio(width: Option<f64>, height: Option<f64>) -> Option<ImageAspectRatio> {
    let (Some(width), Some(height)) = (width, height) else {
        return None;
    };
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let ratio = width / height;
    if ratio > 1.3 {
        Some(ImageAspectRatio::Wide)
    } else if ratio < 0.77 {
        Some(ImageAspectRatio::Tall)
    } else {
        Some(ImageAspectRatio::Square)
    }
}

fn dimension_number(size: &Option<SizingBehavior>) -> Option<f64> {
    match size {
        Some(SizingBehavior::Number(px)) => Some(*px),
        _ => None,
    }
}

fn has_image_area_keyword(name: &str) -> bool {
    let compact: String = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    if compact.ends_with("img")
        || compact.ends_with("image")
        || compact.ends_with("photo")
        || compact.ends_with("cover")
        || compact.ends_with("thumbnail")
        || compact.ends_with("artwork")
        || compact.ends_with("media")
    {
        return true;
    }
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .any(|word| {
            matches!(
                word.as_str(),
                "image"
                    | "photo"
                    | "cover"
                    | "hero"
                    | "thumbnail"
                    | "thumb"
                    | "picture"
                    | "banner"
                    | "poster"
                    | "art"
                    | "artwork"
                    | "album"
                    | "avatar"
                    // The abbreviations weak models actually write: MiniMax-M3
                    // built every destination card around a rectangle named
                    // "img" (and a "ph" placeholder inside a frame named
                    // "img"), so a page of grey boxes shipped with no images at
                    // all (measured test0711-1-m3, 2026-07-12).
                    | "img"
                    | "pic"
                    | "media"
                    | "graphic"
                    | "illustration"
                    | "placeholder"
                    | "ph"
            )
        })
}

fn extract_query_for_node(node: &PenNode, parent_names: &[String]) -> String {
    if let PenNode::Image(image) = node {
        if let Some(query) = image
            .image_search_query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            return query.to_string();
        }
    }

    if let PenNode::Frame(frame) = node {
        if let Some(query) = frame
            .image_search_query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            return query.to_string();
        }
    }

    if is_image_placeholder_frame(node) {
        if let Some(label) = placeholder_label_text(node) {
            return label;
        }
    }

    if let Some(name) = node
        .base()
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if !is_generic_placeholder_name(name) {
            return name.to_string();
        }
    }

    if let Some(parent_name) = parent_semantic_name(parent_names) {
        return parent_name;
    }

    node.base()
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("placeholder")
        .to_string()
}

fn placeholder_label_text(node: &PenNode) -> Option<String> {
    let children = node.children()?;
    for child in children {
        let PenNode::Text(text) = child else {
            continue;
        };
        if text.base.role.as_deref() != Some("image-placeholder-label") {
            continue;
        }
        let label = match &text.content {
            TextContent::Plain(content) => content.trim().to_string(),
            TextContent::Styled(segments) => segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<String>()
                .trim()
                .to_string(),
        };
        if !label.is_empty() {
            return Some(label);
        }
    }
    None
}

fn parent_semantic_name(parent_names: &[String]) -> Option<String> {
    parent_names.iter().take(3).find_map(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty()
            || is_generic_placeholder_name(trimmed)
            || is_layout_context_name(trimmed)
        {
            return None;
        }
        Some(trimmed.to_string())
    })
}

fn is_generic_placeholder_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "image"
            | "photo"
            | "cover"
            | "hero"
            | "thumbnail"
            | "thumb"
            | "picture"
            | "banner"
            | "poster"
            | "image placeholder"
            | "placeholder icon"
            | "placeholder"
            // A slot named "img" / "ph" carries no subject of its own — the
            // picture it wants is named by the card AROUND it ("Santorini").
            | "img"
            | "ph"
            | "pic"
            | "media"
            | "graphic"
            | "card image"
            | "card photo"
            | "product image"
            | "item image"
    )
}

fn is_layout_context_name(name: &str) -> bool {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .any(|word| {
            matches!(
                word.as_str(),
                "card"
                    | "wrapper"
                    | "container"
                    | "section"
                    | "frame"
                    | "root"
                    | "page"
                    | "stack"
                    | "row"
                    | "column"
                    | "content"
            )
        })
}
