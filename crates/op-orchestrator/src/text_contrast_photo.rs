//! Contrast evidence from the actual picture under direct overlay text.
use super::*;
use jian_scene::layout_scene::SceneImageFit;
use serde_json::json;

/// Run the same caption repair when asynchronous image enrichment lands.
pub fn repair_for_image(state: &mut EditorState, image_id: &NodeId) -> usize {
    let root = state
        .active_children()
        .iter()
        .find(|root| {
            op_editor_core::walkers::find_node(std::slice::from_ref(*root), image_id).is_some()
        })
        .map(|n| n.id_str().to_owned());
    let Some(root) = root else {
        return 0;
    };
    let mut sink = crate::loop_finalize::StateDocSink { state };
    repair(&mut sink, &root)
}

pub(super) enum OverlayBackground {
    Unrelated,
    Photo,
    Solid(String, String),
}

pub(super) fn effective_background(
    node: &PenNode,
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    rects: &HashMap<String, ResolvedRect>,
) -> Option<LocatedBackground> {
    match overlay_background(node, ancestors, variables, theme, rects) {
        OverlayBackground::Unrelated => nearest_background(ancestors, variables, theme),
        OverlayBackground::Photo => None,
        OverlayBackground::Solid(color, id) => Some(LocatedBackground {
            colors: vec![color],
            gradient: None,
            source_node_id: Some(id),
            source_index: None,
        }),
    }
}

pub(super) fn overlay_background(
    node: &PenNode,
    ancestors: &[&PenNode],
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    rects: &HashMap<String, ResolvedRect>,
) -> OverlayBackground {
    let Some(parent) = ancestors.iter().rev().find(|parent| {
        let value = serde_json::to_value(parent).unwrap_or_default();
        value.get("layout").and_then(serde_json::Value::as_str) == Some("none")
            && node_children_of(parent)
                .iter()
                .any(|n| matches!(n, PenNode::Image(_)))
    }) else {
        return OverlayBackground::Unrelated;
    };
    let kids = node_children_of(parent);
    let owner = ancestors
        .iter()
        .position(|n| n.id_str() == parent.id_str())
        .unwrap_or(0);
    if ancestors[owner + 1..].iter().any(|n| {
        !matches!(
            resolve_fill_kind(node_fills(n), variables, theme),
            ResolvedFill::Transparent
        )
    }) {
        return OverlayBackground::Unrelated;
    }
    let Some(at) = kids.iter().position(|n| {
        op_editor_core::walkers::find_node(std::slice::from_ref(n), &NodeId::new(node.id_str()))
            .is_some()
    }) else {
        return OverlayBackground::Unrelated;
    };
    let Some(text) = rects.get(node.id_str()) else {
        return OverlayBackground::Unrelated;
    };
    let covers = |r: &ResolvedRect| {
        r.x <= text.x
            && r.y <= text.y
            && r.x + r.w >= text.x + text.w
            && r.y + r.h >= text.y + text.h
    };
    for behind in &kids[at + 1..] {
        if !rects.get(behind.id_str()).is_some_and(covers) {
            continue;
        }
        if let ResolvedFill::Solid(rgba) = resolve_fill_kind(node_fills(behind), variables, theme) {
            if rgba[3] == 255 {
                return OverlayBackground::Solid(rgb_hex(rgba), behind.id_str().into());
            }
        }
        if matches!(behind, PenNode::Image(_)) {
            return OverlayBackground::Photo;
        }
    }
    OverlayBackground::Unrelated
}

fn transparent_caption_texts<'a>(
    node: &'a PenNode,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
    out: &mut Vec<&'a PenNode>,
) {
    if matches!(node, PenNode::Text(_)) {
        out.push(node);
        return;
    }
    if !matches!(
        resolve_fill_kind(node_fills(node), variables, theme),
        ResolvedFill::Transparent
    ) {
        return;
    }
    for child in node_children_of(node) {
        transparent_caption_texts(child, variables, theme, out);
    }
}

pub(super) fn repair(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(sink.state());
    let Some(page) = scene.active_page() else {
        return 0;
    };
    let Some(root) = sink
        .state()
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id)
    else {
        return 0;
    };
    let root = root.clone();
    let doc = document_for_lint(sink.state());
    let variables = doc.variables.clone().unwrap_or_default();
    let theme = op_design_lint::node_util::default_theme(doc.themes.as_ref());
    visit(sink, &root, page, &variables, &theme)
}

fn visit(
    sink: &mut dyn DocSink,
    node: &PenNode,
    page: &jian_scene::layout_scene::ScenePage,
    variables: &op_design_lint::node_util::Variables,
    theme: &op_design_lint::node_util::Theme,
) -> usize {
    let mut count = 0;
    let kids = node_children_of(node);
    let value = serde_json::to_value(node).unwrap_or_default();
    if value.get("layout").and_then(serde_json::Value::as_str) == Some("none") {
        if let Some((photo_index, photo)) = kids.iter().enumerate().rev().find_map(|(i, n)| match n
        {
            PenNode::Image(_) => Some((i, n)),
            _ => None,
        }) {
            if let Some(image) = page.find(photo.id_str()).filter(|n| {
                n.image_transform.is_none()
                    && !n.hidden
                    && n.opacity >= 0.99
                    && n.image_adjustments == Default::default()
                    && n.image_blend_mode == Default::default()
            }) {
                let fit = match image.image_fit {
                    SceneImageFit::Fill | SceneImageFit::Crop => "cover",
                    SceneImageFit::Fit => "contain",
                    SceneImageFit::Stretch => "stretch",
                    _ => "unsupported",
                };
                if let Some(src) = image.image_src.as_deref() {
                    let mut captions = Vec::new();
                    for overlay in &kids[..photo_index] {
                        transparent_caption_texts(overlay, variables, theme, &mut captions);
                    }
                    for caption in captions {
                        let index = kids
                            .iter()
                            .position(|n| {
                                op_editor_core::walkers::find_node(
                                    std::slice::from_ref(n),
                                    &NodeId::new(caption.id_str()),
                                )
                                .is_some()
                            })
                            .unwrap_or(0);
                        if !matches!(caption, PenNode::Text(_))
                            || !is_node_visible(caption)
                            || opacity(caption) < 0.99
                        {
                            continue;
                        }
                        let Some(rect) = page.find(caption.id_str()).map(|n| n.bounds) else {
                            continue;
                        };
                        let region = [
                            rect.origin.x as f64,
                            rect.origin.y as f64,
                            rect.size.x as f64,
                            rect.size.y as f64,
                        ];
                        // A supplied solid backing already has the generic palette
                        // contrast contract; do not add a second panel over it.
                        let backed = kids[index + 1..photo_index].iter().any(|n| {
                            let Some(box_) = page.find(n.id_str()).map(|n| n.bounds) else {
                                return false;
                            };
                            first_usable_fill_kind(node_fills(n)).is_solid_opaque(variables, theme)
                                && box_.origin.x <= rect.origin.x
                                && box_.origin.y <= rect.origin.y
                                && box_.origin.x + box_.size.x >= rect.origin.x + rect.size.x
                                && box_.origin.y + box_.size.y >= rect.origin.y + rect.size.y
                        });
                        if backed {
                            continue;
                        }
                        let ib = image.bounds;
                        let Some(samples) = op_image_enrich::pixels::embedded_rgb_samples_in_box(
                            src,
                            [
                                ib.origin.x as f64,
                                ib.origin.y as f64,
                                ib.size.x as f64,
                                ib.size.y as f64,
                            ],
                            region,
                            fit,
                        ) else {
                            continue;
                        };
                        let Some(ink) = resolved_text_color(node_fills(caption), variables, theme)
                        else {
                            continue;
                        };
                        let colors: Vec<_> = samples
                            .into_iter()
                            .map(|p| rgb_hex([p[0], p[1], p[2], 255]))
                            .collect();
                        if colors.iter().all(|bg| {
                            op_design_lint::color::color_contrast(&ink, bg) >= TARGET_RATIO
                        }) {
                            continue;
                        }
                        // Keep the authored ink and picture. Use an existing
                        // palette surface as a small opaque contrast backing.
                        let token = CANDIDATE_TOKENS.iter().find(|token| {
                            resolve_color_ref(&format!("${token}"), variables, theme).is_some_and(
                                |bg| {
                                    op_design_lint::color::color_contrast(&ink, &bg) >= TARGET_RATIO
                                },
                            )
                        });
                        let (Some(token), Some(parent)) = (token, page.find(node.id_str())) else {
                            continue;
                        };
                        let panel:PenNode=serde_json::from_value(json!({"type":"rectangle","id":"text-backing","name":"Text contrast backing",
                            "x":region[0]-parent.bounds.origin.x as f64-2.0,"y":region[1]-parent.bounds.origin.y as f64-2.0,
                            "width":region[2]+4.0,"height":region[3]+4.0,"cornerRadius":4,
                            "constraints":{"h":"left","v":"top"},"fill":[{"type":"solid","color":format!("${token}")}]})).unwrap();
                        if let Some(ids) = sink.insert_subtree_returning_root_ids(
                            vec![panel],
                            &NodeId::new(node.id_str()),
                        ) {
                            if let Some(id) = ids.first() {
                                let at = op_editor_core::walkers::find_node(
                                    sink.state().active_children(),
                                    &NodeId::new(node.id_str()),
                                )
                                .and_then(|n| n.children())
                                .and_then(|kids| {
                                    kids.iter().position(|n| n.id_str() == caption.id_str())
                                })
                                .unwrap_or(index);
                                sink.apply(EditorCommand::MoveNode {
                                    node_id: NodeId::new(id),
                                    target_parent: NodeId::new(node.id_str()),
                                    page_id: None,
                                    index: Some(at + 1),
                                });
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    for child in kids {
        count += visit(sink, child, page, variables, theme);
    }
    count
}

#[cfg(test)]
#[path = "text_contrast_photo_tests.rs"]
mod tests;

impl FillKind {
    fn is_solid_opaque(
        &self,
        variables: &op_design_lint::node_util::Variables,
        theme: &op_design_lint::node_util::Theme,
    ) -> bool {
        match self {
            Self::Solid(raw) => resolve_color_ref(raw, variables, theme)
                .and_then(|c| parse_color_rgba(&c))
                .is_some_and(|c| c[3] == 255),
            _ => false,
        }
    }
}
