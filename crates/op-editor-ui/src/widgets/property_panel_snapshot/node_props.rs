//! Geometry / container / layout extractors read by
//! [`super::NodeSnapshot::from_node`] — opacity, arc + polygon
//! parameters, flex container props, sizing keywords, corner radii
//! and the icon summary.
//!
//! Split out of `property_panel_snapshot.rs` to keep both files under
//! the openpencil 800-line cap.

use super::{EllipseArcSummary, IconSummary, LayoutPaddingSummary, VideoSummary};
use crate::widgets::property_panel_action::{LayoutAlignValue, LayoutJustifyValue};
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::container::{AlignItems, JustifyContent, Padding};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::pen_node_ext::PenNodeExt;

pub(super) fn node_opacity_percent(node: &PenNode) -> f32 {
    match node.base().opacity.as_ref() {
        Some(NumberOrExpression::Number(value)) if value.is_finite() => {
            (value.clamp(0.0, 1.0) * 100.0) as f32
        }
        Some(NumberOrExpression::Expression(_)) | None | Some(NumberOrExpression::Number(_)) => {
            100.0
        }
    }
}

pub(super) fn polygon_sides_of(node: &PenNode) -> Option<u32> {
    match node {
        PenNode::Polygon(n) => Some(n.polygon_count.clamp(3, 100)),
        _ => None,
    }
}

pub(super) fn video_summary_of(node: &PenNode) -> Option<VideoSummary> {
    let PenNode::Image(image) = node else {
        return None;
    };
    let video = image.video.as_ref()?;
    Some(VideoSummary {
        src: video.src.clone(),
        poster_src: image.src.to_string(),
        autoplay: video.autoplay,
        loop_video: video.r#loop,
        muted: video.muted,
        hold_last_frame: video.hold_last_frame,
        click_to_replay: video.click_to_replay,
    })
}

pub(super) fn ellipse_arc_of(node: &PenNode) -> Option<EllipseArcSummary> {
    match node {
        PenNode::Ellipse(n) => Some(EllipseArcSummary {
            start_deg: n.start_angle.unwrap_or(0.0) as f32,
            sweep_deg: n.sweep_angle.unwrap_or(360.0) as f32,
            inner_percent: (n.inner_radius.unwrap_or(0.0).clamp(0.0, 0.99) * 100.0) as f32,
        }),
        _ => None,
    }
}

fn container_layout(node: &PenNode) -> Option<&LayoutMode> {
    match node {
        PenNode::Frame(n) => n.container.layout.as_ref(),
        PenNode::Group(n) => n.container.layout.as_ref(),
        PenNode::Rectangle(n) => n.container.layout.as_ref(),
        _ => None,
    }
}

pub(super) fn flex_layout_of(node: &PenNode) -> op_editor_core::FlexLayout {
    match container_layout(node) {
        Some(LayoutMode::Vertical) => op_editor_core::FlexLayout::Vertical,
        Some(LayoutMode::Horizontal) => op_editor_core::FlexLayout::Horizontal,
        Some(LayoutMode::None) | None => op_editor_core::FlexLayout::Free,
    }
}

pub(super) fn layout_justify_of(node: &PenNode) -> LayoutJustifyValue {
    let value = match node {
        PenNode::Frame(n) => n.container.justify_content.as_ref(),
        PenNode::Group(n) => n.container.justify_content.as_ref(),
        PenNode::Rectangle(n) => n.container.justify_content.as_ref(),
        _ => None,
    };
    match value.unwrap_or(&JustifyContent::Start) {
        JustifyContent::Start => LayoutJustifyValue::Start,
        JustifyContent::Center => LayoutJustifyValue::Center,
        JustifyContent::End => LayoutJustifyValue::End,
        JustifyContent::SpaceBetween => LayoutJustifyValue::SpaceBetween,
        JustifyContent::SpaceAround => LayoutJustifyValue::SpaceAround,
    }
}

pub(super) fn layout_align_of(node: &PenNode) -> LayoutAlignValue {
    let value = match node {
        PenNode::Frame(n) => n.container.align_items.as_ref(),
        PenNode::Group(n) => n.container.align_items.as_ref(),
        PenNode::Rectangle(n) => n.container.align_items.as_ref(),
        _ => None,
    };
    match value.unwrap_or(&AlignItems::Start) {
        AlignItems::Start => LayoutAlignValue::Start,
        AlignItems::Center => LayoutAlignValue::Center,
        AlignItems::End => LayoutAlignValue::End,
        // `stretch` renders as start (see jian resolve_align), so the
        // panel surfaces Start — matching the rendered alignment.
        AlignItems::Stretch => LayoutAlignValue::Start,
    }
}

pub(super) fn layout_gap_of(node: &PenNode) -> f32 {
    let gap = match node {
        PenNode::Frame(n) => n.container.gap.as_ref(),
        PenNode::Group(n) => n.container.gap.as_ref(),
        PenNode::Rectangle(n) => n.container.gap.as_ref(),
        _ => None,
    };
    match gap {
        Some(NumberOrExpression::Number(v)) => *v as f32,
        Some(NumberOrExpression::Expression(_)) | None => 0.0,
    }
}

pub(super) fn layout_padding_of(node: &PenNode) -> LayoutPaddingSummary {
    let padding = match node {
        PenNode::Frame(n) => n.container.padding.as_ref(),
        PenNode::Group(n) => n.container.padding.as_ref(),
        PenNode::Rectangle(n) => n.container.padding.as_ref(),
        _ => None,
    };
    match padding {
        Some(Padding::Uniform(v)) => {
            let v = *v as f32;
            LayoutPaddingSummary {
                top: v,
                right: v,
                bottom: v,
                left: v,
            }
        }
        Some(Padding::XY(v)) => LayoutPaddingSummary {
            top: v[0] as f32,
            right: v[1] as f32,
            bottom: v[0] as f32,
            left: v[1] as f32,
        },
        Some(Padding::LtrB(v)) => LayoutPaddingSummary {
            top: v[0] as f32,
            right: v[1] as f32,
            bottom: v[2] as f32,
            left: v[3] as f32,
        },
        Some(Padding::Expression(_)) | None => LayoutPaddingSummary::ZERO,
    }
}

pub(super) fn node_width_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.width.as_ref(),
        PenNode::Group(n) => n.container.width.as_ref(),
        PenNode::Rectangle(n) => n.container.width.as_ref(),
        PenNode::Ellipse(n) => n.width.as_ref(),
        PenNode::Polygon(n) => n.width.as_ref(),
        PenNode::Path(n) => n.width.as_ref(),
        PenNode::Text(n) => n.width.as_ref(),
        PenNode::TextInput(n) => n.width.as_ref(),
        PenNode::TextArea(n) => n.width.as_ref(),
        PenNode::Select(n) => n.width.as_ref(),
        PenNode::Switch(n) => n.width.as_ref(),
        PenNode::Checkbox(n) => n.width.as_ref(),
        PenNode::Slider(n) => n.width.as_ref(),
        PenNode::RadioGroup(n) => n.width.as_ref(),
        PenNode::NumberInput(n) => n.width.as_ref(),
        PenNode::Progress(n) => n.width.as_ref(),
        PenNode::Tabs(n) => n.width.as_ref(),
        PenNode::Image(n) => n.width.as_ref(),
        PenNode::IconFont(n) => n.width.as_ref(),
        PenNode::Line(_) | PenNode::Ref(_) => None,
    }
}

pub(super) fn node_height_sizing(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
        PenNode::Frame(n) => n.container.height.as_ref(),
        PenNode::Group(n) => n.container.height.as_ref(),
        PenNode::Rectangle(n) => n.container.height.as_ref(),
        PenNode::Ellipse(n) => n.height.as_ref(),
        PenNode::Polygon(n) => n.height.as_ref(),
        PenNode::Path(n) => n.height.as_ref(),
        PenNode::Text(n) => n.height.as_ref(),
        PenNode::TextInput(n) => n.height.as_ref(),
        PenNode::TextArea(n) => n.height.as_ref(),
        PenNode::Select(n) => n.height.as_ref(),
        PenNode::Switch(n) => n.height.as_ref(),
        PenNode::Checkbox(n) => n.height.as_ref(),
        PenNode::Slider(n) => n.height.as_ref(),
        PenNode::RadioGroup(n) => n.height.as_ref(),
        PenNode::NumberInput(n) => n.height.as_ref(),
        PenNode::Progress(n) => n.height.as_ref(),
        PenNode::Tabs(n) => n.height.as_ref(),
        PenNode::Image(n) => n.height.as_ref(),
        PenNode::IconFont(n) => n.height.as_ref(),
        PenNode::Line(_) | PenNode::Ref(_) => None,
    }
}

pub(super) fn sizing_is(sizing: Option<&SizingBehavior>, keyword: SizingKeyword) -> bool {
    matches!(sizing, Some(SizingBehavior::Keyword(k)) if *k == keyword)
}

pub(super) fn clip_content_of(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => n.container.clip_content.unwrap_or(false),
        PenNode::Group(n) => n.container.clip_content.unwrap_or(false),
        PenNode::Rectangle(n) => n.container.clip_content.unwrap_or(false),
        _ => false,
    }
}

pub(super) fn can_clip_content(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_)
    )
}

pub(super) fn icon_summary_of(node: &PenNode) -> Option<IconSummary> {
    match node {
        PenNode::IconFont(n) => {
            let family = n
                .icon_font_family
                .clone()
                .unwrap_or_else(|| "lucide".to_string());
            Some(IconSummary {
                icon_id: format!("{}:{}", family, n.icon_font_name),
                family,
                name: n.icon_font_name.clone(),
            })
        }
        PenNode::Path(n) => {
            let icon_id = n.icon_id.as_ref()?;
            let (family, name) = icon_id
                .split_once(':')
                .map(|(family, name)| (family.to_string(), name.to_string()))
                .unwrap_or_else(|| ("lucide".to_string(), icon_id.clone()));
            Some(IconSummary {
                family,
                name,
                icon_id: icon_id.clone(),
            })
        }
        _ => None,
    }
}

pub(super) fn has_corner_radius(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_)
            | PenNode::Rectangle(_)
            | PenNode::Ellipse(_)
            | PenNode::Polygon(_)
            | PenNode::Image(_)
    )
}

pub(super) fn can_create_component(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_) | PenNode::Ref(_)
    )
}

/// Uniform corner radius (doc-px) for a container variant — Frame /
/// Group / Rectangle carry a `CornerRadius`. A `PerCorner` radius
/// reports its top-left value. Non-container variants read 0.
pub(super) fn container_corner_radius(node: &PenNode) -> f32 {
    use jian_ops_schema::node::container::CornerRadius;
    let cr = match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref(),
        PenNode::Group(n) => n.container.corner_radius.as_ref(),
        PenNode::Rectangle(n) => n.container.corner_radius.as_ref(),
        PenNode::Image(n) => n.corner_radius.as_ref(),
        _ => None,
    };
    match cr {
        Some(CornerRadius::Uniform(r)) => *r as f32,
        Some(CornerRadius::PerCorner(c)) => c[0] as f32,
        None => match node {
            PenNode::Ellipse(n) => n.corner_radius.unwrap_or(0.0) as f32,
            PenNode::Polygon(n) => n.corner_radius.unwrap_or(0.0) as f32,
            _ => 0.0,
        },
    }
}

pub(super) fn container_corner_radii(node: &PenNode) -> Option<[f32; 4]> {
    use jian_ops_schema::node::container::CornerRadius;
    let radius = match node {
        PenNode::Frame(node) => node.container.corner_radius.as_ref(),
        PenNode::Group(node) => node.container.corner_radius.as_ref(),
        PenNode::Rectangle(node) => node.container.corner_radius.as_ref(),
        PenNode::Image(node) => node.corner_radius.as_ref(),
        _ => return None,
    };
    Some(match radius {
        Some(CornerRadius::Uniform(value)) => [*value as f32; 4],
        Some(CornerRadius::PerCorner(values)) => values.map(|value| value as f32),
        None => [0.0; 4],
    })
}
