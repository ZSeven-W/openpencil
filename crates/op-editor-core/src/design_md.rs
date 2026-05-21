//! `design.md` markdown → structured [`DesignMdSpec`] parser.
//!
//! Hand-rolled (no `regex` dependency), ported from the TS
//! `pen-core/design-md-parser.ts`. A design.md is a free-form
//! markdown brief; this splits it into the structured sections the
//! Design-MD panel renders while keeping the original text in
//! [`DesignMdSpec::raw`] for round-trip fidelity.

use jian_ops_schema::{DesignMdColor, DesignMdSpec, DesignMdTypography};

/// Which structured field a `## ` section maps onto.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SectionKey {
    VisualTheme,
    ColorPalette,
    Typography,
    ComponentStyles,
    LayoutPrinciples,
    GenerationNotes,
    /// Unrecognised — folded into `component_styles` as a catch-all.
    Unknown,
}

/// Match a `## ` section title to a structured field. Case-insensitive
/// keyword test — mirrors the TS `matchSectionKey` fallback chain.
fn match_section_key(title: &str) -> SectionKey {
    let l = title.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| l.contains(n));
    if has(&["theme", "vibe", "aesthetic", "atmosphere", "mood"]) {
        SectionKey::VisualTheme
    } else if has(&["color", "colour", "palette"]) {
        SectionKey::ColorPalette
    } else if has(&["type", "font", "typo"]) {
        SectionKey::Typography
    } else if has(&["component", "style", "element", "button", "card", "input"]) {
        SectionKey::ComponentStyles
    } else if has(&["layout", "spacing", "grid", "whitespace"]) {
        SectionKey::LayoutPrinciples
    } else if has(&["note", "generation", "usage", "stitch", "prompting"]) {
        SectionKey::GenerationNotes
    } else {
        SectionKey::Unknown
    }
}

/// Strip an optional leading `N. ` numeric prefix from a section title.
fn strip_number_prefix(title: &str) -> &str {
    let t = title.trim_start();
    if let Some(dot) = t.find('.') {
        if t[..dot].chars().all(|c| c.is_ascii_digit()) && !t[..dot].is_empty() {
            return t[dot + 1..].trim_start();
        }
    }
    t
}

/// Split markdown into the H1 project name + the `## ` sections.
fn split_sections(markdown: &str) -> (Option<String>, Vec<(SectionKey, String)>) {
    let mut project_name: Option<String> = None;
    let mut sections: Vec<(SectionKey, String)> = Vec::new();
    let mut current: Option<(SectionKey, Vec<&str>)> = None;

    for line in markdown.lines() {
        if let Some(rest) = line.strip_prefix("# ").filter(|_| !line.starts_with("## ")) {
            if project_name.is_none() {
                let name = rest
                    .trim()
                    .strip_prefix("Design System:")
                    .unwrap_or(rest.trim());
                project_name = Some(name.trim().to_string());
            }
        } else if let Some(rest) = line.strip_prefix("## ") {
            if let Some((key, lines)) = current.take() {
                sections.push((key, lines.join("\n").trim().to_string()));
            }
            current = Some((match_section_key(strip_number_prefix(rest)), Vec::new()));
        } else if let Some((_, lines)) = current.as_mut() {
            lines.push(line);
        }
    }
    if let Some((key, lines)) = current.take() {
        sections.push((key, lines.join("\n").trim().to_string()));
    }
    (project_name, sections)
}

/// Index of the first `#RRGGBB` hex token in `line`, plus the hex.
fn find_hex(line: &str) -> Option<(usize, String)> {
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'#' {
            continue;
        }
        let hex = &bytes.get(i + 1..i + 7)?;
        if hex.iter().all(|c| c.is_ascii_hexdigit()) {
            // Reject a 7th hex digit so `#1234567` is not a colour.
            if bytes.get(i + 7).is_some_and(|c| c.is_ascii_hexdigit()) {
                continue;
            }
            let upper: String = (i..i + 7).map(|j| line[j..j + 1].to_uppercase()).collect();
            return Some((i, upper));
        }
    }
    None
}

/// Extract every named colour from a markdown block. A simplified
/// port — handles `**Name** (#hex) — role`, `- Name #hex: role`, and
/// the bare-hex fallback.
fn parse_colors(content: &str) -> Vec<DesignMdColor> {
    let mut colors = Vec::new();
    for line in content.lines() {
        let t = line.trim_start();
        if t.starts_with('{') || t.starts_with('`') {
            continue;
        }
        let Some((idx, hex)) = find_hex(line) else {
            continue;
        };
        let before = &line[..idx];
        let after = &line[(idx + 7).min(line.len())..];
        // Prefer a `**Bold**` label as the colour name.
        let name = bold_span(before).map(str::to_string).unwrap_or_else(|| {
            before
                .trim_end_matches(|c: char| "-*#():–— ".contains(c))
                .trim_start_matches(|c: char| "-* ".contains(c))
                .trim()
                .to_string()
        });
        let role = after
            .trim_start_matches(|c: char| ")(:–—- \t".contains(c))
            .trim()
            .to_string();
        colors.push(DesignMdColor {
            name: if name.is_empty() { hex.clone() } else { name },
            hex,
            role,
        });
    }
    colors
}

/// The content of the first `**bold**` span in `text`, if any.
fn bold_span(text: &str) -> Option<&str> {
    let start = text.find("**")? + 2;
    let end = text[start..].find("**")? + start;
    let inner = text[start..end].trim();
    (!inner.is_empty()).then_some(inner)
}

/// Extract typography guidance — keeps the whole block as `scale`.
fn parse_typography(content: &str) -> DesignMdTypography {
    let mut typo = DesignMdTypography {
        scale: Some(content.to_string()),
        ..DesignMdTypography::default()
    };
    for line in content.lines() {
        let l = line.to_lowercase();
        if (l.contains("font family") || l.contains("primary font")) && typo.font_family.is_none() {
            if let Some(colon) = line.find(':') {
                let v = line[colon + 1..].trim().trim_matches('*').trim();
                if !v.is_empty() {
                    typo.font_family = Some(v.to_string());
                }
            }
        }
    }
    typo
}

/// Join two catch-all blocks with a blank line.
fn append(existing: Option<String>, extra: String) -> String {
    match existing {
        Some(prev) if !prev.is_empty() => format!("{prev}\n\n{extra}"),
        _ => extra,
    }
}

/// Parse a design.md markdown string into a structured [`DesignMdSpec`].
/// The original markdown is preserved verbatim in `raw`.
pub fn parse_design_md(markdown: &str) -> DesignMdSpec {
    let mut spec = DesignMdSpec {
        raw: markdown.to_string(),
        project_name: None,
        visual_theme: None,
        color_palette: None,
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    };
    let (project_name, sections) = split_sections(markdown);
    spec.project_name = project_name;
    let had_sections = !sections.is_empty();

    for (key, content) in sections {
        match key {
            SectionKey::VisualTheme => spec.visual_theme = Some(content),
            SectionKey::ColorPalette => spec.color_palette = Some(parse_colors(&content)),
            SectionKey::Typography => spec.typography = Some(parse_typography(&content)),
            SectionKey::LayoutPrinciples => spec.layout_principles = Some(content),
            SectionKey::GenerationNotes => spec.generation_notes = Some(content),
            SectionKey::ComponentStyles | SectionKey::Unknown => {
                spec.component_styles = Some(append(spec.component_styles.take(), content));
            }
        }
    }

    // Fallbacks — mirror the TS parser so a section-less design.md
    // still yields something to render.
    if spec.color_palette.as_ref().is_none_or(|c| c.is_empty()) {
        let colors = parse_colors(markdown);
        if !colors.is_empty() {
            spec.color_palette = Some(colors);
        }
    }
    if spec.typography.is_none() {
        let typo = parse_typography(markdown);
        if typo.font_family.is_some() {
            spec.typography = Some(typo);
        }
    }
    // An empty palette reads as "no colours" — normalise to `None` so
    // the panel's content check stays a simple `is_some`.
    if spec.color_palette.as_ref().is_some_and(|c| c.is_empty()) {
        spec.color_palette = None;
    }
    if spec.visual_theme.is_none()
        && spec.color_palette.is_none()
        && spec.component_styles.is_none()
        && !had_sections
        && !markdown.trim().is_empty()
    {
        spec.visual_theme = Some(markdown.trim().to_string());
    }
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_project_name_and_sections() {
        let md = "# Design System: Aurora\n\n\
                  ## 1. Visual Theme\nCalm, minimal, airy.\n\n\
                  ## 2. Color Palette\n- **Primary** (#3366FF) — buttons\n\
                  - Accent #FF6633: highlights\n\n\
                  ## 3. Typography\nPrimary Font Family: Inter\n";
        let spec = parse_design_md(md);
        assert_eq!(spec.project_name.as_deref(), Some("Aurora"));
        assert_eq!(spec.visual_theme.as_deref(), Some("Calm, minimal, airy."));
        let colors = spec.color_palette.expect("colors");
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0].name, "Primary");
        assert_eq!(colors[0].hex, "#3366FF");
        assert_eq!(colors[0].role, "buttons");
        assert_eq!(colors[1].hex, "#FF6633");
        assert_eq!(
            spec.typography.and_then(|t| t.font_family).as_deref(),
            Some("Inter")
        );
    }

    #[test]
    fn section_less_markdown_falls_back_to_visual_theme() {
        let spec = parse_design_md("just some freeform notes");
        assert_eq!(
            spec.visual_theme.as_deref(),
            Some("just some freeform notes")
        );
        assert_eq!(spec.raw, "just some freeform notes");
    }

    #[test]
    fn blank_markdown_yields_no_sections() {
        // A whitespace-only import must not fabricate a visual-theme
        // section out of the empty body.
        let spec = parse_design_md("   \n\n  ");
        assert!(spec.visual_theme.is_none());
        assert!(spec.color_palette.is_none());
        assert!(spec.component_styles.is_none());
    }

    #[test]
    fn seven_hex_digits_are_not_a_color() {
        let spec = parse_design_md("## Colors\nref #12345678 not a color\n");
        assert!(spec.color_palette.is_none());
    }
}
