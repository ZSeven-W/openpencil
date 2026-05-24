//! Image-node property helpers shared by the panel snapshot and mutators.

use crate::editor_ui_state::{ImageAdjustmentField, ImageFillMode};
use crate::fills::ImageFillSummary;
use jian_ops_schema::node::PenNode;

/// Summary of a real `ImageNode` using the same UI payload as image
/// fills, so the native image editor popover can edit both surfaces.
pub fn image_node_summary(node: &PenNode) -> Option<ImageFillSummary> {
    let PenNode::Image(image) = node else {
        return None;
    };
    let trimmed_url = image.src.trim();
    Some(ImageFillSummary {
        mode: ImageFillMode::from_image_node_schema(image.object_fit.as_ref()),
        has_image: !trimmed_url.is_empty(),
        image_url: (!trimmed_url.is_empty()).then(|| image.src.clone()),
        exposure: image.exposure.unwrap_or(0.0) as f32,
        contrast: image.contrast.unwrap_or(0.0) as f32,
        saturation: image.saturation.unwrap_or(0.0) as f32,
        temperature: image.temperature.unwrap_or(0.0) as f32,
        tint: image.tint.unwrap_or(0.0) as f32,
        highlights: image.highlights.unwrap_or(0.0) as f32,
        shadows: image.shadows.unwrap_or(0.0) as f32,
    })
}

pub fn set_image_node_mode(node: &mut PenNode, mode: ImageFillMode) -> bool {
    let PenNode::Image(image) = node else {
        return false;
    };
    image.object_fit = Some(mode.to_image_node_schema());
    true
}

pub fn set_image_node_adjustment(
    node: &mut PenNode,
    field: ImageAdjustmentField,
    value: f32,
) -> bool {
    let PenNode::Image(image) = node else {
        return false;
    };
    let value = value.clamp(-100.0, 100.0) as f64;
    match field {
        ImageAdjustmentField::Exposure => image.exposure = Some(value),
        ImageAdjustmentField::Contrast => image.contrast = Some(value),
        ImageAdjustmentField::Saturation => image.saturation = Some(value),
        ImageAdjustmentField::Temperature => image.temperature = Some(value),
        ImageAdjustmentField::Tint => image.tint = Some(value),
        ImageAdjustmentField::Highlights => image.highlights = Some(value),
        ImageAdjustmentField::Shadows => image.shadows = Some(value),
    }
    true
}

pub fn reset_image_node_adjustments(node: &mut PenNode) -> bool {
    let PenNode::Image(image) = node else {
        return false;
    };
    image.exposure = Some(0.0);
    image.contrast = Some(0.0);
    image.saturation = Some(0.0);
    image.temperature = Some(0.0);
    image.tint = Some(0.0);
    image.highlights = Some(0.0);
    image.shadows = Some(0.0);
    true
}
