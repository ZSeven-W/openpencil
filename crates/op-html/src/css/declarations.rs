use super::cascade_display::valid_specified_display;
use super::declaration_syntax::{
    extract_function, find_top_level, is_background_position, is_background_repeat,
    is_border_style, is_border_width, is_css_wide, is_font_size_token, is_font_weight,
    split_once_top_level, split_top_level, split_whitespace_top_level, strip_comments,
    strip_important,
};
use crate::color::parse_css_color;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub value: String,
    pub important: bool,
    pub(crate) deferred_shorthand: Option<String>,
}

#[rustfmt::skip]
const BACKGROUND_INITIALS: &[(&str, &str)] = &[
    ("background-color", "transparent"), ("background-image", "none"),
    ("background-attachment", "scroll"), ("background-position", "0% 0%"),
    ("background-size", "auto"), ("background-repeat", "repeat"),
    ("background-origin", "padding-box"), ("background-clip", "border-box"),
];

pub fn parse_declarations(block: &str) -> Vec<Declaration> {
    let cleaned = strip_comments(block);
    let mut declarations = Vec::new();
    for statement in split_top_level(&cleaned, ';') {
        let Some(colon) = find_top_level(statement, ':') else {
            continue;
        };
        let raw_name = statement[..colon].trim();
        let raw_value = statement[colon + 1..].trim();
        if raw_name.is_empty() || raw_value.is_empty() {
            continue;
        }
        let name = if raw_name.starts_with("--") {
            raw_name.to_string()
        } else {
            normalize_property_name(raw_name)
        };
        let (value, important) = strip_important(raw_value);
        if value.is_empty() {
            continue;
        }
        if name == "display" && !valid_specified_display(value) {
            continue;
        }
        if name == "direction" && !valid_specified_direction(value) {
            continue;
        }
        if name.starts_with("--") {
            push(&mut declarations, &name, value, important);
            continue;
        }
        if deferred::is_shorthand_with_var(&name, value) {
            deferred::push_deferred(&mut declarations, &name, value, important);
            continue;
        }
        expand_declaration(&mut declarations, &name, value, important);
    }
    declarations
}

fn valid_specified_direction(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "ltr" | "rtl" | "inherit" | "initial" | "unset" | "revert" | "revert-layer"
    ) || deferred::contains_var_function(value)
}

fn expand_declaration(out: &mut Vec<Declaration>, name: &str, value: &str, important: bool) {
    match name {
        "margin" | "padding" | "inset" => expand_box(out, name, value, important),
        "margin-block" | "padding-block" | "inset-block" => {
            expand_axis(out, name, value, "top", "bottom", important)
        }
        "margin-inline" => expand_axis(out, name, value, "inline-start", "inline-end", important),
        "padding-inline" | "inset-inline" => {
            expand_axis(out, name, value, "left", "right", important)
        }
        "margin-block-start" => push(out, "margin-top", value, important),
        "margin-block-end" => push(out, "margin-bottom", value, important),
        "margin-inline-start" | "margin-inline-end" => push(out, name, value, important),
        "padding-block-start" => push(out, "padding-top", value, important),
        "padding-block-end" => push(out, "padding-bottom", value, important),
        "padding-inline-start" => push(out, "padding-left", value, important),
        "padding-inline-end" => push(out, "padding-right", value, important),
        "inset-block-start" => push(out, "top", value, important),
        "inset-block-end" => push(out, "bottom", value, important),
        "inset-inline-start" => push(out, "left", value, important),
        "inset-inline-end" => push(out, "right", value, important),
        "border" | "outline" => expand_border(out, name, None, value, important),
        "border-top" | "border-right" | "border-bottom" | "border-left" => expand_border(
            out,
            "border",
            name.strip_prefix("border-"),
            value,
            important,
        ),
        "border-block" => {
            expand_border(out, "border", Some("top"), value, important);
            expand_border(out, "border", Some("bottom"), value, important);
        }
        "border-inline" => {
            expand_border(out, "border", Some("left"), value, important);
            expand_border(out, "border", Some("right"), value, important);
        }
        "border-width" | "border-style" | "border-color" => {
            expand_border_component(out, name, value, important)
        }
        "border-block-width" | "border-block-style" | "border-block-color" => {
            expand_logical_border_component(out, name, value, "top", "bottom", important)
        }
        "border-inline-width" | "border-inline-style" | "border-inline-color" => {
            expand_logical_border_component(out, name, value, "left", "right", important)
        }
        "border-radius" => expand_border_radius(out, value, important),
        "background" => expand_background(out, value, important),
        "font" => expand_font(out, value, important),
        "flex" => expand_flex(out, value, important),
        "flex-flow" => expand_flex_flow(out, value, important),
        "gap" => expand_gap(out, value, important),
        "overflow" => expand_overflow(out, value, important),
        "place-items" => expand_pair(out, value, "align-items", "justify-items", important),
        "place-content" => expand_pair(out, value, "align-content", "justify-content", important),
        "place-self" => expand_pair(out, value, "align-self", "justify-self", important),
        "text-decoration" => expand_text_decoration(out, value, important),
        "columns" => expand_columns(out, value, important),
        _ => push(out, name, value, important),
    }
}

fn normalize_property_name(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    for prefix in ["-webkit-", "-moz-", "-ms-", "-o-"] {
        if let Some(unprefixed) = lower.strip_prefix(prefix) {
            if matches!(
                unprefixed,
                "appearance"
                    | "backdrop-filter"
                    | "background-clip"
                    | "box-sizing"
                    | "filter"
                    | "hyphens"
                    | "line-clamp"
                    | "mask"
                    | "mask-image"
                    | "text-fill-color"
                    | "text-size-adjust"
                    | "transform"
                    | "transform-origin"
                    | "user-select"
            ) {
                return unprefixed.to_string();
            }
        }
    }
    lower
}

fn expand_box(out: &mut Vec<Declaration>, name: &str, value: &str, important: bool) {
    let Some(sides) = four_sides(value) else {
        return;
    };
    let prefix = if name == "inset" { "" } else { name };
    for (side, value) in ["top", "right", "bottom", "left"].into_iter().zip(sides) {
        let property = if prefix.is_empty() {
            side.to_string()
        } else {
            format!("{prefix}-{side}")
        };
        push(out, &property, &value, important);
    }
}

fn four_sides(value: &str) -> Option<[String; 4]> {
    let values = split_whitespace_top_level(value);
    let sides = match values.as_slice() {
        [all] => [all.clone(), all.clone(), all.clone(), all.clone()],
        [vertical, horizontal] => [
            vertical.clone(),
            horizontal.clone(),
            vertical.clone(),
            horizontal.clone(),
        ],
        [top, horizontal, bottom] => [
            top.clone(),
            horizontal.clone(),
            bottom.clone(),
            horizontal.clone(),
        ],
        [top, right, bottom, left] => [top.clone(), right.clone(), bottom.clone(), left.clone()],
        _ => return None,
    };
    Some(sides)
}

fn expand_axis(
    out: &mut Vec<Declaration>,
    name: &str,
    value: &str,
    start: &str,
    end: &str,
    important: bool,
) {
    let values = split_whitespace_top_level(value);
    let (first, second) = match values.as_slice() {
        [both] => (both, both),
        [first, second] => (first, second),
        _ => return,
    };
    let base = name
        .strip_suffix("-block")
        .or_else(|| name.strip_suffix("-inline"))
        .unwrap_or(name);
    let property = |side: &str| {
        if base == "inset" {
            side.to_string()
        } else {
            format!("{base}-{side}")
        }
    };
    push(out, &property(start), first, important);
    push(out, &property(end), second, important);
}

fn expand_border(
    out: &mut Vec<Declaration>,
    base: &str,
    side: Option<&str>,
    value: &str,
    important: bool,
) {
    if is_css_wide(value) {
        for component in ["width", "style", "color"] {
            push_border_longhand(out, base, side, component, value, important);
        }
        return;
    }
    let mut width = None;
    let mut style = None;
    let mut color = None;
    for token in split_whitespace_top_level(value) {
        let is_variable = token.trim_start().to_ascii_lowercase().starts_with("var(");
        if parse_css_color(&token).is_some() || token.eq_ignore_ascii_case("currentcolor") {
            color = Some(token);
        } else if is_border_style(&token) {
            style = Some(token);
        } else if is_variable && (width.is_some() || style.is_some()) {
            color = Some(token);
        } else if is_border_width(&token) {
            width = Some(token);
        }
    }
    let initial_color = if base == "outline" {
        "auto"
    } else {
        "currentcolor"
    };
    for (component, value, initial) in [
        ("width", width.as_deref(), "medium"),
        ("style", style.as_deref(), "none"),
        ("color", color.as_deref(), initial_color),
    ] {
        let value = value.unwrap_or(initial);
        push_border_longhand(out, base, side, component, value, important);
    }
}

fn push_border_longhand(
    out: &mut Vec<Declaration>,
    base: &str,
    side: Option<&str>,
    component: &str,
    value: &str,
    important: bool,
) {
    let prefix = side.map_or_else(|| base.to_string(), |side| format!("{base}-{side}"));
    push(out, &format!("{prefix}-{component}"), value, important);
    if base == "border" && side.is_none() {
        for side in ["top", "right", "bottom", "left"] {
            push(out, &format!("border-{side}-{component}"), value, important);
        }
    }
}

fn expand_border_component(out: &mut Vec<Declaration>, name: &str, value: &str, important: bool) {
    let Some(sides) = four_sides(value) else {
        return;
    };
    let component = name.strip_prefix("border-").unwrap_or(name);
    push(out, name, value, important);
    for (side, value) in ["top", "right", "bottom", "left"].into_iter().zip(sides) {
        push(
            out,
            &format!("border-{side}-{component}"),
            &value,
            important,
        );
    }
}

fn expand_logical_border_component(
    out: &mut Vec<Declaration>,
    name: &str,
    value: &str,
    first_side: &str,
    second_side: &str,
    important: bool,
) {
    let values = split_whitespace_top_level(value);
    let (first, second) = match values.as_slice() {
        [both] => (both, both),
        [first, second] => (first, second),
        _ => return,
    };
    let component = name.rsplit('-').next().unwrap_or(name);
    push(
        out,
        &format!("border-{first_side}-{component}"),
        first,
        important,
    );
    push(
        out,
        &format!("border-{second_side}-{component}"),
        second,
        important,
    );
}

fn expand_border_radius(out: &mut Vec<Declaration>, value: &str, important: bool) {
    if is_css_wide(value) {
        push(out, "border-radius", value, important);
        for corner in ["top-left", "top-right", "bottom-right", "bottom-left"] {
            push(out, &format!("border-{corner}-radius"), value, important);
        }
        return;
    }
    let horizontal = split_top_level(value, '/')
        .into_iter()
        .next()
        .unwrap_or(value)
        .trim();
    let Some(corners) = four_sides(horizontal) else {
        return;
    };
    push(out, "border-radius", horizontal, important);
    for (corner, value) in ["top-left", "top-right", "bottom-right", "bottom-left"]
        .into_iter()
        .zip(corners)
    {
        push(out, &format!("border-{corner}-radius"), &value, important);
    }
}

fn expand_background(out: &mut Vec<Declaration>, value: &str, important: bool) {
    if is_css_wide(value) {
        for &(property, _) in BACKGROUND_INITIALS {
            push(out, property, value, important);
        }
        return;
    }
    for &(property, initial) in BACKGROUND_INITIALS {
        push(out, property, initial, important);
    }
    if value.trim().eq_ignore_ascii_case("none") {
        return;
    }
    let layers = split_top_level(value, ',');
    let mut images = Vec::new();
    let mut positions = Vec::new();
    let mut sizes = Vec::new();
    let mut repeats = Vec::new();
    let mut color = None;
    for (index, layer) in layers.iter().enumerate() {
        let mut has_image = false;
        if let Some(image) = find_image_function(layer) {
            images.push(image);
            has_image = true;
        } else if layer
            .split_whitespace()
            .any(|part| part.eq_ignore_ascii_case("none"))
        {
            images.push("none".to_string());
            has_image = true;
        }
        let tokens = split_whitespace_top_level(layer);
        let mut position_parts = Vec::new();
        let mut size_parts = Vec::new();
        let mut after_slash = false;
        for token in tokens {
            if token == "/" {
                after_slash = true;
                continue;
            }
            let is_variable = token.trim_start().to_ascii_lowercase().starts_with("var(");
            if is_variable && !has_image {
                images.push(token.clone());
                has_image = true;
            }
            if index + 1 == layers.len()
                && (parse_css_color(&token).is_some()
                    || token.eq_ignore_ascii_case("currentcolor")
                    || is_variable)
            {
                color = Some(token);
            } else if is_background_repeat(&token) {
                repeats.push(token);
            } else if !token.contains('(') {
                if after_slash {
                    size_parts.push(token);
                } else if is_background_position(&token) {
                    position_parts.push(token);
                }
            }
        }
        if !position_parts.is_empty() {
            positions.push(position_parts.join(" "));
        }
        if !size_parts.is_empty() {
            sizes.push(size_parts.join(" "));
        }
    }
    if !images.is_empty() {
        push(out, "background-image", &images.join(", "), important);
    }
    if let Some(color) = color {
        push(out, "background-color", &color, important);
    }
    if !positions.is_empty() {
        push(out, "background-position", &positions.join(", "), important);
    }
    if !sizes.is_empty() {
        push(out, "background-size", &sizes.join(", "), important);
    }
    if !repeats.is_empty() {
        push(out, "background-repeat", &repeats.join(" "), important);
    }
}

fn find_image_function(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let mut best = None;
    for needle in [
        "url(",
        "linear-gradient(",
        "repeating-linear-gradient(",
        "radial-gradient(",
        "repeating-radial-gradient(",
        "conic-gradient(",
    ] {
        if let Some(start) = lower.find(needle) {
            if best
                .as_ref()
                .is_none_or(|(best_start, _)| start < *best_start)
            {
                best = extract_function(value, start).map(|function| (start, function));
            }
        }
    }
    best.map(|(_, function)| function)
}

fn expand_font(out: &mut Vec<Declaration>, value: &str, important: bool) {
    if is_css_wide(value) {
        for property in [
            "font-family",
            "font-size",
            "font-style",
            "font-weight",
            "font-stretch",
            "line-height",
        ] {
            push(out, property, value, important);
        }
        return;
    }
    let tokens = split_whitespace_top_level(value);
    // CSS-wide keywords are valid as the entire shorthand only. In
    // particular, `font: 700 15px inherit` must not treat `inherit` as a
    // family name; browsers discard that whole declaration.
    if tokens
        .iter()
        .any(|token| token.split([',', '/']).any(is_css_wide))
    {
        return;
    }
    let Some(size_index) = tokens.iter().position(|token| is_font_size_token(token)) else {
        return;
    };
    push(out, "font-style", "normal", important);
    push(out, "font-weight", "normal", important);
    push(out, "font-stretch", "normal", important);
    push(out, "line-height", "normal", important);
    for token in &tokens[..size_index] {
        let lower = token.to_ascii_lowercase();
        if matches!(lower.as_str(), "italic" | "oblique" | "normal") {
            push(out, "font-style", token, important);
        } else if is_font_weight(token) {
            push(out, "font-weight", token, important);
        } else if lower.ends_with("condensed") || lower.ends_with("expanded") {
            push(out, "font-stretch", token, important);
        }
    }
    let mut size = tokens[size_index].as_str();
    let mut line_height = None;
    if let Some((left, right)) = split_once_top_level(size, '/') {
        size = left.trim();
        line_height = Some(right.trim().to_string());
    }
    let mut family_start = size_index + 1;
    if line_height.is_none() && tokens.get(family_start).is_some_and(|token| token == "/") {
        line_height = tokens.get(family_start + 1).cloned();
        family_start += 2;
    }
    push(out, "font-size", size, important);
    if let Some(line_height) = line_height {
        push(out, "line-height", &line_height, important);
    }
    if family_start < tokens.len() {
        push(
            out,
            "font-family",
            &tokens[family_start..].join(" "),
            important,
        );
    }
}

fn expand_flex(out: &mut Vec<Declaration>, value: &str, important: bool) {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "none" => {
            push(out, "flex-grow", "0", important);
            push(out, "flex-shrink", "0", important);
            push(out, "flex-basis", "auto", important);
            return;
        }
        "auto" => {
            push(out, "flex-grow", "1", important);
            push(out, "flex-shrink", "1", important);
            push(out, "flex-basis", "auto", important);
            return;
        }
        "initial" => {
            push(out, "flex-grow", "0", important);
            push(out, "flex-shrink", "1", important);
            push(out, "flex-basis", "auto", important);
            return;
        }
        _ if is_css_wide(&lower) => {
            for property in ["flex-grow", "flex-shrink", "flex-basis"] {
                push(out, property, &lower, important);
            }
            return;
        }
        _ => {}
    }
    let tokens = split_whitespace_top_level(value);
    let mut numbers = Vec::new();
    let mut basis = None;
    for token in tokens {
        if token.parse::<f64>().is_ok() && numbers.len() < 2 {
            numbers.push(token);
        } else {
            basis = Some(token);
        }
    }
    if let Some(grow) = numbers.first() {
        push(out, "flex-grow", grow, important);
        push(
            out,
            "flex-shrink",
            numbers.get(1).map_or("1", String::as_str),
            important,
        );
        push(
            out,
            "flex-basis",
            basis.as_deref().unwrap_or("0%"),
            important,
        );
    } else if let Some(basis) = basis {
        push(out, "flex-grow", "1", important);
        push(out, "flex-shrink", "1", important);
        push(out, "flex-basis", &basis, important);
    }
}

fn expand_flex_flow(out: &mut Vec<Declaration>, value: &str, important: bool) {
    if is_css_wide(value) {
        push(out, "flex-direction", value, important);
        push(out, "flex-wrap", value, important);
        return;
    }
    let mut direction = None;
    let mut wrap = None;
    for token in split_whitespace_top_level(value) {
        let lower = token.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "row" | "row-reverse" | "column" | "column-reverse"
        ) {
            if direction.replace(token).is_some() {
                return;
            }
        } else if matches!(lower.as_str(), "nowrap" | "wrap" | "wrap-reverse") {
            if wrap.replace(token).is_some() {
                return;
            }
        } else {
            return;
        }
    }
    for (name, value, initial) in [
        ("flex-direction", direction.as_deref(), "row"),
        ("flex-wrap", wrap.as_deref(), "nowrap"),
    ] {
        push(out, name, value.unwrap_or(initial), important);
    }
}

fn expand_gap(out: &mut Vec<Declaration>, value: &str, important: bool) {
    let values = split_whitespace_top_level(value);
    let (row, column) = match values.as_slice() {
        [both] => (both, both),
        [row, column] => (row, column),
        _ => return,
    };
    push(out, "gap", row, important);
    push(out, "row-gap", row, important);
    push(out, "column-gap", column, important);
}

fn expand_overflow(out: &mut Vec<Declaration>, value: &str, important: bool) {
    let values = split_whitespace_top_level(value);
    let (x, y) = match values.as_slice() {
        [both] => (both, both),
        [x, y] => (x, y),
        _ => return,
    };
    push(out, "overflow", value, important);
    push(out, "overflow-x", x, important);
    push(out, "overflow-y", y, important);
}

fn expand_pair(
    out: &mut Vec<Declaration>,
    value: &str,
    first_name: &str,
    second_name: &str,
    important: bool,
) {
    let values = split_whitespace_top_level(value);
    let (first, second) = match values.as_slice() {
        [both] => (both, both),
        [first, second] => (first, second),
        _ => return,
    };
    push(out, first_name, first, important);
    push(out, second_name, second, important);
}

fn expand_text_decoration(out: &mut Vec<Declaration>, value: &str, important: bool) {
    if is_css_wide(value) {
        for property in [
            "text-decoration-line",
            "text-decoration-style",
            "text-decoration-color",
            "text-decoration-thickness",
        ] {
            push(out, property, value, important);
        }
        return;
    }
    let mut lines = Vec::new();
    let mut style = None;
    let mut color = None;
    let mut thickness = None;
    for token in split_whitespace_top_level(value) {
        let lower = token.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "none" | "underline" | "overline" | "line-through" | "blink"
        ) {
            lines.push(lower);
        } else if matches!(
            lower.as_str(),
            "solid" | "double" | "dotted" | "dashed" | "wavy"
        ) {
            style = Some(lower);
        } else if parse_css_color(&token).is_some()
            || lower == "currentcolor"
            || lower.starts_with("var(")
        {
            color = Some(token);
        } else if lower == "from-font" || lower.ends_with("px") || lower.ends_with("em") {
            thickness = Some(token);
        }
    }
    let line = if lines.is_empty() {
        "none".into()
    } else {
        lines.join(" ")
    };
    push(out, "text-decoration-line", &line, important);
    for (name, value, initial) in [
        ("text-decoration-style", style.as_deref(), "solid"),
        ("text-decoration-color", color.as_deref(), "currentcolor"),
        ("text-decoration-thickness", thickness.as_deref(), "auto"),
    ] {
        push(out, name, value.unwrap_or(initial), important);
    }
}

fn expand_columns(out: &mut Vec<Declaration>, value: &str, important: bool) {
    if is_css_wide(value) {
        push(out, "column-width", value, important);
        push(out, "column-count", value, important);
        return;
    }
    let mut width = None;
    let mut count = None;
    for token in split_whitespace_top_level(value) {
        if token.parse::<u32>().is_ok() {
            count = Some(token);
        } else if !token.eq_ignore_ascii_case("auto") {
            width = Some(token);
        }
    }
    push(
        out,
        "column-width",
        width.as_deref().unwrap_or("auto"),
        important,
    );
    push(
        out,
        "column-count",
        count.as_deref().unwrap_or("auto"),
        important,
    );
}

fn push(out: &mut Vec<Declaration>, name: &str, value: &str, important: bool) {
    out.push(Declaration {
        name: name.to_string(),
        value: value.trim().to_string(),
        important,
        deferred_shorthand: None,
    });
}

#[path = "declarations_deferred.rs"]
mod deferred;

pub(crate) use deferred::resolve_deferred_shorthand;

#[cfg(test)]
#[path = "declarations_tests.rs"]
mod tests;
