use super::cascade_display::canonical_display_serialization;
use super::cascade_shared::{keyword_at, matching, split_top_level, top_level_delimiter};
use super::declarations::parse_declarations;
use super::selectors::{parse_selector_list, PseudoClass, Selector};
use crate::color::parse_css_color;
use crate::length::{parse_length, LengthCtx};

const MAX_CONDITION_DEPTH: usize = 64;

/// A single `@media` query that could not be evaluated. Every variant is a
/// non-fatal "ignored" condition: [`media_list`] renders it into the
/// import's warning list, so the `Display` text is user-visible and must
/// stay byte-identical to the strings this replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaQueryError {
    /// `@media` with no condition parts at all.
    EmptyQuery,
    /// A media TYPE (`all`/`screen`/`print`/…) that is not recognised.
    UnsupportedType(String),
    /// A condition that is not a parenthesised feature.
    UnsupportedCondition(String),
    /// `(orientation: …)` with a value that is neither portrait nor landscape.
    InvalidOrientation(String),
    /// A `(name: value)` feature outside the supported width/height family.
    UnsupportedFeature(String),
    /// A range condition whose operand/operator shape is not supported.
    UnsupportedRange(String),
    /// A range condition with an empty operand.
    InvalidRange(String),
    /// A length operand that does not parse; carries the TRIMMED input.
    InvalidLength(String),
}

impl std::fmt::Display for MediaQueryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyQuery => formatter.write_str("empty @media query ignored"),
            Self::UnsupportedType(name) => {
                write!(formatter, "unsupported @media type '{name}' ignored")
            }
            Self::UnsupportedCondition(input) => {
                write!(formatter, "unsupported @media condition '{input}' ignored")
            }
            Self::InvalidOrientation(value) => {
                write!(formatter, "invalid @media orientation '{value}' ignored")
            }
            Self::UnsupportedFeature(name) => {
                write!(formatter, "unsupported @media feature '{name}' ignored")
            }
            Self::UnsupportedRange(input) => {
                write!(formatter, "unsupported @media range '({input})' ignored")
            }
            Self::InvalidRange(input) => {
                write!(formatter, "invalid @media range '({input})' ignored")
            }
            Self::InvalidLength(value) => {
                write!(formatter, "invalid @media length '{value}' ignored")
            }
        }
    }
}

impl std::error::Error for MediaQueryError {}

pub(super) fn media_list(input: &str, viewport: (f64, f64)) -> (bool, Vec<MediaQueryError>) {
    let mut warnings = Vec::new();
    let applies = split_top_level(input, ",").into_iter().any(|query| {
        match media_query(query.trim(), viewport) {
            Ok(value) => value,
            Err(reason) => {
                warnings.push(reason);
                false
            }
        }
    });
    (applies, warnings)
}

fn media_query(input: &str, viewport: (f64, f64)) -> Result<bool, MediaQueryError> {
    let mut query = input.trim();
    // A `<media-condition>` may be or-joined. CSS forbids mixing `and` and
    // `or` at the same level without parentheses, so splitting on the top-level
    // `or` first and running each branch as its own query is unambiguous.
    let alternatives = split_top_level(query, "or");
    if alternatives.len() > 1 {
        let mut applies = false;
        for alternative in alternatives {
            applies |= media_query(alternative, viewport)?;
        }
        return Ok(applies);
    }
    let mut negate = false;
    if let Some(rest) = strip_keyword(query, "not") {
        negate = true;
        query = rest;
    } else if let Some(rest) = strip_keyword(query, "only") {
        query = rest;
    }
    let parts = split_top_level(query, "and");
    if parts.is_empty() {
        return Err(MediaQueryError::EmptyQuery);
    }
    let mut result = true;
    for (index, part) in parts.iter().enumerate() {
        let part = part.trim();
        if index == 0 && !part.starts_with('(') {
            result &= match part.to_ascii_lowercase().as_str() {
                "all" | "screen" => true,
                "print" | "speech" => false,
                other => {
                    return Err(MediaQueryError::UnsupportedType(other.to_string()));
                }
            };
        } else {
            result &= media_feature(part, viewport)?;
        }
    }
    Ok(if negate { !result } else { result })
}

fn media_feature(input: &str, viewport: (f64, f64)) -> Result<bool, MediaQueryError> {
    let inner = strip_outer_parens(input)
        .ok_or_else(|| MediaQueryError::UnsupportedCondition(input.to_string()))?;
    // A parenthesised group can be a whole nested condition rather than a
    // feature: `((max-width:20em) or (orientation:portrait))`.
    if split_top_level(inner, "or").len() > 1 || split_top_level(inner, "and").len() > 1 {
        return media_query(inner, viewport);
    }
    if let Some((name, value)) = split_once_top_level(inner, ':') {
        return media_colon_feature(name, value, viewport);
    }
    media_range_feature(inner, viewport)
}

fn media_colon_feature(
    name: &str,
    value: &str,
    viewport: (f64, f64),
) -> Result<bool, MediaQueryError> {
    let name = name.trim().to_ascii_lowercase();
    let value = value.trim();
    if name == "orientation" {
        return match value.to_ascii_lowercase().as_str() {
            "portrait" => Ok(viewport.1 >= viewport.0),
            "landscape" => Ok(viewport.0 > viewport.1),
            _ => Err(MediaQueryError::InvalidOrientation(value.to_string())),
        };
    }
    let (axis, minimum, maximum) = match name.as_str() {
        "width" => (viewport.0, false, false),
        "min-width" => (viewport.0, true, false),
        "max-width" => (viewport.0, false, true),
        "height" => (viewport.1, false, false),
        "min-height" => (viewport.1, true, false),
        "max-height" => (viewport.1, false, true),
        _ => return Err(MediaQueryError::UnsupportedFeature(name.clone())),
    };
    let target = media_length(value, axis, viewport)?;
    Ok(if minimum {
        axis >= target
    } else if maximum {
        axis <= target
    } else {
        (axis - target).abs() < f64::EPSILON
    })
}

fn media_range_feature(input: &str, viewport: (f64, f64)) -> Result<bool, MediaQueryError> {
    let (operands, operators) = range_tokens(input)?;
    if !(operators.len() == 1 || operators.len() == 2) || operands.len() != operators.len() + 1 {
        return Err(MediaQueryError::UnsupportedRange(input.to_string()));
    }
    let feature_count = operands
        .iter()
        .filter(|operand| {
            matches!(
                operand.trim().to_ascii_lowercase().as_str(),
                "width" | "height"
            )
        })
        .count();
    if feature_count != 1 {
        return Err(MediaQueryError::UnsupportedRange(input.to_string()));
    }
    let axis = if operands
        .iter()
        .any(|operand| operand.trim().eq_ignore_ascii_case("width"))
    {
        viewport.0
    } else {
        viewport.1
    };
    let values = operands
        .iter()
        .map(|operand| {
            if matches!(
                operand.trim().to_ascii_lowercase().as_str(),
                "width" | "height"
            ) {
                Ok(axis)
            } else {
                media_length(operand, axis, viewport)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(operators
        .iter()
        .enumerate()
        .all(|(index, operator)| compare_range(values[index], operator, values[index + 1])))
}

fn range_tokens(input: &str) -> Result<(Vec<&str>, Vec<&str>), MediaQueryError> {
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    let mut start = 0;
    let bytes = input.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if matches!(bytes[at], b'<' | b'>' | b'=') {
            operands.push(input[start..at].trim());
            let width = usize::from(bytes.get(at + 1) == Some(&b'=')) + 1;
            operators.push(&input[at..at + width]);
            at += width;
            start = at;
        } else {
            at += 1;
        }
    }
    operands.push(input[start..].trim());
    if operands.iter().any(|operand| operand.is_empty()) {
        return Err(MediaQueryError::InvalidRange(input.to_string()));
    }
    Ok((operands, operators))
}

fn compare_range(left: f64, operator: &str, right: f64) -> bool {
    match operator {
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        "=" => (left - right).abs() < f64::EPSILON,
        _ => false,
    }
}

fn media_length(value: &str, axis: f64, viewport: (f64, f64)) -> Result<f64, MediaQueryError> {
    let context = LengthCtx {
        font_size: 16.0,
        root_font_size: 16.0,
        viewport_w: viewport.0,
        viewport_h: viewport.1,
    };
    parse_length(value.trim(), &context)
        .map(|length| length.resolve(axis))
        .ok_or_else(|| MediaQueryError::InvalidLength(value.trim().to_string()))
}

pub(super) fn supports_condition(input: &str, depth: usize) -> Option<bool> {
    if depth >= MAX_CONDITION_DEPTH {
        return None;
    }
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Some(rest) = strip_keyword(input, "not") {
        return supports_condition(rest, depth + 1).map(|value| !value);
    }
    for operator in ["or", "and"] {
        let parts = split_top_level(input, operator);
        if parts.len() > 1 {
            let values = parts
                .into_iter()
                .map(|part| supports_condition(part, depth + 1))
                .collect::<Option<Vec<_>>>()?;
            return Some(if operator == "or" {
                values.into_iter().any(|value| value)
            } else {
                values.into_iter().all(|value| value)
            });
        }
    }
    if let Some(body) = function_body(input, "selector") {
        return Some(supports_selector(body));
    }
    if let Some(inner) = strip_outer_parens(input) {
        if split_once_top_level(inner, ':').is_some() {
            return Some(supports_declaration(inner));
        }
        return supports_condition(inner, depth + 1);
    }
    if split_once_top_level(input, ':').is_some() {
        return Some(supports_declaration(input));
    }
    Some(false)
}

fn supports_selector(input: &str) -> bool {
    let selectors = parse_selector_list(input);
    !selectors.is_empty() && selectors.iter().all(selector_is_supported)
}

fn selector_is_supported(selector: &Selector) -> bool {
    selector.compounds.iter().all(|compound| {
        compound.pseudo_classes.iter().all(|pseudo| match pseudo {
            PseudoClass::Unsupported(_) => false,
            PseudoClass::Not(selectors)
            | PseudoClass::Is(selectors)
            | PseudoClass::Where(selectors)
            | PseudoClass::NthChildOf(_, selectors)
            | PseudoClass::NthLastChildOf(_, selectors) => {
                selectors.iter().all(selector_is_supported)
            }
            PseudoClass::Has(selectors) => selectors
                .iter()
                .all(|relative| selector_is_supported(&relative.selector)),
            _ => true,
        })
    })
}

fn supports_declaration(input: &str) -> bool {
    let Some((name, value)) = split_once_top_level(input, ':') else {
        return false;
    };
    let name = name.trim().to_ascii_lowercase();
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if name.starts_with("--") {
        return name.len() > 2;
    }
    if !supported_property(&name) {
        return false;
    }
    if parse_declarations(&format!("{name}:{value}")).is_empty() {
        return false;
    }
    match name.as_str() {
        "display" => canonical_display_serialization(value).is_some(),
        "position" => matches!(
            value.to_ascii_lowercase().as_str(),
            "static" | "relative" | "absolute" | "fixed" | "sticky"
        ),
        "color"
        | "text-fill-color"
        | "-webkit-text-fill-color"
        | "background-color"
        | "border-color" => {
            parse_css_color(value).is_some()
                || value.eq_ignore_ascii_case("currentcolor")
                || value.trim_start().to_ascii_lowercase().starts_with("var(")
        }
        "opacity" => {
            value
                .parse::<f64>()
                .is_ok_and(|number| (0.0..=1.0).contains(&number))
                || value.trim_start().to_ascii_lowercase().starts_with("var(")
        }
        "width" | "height" | "min-width" | "min-height" | "max-width" | "max-height" | "top"
        | "right" | "bottom" | "left" | "margin" | "margin-top" | "margin-right"
        | "margin-bottom" | "margin-left" | "padding" | "padding-top" | "padding-right"
        | "padding-bottom" | "padding-left" | "gap" | "row-gap" | "column-gap" | "font-size"
        | "letter-spacing" => supports_length(value),
        _ => true,
    }
}

fn supports_length(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    let context = LengthCtx {
        font_size: 16.0,
        root_font_size: 16.0,
        viewport_w: 1280.0,
        viewport_h: 800.0,
    };
    let valid = |part: &str| {
        matches!(
            part,
            "auto" | "none" | "normal" | "min-content" | "max-content" | "fit-content"
        ) || part.starts_with("var(")
            || parse_length(part, &context).is_some()
    };
    if valid(&lower) {
        return true;
    }
    lower.split_ascii_whitespace().all(valid)
}

fn supported_property(name: &str) -> bool {
    matches!(
        name,
        "-webkit-text-fill-color"
            | "align-content"
            | "align-items"
            | "align-self"
            | "appearance"
            | "backdrop-filter"
            | "background"
            | "background-attachment"
            | "background-blend-mode"
            | "background-clip"
            | "background-color"
            | "background-image"
            | "background-origin"
            | "background-position"
            | "background-repeat"
            | "background-size"
            | "border"
            | "border-block"
            | "border-bottom"
            | "border-color"
            | "border-inline"
            | "border-left"
            | "border-radius"
            | "border-right"
            | "border-style"
            | "border-top"
            | "border-width"
            | "bottom"
            | "box-shadow"
            | "box-sizing"
            | "color"
            | "column-gap"
            | "columns"
            | "content"
            | "cursor"
            | "display"
            | "filter"
            | "flex"
            | "flex-basis"
            | "flex-direction"
            | "flex-flow"
            | "flex-grow"
            | "flex-shrink"
            | "flex-wrap"
            | "float"
            | "font"
            | "font-family"
            | "font-feature-settings"
            | "font-size"
            | "font-stretch"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "gap"
            | "grid-template-columns"
            | "height"
            | "inset"
            | "justify-content"
            | "justify-items"
            | "justify-self"
            | "left"
            | "letter-spacing"
            | "line-height"
            | "margin"
            | "margin-block"
            | "margin-bottom"
            | "margin-inline"
            | "margin-left"
            | "margin-right"
            | "margin-top"
            | "max-height"
            | "max-width"
            | "min-height"
            | "min-width"
            | "mix-blend-mode"
            | "object-fit"
            | "opacity"
            | "outline"
            | "overflow"
            | "overflow-x"
            | "overflow-y"
            | "padding"
            | "padding-block"
            | "padding-bottom"
            | "padding-inline"
            | "padding-left"
            | "padding-right"
            | "padding-top"
            | "place-content"
            | "place-items"
            | "place-self"
            | "position"
            | "right"
            | "row-gap"
            | "text-align"
            | "text-decoration"
            | "text-decoration-line"
            | "text-fill-color"
            | "text-transform"
            | "top"
            | "transform"
            | "visibility"
            | "white-space"
            | "width"
            | "z-index"
    )
}

fn function_body<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let prefix = input.get(..name.len())?;
    if !prefix.eq_ignore_ascii_case(name) || input.as_bytes().get(name.len()) != Some(&b'(') {
        return None;
    }
    let close = matching(input, name.len(), b'(', b')', input.len())?;
    (close + 1 == input.len()).then(|| input[name.len() + 1..close].trim())
}

fn strip_outer_parens(input: &str) -> Option<&str> {
    let input = input.trim();
    if !input.starts_with('(') {
        return None;
    }
    let close = matching(input, 0, b'(', b')', input.len())?;
    (close + 1 == input.len()).then(|| input[1..close].trim())
}

fn split_once_top_level(input: &str, delimiter: char) -> Option<(&str, &str)> {
    let at = top_level_delimiter(input, delimiter)?;
    Some((&input[..at], &input[at + delimiter.len_utf8()..]))
}

fn strip_keyword<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    keyword_at(input, 0, keyword).then(|| input[keyword.len()..].trim_start())
}
