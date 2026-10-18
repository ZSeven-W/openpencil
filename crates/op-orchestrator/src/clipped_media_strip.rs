//! Static proof of full-page media inside a clipped absolute viewport.
//!
//! This protects authored geometry only; it does not install carousel gestures.
//! Typed rail detection and JSON geometry collectors share the same predicate.

use jian_ops_schema::node::{
    base::{BoolOrExpression, NumberOrExpression},
    container::{LayoutMode, Padding},
    PenNode,
};
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};
use op_editor_core::PenNodeExt;
use serde_json::Value;

const EPS: f64 = 0.5;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Frame,
    Image,
    Decoration,
    Other,
}

struct Shape {
    kind: Kind,
    absolute: bool,
    horizontal: bool,
    vertical: bool,
    width: Option<f64>,
    height: Option<f64>,
    fill_width: bool,
    hug_height: bool,
    origin: bool,
    unpositioned: bool,
    clipped: bool,
    zero_padding: bool,
    gap: Option<f64>,
    image_paint: bool,
    trusted: bool,
}

trait MediaNode: Sized {
    fn shape(&self) -> Shape;
    fn children(&self) -> &[Self];
    fn is_media(&self) -> bool;
}

fn equal(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= EPS
}

fn positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn spans_width(shape: &Shape, width: f64) -> bool {
    shape.fill_width || shape.width.is_some_and(|value| equal(value, width))
}

fn viewport_shape<N: MediaNode>(node: &N, width: f64) -> Option<Shape> {
    let shape = node.shape();
    (positive(Some(width)).is_some()
        && shape.kind == Kind::Frame
        && shape.trusted
        && shape.absolute
        && shape.clipped
        && shape.zero_padding
        && spans_width(&shape, width)
        && (positive(shape.height).is_some() || shape.hug_height))
        .then_some(shape)
}

fn page_shape<N: MediaNode>(node: &N, width: f64, height: f64) -> bool {
    let shape = node.shape();
    shape.trusted
        && shape.zero_padding
        && shape.gap.is_some_and(|gap| gap.is_finite() && gap >= 0.0)
        && shape.unpositioned
        && shape.width.is_some_and(|value| equal(value, width))
        && shape.height.is_some_and(|value| equal(value, height))
        && node.is_media()
}

fn lane_shape<N: MediaNode>(node: &N, width: f64, viewport: &Shape) -> bool {
    let shape = node.shape();
    let Some(height) = positive(shape.height) else {
        return false;
    };
    let Some(gap) = shape.gap.filter(|gap| gap.is_finite() && *gap >= 0.0) else {
        return false;
    };
    let pages = node.children();
    shape.kind == Kind::Frame
        && shape.trusted
        && shape.horizontal
        && !shape.clipped
        && shape.origin
        && shape.zero_padding
        && pages.len() >= 2
        && (viewport.hug_height || viewport.height.is_some_and(|value| equal(value, height)))
        && shape.width.is_some_and(|value| {
            equal(
                value,
                width * pages.len() as f64 + gap * (pages.len() - 1) as f64,
            )
        })
        && pages.iter().all(|page| page_shape(page, width, height))
}

// Icon controls and pagination dots may overlay the media. Ordinary text,
// additional images and component aliases cannot be ignored as decorations.
fn overlay_tree<N: MediaNode>(node: &N) -> bool {
    let shape = node.shape();
    shape.trusted
        && !shape.image_paint
        && matches!(shape.kind, Kind::Frame | Kind::Decoration)
        && node.children().iter().all(overlay_tree)
}

fn overlay<N: MediaNode>(node: &N) -> bool {
    let shape = node.shape();
    shape.kind == Kind::Frame
        && !shape.unpositioned
        && positive(shape.width).is_some()
        && positive(shape.height).is_some()
        && overlay_tree(node)
}

fn find_lane<N: MediaNode>(viewport: &N, width: f64) -> Option<&N> {
    let shape = viewport_shape(viewport, width)?;
    let mut found = None;
    for child in viewport.children() {
        if lane_shape(child, width, &shape) {
            if found.replace(child).is_some() {
                return None;
            }
        } else if !overlay(child) {
            return None;
        }
    }
    found
}

pub(crate) fn media_lane(viewport: &Value, viewport_width: f64) -> Option<&Value> {
    find_lane(viewport, viewport_width)
}

fn full_width_viewport<N: MediaNode>(node: &N, width: f64) -> bool {
    if !node.shape().origin {
        return false;
    }
    if find_lane(node, width).is_some() {
        return true;
    }
    let Some(shape) = viewport_shape(node, width) else {
        return false;
    };
    let mut image_found = false;
    for child in node.children() {
        let image = child.shape();
        let Some(height) = positive(image.height) else {
            return false;
        };
        if image.trusted
            && image.zero_padding
            && image.gap.is_some_and(|gap| gap.is_finite() && gap >= 0.0)
            && image.origin
            && image.width.is_some_and(|value| equal(value, width))
            && (shape.hug_height || shape.height.is_some_and(|value| equal(value, height)))
            && child.is_media()
        {
            if std::mem::replace(&mut image_found, true) {
                return false;
            }
        } else if !overlay(child) {
            return false;
        }
    }
    image_found
}

/// Only the viewport itself or one unpadded, full-width wrapper owns this
/// exception. A media descendant cannot erase an ordinary section's rail.
pub(crate) fn full_width_media_section(section: &PenNode, width: f64) -> bool {
    if full_width_viewport(section, width) {
        return true;
    }
    let shape = section.shape();
    let [child] = MediaNode::children(section) else {
        return false;
    };
    shape.kind == Kind::Frame
        && shape.trusted
        && shape.vertical
        && shape.unpositioned
        && shape.zero_padding
        && spans_width(&shape, width)
        && full_width_viewport(child, width)
}

fn absent(value: Option<&Value>) -> bool {
    value.is_none_or(Value::is_null)
}

fn empty(value: Option<&Value>) -> bool {
    absent(value)
        || value.is_some_and(|value| {
            value.as_object().is_some_and(|map| map.is_empty())
                || value.as_array().is_some_and(Vec::is_empty)
        })
}

fn zero_padding(value: Option<&Value>) -> bool {
    absent(value)
        || value.is_some_and(|value| {
            value.as_f64() == Some(0.0)
                || value.as_array().is_some_and(|values| {
                    matches!(values.len(), 1 | 2 | 4)
                        && values.iter().all(|value| value.as_f64() == Some(0.0))
                })
        })
}

impl MediaNode for Value {
    fn is_media(&self) -> bool {
        if self["type"] == "image" {
            return MediaNode::children(self).is_empty();
        }
        // Only the small canonical tile needs typed decoding; never clone a
        // general subtree or infer image ownership from its name or palette.
        self["type"] == "frame"
            && MediaNode::children(self).len() <= 2
            && MediaNode::children(self)
                .iter()
                .all(|child| MediaNode::children(child).is_empty())
            && serde_json::from_value::<PenNode>(self.clone())
                .is_ok_and(|node| op_image_enrich::is_unchanged_image_fallback(&node))
    }

    fn shape(&self) -> Shape {
        let at_origin = |key| absent(self.get(key)) || self[key].as_f64() == Some(0.0);
        let trusted = self["visible"] != false
            && (absent(self.get("enabled")) || self["enabled"] == true)
            && (absent(self.get("opacity")) || self["opacity"].as_f64().is_some_and(|v| v > 0.0))
            && at_origin("rotation")
            && [
                "constraints",
                "position",
                "minWidth",
                "maxWidth",
                "minHeight",
                "maxHeight",
            ]
            .iter()
            .all(|key| absent(self.get(*key)))
            && empty(self.get("bindings"))
            && self["wrap"] != true
            && self["flipX"] != true
            && self["flipY"] != true
            && !self["role"]
                .as_str()
                .is_some_and(|role| role.starts_with("table"))
            && ["onDrag", "onSwipe"]
                .iter()
                .all(|key| empty(self["events"].get(*key)));
        Shape {
            kind: match self["type"].as_str() {
                Some("frame") => Kind::Frame,
                Some("image") => Kind::Image,
                Some("icon_font" | "ellipse" | "path" | "line" | "polygon" | "rectangle") => {
                    Kind::Decoration
                }
                _ => Kind::Other,
            },
            absolute: absent(self.get("layout")) || self["layout"] == "none",
            horizontal: self["layout"] == "horizontal",
            vertical: self["layout"] == "vertical",
            width: self["width"].as_f64(),
            height: self["height"].as_f64(),
            fill_width: self["width"] == "fill_container",
            hug_height: self["height"] == "fit_content",
            origin: at_origin("x") && at_origin("y"),
            unpositioned: absent(self.get("x")) && absent(self.get("y")),
            clipped: self["clipContent"] == true,
            zero_padding: zero_padding(self.get("padding")),
            gap: if absent(self.get("gap")) {
                Some(0.0)
            } else {
                self["gap"].as_f64()
            },
            image_paint: self["fill"]
                .as_array()
                .is_some_and(|fills| fills.iter().any(|fill| fill["type"] == "image")),
            trusted,
        }
    }

    fn children(&self) -> &[Self] {
        self["children"].as_array().map_or(&[], Vec::as_slice)
    }
}

impl MediaNode for PenNode {
    fn is_media(&self) -> bool {
        matches!(self, PenNode::Image(_))
            || (matches!(self, PenNode::Frame(_))
                && MediaNode::children(self).len() <= 2
                && MediaNode::children(self)
                    .iter()
                    .all(|child| MediaNode::children(child).is_empty())
                && op_image_enrich::is_unchanged_image_fallback(self))
    }

    fn shape(&self) -> Shape {
        let base = self.base();
        let (kind, container, bindings, limits) = match self {
            PenNode::Frame(node) => (
                Kind::Frame,
                Some(&node.container),
                node.bindings.as_ref(),
                node.container.limits,
            ),
            PenNode::Image(node) => (Kind::Image, None, node.bindings.as_ref(), node.limits),
            PenNode::IconFont(node) => {
                (Kind::Decoration, None, node.bindings.as_ref(), node.limits)
            }
            PenNode::Ellipse(node) => (Kind::Decoration, None, node.bindings.as_ref(), node.limits),
            PenNode::Path(node) => (Kind::Decoration, None, node.bindings.as_ref(), node.limits),
            PenNode::Line(node) => (
                Kind::Decoration,
                None,
                node.bindings.as_ref(),
                SizeLimits::default(),
            ),
            PenNode::Polygon(node) => (Kind::Decoration, None, node.bindings.as_ref(), node.limits),
            PenNode::Rectangle(node) => (
                Kind::Decoration,
                Some(&node.container),
                node.bindings.as_ref(),
                node.container.limits,
            ),
            _ => (Kind::Other, None, None, SizeLimits::default()),
        };
        let trusted = base.visible != Some(false)
            && matches!(base.enabled, None | Some(BoolOrExpression::Bool(true)))
            && match base.opacity {
                None => true,
                Some(NumberOrExpression::Number(v)) => v > 0.0,
                _ => false,
            }
            && base.rotation.is_none_or(|v| v == 0.0)
            && base.constraints.is_none()
            && limits == SizeLimits::default()
            && bindings.is_none_or(|values| values.is_empty())
            && base.flip_x != Some(true)
            && base.flip_y != Some(true)
            && !base
                .role
                .as_deref()
                .is_some_and(|role| role.starts_with("table"))
            && self.events().is_none_or(|events| {
                events.on_swipe.as_ref().is_none_or(Vec::is_empty)
                    && empty(events.extra.get("onDrag"))
            });
        Shape {
            kind,
            absolute: container
                .is_none_or(|props| matches!(props.layout, None | Some(LayoutMode::None))),
            horizontal: container
                .is_some_and(|props| matches!(props.layout, Some(LayoutMode::Horizontal))),
            vertical: container
                .is_some_and(|props| matches!(props.layout, Some(LayoutMode::Vertical))),
            width: self.width_px(),
            height: self.height_px(),
            fill_width: container.is_some_and(|props| {
                matches!(
                    props.width,
                    Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
                )
            }),
            hug_height: container.is_some_and(|props| {
                matches!(
                    props.height,
                    Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
                )
            }),
            origin: base.x.is_none_or(|v| v == 0.0) && base.y.is_none_or(|v| v == 0.0),
            unpositioned: base.x.is_none() && base.y.is_none(),
            clipped: container.is_some_and(|props| props.clip_content == Some(true)),
            zero_padding: container.is_none_or(|props| match &props.padding {
                None => true,
                Some(Padding::Uniform(v)) => *v == 0.0,
                Some(Padding::XY(v)) => v.iter().all(|v| *v == 0.0),
                Some(Padding::LtrB(v)) => v.iter().all(|v| *v == 0.0),
                _ => false,
            }),
            gap: container.map_or(Some(0.0), |props| match &props.gap {
                None => Some(0.0),
                Some(NumberOrExpression::Number(v)) => Some(*v),
                _ => None,
            }),
            image_paint: op_editor_core::fills::node_fills(self).is_some_and(|fills| {
                fills
                    .iter()
                    .any(|fill| matches!(fill, jian_ops_schema::style::PenFill::Image(_)))
            }),
            trusted,
        }
    }

    fn children(&self) -> &[Self] {
        PenNodeExt::children(self).map_or(&[], Vec::as_slice)
    }
}

#[cfg(test)]
#[path = "clipped_media_strip_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "clipped_media_strip_fallback_tests.rs"]
mod fallback_tests;
