//! Batch style-property replacement command application.

use crate::command::{StylePropValue, StylePropertyReplacement};
use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::walkers::find_node_mut;
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{ContainerProps, CornerRadius, Padding};
use jian_ops_schema::node::{FontWeight, PenNode};
use jian_ops_schema::style::{PenFill, PenStroke, StrokeThickness};

impl EditorState {
    pub(crate) fn cmd_replace_all_matching_properties(
        &mut self,
        page_id: &Option<String>,
        parent_ids: &[NodeId],
        replacements: &[StylePropertyReplacement],
    ) -> bool {
        let Some(children) = page_children_mut(self, page_id) else {
            return false;
        };
        replace_matching_properties_in_roots(children, parent_ids, replacements)
            .map(|count| count > 0)
            .unwrap_or(false)
    }
}

pub fn replace_matching_properties_in_roots(
    roots: &mut [PenNode],
    parent_ids: &[NodeId],
    replacements: &[StylePropertyReplacement],
) -> Result<usize, String> {
    validate_replacements(replacements)?;
    let mut count = 0;
    for parent_id in parent_ids {
        if !parent_id.is_real() {
            continue;
        }
        if let Some(node) = find_node_mut(roots, parent_id) {
            replace_descendants(node, replacements, &mut count);
        }
    }
    Ok(count)
}

fn page_children_mut<'a>(
    state: &'a mut EditorState,
    page_id: &Option<String>,
) -> Option<&'a mut Vec<PenNode>> {
    let Some(page_id) = page_id.as_deref() else {
        return Some(state.active_children_mut());
    };
    match state.doc.pages.as_mut() {
        Some(pages) if !pages.is_empty() => pages
            .iter_mut()
            .find(|page| page.id == page_id)
            .map(|page| &mut page.children),
        _ if page_id == "0" => Some(&mut state.doc.children),
        _ => None,
    }
}

fn validate_replacements(replacements: &[StylePropertyReplacement]) -> Result<(), String> {
    for replacement in replacements {
        match replacement.property.as_str() {
            "fillColor" | "textColor" | "strokeColor" | "fontFamily" => {
                require_string(&replacement.from, &replacement.property)?;
                require_string(&replacement.to, &replacement.property)?;
            }
            "strokeThickness" => {
                thickness_from_value(&replacement.from)?;
                thickness_from_value(&replacement.to)?;
            }
            "cornerRadius" => {
                corner_radius_from_value(&replacement.from)?;
                corner_radius_from_value(&replacement.to)?;
            }
            "padding" => {
                padding_from_value(&replacement.from)?;
                padding_from_value(&replacement.to)?;
            }
            "gap" => {
                number_or_expression_from_value(&replacement.from)?;
                number_or_expression_from_value(&replacement.to)?;
            }
            "fontSize" => {
                number_from_value(&replacement.from, "fontSize")?;
                number_from_value(&replacement.to, "fontSize")?;
            }
            "fontWeight" => {
                font_weight_from_value(&replacement.to)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn replace_descendants(
    node: &mut PenNode,
    replacements: &[StylePropertyReplacement],
    count: &mut usize,
) {
    for replacement in replacements {
        if apply_replacement(node, replacement) {
            *count += 1;
        }
    }
    if let Some(children) = existing_children_mut(node) {
        for child in children {
            replace_descendants(child, replacements, count);
        }
    }
}

fn apply_replacement(node: &mut PenNode, replacement: &StylePropertyReplacement) -> bool {
    match replacement.property.as_str() {
        "fillColor" => {
            if matches!(node, PenNode::Text(_)) {
                return false;
            }
            let Ok(from) = require_string(&replacement.from, "fillColor") else {
                return false;
            };
            let Ok(to) = require_string(&replacement.to, "fillColor") else {
                return false;
            };
            node_fill_mut(node)
                .map(|fills| replace_fill_color(fills, from, to))
                .unwrap_or(false)
        }
        "textColor" => {
            let PenNode::Text(text) = node else {
                return false;
            };
            let Ok(from) = require_string(&replacement.from, "textColor") else {
                return false;
            };
            let Ok(to) = require_string(&replacement.to, "textColor") else {
                return false;
            };
            text.fill
                .as_mut()
                .map(|fills| replace_fill_color(fills, from, to))
                .unwrap_or(false)
        }
        "strokeColor" => {
            let Ok(from) = require_string(&replacement.from, "strokeColor") else {
                return false;
            };
            let Ok(to) = require_string(&replacement.to, "strokeColor") else {
                return false;
            };
            node_stroke_mut(node)
                .and_then(|stroke| stroke.fill.as_mut())
                .map(|fills| replace_fill_color(fills, from, to))
                .unwrap_or(false)
        }
        "strokeThickness" => replace_stroke_thickness(node, replacement),
        "cornerRadius" => replace_corner_radius(node, replacement),
        "padding" => replace_padding(node, replacement),
        "gap" => replace_gap(node, replacement),
        "fontSize" => replace_text_number(node, replacement, "fontSize"),
        "fontFamily" => replace_font_family(node, replacement),
        "fontWeight" => replace_font_weight(node, replacement),
        _ => false,
    }
}

fn replace_fill_color(fills: &mut [PenFill], from: &str, to: &str) -> bool {
    let mut replaced = false;
    for fill in fills {
        if let PenFill::Solid(body) = fill {
            if body.color == from {
                body.color = to.to_string();
                replaced = true;
            }
        }
    }
    replaced
}

fn replace_stroke_thickness(node: &mut PenNode, replacement: &StylePropertyReplacement) -> bool {
    let Some(stroke) = node_stroke_mut(node) else {
        return false;
    };
    if !style_value_equal(
        &stroke_thickness_value(&stroke.thickness),
        &replacement.from,
    ) {
        return false;
    }
    let Ok(to) = thickness_from_value(&replacement.to) else {
        return false;
    };
    stroke.thickness = to;
    true
}

fn replace_corner_radius(node: &mut PenNode, replacement: &StylePropertyReplacement) -> bool {
    match node {
        PenNode::Frame(n) => replace_container_corner_radius(&mut n.container, replacement),
        PenNode::Group(n) => replace_container_corner_radius(&mut n.container, replacement),
        PenNode::Rectangle(n) => replace_container_corner_radius(&mut n.container, replacement),
        PenNode::Image(n) => {
            let Some(current) = n.corner_radius.as_ref().map(corner_radius_value) else {
                return false;
            };
            if !style_value_equal(&current, &replacement.from) {
                return false;
            }
            let Ok(to) = corner_radius_from_value(&replacement.to) else {
                return false;
            };
            n.corner_radius = Some(to);
            true
        }
        PenNode::Ellipse(n) => replace_scalar_radius(&mut n.corner_radius, replacement),
        PenNode::Polygon(n) => replace_scalar_radius(&mut n.corner_radius, replacement),
        _ => false,
    }
}

fn replace_container_corner_radius(
    container: &mut ContainerProps,
    replacement: &StylePropertyReplacement,
) -> bool {
    let Some(current) = container.corner_radius.as_ref().map(corner_radius_value) else {
        return false;
    };
    if !style_value_equal(&current, &replacement.from) {
        return false;
    }
    let Ok(to) = corner_radius_from_value(&replacement.to) else {
        return false;
    };
    container.corner_radius = Some(to);
    true
}

fn replace_scalar_radius(slot: &mut Option<f64>, replacement: &StylePropertyReplacement) -> bool {
    let Some(current) = slot.and_then(|value| finite_number_value(value).ok()) else {
        return false;
    };
    if !style_value_equal(&current, &replacement.from) {
        return false;
    }
    let Ok(to) = number_from_value(&replacement.to, "cornerRadius") else {
        return false;
    };
    *slot = Some(to);
    true
}

fn replace_padding(node: &mut PenNode, replacement: &StylePropertyReplacement) -> bool {
    let Some(container) = container_mut(node) else {
        return false;
    };
    let Some(current) = container.padding.as_ref().map(padding_value) else {
        return false;
    };
    if !style_value_equal(&current, &replacement.from) {
        return false;
    }
    let Ok(to) = padding_from_value(&replacement.to) else {
        return false;
    };
    container.padding = Some(to);
    true
}

fn replace_gap(node: &mut PenNode, replacement: &StylePropertyReplacement) -> bool {
    let Some(container) = container_mut(node) else {
        return false;
    };
    let Some(current) = container.gap.as_ref().map(number_or_expression_value) else {
        return false;
    };
    if !style_value_equal(&current, &replacement.from) {
        return false;
    }
    let Ok(to) = number_or_expression_from_value(&replacement.to) else {
        return false;
    };
    container.gap = Some(to);
    true
}

fn replace_text_number(
    node: &mut PenNode,
    replacement: &StylePropertyReplacement,
    property: &str,
) -> bool {
    let PenNode::Text(text) = node else {
        return false;
    };
    let Some(current) = text.font_size.and_then(|v| finite_number_value(v).ok()) else {
        return false;
    };
    if !style_value_equal(&current, &replacement.from) {
        return false;
    }
    let Ok(to) = number_from_value(&replacement.to, property) else {
        return false;
    };
    text.font_size = Some(to);
    true
}

fn replace_font_family(node: &mut PenNode, replacement: &StylePropertyReplacement) -> bool {
    let PenNode::Text(text) = node else {
        return false;
    };
    let Some(current) = text.font_family.as_ref() else {
        return false;
    };
    let Ok(from) = require_string(&replacement.from, "fontFamily") else {
        return false;
    };
    if current != from {
        return false;
    }
    let Ok(to) = require_string(&replacement.to, "fontFamily") else {
        return false;
    };
    text.font_family = Some(to.to_string());
    true
}

fn replace_font_weight(node: &mut PenNode, replacement: &StylePropertyReplacement) -> bool {
    let PenNode::Text(text) = node else {
        return false;
    };
    let Some(current) = text.font_weight.as_ref() else {
        return false;
    };
    if font_weight_key(current) != value_key(&replacement.from) {
        return false;
    }
    let Ok(to) = font_weight_from_value(&replacement.to) else {
        return false;
    };
    text.font_weight = Some(to);
    true
}

fn node_fill_mut(node: &mut PenNode) -> Option<&mut Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => n.container.fill.as_mut(),
        PenNode::Group(n) => n.container.fill.as_mut(),
        PenNode::Rectangle(n) => n.container.fill.as_mut(),
        PenNode::Ellipse(n) => n.fill.as_mut(),
        PenNode::Polygon(n) => n.fill.as_mut(),
        PenNode::Path(n) => n.fill.as_mut(),
        PenNode::Text(n) => n.fill.as_mut(),
        PenNode::TextInput(n) => n.fill.as_mut(),
        PenNode::IconFont(n) => n.fill.as_mut(),
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

fn node_stroke_mut(node: &mut PenNode) -> Option<&mut PenStroke> {
    match node {
        PenNode::Frame(n) => n.container.stroke.as_mut(),
        PenNode::Group(n) => n.container.stroke.as_mut(),
        PenNode::Rectangle(n) => n.container.stroke.as_mut(),
        PenNode::Ellipse(n) => n.stroke.as_mut(),
        PenNode::Line(n) => n.stroke.as_mut(),
        PenNode::Polygon(n) => n.stroke.as_mut(),
        PenNode::Path(n) => n.stroke.as_mut(),
        PenNode::TextInput(n) => n.stroke.as_mut(),
        PenNode::IconFont(n) => n.stroke.as_mut(),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

fn container_mut(node: &mut PenNode) -> Option<&mut ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container),
        PenNode::Group(n) => Some(&mut n.container),
        PenNode::Rectangle(n) => Some(&mut n.container),
        _ => None,
    }
}

fn existing_children_mut(node: &mut PenNode) -> Option<&mut Vec<PenNode>> {
    match node {
        PenNode::Frame(n) => n.children.as_mut(),
        PenNode::Group(n) => n.children.as_mut(),
        PenNode::Rectangle(n) => n.children.as_mut(),
        PenNode::Ref(n) => n.children.as_mut(),
        _ => None,
    }
}

fn stroke_thickness_value(thickness: &StrokeThickness) -> StylePropValue {
    match thickness {
        StrokeThickness::Uniform(v) => StylePropValue::Number(*v as f64),
        StrokeThickness::PerSide(v) => {
            StylePropValue::NumberArray(v.iter().map(|n| *n as f64).collect())
        }
        StrokeThickness::Sided(v) => StylePropValue::NumberArray(vec![
            v.top.unwrap_or(0.0) as f64,
            v.right.unwrap_or(0.0) as f64,
            v.bottom.unwrap_or(0.0) as f64,
            v.left.unwrap_or(0.0) as f64,
        ]),
    }
}

fn corner_radius_value(radius: &CornerRadius) -> StylePropValue {
    match radius {
        CornerRadius::Uniform(v) => StylePropValue::Number(*v),
        CornerRadius::PerCorner(v) => StylePropValue::NumberArray(v.to_vec()),
    }
}

fn padding_value(padding: &Padding) -> StylePropValue {
    match padding {
        Padding::Uniform(v) => StylePropValue::Number(*v),
        Padding::XY(v) => StylePropValue::NumberArray(v.to_vec()),
        Padding::LtrB(v) => StylePropValue::NumberArray(v.to_vec()),
        Padding::Expression(v) => StylePropValue::String(v.clone()),
    }
}

fn number_or_expression_value(value: &NumberOrExpression) -> StylePropValue {
    match value {
        NumberOrExpression::Number(v) => StylePropValue::Number(*v),
        NumberOrExpression::Expression(v) => StylePropValue::String(v.clone()),
    }
}

fn require_string<'a>(value: &'a StylePropValue, property: &str) -> Result<&'a str, String> {
    match value {
        StylePropValue::String(s) => Ok(s),
        _ => Err(format!("{property} replacement values must be strings")),
    }
}

fn number_from_value(value: &StylePropValue, property: &str) -> Result<f64, String> {
    match value {
        StylePropValue::Number(v) if v.is_finite() => Ok(*v),
        _ => Err(format!(
            "{property} replacement values must be finite numbers"
        )),
    }
}

fn finite_number_value(value: f64) -> Result<StylePropValue, String> {
    if value.is_finite() {
        Ok(StylePropValue::Number(value))
    } else {
        Err("number is not finite".into())
    }
}

fn number_or_expression_from_value(value: &StylePropValue) -> Result<NumberOrExpression, String> {
    match value {
        StylePropValue::Number(v) if v.is_finite() => Ok(NumberOrExpression::Number(*v)),
        StylePropValue::String(s) => Ok(NumberOrExpression::Expression(s.clone())),
        _ => Err("gap replacement values must be numbers or strings".into()),
    }
}

fn thickness_from_value(value: &StylePropValue) -> Result<StrokeThickness, String> {
    match value {
        StylePropValue::Number(v) if v.is_finite() => Ok(StrokeThickness::Uniform(*v as f32)),
        StylePropValue::NumberArray(v) if v.len() == 4 && v.iter().all(|n| n.is_finite()) => {
            Ok(StrokeThickness::PerSide([
                v[0] as f32,
                v[1] as f32,
                v[2] as f32,
                v[3] as f32,
            ]))
        }
        StylePropValue::NumberArray(v) if v.len() == 1 && v[0].is_finite() => {
            Ok(StrokeThickness::Uniform(v[0] as f32))
        }
        _ => Err("strokeThickness values must be a number or four-number array".into()),
    }
}

fn corner_radius_from_value(value: &StylePropValue) -> Result<CornerRadius, String> {
    match value {
        StylePropValue::Number(v) if v.is_finite() => Ok(CornerRadius::Uniform(*v)),
        StylePropValue::NumberArray(v) if v.len() == 4 && v.iter().all(|n| n.is_finite()) => {
            Ok(CornerRadius::PerCorner([v[0], v[1], v[2], v[3]]))
        }
        _ => Err("cornerRadius values must be a number or four-number array".into()),
    }
}

fn padding_from_value(value: &StylePropValue) -> Result<Padding, String> {
    match value {
        StylePropValue::Number(v) if v.is_finite() => Ok(Padding::Uniform(*v)),
        StylePropValue::NumberArray(v) if v.len() == 2 && v.iter().all(|n| n.is_finite()) => {
            Ok(Padding::XY([v[0], v[1]]))
        }
        StylePropValue::NumberArray(v) if v.len() == 4 && v.iter().all(|n| n.is_finite()) => {
            Ok(Padding::LtrB([v[0], v[1], v[2], v[3]]))
        }
        StylePropValue::String(s) => Ok(Padding::Expression(s.clone())),
        _ => Err("padding values must be a number, two/four-number array, or string".into()),
    }
}

fn font_weight_from_value(value: &StylePropValue) -> Result<FontWeight, String> {
    match value {
        StylePropValue::Number(v) if v.is_finite() && *v >= 0.0 => {
            Ok(FontWeight::Number(*v as u32))
        }
        StylePropValue::String(s) => Ok(FontWeight::Keyword(s.clone())),
        _ => Err("fontWeight replacement target must be a number or string".into()),
    }
}

fn style_value_equal(a: &StylePropValue, b: &StylePropValue) -> bool {
    match (a, b) {
        (StylePropValue::String(a), StylePropValue::String(b)) => a == b,
        (StylePropValue::Number(a), StylePropValue::Number(b)) => a == b,
        (StylePropValue::NumberArray(a), StylePropValue::NumberArray(b)) => a == b,
        _ => false,
    }
}

fn font_weight_key(weight: &FontWeight) -> String {
    match weight {
        FontWeight::Number(v) => v.to_string(),
        FontWeight::Keyword(v) => v.clone(),
    }
}

fn value_key(value: &StylePropValue) -> String {
    match value {
        StylePropValue::String(v) => v.clone(),
        StylePropValue::Number(v) => v.to_string(),
        StylePropValue::NumberArray(v) => format!("{v:?}"),
    }
}
