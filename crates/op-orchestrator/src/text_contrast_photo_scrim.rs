//! One sampled, gradual scrim for loose captions instead of opaque text tiles.
use super::super::gradient::{resolve_gradient, sample_at_text, GradientSource};
use super::*;

pub(super) struct Context<'a> {
    pub node: &'a PenNode,
    pub page: &'a jian_scene::layout_scene::ScenePage,
    pub variables: &'a op_design_lint::node_util::Variables,
    pub theme: &'a op_design_lint::node_util::Theme,
    pub image_box: [f64; 4],
    pub src: &'a str,
    pub fit: &'a str,
}

struct Evidence {
    ink: String,
    region: [f64; 4],
    colors: Vec<String>,
}

/// Abstain on mixed paint order, badges, transformed or unavailable pictures.
/// Those keep the existing conservative repair rather than guessed contrast.
pub(super) fn repair(
    sink: &mut dyn DocSink,
    cx: Context<'_>,
    captions: &[&PenNode],
    photo_index: usize,
) -> Option<usize> {
    if captions.is_empty() {
        return Some(0);
    }
    let kids = node_children_of(cx.node);
    let b = cx.page.find(cx.node.id_str())?.bounds;
    let owner = ResolvedRect {
        x: b.origin.x as f64,
        y: b.origin.y as f64,
        w: b.size.x as f64,
        h: b.size.y as f64,
    };
    if owner.w <= 0.0
        || owner.h <= 0.0
        || cx.image_box[0] > owner.x + 0.5
        || cx.image_box[1] > owner.y + 0.5
        || cx.image_box[0] + cx.image_box[2] < owner.x + owner.w - 0.5
        || cx.image_box[1] + cx.image_box[3] < owner.y + owner.h - 0.5
    {
        return None;
    }
    let mut evidence = Vec::new();
    let mut branches = Vec::new();
    let mut union = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for caption in captions {
        if !is_node_visible(caption) || opacity(caption) < 0.99 {
            return None;
        }
        let index = kids.iter().position(|n| {
            op_editor_core::walkers::find_node(
                std::slice::from_ref(n),
                &NodeId::new(caption.id_str()),
            )
            .is_some()
        })?;
        if !caption_tints(&kids[index], caption.id_str(), cx.variables, cx.theme).is_empty() {
            return None;
        }
        let rect = cx.page.find(caption.id_str())?.bounds;
        let region = [
            rect.origin.x as f64,
            rect.origin.y as f64,
            rect.size.x as f64,
            rect.size.y as f64,
        ];
        if region[2] <= 0.0
            || region[3] <= 0.0
            || region[0] < owner.x
            || region[1] < owner.y
            || region[0] + region[2] > owner.x + owner.w + 0.5
            || region[1] + region[3] > owner.y + owner.h + 0.5
        {
            return None;
        }
        let samples = op_image_enrich::pixels::embedded_rgb_samples_in_box(
            cx.src,
            cx.image_box,
            region,
            cx.fit,
        )?;
        let colors = layers::composite_samples(
            &samples,
            &kids[index + 1..photo_index],
            cx.page,
            region,
            cx.variables,
            cx.theme,
        )?;
        let ink = resolved_text_color(node_fills(caption), cx.variables, cx.theme)?;
        if parse_color_rgba(&ink)?[3] != 255 {
            return None;
        }
        union[0] = union[0].min(region[0]);
        union[1] = union[1].min(region[1]);
        union[2] = union[2].max(region[0] + region[2]);
        union[3] = union[3].max(region[1] + region[3]);
        branches.push(index);
        evidence.push(Evidence {
            ink,
            region,
            colors,
        });
    }
    if evidence.iter().all(|e| {
        e.colors
            .iter()
            .all(|bg| op_design_lint::color::color_contrast(&e.ink, bg) >= TARGET_RATIO)
    }) {
        return Some(0);
    }
    let at = *branches.iter().max()? + 1;
    // New scrim must paint above existing background layers, below every
    // caption branch. Interleaved paint cannot be reduced to that contract.
    let first = *branches.iter().min()?;
    if kids[first..at].iter().enumerate().any(|(offset, n)| {
        let i = first + offset;
        !branches.contains(&i)
            && !matches!(
                resolve_fill_kind(node_fills(n), cx.variables, cx.theme),
                ResolvedFill::Transparent
            )
    }) {
        return None;
    }
    // Shade the nearest edge through the complete text bounds, then fade.
    let (angle, extent) = [
        (90.0, (union[2] - owner.x) / owner.w),
        (270.0, (owner.x + owner.w - union[0]) / owner.w),
        (180.0, (union[3] - owner.y) / owner.h),
        (0.0, (owner.y + owner.h - union[1]) / owner.h),
    ]
    .into_iter()
    .min_by(|a, b| a.1.total_cmp(&b.1))?;
    let plateau = (extent + 0.025).clamp(0.025, 0.99);
    let mut chosen = None;
    'colors: for token in CANDIDATE_TOKENS {
        let Some(color) = resolve_color_ref(&format!("${token}"), cx.variables, cx.theme) else {
            continue;
        };
        let Some(rgba) = parse_color_rgba(&color) else {
            continue;
        };
        if rgba[3] != 255 {
            continue;
        }
        let rgb = format!("#{:02X}{:02X}{:02X}", rgba[0], rgba[1], rgba[2]);
        for alpha in (8..=248).step_by(8) {
            let shade = format!("{rgb}{alpha:02X}");
            let clear = format!("{rgb}00");
            let stops = vec![(0.0, shade.clone()), (plateau, shade), (1.0, clear)];
            let gradient = resolve_gradient(
                GradientSource::linear(angle, stops.clone()),
                cx.variables,
                cx.theme,
            )?;
            let passes = evidence.iter().all(|e| {
                e.colors.iter().enumerate().all(|(index, bg)| {
                    let Some(under) = parse_color_rgba(bg) else {
                        return false;
                    };
                    let mut sampled = gradient.clone();
                    sampled.flatten_over(under);
                    let point = ResolvedRect {
                        x: e.region[0] + e.region[2] * ((index % 5) as f64 + 0.5) / 5.0 - 0.005,
                        y: e.region[1] + e.region[3] * ((index / 5) as f64 + 0.5) / 5.0 - 0.005,
                        w: 0.01,
                        h: 0.01,
                    };
                    sample_at_text(&sampled, owner, point).is_some_and(|paint| {
                        op_design_lint::color::color_contrast(&e.ink, &paint) >= TARGET_RATIO + 0.1
                    })
                })
            });
            if passes {
                chosen = Some(stops);
                break 'colors;
            }
        }
    }
    let stops: Vec<_> = chosen?
        .into_iter()
        .map(|(offset, color)| json!({"offset":offset,"color":color}))
        .collect();
    let scrim:PenNode = serde_json::from_value(json!({"type":"rectangle","id":"photo-scrim",
        "name":"Photo contrast scrim","x":0,"y":0,"width":owner.w,"height":owner.h,
        "constraints":{"h":"left","v":"top"},"fill":[{"type":"linear_gradient","angle":angle,"stops":stops}]})).ok()?;
    let ids =
        sink.insert_subtree_returning_root_ids(vec![scrim], &NodeId::new(cx.node.id_str()))?;
    let id = ids.first()?;
    sink.apply(EditorCommand::MoveNode {
        node_id: NodeId::new(id),
        target_parent: NodeId::new(cx.node.id_str()),
        page_id: None,
        index: Some(at),
    });
    Some(1)
}
