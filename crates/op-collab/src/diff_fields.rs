use std::collections::BTreeSet;

use jian_ops_schema::node::PenNode;
use serde_json::{Map, Value};

use crate::{
    diff_index::{is_m1_basic_kind, node_kind, TreeIndex},
    serde_context::{inline_node, to_isolated_value},
    DiffError, FieldValue, NodeFieldChange, SupportedNodeField,
};

pub(crate) fn collect_property_changes(
    before: &TreeIndex<'_>,
    after: &TreeIndex<'_>,
) -> Result<Vec<NodeFieldChange>, DiffError> {
    let mut changes = Vec::new();
    for id in &after.preorder {
        let Some(before_entry) = before.entry(id) else {
            continue;
        };
        let after_entry = after
            .entry(id)
            .expect("after preorder contains indexed nodes");
        let before_kind = node_kind(before_entry.node);
        let after_kind = node_kind(after_entry.node);
        if before_kind != after_kind {
            return Err(DiffError::NodeKindChanged {
                node_id: id.clone(),
                before: before_kind.to_owned(),
                after: after_kind.to_owned(),
            });
        }
        let before_map = node_map(before_entry.node)?;
        let after_map = node_map(after_entry.node)?;
        let keys: BTreeSet<&str> = before_map
            .keys()
            .chain(after_map.keys())
            .map(String::as_str)
            .collect();
        for key in keys {
            if matches!(key, "type" | "id" | "children") {
                continue;
            }
            let before_value = before_map.get(key);
            let after_value = after_map.get(key);
            if before_value == after_value {
                continue;
            }
            let field = SupportedNodeField::from_wire_name(key).ok_or_else(|| {
                DiffError::UnsupportedNodeField {
                    node_id: id.clone(),
                    field: key.to_owned(),
                }
            })?;
            if !field_allowed(before_entry.node, field) {
                return Err(DiffError::UnsupportedNodeField {
                    node_id: id.clone(),
                    field: key.to_owned(),
                });
            }
            validate_field_value(id, field, after_value)?;
            changes.push(NodeFieldChange {
                page: after_entry.page.clone(),
                node_id: id.clone(),
                field,
                before: field_value(before_value),
                desired: field_value(after_value),
            });
        }
    }
    Ok(changes)
}

pub(crate) fn validate_new_node(node: &PenNode) -> Result<(), DiffError> {
    let id = crate::apply_tree::node_id_of(node);
    if !is_m1_basic_kind(node) {
        return Err(DiffError::UnsupportedNodeKind {
            node_id: id.to_owned(),
            kind: node_kind(node).to_owned(),
        });
    }
    let map = node_map(node)?;
    for forbidden in [
        "role",
        "explain",
        "constraints",
        "enabled",
        "visible",
        "locked",
        "flipX",
        "flipY",
        "mask",
        "maskType",
        "blendMode",
        "theme",
        "effects",
        "state",
        "bindings",
        "events",
        "lifecycle",
        "semantics",
        "gestures",
        "route",
        "screen",
        "breakpoint",
        "reusable",
        "slot",
        "iconId",
        "imageSearchQuery",
    ] {
        if map.contains_key(forbidden) {
            return Err(DiffError::UnsupportedNodeField {
                node_id: id.to_owned(),
                field: forbidden.to_owned(),
            });
        }
    }
    for field in [
        SupportedNodeField::Opacity,
        SupportedNodeField::Width,
        SupportedNodeField::Height,
        SupportedNodeField::Gap,
        SupportedNodeField::Padding,
        SupportedNodeField::Fill,
        SupportedNodeField::Stroke,
        SupportedNodeField::Content,
    ] {
        if let Some(value) = map.get(field.wire_name()) {
            validate_field_value(id, field, Some(value))?;
        }
    }
    Ok(())
}

pub(crate) fn replacement_with_changes(
    current: &PenNode,
    changes: &[NodeFieldChange],
) -> Result<PenNode, DiffError> {
    let mut value = node_value(current)?;
    let object = value
        .as_object_mut()
        .expect("typed PenNode always serializes as an object");
    for change in changes {
        set_field(object, change.field, &change.desired);
    }
    serde_json::from_value(value).map_err(DiffError::from)
}

pub(crate) fn apply_changes_to_node(
    current: &PenNode,
    changes: &[NodeFieldChange],
) -> Result<PenNode, DiffError> {
    replacement_with_changes(current, changes)
}

pub(crate) fn current_field_value(
    node: &PenNode,
    field: SupportedNodeField,
) -> Result<FieldValue, DiffError> {
    let map = node_map(node)?;
    Ok(field_value(map.get(field.wire_name())))
}

fn set_field(object: &mut Map<String, Value>, field: SupportedNodeField, value: &FieldValue) {
    match value {
        FieldValue::Missing => {
            object.remove(field.wire_name());
        }
        FieldValue::Value(value) => {
            object.insert(field.wire_name().to_owned(), value.clone());
        }
    }
}

fn field_allowed(node: &PenNode, field: SupportedNodeField) -> bool {
    use SupportedNodeField as Field;
    if matches!(
        field,
        Field::Name | Field::X | Field::Y | Field::Rotation | Field::Opacity
    ) {
        return true;
    }
    match node {
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_) => matches!(
            field,
            Field::Width
                | Field::Height
                | Field::MinWidth
                | Field::MaxWidth
                | Field::MinHeight
                | Field::MaxHeight
                | Field::Layout
                | Field::Gap
                | Field::Padding
                | Field::JustifyContent
                | Field::AlignItems
                | Field::ClipContent
                | Field::CornerRadius
                | Field::Fill
                | Field::Stroke
        ),
        PenNode::Ellipse(_) | PenNode::Polygon(_) | PenNode::Path(_) => matches!(
            field,
            Field::Width
                | Field::Height
                | Field::MinWidth
                | Field::MaxWidth
                | Field::MinHeight
                | Field::MaxHeight
                | Field::CornerRadius
                | Field::Fill
                | Field::Stroke
        ),
        PenNode::Text(_) => matches!(
            field,
            Field::Width
                | Field::Height
                | Field::MinWidth
                | Field::MaxWidth
                | Field::MinHeight
                | Field::MaxHeight
                | Field::Fill
                | Field::Content
        ),
        PenNode::Line(_) => matches!(field, Field::Stroke | Field::X2 | Field::Y2),
        _ => false,
    }
}

fn validate_field_value(
    node_id: &str,
    field: SupportedNodeField,
    value: Option<&Value>,
) -> Result<(), DiffError> {
    let Some(value) = value else {
        return Ok(());
    };
    let supported = match field {
        SupportedNodeField::Opacity => value.is_number(),
        SupportedNodeField::Width | SupportedNodeField::Height => {
            value.is_number() || matches!(value.as_str(), Some("fit_content" | "fill_container"))
        }
        SupportedNodeField::Gap | SupportedNodeField::Padding => !value.is_string(),
        SupportedNodeField::Fill => fills_are_solid(value),
        SupportedNodeField::Stroke => stroke_is_solid(value),
        SupportedNodeField::Content => value.is_string(),
        _ => true,
    };
    if supported {
        Ok(())
    } else {
        Err(DiffError::UnsupportedFieldValue {
            node_id: node_id.to_owned(),
            field: field.wire_name().to_owned(),
        })
    }
}

fn fills_are_solid(value: &Value) -> bool {
    value.as_array().is_some_and(|fills| {
        fills.iter().all(|fill| {
            fill.as_object().is_some_and(|fill| {
                fill.get("type").and_then(Value::as_str) == Some("solid")
                    && fill
                        .get("color")
                        .and_then(Value::as_str)
                        .is_some_and(|color| !color.starts_with('$'))
            })
        })
    })
}

fn stroke_is_solid(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|stroke| stroke.get("fill").is_none_or(fills_are_solid))
}

fn field_value(value: Option<&Value>) -> FieldValue {
    value.map_or(FieldValue::Missing, |value| {
        FieldValue::Value(value.clone())
    })
}

fn node_map(node: &PenNode) -> Result<Map<String, Value>, DiffError> {
    let value = node_value(node)?;
    Ok(value
        .as_object()
        .expect("typed PenNode always serializes as an object")
        .clone())
}

fn node_value(node: &PenNode) -> Result<Value, DiffError> {
    let mut isolated = to_isolated_value(node)?;
    inline_node(&mut isolated.value, &isolated.images);
    Ok(isolated.value)
}
