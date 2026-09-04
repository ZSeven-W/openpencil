//! Intelligent and deterministic `design.md` extraction contracts.
//!
//! The browser-facing request has two deliberately separate paths:
//!
//! * the paired desktop host may turn the bounded evidence into a richer guide
//!   with the user's selected model;
//! * this module can always render the same evidence locally, without a model
//!   or a browser engine, so an old/offline host never loses the capture.
//!
//! Both paths are trust boundaries. A captured page controls every string in
//! the evidence and a loopback process controls every reply, so sizes, shapes,
//! and display text are checked again here before anything is downloaded.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::js_text::js_trim;

/// Maximum UTF-8 request body accepted by both the extension and host.
pub const MAX_EVIDENCE_BYTES: usize = 256 * 1024;

const MAX_COLORS: usize = 64;
const MAX_TYPOGRAPHY: usize = 64;
const MAX_SPACING: usize = 64;
const MAX_RADII: usize = 64;
const MAX_SHADOWS: usize = 32;
const MAX_COMPONENTS: usize = 64;
const MAX_COMPONENT_SAMPLES: usize = 4;
const MAX_GRADIENTS: usize = 32;
const MAX_MEDIA_QUERIES: usize = 32;
const MAX_CSS_VARIABLES: usize = 64;
const MAX_COUNT: u64 = 10_000_000;
const MAX_CSS_LENGTH: f64 = 100_000.0;

/// Convert extractor evidence into a deterministic corpus-compatible guide.
pub fn evidence_to_design_md(json: &str) -> Result<String, EvidenceError> {
    if json.len() > MAX_EVIDENCE_BYTES {
        return Err(EvidenceError::TooLarge);
    }
    let value: Value = serde_json::from_str(json).map_err(|_| EvidenceError::Malformed("json"))?;
    let evidence = Evidence::parse(&value)?;
    Ok(crate::design_md_render::render(&evidence))
}

/// Why deterministic extraction refused an evidence document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    TooLarge,
    Malformed(&'static str),
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => f.write_str("design evidence exceeds 256 KiB"),
            Self::Malformed(field) => write!(f, "design evidence is malformed: {field}"),
        }
    }
}

impl std::error::Error for EvidenceError {}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Evidence {
    pub title: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub viewport_dpr: f64,
    pub color_scheme: Option<String>,
    pub page_background: Option<String>,
    pub colors: Vec<ColorEvidence>,
    pub typography: Vec<TypographyEvidence>,
    pub spacing: Vec<SpacingEvidence>,
    pub radii: Vec<RadiusEvidence>,
    pub shadows: Vec<CountedText>,
    pub components: Vec<ComponentEvidence>,
    pub gradients: Vec<CountedText>,
    pub media_queries: Vec<String>,
    pub css_variables: Vec<CssVariableEvidence>,
    pub element_count: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ColorEvidence {
    pub value: String,
    pub usage: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct TypographyEvidence {
    pub role: String,
    pub family: String,
    pub size: f64,
    pub weight: u16,
    pub line_height: Option<f64>,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SpacingEvidence {
    pub property: String,
    pub value: f64,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RadiusEvidence {
    pub value: u32,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CountedText {
    pub value: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ComponentEvidence {
    pub kind: String,
    pub count: u64,
    pub samples: Vec<ComponentSample>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ComponentSample {
    pub background: Option<String>,
    pub color: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u16>,
    pub line_height: Option<f64>,
    pub padding: Option<String>,
    pub gap: Option<f64>,
    pub radius: Option<u32>,
    pub border: Option<String>,
    pub shadow: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CssVariableEvidence {
    pub name: String,
    pub value: String,
    pub kind: String,
}

impl Evidence {
    fn parse(value: &Value) -> Result<Self, EvidenceError> {
        let root = as_object(value, "root")?;
        if integer(root.get("version"), "version")? != 1 {
            return Err(EvidenceError::Malformed("version"));
        }
        let title = text(root.get("title"), 120, "title")?;
        let viewport = as_object(required(root, "viewport")?, "viewport")?;
        let viewport_width = bounded_u32(viewport.get("width"), 1, 100_000, "viewport.width")?;
        let viewport_height = bounded_u32(viewport.get("height"), 1, 100_000, "viewport.height")?;
        let viewport_dpr = match viewport.get("dpr") {
            Some(value) => bounded_number(Some(value), 0.1, 16.0, "viewport.dpr")?,
            None => 1.0,
        };
        let color_scheme = optional_text(root.get("colorScheme"), 16, "colorScheme")?;
        let page_background = match root.get("pageBackground") {
            Some(Value::Null) => None,
            Some(value) => Some(hex_color(value, "pageBackground")?),
            None => return Err(EvidenceError::Malformed("pageBackground")),
        };

        Ok(Self {
            title,
            viewport_width,
            viewport_height,
            viewport_dpr,
            color_scheme,
            page_background,
            colors: parse_colors(required(root, "colors")?)?,
            typography: parse_typography(required(root, "typography")?)?,
            spacing: parse_spacing(required(root, "spacing")?)?,
            radii: parse_radii(required(root, "radii")?)?,
            shadows: parse_counted_text(required(root, "shadows")?, MAX_SHADOWS, 160, "shadows")?,
            components: parse_components(required(root, "components")?)?,
            gradients: parse_counted_text(
                required(root, "gradients")?,
                MAX_GRADIENTS,
                200,
                "gradients",
            )?,
            media_queries: parse_text_array(
                required(root, "mediaQueries")?,
                MAX_MEDIA_QUERIES,
                160,
                "mediaQueries",
            )?,
            css_variables: parse_css_variables(required(root, "cssVariables")?)?,
            element_count: match root.get("elementCount") {
                Some(value) => bounded_integer(Some(value), 0, MAX_COUNT, "elementCount")?,
                None => 0,
            },
            truncated: match root.get("truncated") {
                Some(value) => value
                    .as_bool()
                    .ok_or(EvidenceError::Malformed("truncated"))?,
                None => false,
            },
        })
    }
}

fn parse_colors(value: &Value) -> Result<Vec<ColorEvidence>, EvidenceError> {
    let values = bounded_array(value, MAX_COLORS, "colors")?;
    values
        .iter()
        .map(|value| {
            let object = as_object(value, "colors[]")?;
            let usage = color_usage(object)?;
            if !matches!(
                usage.as_str(),
                "text" | "background" | "border" | "shadow" | "gradient"
            ) {
                return Err(EvidenceError::Malformed("colors[].usage"));
            }
            Ok(ColorEvidence {
                value: hex_color(required(object, "value")?, "colors[].value")?,
                usage,
                count: count(object.get("count"), "colors[].count")?,
            })
        })
        .collect()
}

fn color_usage(object: &Map<String, Value>) -> Result<String, EvidenceError> {
    if object.contains_key("usage") {
        return text(object.get("usage"), 16, "colors[].usage");
    }
    let uses = bounded_array(required(object, "uses")?, 8, "colors[].uses")?;
    let mut present = BTreeSet::new();
    for value in uses {
        present.insert(text(Some(value), 16, "colors[].uses[]")?);
    }
    ["text", "background", "border", "gradient", "shadow"]
        .into_iter()
        .find(|usage| present.contains(*usage))
        .map(str::to_owned)
        .ok_or(EvidenceError::Malformed("colors[].uses"))
}

fn parse_typography(value: &Value) -> Result<Vec<TypographyEvidence>, EvidenceError> {
    let values = bounded_array(value, MAX_TYPOGRAPHY, "typography")?;
    values
        .iter()
        .map(|value| {
            let object = as_object(value, "typography[]")?;
            let role = text(object.get("role"), 16, "typography[].role")?;
            if !matches!(
                role.as_str(),
                "display" | "heading" | "body" | "label" | "control" | "code"
            ) {
                return Err(EvidenceError::Malformed("typography[].role"));
            }
            Ok(TypographyEvidence {
                role,
                family: text(object.get("family"), 96, "typography[].family")?,
                size: bounded_number(object.get("size"), 1.0, 1_000.0, "typography[].size")?,
                weight: bounded_u32(object.get("weight"), 1, 1_000, "typography[].weight")? as u16,
                line_height: optional_number(
                    object.get("lineHeight"),
                    1.0,
                    2_048.0,
                    "typography[].lineHeight",
                )?,
                count: count(object.get("count"), "typography[].count")?,
            })
        })
        .collect()
}

fn parse_spacing(value: &Value) -> Result<Vec<SpacingEvidence>, EvidenceError> {
    let values = bounded_array(value, MAX_SPACING, "spacing")?;
    values
        .iter()
        .map(|value| {
            let object = as_object(value, "spacing[]")?;
            let property = spacing_property(object)?;
            if !matches!(property.as_str(), "margin" | "padding" | "gap") {
                return Err(EvidenceError::Malformed("spacing[].property"));
            }
            Ok(SpacingEvidence {
                property,
                value: bounded_number(
                    object.get("value"),
                    -MAX_CSS_LENGTH,
                    MAX_CSS_LENGTH,
                    "spacing[].value",
                )?,
                count: count(object.get("count"), "spacing[].count")?,
            })
        })
        .collect()
}

fn spacing_property(object: &Map<String, Value>) -> Result<String, EvidenceError> {
    if object.contains_key("property") {
        return text(object.get("property"), 16, "spacing[].property");
    }
    let uses = bounded_array(required(object, "uses")?, 8, "spacing[].uses")?;
    let mut present = BTreeSet::new();
    for value in uses {
        let value = text(Some(value), 32, "spacing[].uses[]")?;
        for property in ["gap", "padding", "margin"] {
            if value == property || value.starts_with(&format!("{property}-")) {
                present.insert(property);
            }
        }
    }
    ["gap", "padding", "margin"]
        .into_iter()
        .find(|property| present.contains(property))
        .map(str::to_owned)
        .ok_or(EvidenceError::Malformed("spacing[].uses"))
}

fn parse_radii(value: &Value) -> Result<Vec<RadiusEvidence>, EvidenceError> {
    let values = bounded_array(value, MAX_RADII, "radii")?;
    values
        .iter()
        .map(|value| {
            let object = as_object(value, "radii[]")?;
            Ok(RadiusEvidence {
                value: bounded_u32(object.get("value"), 0, 100_000, "radii[].value")?,
                count: count(object.get("count"), "radii[].count")?,
            })
        })
        .collect()
}

fn parse_counted_text(
    value: &Value,
    max_items: usize,
    max_chars: usize,
    field: &'static str,
) -> Result<Vec<CountedText>, EvidenceError> {
    bounded_array(value, max_items, field)?
        .iter()
        .map(|value| {
            let object = as_object(value, field)?;
            Ok(CountedText {
                value: text(object.get("value"), max_chars, field)?,
                count: count(object.get("count"), field)?,
            })
        })
        .collect()
}

fn parse_components(value: &Value) -> Result<Vec<ComponentEvidence>, EvidenceError> {
    bounded_array(value, MAX_COMPONENTS, "components")?
        .iter()
        .map(|value| {
            let object = as_object(value, "components[]")?;
            let samples = match object.get("samples") {
                Some(value) => bounded_array(value, MAX_COMPONENT_SAMPLES, "components[].samples")?
                    .iter()
                    .map(parse_component_sample)
                    .collect::<Result<Vec<_>, _>>()?,
                None => Vec::new(),
            };
            Ok(ComponentEvidence {
                kind: text(object.get("kind"), 64, "components[].kind")?,
                count: count(object.get("count"), "components[].count")?,
                samples,
            })
        })
        .collect()
}

fn parse_component_sample(value: &Value) -> Result<ComponentSample, EvidenceError> {
    let object = as_object(value, "components[].samples[]")?;
    Ok(ComponentSample {
        background: optional_hex(object.get("background"), "sample.background")?,
        color: optional_hex(object.get("color"), "sample.color")?,
        font_family: optional_text(object.get("fontFamily"), 96, "sample.fontFamily")?,
        font_size: optional_number(object.get("fontSize"), 1.0, 1_000.0, "sample.fontSize")?,
        font_weight: optional_u16(object.get("fontWeight"), 1, 1_000, "sample.fontWeight")?,
        line_height: optional_number(object.get("lineHeight"), 1.0, 2_048.0, "sample.lineHeight")?,
        padding: optional_text(object.get("padding"), 64, "sample.padding")?,
        gap: optional_number(object.get("gap"), 0.0, MAX_CSS_LENGTH, "sample.gap")?,
        radius: optional_u32(object.get("radius"), 0, 100_000, "sample.radius")?,
        border: optional_text(object.get("border"), 96, "sample.border")?,
        shadow: optional_text(object.get("shadow"), 160, "sample.shadow")?,
        width: optional_u32(object.get("width"), 0, 100_000, "sample.width")?,
        height: optional_u32(object.get("height"), 0, 100_000, "sample.height")?,
    })
}

fn parse_css_variables(value: &Value) -> Result<Vec<CssVariableEvidence>, EvidenceError> {
    bounded_array(value, MAX_CSS_VARIABLES, "cssVariables")?
        .iter()
        .map(|value| {
            let object = as_object(value, "cssVariables[]")?;
            let name = text(object.get("name"), 66, "cssVariables[].name")?;
            if !name.starts_with("--") {
                return Err(EvidenceError::Malformed("cssVariables[].name"));
            }
            let kind = text(object.get("kind"), 16, "cssVariables[].kind")?;
            if !matches!(kind.as_str(), "color" | "length" | "font") {
                return Err(EvidenceError::Malformed("cssVariables[].kind"));
            }
            Ok(CssVariableEvidence {
                name,
                value: text(object.get("value"), 120, "cssVariables[].value")?,
                kind,
            })
        })
        .collect()
}

fn parse_text_array(
    value: &Value,
    max_items: usize,
    max_chars: usize,
    field: &'static str,
) -> Result<Vec<String>, EvidenceError> {
    bounded_array(value, max_items, field)?
        .iter()
        .map(|value| text(Some(value), max_chars, field))
        .collect()
}

fn required<'a>(
    object: &'a Map<String, Value>,
    key: &'static str,
) -> Result<&'a Value, EvidenceError> {
    object.get(key).ok_or(EvidenceError::Malformed(key))
}

fn as_object<'a>(
    value: &'a Value,
    field: &'static str,
) -> Result<&'a Map<String, Value>, EvidenceError> {
    value.as_object().ok_or(EvidenceError::Malformed(field))
}

fn bounded_array<'a>(
    value: &'a Value,
    max: usize,
    field: &'static str,
) -> Result<&'a [Value], EvidenceError> {
    let values = value.as_array().ok_or(EvidenceError::Malformed(field))?;
    if values.len() > max {
        return Err(EvidenceError::Malformed(field));
    }
    Ok(values)
}

fn text(
    value: Option<&Value>,
    max_chars: usize,
    field: &'static str,
) -> Result<String, EvidenceError> {
    let raw = value
        .and_then(Value::as_str)
        .ok_or(EvidenceError::Malformed(field))?;
    let trimmed = js_trim(raw);
    if trimmed.chars().count() > max_chars
        || crate::design_md_validate::contains_external_reference(trimmed)
        || trimmed.contains(['<', '>'])
        || trimmed
            .chars()
            .any(|ch| ch.is_control() || ch == '\u{2028}' || ch == '\u{2029}')
    {
        return Err(EvidenceError::Malformed(field));
    }
    Ok(trimmed.to_owned())
}

fn optional_text(
    value: Option<&Value>,
    max_chars: usize,
    field: &'static str,
) -> Result<Option<String>, EvidenceError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => text(Some(value), max_chars, field).map(Some),
    }
}

fn integer(value: Option<&Value>, field: &'static str) -> Result<u64, EvidenceError> {
    value
        .and_then(Value::as_u64)
        .ok_or(EvidenceError::Malformed(field))
}

fn bounded_integer(
    value: Option<&Value>,
    min: u64,
    max: u64,
    field: &'static str,
) -> Result<u64, EvidenceError> {
    let value = integer(value, field)?;
    if !(min..=max).contains(&value) {
        return Err(EvidenceError::Malformed(field));
    }
    Ok(value)
}

fn bounded_u32(
    value: Option<&Value>,
    min: u32,
    max: u32,
    field: &'static str,
) -> Result<u32, EvidenceError> {
    Ok(bounded_integer(value, u64::from(min), u64::from(max), field)? as u32)
}

fn count(value: Option<&Value>, field: &'static str) -> Result<u64, EvidenceError> {
    bounded_integer(value, 1, MAX_COUNT, field)
}

fn bounded_number(
    value: Option<&Value>,
    min: f64,
    max: f64,
    field: &'static str,
) -> Result<f64, EvidenceError> {
    let value = value
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= min && *value <= max)
        .ok_or(EvidenceError::Malformed(field))?;
    Ok(value)
}

fn optional_number(
    value: Option<&Value>,
    min: f64,
    max: f64,
    field: &'static str,
) -> Result<Option<f64>, EvidenceError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => bounded_number(Some(value), min, max, field).map(Some),
    }
}

fn optional_u32(
    value: Option<&Value>,
    min: u32,
    max: u32,
    field: &'static str,
) -> Result<Option<u32>, EvidenceError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => bounded_u32(Some(value), min, max, field).map(Some),
    }
}

fn optional_u16(
    value: Option<&Value>,
    min: u32,
    max: u32,
    field: &'static str,
) -> Result<Option<u16>, EvidenceError> {
    optional_u32(value, min, max, field).map(|value| value.map(|number| number as u16))
}

fn hex_color(value: &Value, field: &'static str) -> Result<String, EvidenceError> {
    let value = value.as_str().ok_or(EvidenceError::Malformed(field))?;
    let bytes = value.as_bytes();
    if !matches!(bytes.len(), 7 | 9)
        || bytes.first() != Some(&b'#')
        || !bytes[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return Err(EvidenceError::Malformed(field));
    }
    Ok(value.to_ascii_uppercase())
}

fn optional_hex(
    value: Option<&Value>,
    field: &'static str,
) -> Result<Option<String>, EvidenceError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => hex_color(value, field).map(Some),
    }
}
