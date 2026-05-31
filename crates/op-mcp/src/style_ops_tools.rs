//! TS-compatible style operation MCP tools.

use std::collections::BTreeMap;

use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{ContainerProps, CornerRadius, Padding};
use jian_ops_schema::node::{FontWeight, PenNode};
use jian_ops_schema::style::{PenFill, PenStroke, StrokeThickness};
use op_editor_core::command_style_replace::replace_matching_properties_in_roots;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::walkers::find_node;
use op_editor_core::{
    EditorCommand, EditorState, NodeId, StylePropValue, StylePropertyReplacement,
};
use serde_json::Value;

use super::read_nodes::{page_nodes_snapshots, PageNodes};
use super::{McpTool, ToolErrorCode, ToolOutcome};

type ToolCallError = (ToolErrorCode, String);

pub struct SearchAllUniqueProperties {
    pages: Vec<PageNodes>,
    active_page_id: String,
}

pub struct ReplaceAllMatchingProperties {
    pages: Vec<PageNodes>,
    active_page_id: String,
}

impl McpTool for SearchAllUniqueProperties {
    fn name(&self) -> &str {
        "search_all_unique_properties"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let page = match self.page_nodes(args) {
            Ok(page) => page,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let parents = match parse_string_list_arg(args, &["parents"]) {
            Ok(ids) => ids,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let properties = match parse_properties(args) {
            Ok(properties) => properties,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };

        let mut nodes = Vec::new();
        for parent_id in parents {
            let Some(node_id) = NodeId::new_opt(parent_id) else {
                continue;
            };
            if let Some(node) = find_node(&page.roots, &node_id) {
                collect_descendants(node, &mut nodes);
            }
        }

        let mut result = serde_json::Map::new();
        for property in properties {
            let mut seen = Vec::<String>::new();
            let mut values = Vec::<Value>::new();
            for node in &nodes {
                let Some(value) = property_value(node, property) else {
                    continue;
                };
                let key = serde_json::to_string(&value).unwrap_or_else(|_| value.to_string());
                if !seen.iter().any(|existing| existing == &key) {
                    seen.push(key);
                    values.push(value);
                }
            }
            result.insert(property.as_str().to_string(), Value::Array(values));
        }

        let properties_json = match serde_json::to_string_pretty(&Value::Object(result)) {
            Ok(json) => json,
            Err(e) => {
                return ToolOutcome::Err(
                    ToolErrorCode::Internal,
                    format!("serialize style properties failed: {e}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("properties".into(), properties_json);
        ToolOutcome::Ok(out)
    }
}

impl McpTool for ReplaceAllMatchingProperties {
    fn name(&self) -> &str {
        "replace_all_matching_properties"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let page = match self.page_nodes(args) {
            Ok(page) => page,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let parent_ids = match parse_parent_node_ids(args) {
            Ok(ids) => ids,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };
        let replacements = match parse_replacements(args) {
            Ok(replacements) => replacements,
            Err((code, msg)) => return ToolOutcome::Err(code, msg),
        };

        let mut roots = page.roots.clone();
        let replaced_count =
            match replace_matching_properties_in_roots(&mut roots, &parent_ids, &replacements) {
                Ok(count) => count,
                Err(msg) => return ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg),
            };

        let mut out = BTreeMap::new();
        out.insert("replacedCount".into(), replaced_count.to_string());
        if replaced_count == 0 {
            return ToolOutcome::Ok(out);
        }
        let page_id = arg_alias(args, &["pageId", "page_id", "page"]).map(str::to_string);
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::ReplaceAllMatchingProperties {
                page_id,
                parent_ids,
                replacements,
            },
        )
    }
}

pub fn search_all_unique_properties_snapshot(state: &EditorState) -> SearchAllUniqueProperties {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    SearchAllUniqueProperties {
        pages,
        active_page_id,
    }
}

pub fn replace_all_matching_properties_snapshot(
    state: &EditorState,
) -> ReplaceAllMatchingProperties {
    let (pages, active_page_id) = page_nodes_snapshots(state);
    ReplaceAllMatchingProperties {
        pages,
        active_page_id,
    }
}

impl SearchAllUniqueProperties {
    fn page_nodes(&self, args: &BTreeMap<String, String>) -> Result<&PageNodes, ToolCallError> {
        let target =
            arg_alias(args, &["pageId", "page_id", "page"]).unwrap_or(&self.active_page_id);
        self.pages
            .iter()
            .find(|page| page.id == target)
            .ok_or_else(|| {
                (
                    ToolErrorCode::ToolFailed,
                    format!("page not found: {target}"),
                )
            })
    }
}

impl ReplaceAllMatchingProperties {
    fn page_nodes(&self, args: &BTreeMap<String, String>) -> Result<&PageNodes, ToolCallError> {
        let target =
            arg_alias(args, &["pageId", "page_id", "page"]).unwrap_or(&self.active_page_id);
        self.pages
            .iter()
            .find(|page| page.id == target)
            .ok_or_else(|| {
                (
                    ToolErrorCode::ToolFailed,
                    format!("page not found: {target}"),
                )
            })
    }
}

#[derive(Clone, Copy)]
enum StyleProperty {
    FillColor,
    TextColor,
    StrokeColor,
    StrokeThickness,
    CornerRadius,
    Padding,
    Gap,
    FontSize,
    FontFamily,
    FontWeight,
}

impl StyleProperty {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "fillColor" => Some(Self::FillColor),
            "textColor" => Some(Self::TextColor),
            "strokeColor" => Some(Self::StrokeColor),
            "strokeThickness" => Some(Self::StrokeThickness),
            "cornerRadius" => Some(Self::CornerRadius),
            "padding" => Some(Self::Padding),
            "gap" => Some(Self::Gap),
            "fontSize" => Some(Self::FontSize),
            "fontFamily" => Some(Self::FontFamily),
            "fontWeight" => Some(Self::FontWeight),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FillColor => "fillColor",
            Self::TextColor => "textColor",
            Self::StrokeColor => "strokeColor",
            Self::StrokeThickness => "strokeThickness",
            Self::CornerRadius => "cornerRadius",
            Self::Padding => "padding",
            Self::Gap => "gap",
            Self::FontSize => "fontSize",
            Self::FontFamily => "fontFamily",
            Self::FontWeight => "fontWeight",
        }
    }
}

fn parse_properties(args: &BTreeMap<String, String>) -> Result<Vec<StyleProperty>, ToolCallError> {
    parse_string_list_arg(args, &["properties"])?
        .into_iter()
        .map(|raw| {
            StyleProperty::parse(&raw).ok_or_else(|| {
                (
                    ToolErrorCode::InvalidArgument,
                    format!("unknown style property: {raw}"),
                )
            })
        })
        .collect()
}

fn parse_parent_node_ids(args: &BTreeMap<String, String>) -> Result<Vec<NodeId>, ToolCallError> {
    Ok(parse_string_list_arg(args, &["parents"])?
        .into_iter()
        .filter_map(NodeId::new_opt)
        .collect())
}

fn parse_replacements(
    args: &BTreeMap<String, String>,
) -> Result<Vec<StylePropertyReplacement>, ToolCallError> {
    let Some(raw) = arg_alias(args, &["properties"]) else {
        return Err((
            ToolErrorCode::MissingArgument,
            "properties is required".into(),
        ));
    };
    let value = serde_json::from_str::<Value>(raw).map_err(|e| {
        (
            ToolErrorCode::InvalidArgument,
            format!("properties must be a JSON object: {e}"),
        )
    })?;
    let Value::Object(map) = value else {
        return Err((
            ToolErrorCode::InvalidArgument,
            "properties must be a JSON object".into(),
        ));
    };
    let mut replacements = Vec::new();
    for (property, rules_value) in map {
        let Value::Array(rules) = rules_value else {
            continue;
        };
        for rule in rules {
            let Value::Object(mut rule) = rule else {
                return Err((
                    ToolErrorCode::InvalidArgument,
                    format!("{property} rules must be {{from,to}} objects"),
                ));
            };
            let Some(from) = rule.remove("from") else {
                return Err((
                    ToolErrorCode::InvalidArgument,
                    format!("{property} rule is missing from"),
                ));
            };
            let Some(to) = rule.remove("to") else {
                return Err((
                    ToolErrorCode::InvalidArgument,
                    format!("{property} rule is missing to"),
                ));
            };
            replacements.push(StylePropertyReplacement {
                property: property.clone(),
                from: style_prop_value(from)?,
                to: style_prop_value(to)?,
            });
        }
    }
    Ok(replacements)
}

fn style_prop_value(value: Value) -> Result<StylePropValue, ToolCallError> {
    match value {
        Value::String(s) => Ok(StylePropValue::String(s)),
        Value::Number(n) => n.as_f64().map(StylePropValue::Number).ok_or_else(|| {
            (
                ToolErrorCode::InvalidArgument,
                format!("number value is not representable as f64: {n}"),
            )
        }),
        Value::Array(items) => {
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                let Value::Number(n) = item else {
                    return Err((
                        ToolErrorCode::InvalidArgument,
                        "array replacement values must contain only numbers".into(),
                    ));
                };
                let Some(n) = n.as_f64() else {
                    return Err((
                        ToolErrorCode::InvalidArgument,
                        "array number is not representable as f64".into(),
                    ));
                };
                values.push(n);
            }
            Ok(StylePropValue::NumberArray(values))
        }
        other => Err((
            ToolErrorCode::InvalidArgument,
            format!("unsupported replacement value: {other}"),
        )),
    }
}

fn parse_string_list_arg(
    args: &BTreeMap<String, String>,
    keys: &[&str],
) -> Result<Vec<String>, ToolCallError> {
    let Some(raw) = arg_alias(args, keys) else {
        return Err((
            ToolErrorCode::MissingArgument,
            format!("{} is required", keys[0]),
        ));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<String>>(trimmed).map_err(|e| {
            (
                ToolErrorCode::InvalidArgument,
                format!("{} must be a JSON string array: {e}", keys[0]),
            )
        });
    }
    Ok(trimmed
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect())
}

fn collect_descendants<'a>(node: &'a PenNode, out: &mut Vec<&'a PenNode>) {
    out.push(node);
    if let Some(children) = node.children() {
        for child in children {
            collect_descendants(child, out);
        }
    }
}

fn property_value(node: &PenNode, property: StyleProperty) -> Option<Value> {
    match property {
        StyleProperty::FillColor => {
            if matches!(node, PenNode::Text(_)) {
                return None;
            }
            node_fill(node)
                .and_then(|fills| extract_fill_color(fills))
                .map(|color| Value::String(color.to_string()))
        }
        StyleProperty::TextColor => match node {
            PenNode::Text(t) => t
                .fill
                .as_ref()
                .and_then(|fills| extract_fill_color(fills))
                .map(|color| Value::String(color.to_string())),
            _ => None,
        },
        StyleProperty::StrokeColor => node_stroke(node)
            .and_then(|stroke| stroke.fill.as_ref())
            .and_then(|fills| extract_fill_color(fills))
            .map(|color| Value::String(color.to_string())),
        StyleProperty::StrokeThickness => {
            node_stroke(node).map(|stroke| stroke_thickness_value(&stroke.thickness))
        }
        StyleProperty::CornerRadius => corner_radius(node),
        StyleProperty::Padding => container(node)
            .and_then(|c| c.padding.as_ref())
            .map(padding_value),
        StyleProperty::Gap => container(node)
            .and_then(|c| c.gap.as_ref())
            .and_then(number_or_expression_value),
        StyleProperty::FontSize => match node {
            PenNode::Text(t) => t.font_size.and_then(number_value),
            _ => None,
        },
        StyleProperty::FontFamily => match node {
            PenNode::Text(t) => t.font_family.as_ref().map(|v| Value::String(v.clone())),
            _ => None,
        },
        StyleProperty::FontWeight => match node {
            PenNode::Text(t) => t.font_weight.as_ref().map(font_weight_value),
            _ => None,
        },
    }
}

fn node_fill(node: &PenNode) -> Option<&Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => n.container.fill.as_ref(),
        PenNode::Group(n) => n.container.fill.as_ref(),
        PenNode::Rectangle(n) => n.container.fill.as_ref(),
        PenNode::Ellipse(n) => n.fill.as_ref(),
        PenNode::Polygon(n) => n.fill.as_ref(),
        PenNode::Path(n) => n.fill.as_ref(),
        PenNode::Text(n) => n.fill.as_ref(),
        PenNode::TextInput(n) => n.fill.as_ref(),
        PenNode::IconFont(n) => n.fill.as_ref(),
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

fn node_stroke(node: &PenNode) -> Option<&PenStroke> {
    match node {
        PenNode::Frame(n) => n.container.stroke.as_ref(),
        PenNode::Group(n) => n.container.stroke.as_ref(),
        PenNode::Rectangle(n) => n.container.stroke.as_ref(),
        PenNode::Ellipse(n) => n.stroke.as_ref(),
        PenNode::Line(n) => n.stroke.as_ref(),
        PenNode::Polygon(n) => n.stroke.as_ref(),
        PenNode::Path(n) => n.stroke.as_ref(),
        PenNode::TextInput(n) => n.stroke.as_ref(),
        PenNode::IconFont(n) => n.stroke.as_ref(),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

fn container(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

fn extract_fill_color(fill: &[PenFill]) -> Option<&str> {
    fill.iter().find_map(|f| match f {
        PenFill::Solid(body) => Some(body.color.as_str()),
        _ => None,
    })
}

fn stroke_thickness_value(thickness: &StrokeThickness) -> Value {
    match thickness {
        StrokeThickness::Uniform(v) => number_value(*v as f64).unwrap_or(Value::Null),
        StrokeThickness::PerSide(v) => {
            Value::Array(v.iter().filter_map(|n| number_value(*n as f64)).collect())
        }
        StrokeThickness::Sided(v) => {
            let mut map = serde_json::Map::new();
            if let Some(top) = v.top.and_then(|n| number_value(n as f64)) {
                map.insert("top".into(), top);
            }
            if let Some(right) = v.right.and_then(|n| number_value(n as f64)) {
                map.insert("right".into(), right);
            }
            if let Some(bottom) = v.bottom.and_then(|n| number_value(n as f64)) {
                map.insert("bottom".into(), bottom);
            }
            if let Some(left) = v.left.and_then(|n| number_value(n as f64)) {
                map.insert("left".into(), left);
            }
            Value::Object(map)
        }
    }
}

fn corner_radius(node: &PenNode) -> Option<Value> {
    match node {
        PenNode::Frame(n) => n.container.corner_radius.as_ref().map(corner_radius_value),
        PenNode::Group(n) => n.container.corner_radius.as_ref().map(corner_radius_value),
        PenNode::Rectangle(n) => n.container.corner_radius.as_ref().map(corner_radius_value),
        PenNode::Ellipse(n) => n.corner_radius.and_then(number_value),
        PenNode::Polygon(n) => n.corner_radius.and_then(number_value),
        PenNode::Image(n) => n.corner_radius.as_ref().map(corner_radius_value),
        _ => None,
    }
}

fn corner_radius_value(radius: &CornerRadius) -> Value {
    match radius {
        CornerRadius::Uniform(v) => number_value(*v).unwrap_or(Value::Null),
        CornerRadius::PerCorner(v) => {
            Value::Array(v.iter().filter_map(|n| number_value(*n)).collect())
        }
    }
}

fn padding_value(padding: &Padding) -> Value {
    match padding {
        Padding::Uniform(v) => number_value(*v).unwrap_or(Value::Null),
        Padding::XY(v) => Value::Array(v.iter().filter_map(|n| number_value(*n)).collect()),
        Padding::LtrB(v) => Value::Array(v.iter().filter_map(|n| number_value(*n)).collect()),
        Padding::Expression(v) => Value::String(v.clone()),
    }
}

fn number_or_expression_value(value: &NumberOrExpression) -> Option<Value> {
    match value {
        NumberOrExpression::Number(v) => number_value(*v),
        NumberOrExpression::Expression(v) => Some(Value::String(v.clone())),
    }
}

fn font_weight_value(weight: &FontWeight) -> Value {
    match weight {
        FontWeight::Number(v) => Value::from(*v),
        FontWeight::Keyword(v) => Value::String(v.clone()),
    }
}

fn number_value(value: f64) -> Option<Value> {
    serde_json::Number::from_f64(value).map(Value::Number)
}

fn arg_alias<'a>(args: &'a BTreeMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| args.get(*key).map(String::as_str))
}
