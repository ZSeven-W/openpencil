//! Token-table dialect: reading a community `DESIGN.md`'s values.
//!
//! [`super::parser`] reads the shipped corpus's grammar — `## Color System`
//! → `### Background Colors` → `Page Background: #0A0A0F`. Files from the
//! wider `DESIGN.md` ecosystem do not write that. They write token tables:
//!
//! ```text
//! ## Tokens — Colors
//! | Name | Value | Token | Role |
//! | Void Canvas | `#0a0a0a` | `--color-void-canvas` | Primary page background… |
//! ```
//!
//! Measured on a real imported guide, the corpus parser returned `None` for
//! all eight fields. That is worse than it sounds: at the summary model tiers
//! the sub-agent prompt lists the palette rather than the whole document, so
//! an empty extraction produced "use these EXACT hex colors" followed by
//! nothing at all — an instruction to obey an empty list.
//!
//! This module is a **gap filler, not a replacement**. It runs per channel and
//! only when the corpus parser found nothing in that channel, so a guide
//! written in the corpus grammar reaches exactly the values it always did —
//! locked by a test that runs both paths across the whole shipped registry.
//!
//! Its rule throughout: read what the file says, and where it says nothing,
//! leave the field empty rather than invent a value.

use super::parser::{StyleColors, StyleGuideValues, StyleRadius, StyleTypography};

/// Fill any channel the corpus parser left completely empty.
pub(super) fn fill_gaps(values: &mut StyleGuideValues, content: &str) {
    if values.colors == StyleColors::default() {
        values.colors = extract_table_colors(content);
    }
    if values.typography == StyleTypography::default() {
        values.typography = extract_heading_fonts(content);
    }
    if values.radius == StyleRadius::default() {
        values.radius = extract_prose_radius(content);
    }
}

/// A colour found in a guide, with whatever the document said about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteSample {
    /// `#RRGGBB`.
    pub color: String,
    /// The surrounding text, verbatim and trimmed — the author's own words
    /// for what this colour is, not a role we inferred.
    pub role: String,
}

/// Colours a guide mentions, in document order, for prompts that must show a
/// palette even when field extraction found nothing to fill.
///
/// This is the floor under the summary model tiers. Those tiers send a palette
/// list instead of the whole document, and an empty list under the sentence
/// "use these EXACT hex colors" is worse than saying nothing at all — it is an
/// instruction to obey nothing. Roles are passed through unclassified on
/// purpose: at this point the structured pass has already failed, and guessing
/// which colour is the background is exactly what would put the accent colour
/// behind the whole page.
pub fn sample_palette(content: &str, limit: usize) -> Vec<PaletteSample> {
    let mut out: Vec<PaletteSample> = Vec::new();
    for row in token_rows(content) {
        push_sample(&mut out, row.color, &row.description, limit);
        if out.len() >= limit {
            return out;
        }
    }
    if !out.is_empty() {
        return out;
    }
    // Not a table document: take the line the colour sits on as its role.
    // Every colour on the line, not just the first — a prose guide routinely
    // names its whole palette in one sentence, and stopping at the first would
    // send the model a one-colour palette for a file that listed six.
    for line in content.lines() {
        for color in all_colors(line) {
            push_sample(&mut out, color, line, limit);
        }
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn push_sample(out: &mut Vec<PaletteSample>, color: String, role: &str, limit: usize) {
    if out.len() >= limit || out.iter().any(|held| held.color == color) {
        return;
    }
    let role: String = role
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '|' || c == '`' || c == '-' || c == '#')
        .trim()
        .chars()
        .take(90)
        .collect();
    out.push(PaletteSample { color, role });
}

/// One parsed markdown table row.
struct TokenRow {
    /// Every cell but the value, lowercased and joined — the name, the CSS
    /// custom-property token, and the role description all describe what the
    /// colour is *for*, and different files put that information in different
    /// columns.
    description: String,
    /// The row's colour, normalized to `#RRGGBB`.
    color: String,
}

/// Colour rows from every markdown table in the document.
fn token_rows(content: &str) -> Vec<TokenRow> {
    let mut rows = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect();
        if cells.len() < 2 {
            continue;
        }
        // Separator row (`| --- | --- |`) and header row.
        if cells
            .iter()
            .all(|cell| cell.chars().all(|c| c == '-' || c == ':'))
        {
            continue;
        }
        let Some((value_index, color)) = cells
            .iter()
            .enumerate()
            .find_map(|(index, cell)| read_color(cell).map(|color| (index, color)))
        else {
            continue;
        };
        let description = cells
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != value_index)
            .map(|(_, cell)| cell.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        rows.push(TokenRow { description, color });
    }
    rows
}

/// A cell's colour: a hex literal, or the first `rgb()` / `rgba()` inside it.
///
/// The gradient case is why this is not just a hex scan. A guide's single
/// chromatic accent is routinely written as
/// `linear-gradient(90deg, …rgba(107, 98, 242, 0.565)…)`, and dropping it
/// costs exactly the colour that makes the style recognizable.
fn read_color(cell: &str) -> Option<String> {
    if let Some(hex) = first_hex_literal(cell) {
        return Some(hex);
    }
    first_rgb_functional(cell)
}

/// Every visible colour in `text`, in reading order.
///
/// Hex literals first, then functional colours, rather than interleaved by
/// position: the two scanners answer independently and the order within a
/// single line only decides which colour a prose palette lists first.
fn all_colors(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(hex) = first_hex_literal(rest) {
        // Advance past this match by finding where it started.
        let Some(at) = rest.find('#') else { break };
        push_unique(&mut out, hex);
        rest = &rest[at + 1..];
    }
    let mut rest = text;
    while let Some(color) = first_rgb_functional(rest) {
        let Some(at) = rest.to_lowercase().find(')') else {
            break;
        };
        push_unique(&mut out, color);
        rest = &rest[at + 1..];
    }
    out
}

fn push_unique(out: &mut Vec<String>, color: String) {
    if !out.contains(&color) {
        out.push(color);
    }
}

/// The first `#RGB` / `#RRGGBB` / `#RRGGBBAA` in `text`, as `#RRGGBB`.
fn first_hex_literal(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '#' {
            index += 1;
            continue;
        }
        let mut digits = 0;
        while index + 1 + digits < chars.len() && chars[index + 1 + digits].is_ascii_hexdigit() {
            digits += 1;
        }
        let bounded = !matches!(chars.get(index + 1 + digits), Some(c) if c.is_alphanumeric());
        if bounded && matches!(digits, 3 | 6 | 8) {
            let body: String = chars[index + 1..index + 1 + digits].iter().collect();
            return Some(expand_hex(&body));
        }
        index += 1 + digits.max(1);
    }
    None
}

/// `#abc` / `#rrggbbaa` → `#RRGGBB`. Alpha is dropped: the fields this feeds
/// are opaque fill colours.
fn expand_hex(body: &str) -> String {
    let rgb: String = if body.len() == 3 {
        body.chars().flat_map(|c| [c, c]).collect()
    } else {
        body.chars().take(6).collect()
    };
    format!("#{}", rgb.to_uppercase())
}

/// The first *visible* `rgb(...)` / `rgba(...)` in `text`, as `#RRGGBB`.
///
/// Fully transparent stops are skipped, and that is the whole reason this
/// scans rather than taking the first match. A gradient is routinely written
/// as `linear-gradient(90deg, rgba(0,0,0,0), rgba(107,98,242,.565) 50%,
/// rgba(0,0,0,0))` — it fades in from nothing — so the first `rgba()` in the
/// string is an invisible black stop. Taking it reported the signature violet
/// accent of a real imported guide as `#000000`.
fn first_rgb_functional(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let mut search_from = 0;
    while let Some(found) = lower[search_from..].find("rgb") {
        let start = search_from + found;
        search_from = start + 3;
        let (Some(open), Some(close)) = (
            lower[start..].find('(').map(|i| start + i),
            lower[start..].find(')').map(|i| start + i),
        ) else {
            continue;
        };
        if open >= close {
            continue;
        }
        let parts: Vec<f64> = lower[open + 1..close]
            .split([',', ' ', '/'])
            .filter(|part| !part.trim().is_empty())
            .filter_map(|part| part.trim().parse::<f64>().ok())
            .collect();
        if parts.len() < 3 {
            continue;
        }
        // A zero-alpha stop contributes no colour to look at.
        if parts.get(3).is_some_and(|alpha| *alpha <= 0.0) {
            continue;
        }
        let channel = |index: usize| parts[index].clamp(0.0, 255.0).round() as u8;
        return Some(format!(
            "#{:02X}{:02X}{:02X}",
            channel(0),
            channel(1),
            channel(2)
        ));
    }
    None
}

/// Assign token rows to the colour fields by what their role text says.
///
/// Fields claim rows in a fixed order and a claimed row is not offered again.
/// That ordering is load-bearing: a role reading "Primary page background,
/// base surface" describes one colour, and without claiming it would land in
/// both `background` and `surface`, painting cards the same colour as the page.
fn extract_table_colors(content: &str) -> StyleColors {
    let rows = token_rows(content);
    let mut claimed = vec![false; rows.len()];

    // Two tiers per field, and the tiers matter more than the keywords. A
    // table lists colours in the author's order, not ours, so a first-match
    // scan lets a row that merely *mentions* a word beat the row that says it
    // outright: on a real guide "Primary CTA fill, headline text" sat above
    // "Primary readable text" and took the primary-text field with it. Strong
    // cues are therefore scanned across every row before any weak cue is
    // considered.
    let mut claim = |tiers: &[&dyn Fn(&str) -> bool]| -> Option<String> {
        for predicate in tiers {
            for (index, row) in rows.iter().enumerate() {
                if !claimed[index] && predicate(&row.description) {
                    claimed[index] = true;
                    return Some(row.color.clone());
                }
            }
        }
        None
    };

    let background = claim(&[
        &|d: &str| {
            d.contains("page background") || d.contains("base surface") || d.contains("canvas")
        },
        &|d: &str| d.contains("背景") || (d.contains("background") && !d.contains("blur")),
    ]);
    // Deliberately not a bare "surface": the page background row above
    // routinely calls itself the base surface, and it has already been taken.
    let surface = claim(&[
        &|d: &str| {
            d.contains("elevated")
                || d.contains("panel")
                || d.contains("glass")
                || d.contains("sheet")
                || d.contains("modal")
                || d.contains("raised")
        },
        &|d: &str| d.contains("card"),
    ]);
    let accent = claim(&[
        &|d: &str| d.contains("accent") || d.contains("chromatic") || d.contains("强调"),
        &|d: &str| d.contains("brand") || d.contains("highlight"),
    ]);
    let text_primary = claim(&[
        &|d: &str| {
            d.contains("primary text")
                || d.contains("text primary")
                || d.contains("primary readable text")
                || d.contains("primary body text")
        },
        &|d: &str| {
            d.contains("headline") || d.contains("high-emphasis") || d.contains("high emphasis")
        },
    ]);
    let text_secondary = claim(&[
        &|d: &str| d.contains("secondary text") || d.contains("secondary body text"),
        &|d: &str| {
            d.contains("body text") || d.contains("mid-emphasis") || d.contains("mid emphasis")
        },
    ]);
    let text_muted = claim(&[
        &|d: &str| d.contains("muted") || d.contains("tertiary"),
        &|d: &str| {
            d.contains("disabled")
                || d.contains("placeholder")
                || d.contains("low-emphasis")
                || d.contains("low emphasis")
        },
    ]);
    // "Stroke" is deliberately absent: on a real guide an "Icon strokes, SVG
    // fill" row claimed the border field and reported the border colour as
    // pure black, when the document had a `Hairline` row saying "1px borders"
    // three rows further down.
    let border = claim(&[
        &|d: &str| d.contains("border") || d.contains("hairline"),
        &|d: &str| d.contains("divider") || d.contains("outline"),
    ]);

    StyleColors {
        background,
        surface,
        accent,
        text_primary,
        text_secondary,
        text_muted,
        border,
    }
}

/// Font families written as `### DM Sans — …` headings under a typography
/// section.
///
/// The corpus puts families in a `| Role | Family |` table; this dialect makes
/// the heading itself the family name and the reading order the roles, which
/// is how the ecosystem files are laid out.
fn extract_heading_fonts(content: &str) -> StyleTypography {
    let mut families: Vec<String> = Vec::new();
    let mut in_type_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            let lower = heading.to_lowercase();
            in_type_section = lower.contains("typograph")
                || lower.contains("font")
                || lower.contains("type scale")
                || lower.contains("字体");
            continue;
        }
        if !in_type_section {
            continue;
        }
        let Some(heading) = trimmed.strip_prefix("### ") else {
            continue;
        };
        // `### DM Sans — Interface` → `DM Sans`. The dash is the ecosystem's
        // convention for "family — what it is for".
        let family = heading
            .split(['—', '–', '|', ':'])
            .next()
            .unwrap_or(heading)
            .trim()
            .trim_matches('*')
            .trim();
        if family.is_empty() || family.len() > 48 {
            continue;
        }
        if !families.iter().any(|held| held == family) {
            families.push(family.to_string());
        }
    }

    let mono = families
        .iter()
        .find(|family| {
            let lower = family.to_lowercase();
            lower.contains("mono") || lower.contains("code")
        })
        .cloned();
    let mut proportional = families
        .iter()
        .filter(|family| Some(*family) != mono.as_ref());

    StyleTypography {
        display_font: proportional.next().cloned(),
        body_font: proportional.next().cloned().or_else(|| {
            // One family doing every job is a real and common choice; saying
            // so beats leaving the body font unstated.
            families.first().cloned().filter(|_| families.len() == 1)
        }),
        data_font: mono,
    }
}

/// Corner radii stated in prose — `24px card radii`, `9999px pills`.
///
/// Narrow on purpose: only lines that say they are about radius are read, so
/// a spacing scale or a font size elsewhere in the document cannot be mistaken
/// for one.
fn extract_prose_radius(content: &str) -> StyleRadius {
    let mut card: Option<i64> = None;
    let mut button: Option<i64> = None;
    for line in content.lines() {
        let lower = line.to_lowercase();
        if !(lower.contains("radius") || lower.contains("radii") || lower.contains("rounded")) {
            continue;
        }
        for (value, context) in px_values(&lower) {
            // A pill is a shape, not a card corner: `9999px` describes a fully
            // rounded control however the sentence around it reads.
            let is_pill = value >= 999;
            if button.is_none()
                && (is_pill
                    || context.contains("pill")
                    || context.contains("button")
                    || context.contains("control")
                    || context.contains("input")
                    || context.contains("ui"))
            {
                button = Some(value);
                continue;
            }
            if card.is_none()
                && !is_pill
                && (context.contains("card")
                    || context.contains("panel")
                    || context.contains("container")
                    || context.contains("modal")
                    || context.contains("surface"))
            {
                card = Some(value);
            }
        }
    }
    StyleRadius { card, button }
}

/// Every `<digits>px` on a line, each with the words that qualify it.
///
/// The context is per number, not per line, because a single sentence names
/// several radii at once — "9999px pill buttons, 10px UI radii, 24px card
/// radii". Classifying those against the whole line makes every keyword true
/// for every number, and the card radius came out as 10 instead of 24.
fn px_values(line: &str) -> Vec<(i64, String)> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if !line[index..].starts_with("px") {
            continue;
        }
        let Ok(value) = line[start..index].parse::<i64>() else {
            continue;
        };
        // Up to the next clause break — that is where this number's
        // description ends and the next one's begins.
        let tail = &line[index..];
        let end = tail.floor_char_boundary(
            tail.find([',', ';', '.', ')', '(', '—'])
                .unwrap_or(tail.len())
                .min(48),
        );
        out.push((value, tail[..end].to_string()));
    }
    out
}

#[cfg(test)]
#[path = "token_table_tests.rs"]
mod token_table_tests;
