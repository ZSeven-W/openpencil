//! Conservative structural recognition of clipped, fixed-card horizontal lanes.

use jian_ops_schema::node::{LayoutMode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use serde_json::Value;

/// Recognize a clipped flow viewport containing exactly one horizontal Hug
/// lane of fixed-width container cards. Names are deliberately irrelevant;
/// ordinary rows, table roles and custom horizontal gestures do not qualify.
///
/// This recognizes authored structure, not measured overflow. Callers needing
/// proof of overflow must additionally compare the lane with its viewport.
pub fn is_clipped_horizontal_lane_viewport(value: &Value) -> bool {
    is_lane_viewport(value)
}

/// Typed equivalent that borrows fields without serializing the subtree or
/// copying embedded image data.
pub fn is_clipped_horizontal_lane_node(node: &PenNode) -> bool {
    is_lane_viewport(node)
}

struct Shape {
    flow: bool,
    horizontal: bool,
    hug_width: bool,
    numeric_width: Option<f64>,
    clipped: bool,
    gesture_owned: bool,
    table_role: bool,
}

trait ScrollNode: Sized {
    fn shape(&self) -> Option<Shape>;
    fn children(&self) -> &[Self];
}

fn is_lane_viewport<N: ScrollNode>(node: &N) -> bool {
    let Some(viewport) = node.shape() else {
        return false;
    };
    if !viewport.flow || !viewport.clipped || viewport.gesture_owned || viewport.table_role {
        return false;
    }
    let [lane] = node.children() else {
        return false;
    };
    let Some(lane_shape) = lane.shape() else {
        return false;
    };
    if !lane_shape.horizontal
        || !lane_shape.hug_width
        || lane_shape.gesture_owned
        || lane_shape.table_role
        || lane.children().len() < 2
    {
        return false;
    }
    lane.children().iter().all(|item| {
        item.shape().is_some_and(|shape| {
            !shape.table_role
                && shape
                    .numeric_width
                    .is_some_and(|width| width.is_finite() && width > 0.0)
        })
    })
}

fn is_table_role(role: Option<&str>) -> bool {
    matches!(
        role,
        Some("table" | "table-row" | "table-cell" | "table-header" | "table-header-cell")
    )
}

fn has_actions(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Array(actions)) => !actions.is_empty(),
        // Unknown handler shapes still reserve the gesture conservatively.
        Some(_) => true,
    }
}

impl ScrollNode for Value {
    fn shape(&self) -> Option<Shape> {
        if !matches!(self["type"].as_str(), Some("frame" | "group")) {
            return None;
        }
        Some(Shape {
            flow: matches!(self.get("layout"), None | Some(Value::Null))
                || matches!(self["layout"].as_str(), Some("vertical" | "horizontal")),
            horizontal: self["layout"] == "horizontal",
            hug_width: self["width"] == "fit_content",
            numeric_width: self["width"].as_f64(),
            clipped: self["clipContent"] == true,
            gesture_owned: ["onDrag", "onSwipe"]
                .iter()
                .any(|key| has_actions(self["events"].get(*key))),
            table_role: is_table_role(self["role"].as_str()),
        })
    }

    fn children(&self) -> &[Self] {
        self["children"].as_array().map_or(&[], Vec::as_slice)
    }
}

impl ScrollNode for PenNode {
    fn shape(&self) -> Option<Shape> {
        let (container, role, events) = match self {
            PenNode::Frame(node) => (
                &node.container,
                node.base.role.as_deref(),
                node.events.as_ref(),
            ),
            PenNode::Group(node) => (
                &node.container,
                node.base.role.as_deref(),
                node.events.as_ref(),
            ),
            _ => return None,
        };
        Some(Shape {
            flow: !matches!(container.layout, Some(LayoutMode::None)),
            horizontal: matches!(container.layout, Some(LayoutMode::Horizontal)),
            hug_width: matches!(
                container.width,
                Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
            ),
            numeric_width: match container.width {
                Some(SizingBehavior::Number(width)) => Some(width),
                _ => None,
            },
            clipped: container.clip_content == Some(true),
            gesture_owned: events.is_some_and(|events| {
                events
                    .on_swipe
                    .as_ref()
                    .is_some_and(|actions| !actions.is_empty())
                    || has_actions(events.extra.get("onDrag"))
            }),
            table_role: is_table_role(role),
        })
    }

    fn children(&self) -> &[Self] {
        crate::node_util::children(self)
    }
}

#[cfg(test)]
#[path = "horizontal_scroll_tests.rs"]
mod tests;
