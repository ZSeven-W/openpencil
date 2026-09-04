//! Canonical Markdown rendering for bounded browser design evidence.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use crate::design_md::{
    ComponentEvidence, ComponentSample, CountedText, Evidence, RadiusEvidence, SpacingEvidence,
    TypographyEvidence,
};
use crate::design_md_palette::{opaque_hex, relative_luminance, select_palette, PaletteColor};

pub(super) fn render(evidence: &Evidence) -> String {
    let title = if evidence.title.trim().is_empty() {
        "Extracted Web Page".to_owned()
    } else {
        evidence.title.clone()
    };
    let background = evidence
        .page_background
        .as_deref()
        .map(|value| opaque_hex(value, "#FFFFFF"))
        .unwrap_or_else(|| "#FFFFFF".to_owned());
    let scheme = evidence
        .color_scheme
        .as_deref()
        .filter(|value| matches!(*value, "light" | "dark"))
        .unwrap_or_else(|| {
            if relative_luminance(&background) < 0.35 {
                "dark"
            } else {
                "light"
            }
        });
    let mut out = String::new();
    let palette = select_palette(evidence, &background);
    let _ = writeln!(out, "---");
    let _ = writeln!(out, "name: '{}'", yaml_quote(&slugify(&title)));
    let _ = writeln!(out, "tags: [extracted, web, {scheme}-mode]");
    let _ = writeln!(out, "platform: webapp");
    let _ = writeln!(out, "---\n");
    let _ = writeln!(out, "# Design System: {}\n", markdown_text(&title));

    render_summary(&mut out, evidence, scheme, &palette);
    render_colors(&mut out, &palette);
    render_typography(&mut out, &evidence.typography);
    render_spacing(&mut out, &evidence.spacing);
    render_radii(&mut out, &evidence.radii, &evidence.components);
    render_effects(&mut out, &evidence.shadows, &evidence.gradients);
    render_components(&mut out, &evidence.components);
    render_variables(&mut out, evidence);
    render_layout(&mut out, evidence);
    out
}

fn render_summary(out: &mut String, evidence: &Evidence, scheme: &str, palette: &[PaletteColor]) {
    let _ = writeln!(out, "## Style Summary\n");
    let _ = writeln!(
        out,
        "A deterministic extraction of the rendered page's reusable visual system. The evidence covers {} rendered elements at a {} × {} {}-mode viewport.\n",
        evidence.element_count, evidence.viewport_width, evidence.viewport_height, scheme
    );
    let key = [0, 1, 2, 3, 6]
        .into_iter()
        .filter_map(|index| palette.get(index))
        .map(|color| color.hex.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "Key palette: {key}\n");
    if evidence.truncated {
        let _ = writeln!(
            out,
            "> The source page exceeded the extractor's evidence budget. Treat uncommon styles as provisional; the dominant tokens below are complete.\n"
        );
    }
}

fn render_colors(out: &mut String, palette: &[PaletteColor]) {
    let _ = writeln!(out, "## Color System\n");
    let _ = writeln!(out, "| Token | Value | Usage |");
    let _ = writeln!(out, "| --- | --- | --- |");
    for color in palette {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            color.role, color.hex, color.description
        );
    }
    let _ = writeln!(out);
}

fn render_typography(out: &mut String, values: &[TypographyEvidence]) {
    let mut values = values.to_vec();
    values.sort_by(|a, b| {
        role_rank(&a.role)
            .cmp(&role_rank(&b.role))
            .then_with(|| compare_f64(b.size, a.size))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.family.cmp(&b.family))
            .then_with(|| a.weight.cmp(&b.weight))
            .then_with(|| compare_optional_f64(a.line_height, b.line_height))
    });

    let headings = values
        .iter()
        .find(|value| matches!(value.role.as_str(), "display" | "heading"))
        .or_else(|| values.first());
    let body = values
        .iter()
        .find(|value| matches!(value.role.as_str(), "body" | "label" | "control"))
        .or(headings);
    let heading_family = headings
        .map(|value| value.family.as_str())
        .unwrap_or("System UI");
    let body_family = body
        .map(|value| value.family.as_str())
        .unwrap_or(heading_family);
    let heading_count = headings.map(|value| value.count).unwrap_or(0);
    let body_count = body.map(|value| value.count).unwrap_or(0);
    let _ = writeln!(out, "## Typography\n");
    let _ = writeln!(
        out,
        "Primary Font Family: {}\n",
        markdown_cell(heading_family)
    );
    let _ = writeln!(out, "### Font Families\n");
    let _ = writeln!(out, "| Role | Family | Usage |");
    let _ = writeln!(out, "| --- | --- | --- |");
    let _ = writeln!(
        out,
        "| Headings | {} | {} observed display/heading styles |",
        markdown_cell(heading_family),
        heading_count
    );
    let _ = writeln!(
        out,
        "| Body / Functional | {} | {} observed body/control styles |",
        markdown_cell(body_family),
        body_count
    );
    if let Some(code) = values.iter().find(|value| value.role == "code") {
        let _ = writeln!(
            out,
            "| Data / Code | {} | {} observed code/data styles |",
            markdown_cell(&code.family),
            code.count
        );
    }

    let _ = writeln!(out, "\n### Type Scale\n");
    let _ = writeln!(
        out,
        "| Level | Size | Font | Weight | Line Height | Usage |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
    for value in values.iter().take(12) {
        let line_height = value
            .line_height
            .map(fmt_px)
            .unwrap_or_else(|| "normal".to_owned());
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} samples |",
            role_label(&value.role),
            fmt_px(value.size),
            markdown_cell(&value.family),
            value.weight,
            line_height,
            value.count
        );
    }
    if values.is_empty() {
        let _ = writeln!(
            out,
            "| Body | 16px | System UI | 400 | normal | Browser fallback |",
        );
    }
    let _ = writeln!(out);
}

fn render_spacing(out: &mut String, values: &[SpacingEvidence]) {
    let mut values = values
        .iter()
        .filter(|value| value.value >= 0.0)
        .cloned()
        .collect::<Vec<_>>();
    values.sort_by(|a, b| {
        compare_f64(a.value, b.value)
            .then_with(|| property_rank(&a.property).cmp(&property_rank(&b.property)))
            .then_with(|| b.count.cmp(&a.count))
    });
    let _ = writeln!(out, "## Spacing System\n");
    let _ = writeln!(out, "| Value | Property | Evidence |");
    let _ = writeln!(out, "| --- | --- | --- |");
    let mut seen = BTreeSet::new();
    for value in values {
        let key = (fmt_number(value.value), value.property.clone());
        if !seen.insert(key.clone()) {
            continue;
        }
        let _ = writeln!(
            out,
            "| {}px | {} | {} occurrences |",
            key.0,
            title_case(&key.1),
            value.count
        );
        if seen.len() == 12 {
            break;
        }
    }
    if seen.is_empty() {
        let _ = writeln!(out, "| 0px | Gap | No stable spacing token observed |",);
    }
    let _ = writeln!(out);
}

fn render_radii(out: &mut String, values: &[RadiusEvidence], components: &[ComponentEvidence]) {
    let mut by_radius = BTreeMap::<u32, u64>::new();
    for value in values {
        *by_radius.entry(value.value).or_default() += value.count;
    }
    let generic = radius_mode(&by_radius).unwrap_or(0);
    let card = component_radius_mode(components, is_card_like).unwrap_or(generic);
    let button = component_radius_mode(components, is_control_like).unwrap_or(generic);
    let _ = writeln!(out, "## Corner Radius\n");
    let _ = writeln!(out, "Card / Standard: {card}px");
    let _ = writeln!(out, "Button / Input: {button}px\n");
    let _ = writeln!(out, "| Role | Value | Usage |");
    let _ = writeln!(out, "| --- | --- | --- |");
    let _ = writeln!(
        out,
        "| Card / Standard | {card}px | Primary container radius |"
    );
    let _ = writeln!(
        out,
        "| Button / Input | {button}px | Primary interactive radius |"
    );
    for (radius, count) in by_radius.into_iter().take(12) {
        if radius == card || radius == button {
            continue;
        }
        let usage = if radius == 0 {
            format!("Square corners observed {count} times")
        } else if radius >= 9_999 {
            format!("Pills and circular controls observed {count} times")
        } else {
            format!("Rendered components observed {count} times")
        };
        let _ = writeln!(out, "| Observed | {radius}px | {usage} |");
    }
    let _ = writeln!(out);
}

fn component_radius_mode(
    components: &[ComponentEvidence],
    matches: fn(&str) -> bool,
) -> Option<u32> {
    let mut counts = BTreeMap::<u32, u64>::new();
    for component in components {
        if !matches(&component.kind) {
            continue;
        }
        for radius in component.samples.iter().filter_map(|sample| sample.radius) {
            *counts.entry(radius).or_default() += component.count;
        }
    }
    radius_mode(&counts)
}

fn radius_mode(counts: &BTreeMap<u32, u64>) -> Option<u32> {
    let mut values = counts
        .iter()
        .filter(|(radius, _)| **radius < 9_999)
        .collect::<Vec<_>>();
    values.sort_by(|(radius_a, count_a), (radius_b, count_b)| {
        count_b.cmp(count_a).then_with(|| radius_a.cmp(radius_b))
    });
    values.first().map(|(radius, _)| **radius)
}

fn is_card_like(kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    [
        "card",
        "panel",
        "modal",
        "dialog",
        "popover",
        "container",
        "article",
        "section",
        "aside",
        "fieldset",
        "form",
        "table",
        "menu",
        "alert",
    ]
    .iter()
    .any(|token| kind.contains(token))
}

fn is_control_like(kind: &str) -> bool {
    let kind = kind.to_ascii_lowercase();
    [
        "button", "input", "select", "textarea", "control", "tab", "chip", "badge", "checkbox",
        "radio", "switch", "slider", "textbox", "listbox", "search", "link",
    ]
    .iter()
    .any(|token| kind.contains(token))
}

fn render_effects(out: &mut String, shadows: &[CountedText], gradients: &[CountedText]) {
    let _ = writeln!(out, "## Effects\n");
    let _ = writeln!(out, "### Shadows\n");
    render_counted_text_table(out, shadows, 8);
    let _ = writeln!(out, "\n### Gradients\n");
    render_counted_text_table(out, gradients, 8);
    let _ = writeln!(out);
}

fn render_counted_text_table(out: &mut String, values: &[CountedText], max: usize) {
    let _ = writeln!(out, "| Value | Evidence |");
    let _ = writeln!(out, "| --- | --- |");
    let mut values = values.to_vec();
    values.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
    for value in values.iter().take(max) {
        let _ = writeln!(
            out,
            "| {} | {} occurrences |",
            markdown_cell(&value.value),
            value.count
        );
    }
    if values.is_empty() {
        let _ = writeln!(out, "| None observed | 0 occurrences |",);
    }
}

fn render_components(out: &mut String, values: &[ComponentEvidence]) {
    let mut values = values
        .iter()
        .map(|value| {
            let mut treatments = value
                .samples
                .iter()
                .map(component_treatment)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            treatments.sort();
            let representative = treatments
                .first()
                .cloned()
                .unwrap_or_else(|| "Use the dominant page tokens".to_owned());
            let signature = treatments.join("\u{0}");
            (value, representative, signature)
        })
        .collect::<Vec<_>>();
    values.sort_by(|(a, _, signature_a), (b, _, signature_b)| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.kind.cmp(&b.kind))
            .then_with(|| signature_a.cmp(signature_b))
    });
    let _ = writeln!(out, "## Component Styles\n");
    let _ = writeln!(out, "| Component | Count | Representative treatment |");
    let _ = writeln!(out, "| --- | --- | --- |");
    for (value, treatment, _) in values.iter().take(16) {
        let _ = writeln!(
            out,
            "| {} | {} | {} |",
            markdown_cell(&title_case(&value.kind)),
            value.count,
            treatment
        );
    }
    if values.is_empty() {
        let _ = writeln!(
            out,
            "| Generic UI | 0 | No repeated component treatment observed |",
        );
    }
    let _ = writeln!(out);
}

fn render_variables(out: &mut String, evidence: &Evidence) {
    let _ = writeln!(out, "## CSS Variables\n");
    let _ = writeln!(out, "| Token | Value | Kind |");
    let _ = writeln!(out, "| --- | --- | --- |");
    let mut values = evidence.css_variables.clone();
    values.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.value.cmp(&b.value))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    for value in values.iter().take(24) {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} |",
            markdown_code(&value.name),
            markdown_cell(&value.value),
            title_case(&value.kind)
        );
    }
    if values.is_empty() {
        let _ = writeln!(out, "| None observed | — | — |",);
    }
    let _ = writeln!(out);
}

fn render_layout(out: &mut String, evidence: &Evidence) {
    let _ = writeln!(out, "## Layout Principles\n");
    let _ = writeln!(
        out,
        "- Reference viewport: {} × {} CSS px at {}× device pixel ratio.",
        evidence.viewport_width,
        evidence.viewport_height,
        fmt_number(evidence.viewport_dpr)
    );
    let _ = writeln!(
        out,
        "- Preserve the observed spacing scale and component density when extending the page."
    );
    if evidence.media_queries.is_empty() {
        let _ = writeln!(
            out,
            "- No explicit responsive media-query breakpoint was observed."
        );
    } else {
        let mut queries = evidence.media_queries.clone();
        queries.sort();
        queries.dedup();
        let _ = writeln!(out, "- Observed responsive conditions:");
        for query in queries.iter().take(12) {
            let _ = writeln!(out, "  - `{}`", markdown_code(query));
        }
    }
    let _ = writeln!(out);
}

fn component_treatment(sample: &ComponentSample) -> String {
    let mut parts = Vec::new();
    if let Some(value) = &sample.background {
        parts.push(format!("background {value}"));
    }
    if let Some(value) = &sample.color {
        parts.push(format!("text {value}"));
    }
    if let Some(value) = &sample.font_family {
        parts.push(format!("{} type", markdown_cell(value)));
    }
    match (sample.font_size, sample.font_weight) {
        (Some(size), Some(weight)) => parts.push(format!("{} / {weight}", fmt_px(size))),
        (Some(size), None) => parts.push(format!("{} type", fmt_px(size))),
        (None, Some(weight)) => parts.push(format!("weight {weight}")),
        (None, None) => {}
    }
    if let Some(value) = sample.line_height {
        parts.push(format!("{} line height", fmt_px(value)));
    }
    if let Some(value) = &sample.padding {
        parts.push(format!("padding {}", markdown_cell(value)));
    }
    if let Some(value) = sample.gap {
        parts.push(format!("{} gap", fmt_px(value)));
    }
    if let Some(value) = sample.radius {
        parts.push(format!("{value}px radius"));
    }
    if let Some(value) = &sample.border {
        parts.push(format!("border {}", markdown_cell(value)));
    }
    if let Some(value) = &sample.shadow {
        parts.push(format!("shadow {}", markdown_cell(value)));
    }
    if let (Some(width), Some(height)) = (sample.width, sample.height) {
        parts.push(format!("{width} × {height}px"));
    }
    parts.join(", ")
}

fn role_rank(role: &str) -> u8 {
    match role {
        "display" => 0,
        "heading" => 1,
        "body" => 2,
        "label" => 3,
        "control" => 4,
        "code" => 5,
        _ => 6,
    }
}

fn role_label(role: &str) -> &'static str {
    match role {
        "display" => "Display",
        "heading" => "Headings",
        "body" => "Body",
        "label" => "Labels",
        "control" => "Controls",
        "code" => "Data / Code",
        _ => "Text",
    }
}

fn property_rank(property: &str) -> u8 {
    match property {
        "gap" => 0,
        "padding" => 1,
        "margin" => 2,
        _ => 3,
    }
}

fn title_case(value: &str) -> String {
    value
        .split(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else {
            dash = true;
        }
        if out.chars().count() >= 48 {
            break;
        }
    }
    if out.is_empty() {
        "extracted-web-design".to_owned()
    } else {
        out
    }
}

fn yaml_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn markdown_text(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '!' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn markdown_cell(value: &str) -> String {
    markdown_text(value).replace('|', "\\|")
}

fn markdown_code(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('`', "&#96;")
}

fn fmt_px(value: f64) -> String {
    format!("{}px", fmt_number(value))
}

fn fmt_number(value: f64) -> String {
    if value.fract().abs() < 0.000_001 {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.2}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

fn compare_f64(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn compare_optional_f64(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => compare_f64(left, right),
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
