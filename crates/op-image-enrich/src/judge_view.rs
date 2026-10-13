//! The visible geometry of an automatic image slot, independent of transport.

use jian_ops_schema::node::{ImageFitMode, PenNode};
use jian_ops_schema::style::{ImageFillMode, PenFill};

/// Sampling modes with a deterministic, centered image view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageJudgeFit {
    Cover,
    Contain,
    Stretch,
}

/// Exact slot ratio, rather than the coarse catalogue aspect bucket. This
/// affects only the judge preview; the saved asset always remains uncropped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageJudgeView {
    ratio: f64,
    fit: ImageJudgeFit,
}

impl ImageJudgeView {
    pub fn new(width: f64, height: f64, fit: ImageJudgeFit) -> Option<Self> {
        let ratio = width / height;
        (width.is_finite()
            && height.is_finite()
            && width > 0.0
            && height > 0.0
            && ratio.is_finite()
            && ratio > 0.0)
            .then_some(Self { ratio, fit })
    }

    pub fn ratio(self) -> f64 {
        self.ratio
    }
    pub fn fit(self) -> ImageJudgeFit {
        self.fit
    }

    /// Equivalent scaled slots share a view, but different ratios in the
    /// same Wide/Tall bucket or different fit modes must never share a judge.
    pub fn cache_key(self) -> String {
        format!("{:?}:{:016x}", self.fit, self.ratio.to_bits())
    }
}

pub(crate) fn for_node(
    node: &PenNode,
    width: Option<f64>,
    height: Option<f64>,
) -> Option<ImageJudgeView> {
    let fit = match node {
        PenNode::Image(image) => match image.object_fit {
            Some(ImageFitMode::Fit) => ImageJudgeFit::Contain,
            Some(ImageFitMode::Tile) => return None,
            _ => ImageJudgeFit::Cover,
        },
        PenNode::Frame(frame) => fill_fit(frame.container.fill.as_deref())?,
        PenNode::Rectangle(rect) => fill_fit(rect.container.fill.as_deref())?,
        _ => return None,
    };
    ImageJudgeView::new(width?, height?, fit)
}

fn fill_fit(fills: Option<&[PenFill]>) -> Option<ImageJudgeFit> {
    // apply_result preserves an existing single image fill, otherwise creates
    // a centered Crop fill. Do not pretend an affine/tiled paint is centered.
    let Some([PenFill::Image(body)]) = fills else {
        return Some(ImageJudgeFit::Cover);
    };
    if body.transform.is_some() {
        return None;
    }
    match body.mode {
        Some(ImageFillMode::Fit) => Some(ImageJudgeFit::Contain),
        Some(ImageFillMode::Stretch) => Some(ImageJudgeFit::Stretch),
        Some(ImageFillMode::Tile) => None,
        _ => Some(ImageJudgeFit::Cover),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_node(fit: &str) -> PenNode {
        serde_json::from_value(serde_json::json!({
            "id":"image", "type":"image", "src":"", "width":132, "height":74,
            "objectFit":fit
        }))
        .unwrap()
    }

    #[test]
    fn exact_ratio_and_fit_define_view_identity() {
        let a = ImageJudgeView::new(132.0, 74.0, ImageJudgeFit::Cover).unwrap();
        let b = ImageJudgeView::new(264.0, 148.0, ImageJudgeFit::Cover).unwrap();
        let c = ImageJudgeView::new(148.0, 74.0, ImageJudgeFit::Cover).unwrap();
        let fit = ImageJudgeView::new(132.0, 74.0, ImageJudgeFit::Contain).unwrap();
        assert_eq!(a.cache_key(), b.cache_key());
        assert_ne!(a.cache_key(), c.cache_key());
        assert_ne!(a.cache_key(), fit.cache_key());
    }

    #[test]
    fn invalid_or_missing_geometry_never_invents_a_ratio() {
        for (w, h) in [
            (0.0, 74.0),
            (132.0, 0.0),
            (-1.0, 10.0),
            (f64::NAN, 1.0),
            (1.0, f64::INFINITY),
        ] {
            assert!(ImageJudgeView::new(w, h, ImageJudgeFit::Cover).is_none());
        }
        assert!(for_node(&image_node("fill"), None, Some(74.0)).is_none());
    }

    #[test]
    fn authored_fit_uses_resolved_dimensions_without_mutating_node() {
        let node = image_node("fit");
        let before = node.clone();
        let view = for_node(&node, Some(200.0), Some(100.0)).unwrap();
        assert_eq!(view.fit(), ImageJudgeFit::Contain);
        assert_eq!(view.ratio(), 2.0);
        assert_eq!(node, before);
        assert!(for_node(&image_node("tile"), Some(132.0), Some(74.0)).is_none());
    }

    #[test]
    fn placeholder_fill_matches_apply_result_crop_default() {
        let node: PenNode = serde_json::from_value(serde_json::json!({
            "id":"slot", "type":"frame", "width":132, "height":74
        }))
        .unwrap();
        assert_eq!(
            for_node(&node, Some(132.0), Some(74.0)).unwrap().fit(),
            ImageJudgeFit::Cover
        );
    }
}
