//! Structured style-guide value extraction — port of
//! `style-guide-parser.ts`. Pulls colours / fonts / corner radii out
//! of a style guide's `## Color System` / `## Typography` /
//! `## Corner Radius` sections so two guides can be diffed for
//! style-switching (see [`crate::style_guide::mapping`]).

use std::collections::HashMap;

/// Colour channel of [`StyleGuideValues`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleColors {
    pub background: Option<String>,
    pub surface: Option<String>,
    pub accent: Option<String>,
    pub text_primary: Option<String>,
    pub text_secondary: Option<String>,
    pub text_muted: Option<String>,
    pub border: Option<String>,
}

/// Typography channel of [`StyleGuideValues`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleTypography {
    pub display_font: Option<String>,
    pub body_font: Option<String>,
    pub data_font: Option<String>,
}

/// Corner-radius channel of [`StyleGuideValues`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleRadius {
    pub card: Option<i64>,
    pub button: Option<i64>,
}

/// Structured values extracted from a style guide's markdown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleGuideValues {
    pub colors: StyleColors,
    pub typography: StyleTypography,
    pub radius: StyleRadius,
}

/// Split markdown into `## `-delimited sections, keyed by lowercased
/// heading.
fn get_sections(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current: Option<(String, String)> = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            if let Some((h, b)) = current.take() {
                map.insert(h, b);
            }
            current = Some((rest.trim().to_lowercase(), String::new()));
        } else if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some((h, b)) = current {
        map.insert(h, b);
    }
    map
}

/// The first `#RRGGBB` colour on a line (uppercased). A 7th hex digit
/// disqualifies the match (TS `\b` after `{6}`).
fn first_hex(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let n = bytes.len();
    for i in 0..n {
        if bytes[i] != b'#' || i + 7 > n {
            continue;
        }
        let hex = &bytes[i + 1..i + 7];
        if !hex.iter().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let after_ok = i + 7 >= n || {
            let c = bytes[i + 7];
            !(c.is_ascii_alphanumeric() || c == b'_')
        };
        if after_ok {
            return Some(format!("#{}", line[i + 1..i + 7].to_uppercase()));
        }
    }
    None
}

/// Lowercase a line and collapse whitespace runs — the form label
/// matchers expect.
fn normalize(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// First hex colour on a line whose normalized form satisfies `label`.
fn hex_near(text: &str, label: impl Fn(&str) -> bool) -> Option<String> {
    for line in text.lines() {
        if label(&normalize(line)) {
            if let Some(hex) = first_hex(line) {
                return Some(hex);
            }
        }
    }
    None
}

/// True when `norm` contains `a` then `b` (in that order) — the TS
/// `a.*b` regex form.
fn ordered(norm: &str, a: &str, b: &str) -> bool {
    match (norm.find(a), norm.rfind(b)) {
        (Some(i), Some(j)) => i < j,
        _ => false,
    }
}

fn extract_colors(sections: &HashMap<String, String>) -> StyleColors {
    let empty = String::new();
    let color_section = sections
        .get("color system")
        .unwrap_or(&empty)
        .to_lowercase();

    let background = hex_near(&color_section, |l| {
        l.contains("page background") || ordered(l, "root", "background")
    });
    let surface = hex_near(&color_section, |l| {
        l.contains("card surface") || l.contains("surface")
    });
    let border = hex_near(&color_section, |l| {
        l.contains("default border") || l.contains("border") || l.contains("divider")
    });

    // Accent: prefer the "Accent Colors" subsection.
    let accent_block = match color_section.find("accent color") {
        Some(idx) => &color_section[idx..],
        None => &color_section,
    };
    let accent = hex_near(accent_block, |l| {
        l.contains("primary accent") || l.contains("accent")
    });

    // Text colours: prefer the "Text Colors" subsection.
    let text_block = match color_section.find("text color") {
        Some(idx) => &color_section[idx..],
        None => &color_section,
    };
    let text_primary = hex_near(text_block, |l| l.contains("primary text"));
    let text_secondary = hex_near(text_block, |l| l.contains("secondary text"));
    let text_muted = hex_near(text_block, |l| {
        l.contains("muted text") || l.contains("tertiary text")
    });

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

fn extract_typography(sections: &HashMap<String, String>) -> StyleTypography {
    let empty = String::new();
    let typo_section = sections.get("typography").unwrap_or(&empty);
    let lower = typo_section.to_lowercase();
    let Some(fam_idx) = lower.find("font families") else {
        return StyleTypography::default();
    };
    let fam_block = &typo_section[fam_idx..];
    // Stop at the next `###` heading (searching past the heading itself).
    let family_text = match fam_block.get(10..).and_then(|s| s.find("\n###")) {
        Some(rel) => &fam_block[..rel + 10],
        None => fam_block,
    };

    // Parse `| Role | Family | Usage |` table rows.
    let mut rows: Vec<(String, String)> = Vec::new();
    for line in family_text.lines() {
        let cells: Vec<String> = line
            .split('|')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let role = cells[0].to_lowercase();
        if role == "role" || role.starts_with('-') {
            continue;
        }
        rows.push((role, cells[1].clone()));
    }

    let mut display_font: Option<String> = None;
    let mut body_font: Option<String> = None;
    let mut data_font: Option<String> = None;
    let has = |role: &str, words: &[&str]| words.iter().any(|w| role.contains(w));
    for (role, family) in &rows {
        if display_font.is_none() && has(role, &["display", "logo", "heading", "serif"]) {
            display_font = Some(family.clone());
        } else if body_font.is_none() && has(role, &["body", "functional", "ui"]) {
            body_font = Some(family.clone());
        } else if data_font.is_none() && has(role, &["data", "mono"]) {
            data_font = Some(family.clone());
        }
    }
    // Positional fallback when role keywords didn't match.
    if display_font.is_none() {
        display_font = rows.first().map(|r| r.1.clone());
    }
    if body_font.is_none() {
        body_font = rows.get(1).map(|r| r.1.clone());
    }
    if data_font.is_none() {
        data_font = rows.get(2).map(|r| r.1.clone());
    }

    StyleTypography {
        display_font,
        body_font,
        data_font,
    }
}

/// First `<digits>px` integer on a line.
fn px_number(line: &str) -> Option<i64> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if line[i..].starts_with("px") {
                return line[start..i].parse().ok();
            }
        } else {
            i += 1;
        }
    }
    None
}

fn extract_radius(sections: &HashMap<String, String>) -> StyleRadius {
    let empty = String::new();
    let radius_section = sections.get("corner radius").unwrap_or(&empty);
    if radius_section.is_empty() {
        return StyleRadius::default();
    }

    let mut card: Option<i64> = None;
    let mut button: Option<i64> = None;
    for line in radius_section.lines() {
        let Some(value) = px_number(line) else {
            continue;
        };
        let lower = line.to_lowercase();
        if card.is_none()
            && (lower.contains("card")
                || lower.contains("standard")
                || lower.contains("container")
                || ordered(&lower, "primary", "radius"))
        {
            card = Some(value);
        }
        if button.is_none()
            && (lower.contains("button") || lower.contains("input") || lower.contains("small"))
        {
            button = Some(value);
        }
    }

    // Brutalist "Everything: 0px" fallback.
    if card.is_none() && button.is_none() {
        for line in radius_section.lines() {
            if line.to_lowercase().contains("everything") {
                if let Some(v) = px_number(line) {
                    card = Some(v);
                    button = Some(v);
                    break;
                }
            }
        }
    }

    StyleRadius { card, button }
}

/// Extract structured colour / typography / radius values from a
/// style guide's markdown content.
pub fn extract_style_guide_values(content: &str) -> StyleGuideValues {
    let sections = get_sections(content);
    StyleGuideValues {
        colors: extract_colors(&sections),
        typography: extract_typography(&sections),
        radius: extract_radius(&sections),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
## Color System

### Background Colors
Page Background: #0A0A0F
Card Surface: #16161D

### Accent Colors
Primary Accent: #6E56CF

### Text Colors
Primary Text: #FFFFFF
Secondary Text: #A1A1AA
Muted Text: #52525B

### Borders
Default Border: #27272A

## Typography

### Font Families
| Role | Family | Usage |
| ---- | ------ | ----- |
| Display | Playfair Display | headings |
| Body | Inter | UI text |
| Data | JetBrains Mono | numerics |

## Corner Radius
Card / Standard: 12px
Button / Input: 8px
";

    #[test]
    fn extracts_colors() {
        let v = extract_style_guide_values(SAMPLE);
        assert_eq!(v.colors.background.as_deref(), Some("#0A0A0F"));
        assert_eq!(v.colors.surface.as_deref(), Some("#16161D"));
        assert_eq!(v.colors.accent.as_deref(), Some("#6E56CF"));
        assert_eq!(v.colors.text_primary.as_deref(), Some("#FFFFFF"));
        assert_eq!(v.colors.text_secondary.as_deref(), Some("#A1A1AA"));
        assert_eq!(v.colors.text_muted.as_deref(), Some("#52525B"));
        assert_eq!(v.colors.border.as_deref(), Some("#27272A"));
    }

    #[test]
    fn extracts_typography_by_role() {
        let v = extract_style_guide_values(SAMPLE);
        assert_eq!(
            v.typography.display_font.as_deref(),
            Some("Playfair Display")
        );
        assert_eq!(v.typography.body_font.as_deref(), Some("Inter"));
        assert_eq!(v.typography.data_font.as_deref(), Some("JetBrains Mono"));
    }

    #[test]
    fn extracts_radius() {
        let v = extract_style_guide_values(SAMPLE);
        assert_eq!(v.radius.card, Some(12));
        assert_eq!(v.radius.button, Some(8));
    }

    #[test]
    fn first_hex_rejects_seven_digit_runs() {
        assert_eq!(first_hex("color #abcdef here"), Some("#ABCDEF".to_string()));
        // A 7th hex digit breaks the word boundary.
        assert_eq!(first_hex("color #abcdef0 here"), None);
        assert_eq!(first_hex("no color here"), None);
    }

    #[test]
    fn brutalist_everything_radius_fallback() {
        let md = "## Corner Radius\nEverything: 0px (sharp corners)\n";
        let v = extract_style_guide_values(md);
        assert_eq!(v.radius.card, Some(0));
        assert_eq!(v.radius.button, Some(0));
    }
}
