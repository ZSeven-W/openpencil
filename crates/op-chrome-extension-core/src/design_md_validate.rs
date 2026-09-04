//! Defensive validation of model-produced `design.md`.
//!
//! The desktop host validates before replying, but early/unmanaged loopback
//! builds may return Markdown directly. Mirroring the structural checks here
//! makes those replies safe to accept without trusting that server version.

use std::collections::BTreeSet;

use serde_json::Value;

const STYLE: &str = "## Style Summary";
const COLORS: &str = "## Color System";
const TYPOGRAPHY: &str = "## Typography";
const RADII: &str = "## Corner Radius";

pub(super) fn is_valid(markdown: &str) -> bool {
    let first = markdown.lines().next().unwrap_or_default();
    if first != "# Design System: Extracted Web Style"
        || markdown.lines().any(|line| {
            let line = line.trim();
            line.starts_with("```") || line.starts_with("~~~")
        })
        || contains_external_reference(markdown)
        || markdown.contains(['<', '>'])
        || markdown.contains("](")
        || markdown.contains("![")
        || markdown
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return false;
    }

    let Some(style) = unique_heading(markdown, STYLE) else {
        return false;
    };
    let Some(colors) = unique_heading(markdown, COLORS) else {
        return false;
    };
    let Some(typography) = unique_heading(markdown, TYPOGRAPHY) else {
        return false;
    };
    let Some(radii) = unique_heading(markdown, RADII) else {
        return false;
    };
    if !(style < colors && colors < typography && typography < radii) {
        return false;
    }
    if markdown[first.len()..style]
        .lines()
        .any(|line| !line.trim().is_empty())
    {
        return false;
    }
    let h2s = markdown
        .lines()
        .filter(|line| line.starts_with("## "))
        .collect::<Vec<_>>();
    let required_h2s = [STYLE, COLORS, TYPOGRAPHY, RADII];
    if h2s.len() < required_h2s.len()
        || h2s
            .iter()
            .take(required_h2s.len())
            .copied()
            .ne(required_h2s)
    {
        return false;
    }
    let mut optional_rank = 0;
    for heading in h2s.iter().skip(required_h2s.len()) {
        let rank = match *heading {
            "## Spacing" => 1,
            "## Shadows" => 2,
            "## Gradients" => 3,
            "## CSS Variables" => 4,
            "## Components" => 5,
            "## Component Treatments" => 6,
            "## Responsive Behavior" => 7,
            _ => return false,
        };
        if rank <= optional_rank {
            return false;
        }
        optional_rank = rank;
    }
    for (index, heading) in h2s.iter().enumerate().skip(required_h2s.len()) {
        let Some(start) = unique_heading(markdown, heading) else {
            return false;
        };
        let end = h2s
            .get(index + 1)
            .and_then(|next| unique_heading(markdown, next))
            .unwrap_or(markdown.len());
        let lines = markdown[start..end]
            .lines()
            .skip(1)
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>();
        let mut unique = BTreeSet::new();
        if lines.is_empty()
            || lines
                .iter()
                .any(|line| !unique.insert(*line) || !optional_line_is_valid(heading, line))
        {
            return false;
        }
    }

    let style_lines = markdown[style..colors]
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if style_lines.len() != 1 || !key_palette_is_exact(style_lines[0]) {
        return false;
    }

    let color_section = &markdown[colors..typography];
    let roles = [
        "Page Background",
        "Card Surface",
        "Primary Accent",
        "Primary Text",
        "Secondary Text",
        "Muted Text",
        "Default Border",
    ];
    let color_lines = color_section
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if color_lines.len() != roles.len()
        || color_lines
            .into_iter()
            .zip(roles)
            .any(|(line, role)| !color_role_line_is_exact(line, role))
    {
        return false;
    }

    let typography_lines = markdown[typography..radii]
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if typography_lines.len() != 6
        || !primary_font_line_is_valid(typography_lines[0])
        || typography_lines[1] != "### Font Families"
        || !table_header_is_valid(typography_lines[2])
        || !table_separator_is_valid(typography_lines[3])
        || !table_role_is_valid(typography_lines[4], "Headings")
        || !table_role_is_valid(typography_lines[5], "Body / Functional")
    {
        return false;
    }

    let radius_end = first_line_offset_after(markdown, radii, |line| line.starts_with("## "))
        .unwrap_or(markdown.len());
    let radius_section = &markdown[radii..radius_end];
    let required = ["Card / Standard:", "Button / Input:"];
    let radius_lines = radius_section
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    radius_lines.len() == 2
        && radius_lines
            .iter()
            .zip(required)
            .all(|(line, label)| radius_line_is_integer_px(line, label))
}

fn key_palette_is_exact(line: &str) -> bool {
    let Some(values) = line.trim().strip_prefix("Key palette: ") else {
        return false;
    };
    let values = values.split(", ").collect::<Vec<_>>();
    values.len() == 5 && values.into_iter().all(is_upper_hex)
}

fn optional_line_is_valid(heading: &str, line: &str) -> bool {
    match heading {
        "## Spacing" => spacing_line_is_valid(line),
        "## Shadows" => prefixed_plain_value(line, "Shadow: ", 160),
        "## Gradients" => line
            .strip_prefix("Gradient: ")
            .is_some_and(gradient_value_is_valid),
        "## CSS Variables" => line
            .strip_prefix("Variable: ")
            .is_some_and(variable_json_is_valid),
        "## Components" => line
            .strip_prefix("Component: ")
            .is_some_and(component_kind_is_valid),
        "## Component Treatments" => line
            .strip_prefix("Treatment: ")
            .is_some_and(treatment_json_is_valid),
        "## Responsive Behavior" => line
            .strip_prefix("Media Query: ")
            .is_some_and(media_query_is_valid),
        _ => false,
    }
}

fn spacing_line_is_valid(line: &str) -> bool {
    ["Margin: ", "Padding: ", "Gap: "]
        .iter()
        .find_map(|prefix| line.strip_prefix(prefix))
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|value| value.is_finite() && (-1_000_000.0..=1_000_000.0).contains(&value))
}

fn prefixed_plain_value(line: &str, prefix: &str, max_chars: usize) -> bool {
    line.strip_prefix(prefix)
        .is_some_and(|value| plain_value_is_valid(value, max_chars))
}

fn plain_value_is_valid(value: &str, max_chars: usize) -> bool {
    !value.is_empty()
        && value.chars().count() <= max_chars
        && !value.contains(['`', '[', ']', '<', '>'])
        && !contains_external_reference(value)
        && !contains_prompt_directive(value)
        && !value.chars().any(char::is_control)
}

fn contains_prompt_directive(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "ignore previous",
        "ignore prior",
        "disregard",
        "forget previous",
        "system prompt",
        "developer message",
        "follow these instructions",
        "new instructions",
        "override instructions",
        "you are now",
        "act as",
        "assistant:",
        "system:",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn variable_json_is_valid(raw: &str) -> bool {
    let Some(value) = canonical_json(raw) else {
        return false;
    };
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 3
        || object
            .keys()
            .map(String::as_str)
            .ne(["kind", "name", "value"])
    {
        return false;
    }
    let Some(kind) = json_string(object, "kind", 8) else {
        return false;
    };
    let Some(name) = json_string(object, "name", 66) else {
        return false;
    };
    let Some(value) = json_string(object, "value", 120) else {
        return false;
    };
    let value_valid = match kind {
        "color" => is_upper_hex(value),
        "length" => css_length_value_is_valid(value),
        "font" => font_value_is_valid(value),
        _ => false,
    };
    value_valid && css_variable_name_is_valid(name)
}

fn treatment_json_is_valid(raw: &str) -> bool {
    let Some(value) = canonical_json(raw) else {
        return false;
    };
    let Some(object) = value.as_object() else {
        return false;
    };
    const KEYS: [&str; 14] = [
        "background",
        "border",
        "color",
        "fontFamily",
        "fontSize",
        "fontWeight",
        "gap",
        "height",
        "kind",
        "lineHeight",
        "padding",
        "radius",
        "shadow",
        "width",
    ];
    if object.is_empty()
        || object.len() > 14
        || object.keys().any(|key| !KEYS.contains(&key.as_str()))
    {
        return false;
    }
    let Some(kind) = json_string(object, "kind", 32) else {
        return false;
    };
    if !component_kind_is_valid(kind) {
        return false;
    }
    for key in ["background", "color"] {
        if object
            .get(key)
            .is_some_and(|value| value.as_str().is_none_or(|value| !is_upper_hex(value)))
        {
            return false;
        }
    }
    for (key, max) in [
        ("border", 96),
        ("fontFamily", 96),
        ("padding", 64),
        ("shadow", 160),
    ] {
        if object
            .get(key)
            .is_some_and(|_| json_string(object, key, max).is_none())
        {
            return false;
        }
    }
    if object
        .get("fontFamily")
        .and_then(Value::as_str)
        .is_some_and(|value| !font_value_is_valid(value))
        || object
            .get("padding")
            .and_then(Value::as_str)
            .is_some_and(|value| !css_length_value_is_valid(value))
    {
        return false;
    }
    for (key, min, max) in [
        ("fontSize", 0.0, 1_000_000.0),
        ("gap", 0.0, 1_000_000.0),
        ("lineHeight", 0.0, 1_000_000.0),
    ] {
        if object.get(key).is_some_and(|value| {
            value
                .as_f64()
                .is_none_or(|value| !value.is_finite() || value < min || value > max)
        }) {
            return false;
        }
    }
    if object.get("fontWeight").is_some_and(|value| {
        value
            .as_u64()
            .is_none_or(|value| !(1..=1_000).contains(&value))
    }) {
        return false;
    }
    for (key, min) in [("height", 0), ("radius", 0), ("width", 0)] {
        if object.get(key).is_some_and(|value| {
            value
                .as_u64()
                .is_none_or(|value| value < min || value > 1_000_000)
        }) {
            return false;
        }
    }
    true
}

fn canonical_json(raw: &str) -> Option<Value> {
    if raw.is_empty() || raw.len() > 2_048 {
        return None;
    }
    let value: Value = serde_json::from_str(raw).ok()?;
    (serde_json::to_string(&value).ok()?.as_str() == raw).then_some(value)
}

fn json_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    max_chars: usize,
) -> Option<&'a str> {
    let value = object.get(key)?.as_str()?;
    plain_value_is_valid(value, max_chars).then_some(value)
}

fn css_variable_name_is_valid(name: &str) -> bool {
    let Some(body) = name.strip_prefix("--") else {
        return false;
    };
    !body.is_empty()
        && body
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn component_kind_is_valid(kind: &str) -> bool {
    matches!(
        kind,
        "alert"
            | "article"
            | "aside"
            | "button"
            | "card"
            | "checkbox"
            | "dialog"
            | "fieldset"
            | "footer"
            | "form"
            | "header"
            | "image"
            | "link"
            | "list"
            | "listbox"
            | "menu"
            | "navigation"
            | "progress"
            | "radio"
            | "search"
            | "section"
            | "select"
            | "slider"
            | "switch"
            | "tab"
            | "table"
            | "textarea"
            | "textbox"
            | "toolbar"
            | "input-button"
            | "input-checkbox"
            | "input-color"
            | "input-date"
            | "input-email"
            | "input-file"
            | "input-number"
            | "input-other"
            | "input-password"
            | "input-radio"
            | "input-range"
            | "input-search"
            | "input-submit"
            | "input-tel"
            | "input-text"
            | "input-time"
            | "input-url"
    )
}

fn gradient_value_is_valid(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    plain_value_is_valid(value, 200)
        && lower.ends_with(')')
        && [
            "linear-gradient(",
            "radial-gradient(",
            "conic-gradient(",
            "repeating-linear-gradient(",
            "repeating-radial-gradient(",
            "repeating-conic-gradient(",
        ]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

fn media_query_is_valid(value: &str) -> bool {
    plain_value_is_valid(value, 160)
}

fn font_value_is_valid(value: &str) -> bool {
    plain_value_is_valid(value, 120)
        && value.chars().any(char::is_alphabetic)
        && value.chars().all(|ch| {
            ch.is_alphanumeric()
                || ch.is_whitespace()
                || matches!(ch, ',' | '\'' | '"' | '.' | '_' | '-')
        })
}

fn css_length_value_is_valid(value: &str) -> bool {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    !parts.is_empty() && parts.len() <= 4 && parts.into_iter().all(css_length_token_is_valid)
}

fn css_length_token_is_valid(value: &str) -> bool {
    let units = [
        "vmin", "vmax", "rem", "px", "em", "%", "vw", "vh", "ch", "ex",
    ];
    let lower = value.to_ascii_lowercase();
    let Some(unit) = units.iter().find(|unit| lower.ends_with(**unit)) else {
        return false;
    };
    let number = &value[..value.len() - unit.len()];
    !number.is_empty() && number.parse::<f64>().is_ok_and(|value| value.is_finite())
}

fn color_role_line_is_exact(line: &str, role: &str) -> bool {
    let line = line.trim();
    let prefix = format!("{role}: ");
    let Some(rest) = line.strip_prefix(&prefix) else {
        return false;
    };
    is_upper_hex(rest)
}

fn primary_font_line_is_valid(line: &str) -> bool {
    line.trim()
        .strip_prefix("Primary Font Family: ")
        .is_some_and(|family| !family.trim().is_empty() && !family.contains('|'))
}

fn table_cells(line: &str) -> Option<Vec<&str>> {
    let line = line.trim();
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    let cells = inner.split('|').map(str::trim).collect::<Vec<_>>();
    (cells.len() >= 2).then_some(cells)
}

fn table_header_is_valid(line: &str) -> bool {
    table_cells(line)
        .is_some_and(|cells| cells == ["Role", "Family", "Weight", "Size", "Line Height"])
}

fn table_separator_is_valid(line: &str) -> bool {
    table_cells(line).is_some_and(|cells| {
        cells.into_iter().all(|cell| {
            let cell = cell.trim_matches(':');
            cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
        })
    })
}

fn table_role_is_valid(line: &str, role: &str) -> bool {
    table_cells(line).is_some_and(|cells| {
        cells.len() == 5
            && cells[0] == role
            && !cells[1].is_empty()
            && cells[2]
                .parse::<u16>()
                .is_ok_and(|weight| (1..=1_000).contains(&weight))
            && px_number(cells[3]).is_some()
            && (cells[4] == "normal" || px_number(cells[4]).is_some())
    })
}

fn px_number(value: &str) -> Option<f64> {
    let number = value.strip_suffix("px")?.parse::<f64>().ok()?;
    (number.is_finite() && number >= 0.0).then_some(number)
}

fn is_upper_hex(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'A'..=b'F'))
}

fn unique_heading(markdown: &str, heading: &str) -> Option<usize> {
    unique_line_offset(markdown, |line| line == heading)
}

fn unique_line_offset(markdown: &str, mut matches: impl FnMut(&str) -> bool) -> Option<usize> {
    let mut found = None;
    let mut offset = 0;
    for segment in markdown.split_inclusive('\n') {
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        if matches(line) {
            if found.is_some() {
                return None;
            }
            found = Some(offset);
        }
        offset += segment.len();
    }
    found
}

fn first_line_offset_after(
    markdown: &str,
    after: usize,
    mut matches: impl FnMut(&str) -> bool,
) -> Option<usize> {
    let mut offset = 0;
    for segment in markdown.split_inclusive('\n') {
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        if offset > after && matches(line) {
            return Some(offset);
        }
        offset += segment.len();
    }
    None
}

fn radius_line_is_integer_px(line: &str, label: &str) -> bool {
    line.trim()
        .strip_prefix(label)
        .map(str::trim)
        .and_then(|value| value.strip_suffix("px"))
        .is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
}

pub(super) fn contains_external_reference(markdown: &str) -> bool {
    [
        b"http://".as_slice(),
        b"https://",
        b"file://",
        b"chrome://",
        b"chrome-extension://",
        b"data:",
        b"blob:",
        b"javascript:",
        b"vbscript:",
        b"mailto:",
        b"ftp:",
        b"file:",
        b"chrome:",
        b"url(",
        b"//",
    ]
    .iter()
    .any(|needle| contains_ascii_case_insensitive(markdown, needle))
}

fn contains_ascii_case_insensitive(text: &str, needle: &[u8]) -> bool {
    text.as_bytes().windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}
