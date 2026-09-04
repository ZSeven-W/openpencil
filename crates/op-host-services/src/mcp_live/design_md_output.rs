//! Strict output grammar and evidence-provenance validation for the
//! extension design.md job route.

use super::design_md_route::DesignMdResponseError;
use std::fmt::Write;

pub(super) fn validate_markdown(
    markdown: String,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> Result<String, DesignMdResponseError> {
    if markdown.len() > crate::design_md_evidence::MAX_DESIGN_MD_OUTPUT_BYTES {
        return Err(DesignMdResponseError::OutputTooLarge);
    }
    let markdown = markdown.trim().to_string();
    if markdown.is_empty() {
        return Err(DesignMdResponseError::EmptyOutput);
    }
    let first_line = markdown.lines().next().unwrap_or_default();
    let markdown_lower = markdown.to_ascii_lowercase();
    if first_line != "# Design System: Extracted Web Style"
        || markdown.lines().any(|line| {
            let fence = line.trim();
            fence.starts_with("```") || fence.starts_with("~~~")
        })
        || [
            "http://",
            "https://",
            "data:",
            "javascript:",
            "vbscript:",
            "mailto:",
            "ftp:",
            "//",
            "file:",
            "blob:",
            "chrome:",
            "chrome-extension:",
        ]
        .iter()
        .any(|needle| markdown_lower.contains(needle))
        || markdown.contains(['<', '>'])
        || markdown.contains("](")
        || markdown.contains("![")
        || markdown
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    let style_summary = "## Style Summary";
    if markdown
        .lines()
        .filter(|line| *line == style_summary)
        .count()
        != 1
    {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    let style_position = exact_line_position(&markdown, style_summary)
        .ok_or(DesignMdResponseError::InvalidOutput)?;
    let before_style = &markdown[first_line.len()..style_position];
    if before_style.lines().any(|line| !line.trim().is_empty()) {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    let color_position = exact_line_position(&markdown, "## Color System")
        .ok_or(DesignMdResponseError::InvalidOutput)?;
    let typography_position = exact_line_position(&markdown, "## Typography")
        .ok_or(DesignMdResponseError::InvalidOutput)?;
    let radius_position = exact_line_position(&markdown, "## Corner Radius")
        .ok_or(DesignMdResponseError::InvalidOutput)?;
    let h2s = h2_positions(&markdown);
    let required_h2s = [
        "## Style Summary",
        "## Color System",
        "## Typography",
        "## Corner Radius",
    ];
    if h2s.len() != required_h2s.len()
        || h2s
            .iter()
            .take(required_h2s.len())
            .map(|(_, heading)| *heading)
            .ne(required_h2s)
    {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    validate_style(&markdown[style_position..color_position], provenance)?;
    validate_colors(&markdown[color_position..typography_position], provenance)?;
    validate_typography(&markdown[typography_position..radius_position], provenance)?;
    validate_radii(&markdown[radius_position..], provenance)?;
    Ok(markdown)
}

pub(super) fn append_evidence_appendix(
    mut markdown: String,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> String {
    let appendix = &provenance.appendix;
    append_section(&mut markdown, "Spacing", &appendix.spacing, "");
    append_section(&mut markdown, "Shadows", &appendix.shadows, "Shadow: ");
    append_section(
        &mut markdown,
        "Gradients",
        &appendix.gradients,
        "Gradient: ",
    );
    append_section(
        &mut markdown,
        "CSS Variables",
        &appendix.css_variables,
        "Variable: ",
    );
    append_section(
        &mut markdown,
        "Components",
        &appendix.components,
        "Component: ",
    );
    append_section(
        &mut markdown,
        "Component Treatments",
        &appendix.treatments,
        "Treatment: ",
    );
    append_section(
        &mut markdown,
        "Responsive Behavior",
        &appendix.media_queries,
        "Media Query: ",
    );
    markdown
}

fn append_section(
    markdown: &mut String,
    heading: &str,
    values: &std::collections::BTreeSet<String>,
    prefix: &str,
) {
    if values.is_empty() {
        return;
    }
    let _ = write!(markdown, "\n\n## {heading}\n");
    for value in values {
        let _ = writeln!(markdown, "{prefix}{value}");
    }
    while markdown.ends_with('\n') {
        markdown.pop();
    }
}

fn validate_style(
    section: &str,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> Result<(), DesignMdResponseError> {
    let lines: Vec<&str> = content_lines(section).collect();
    if lines.len() != 1 {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    let palette = parse_key_palette(lines[0]).ok_or(DesignMdResponseError::InvalidOutput)?;
    if palette
        .iter()
        .any(|color| !provenance.colors.contains(color))
    {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    Ok(())
}

fn validate_colors(
    section: &str,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> Result<(), DesignMdResponseError> {
    let roles = [
        "Page Background",
        "Card Surface",
        "Primary Accent",
        "Primary Text",
        "Secondary Text",
        "Muted Text",
        "Default Border",
    ];
    let lines: Vec<&str> = content_lines(section).collect();
    if lines.len() != roles.len() {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    for (line, role) in lines.into_iter().zip(roles) {
        let Some(color) = parse_color_role_line(line, role) else {
            return Err(DesignMdResponseError::InvalidOutput);
        };
        if !provenance
            .role_colors
            .get(role)
            .is_some_and(|allowed| allowed.contains(&color))
        {
            return Err(DesignMdResponseError::InvalidOutput);
        }
    }
    Ok(())
}

fn validate_typography(
    section: &str,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> Result<(), DesignMdResponseError> {
    let lines: Vec<&str> = content_lines(section).collect();
    if lines.len() != 6 {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    let Some(primary_font) = parse_primary_font_line(lines[0]) else {
        return Err(DesignMdResponseError::InvalidOutput);
    };
    if !provenance.fonts.contains(&primary_font)
        || lines[1] != "### Font Families"
        || !font_table_header(lines[2])
        || !font_table_separator(lines[3])
        || !font_table_row_matches(lines[4], "Headings", provenance)
        || !font_table_row_matches(lines[5], "Body / Functional", provenance)
    {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    Ok(())
}

fn validate_radii(
    section: &str,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> Result<(), DesignMdResponseError> {
    let lines: Vec<&str> = content_lines(section).collect();
    if lines.len() != 2 {
        return Err(DesignMdResponseError::InvalidOutput);
    }
    for (line, label) in [
        (lines[0], "Card / Standard:"),
        (lines[1], "Button / Input:"),
    ] {
        let Some(radius) = parse_radius_line(line, label) else {
            return Err(DesignMdResponseError::InvalidOutput);
        };
        if !provenance.radii.contains(&radius) {
            return Err(DesignMdResponseError::InvalidOutput);
        }
    }
    Ok(())
}

fn exact_line_position(markdown: &str, target: &str) -> Option<usize> {
    let mut offset = 0;
    for segment in markdown.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line == target {
            return Some(offset);
        }
        offset += segment.len();
    }
    None
}

fn h2_positions(markdown: &str) -> Vec<(usize, &str)> {
    let mut positions = Vec::new();
    let mut offset = 0;
    for segment in markdown.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.starts_with("## ") {
            positions.push((offset, line));
        }
        offset += segment.len();
    }
    positions
}

fn content_lines(section: &str) -> impl Iterator<Item = &str> {
    section
        .lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
}

fn parse_key_palette(line: &str) -> Option<Vec<String>> {
    let colors: Vec<String> = line
        .trim()
        .strip_prefix("Key palette: ")?
        .split(", ")
        .map(parse_upper_hex)
        .collect::<Option<Vec<_>>>()?;
    (colors.len() == 5).then_some(colors)
}

fn parse_color_role_line(line: &str, role: &str) -> Option<String> {
    parse_upper_hex(line.trim().strip_prefix(&format!("{role}: "))?)
}

fn parse_upper_hex(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let valid = bytes.len() == 7
        && bytes[0] == b'#'
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'A'..=b'F'));
    valid.then(|| value.to_string())
}

fn parse_primary_font_line(line: &str) -> Option<String> {
    let value = line.strip_prefix("Primary Font Family: ")?.trim();
    (!value.is_empty()).then(|| normalize_output_font(value))
}

fn table_cells(line: &str) -> Option<Vec<&str>> {
    let line = line.trim();
    (line.starts_with('|') && line.ends_with('|'))
        .then(|| line.trim_matches('|').split('|').map(str::trim).collect())
}

fn font_table_header(line: &str) -> bool {
    table_cells(line).is_some_and(|cells| {
        cells.as_slice() == ["Role", "Family", "Weight", "Size", "Line Height"]
    })
}

fn font_table_separator(line: &str) -> bool {
    table_cells(line).is_some_and(|cells| {
        cells.len() == 5
            && cells.iter().all(|cell| {
                cell.len() >= 3 && cell.bytes().all(|byte| matches!(byte, b'-' | b':' | b' '))
            })
    })
}

fn font_table_row_matches(
    line: &str,
    role: &str,
    provenance: &crate::design_md_evidence::DesignMdEvidenceProvenance,
) -> bool {
    let Some(cells) = table_cells(line) else {
        return false;
    };
    if cells.len() != 5 || cells[0] != role || cells[1].is_empty() {
        return false;
    }
    let font = normalize_output_font(cells[1]);
    let Some(weight) = cells[2].parse::<u16>().ok() else {
        return false;
    };
    let Some(size) = parse_px_number(cells[3]) else {
        return false;
    };
    let line_height = if cells[4] == "normal" {
        None
    } else {
        let Some(value) = parse_px_number(cells[4]) else {
            return false;
        };
        Some(value)
    };
    provenance.typography.iter().any(|token| {
        token.font == font
            && token.weight == weight
            && (token.size - size).abs() < 0.001
            && option_number_eq(token.line_height, line_height)
    })
}

fn normalize_output_font(value: &str) -> String {
    value.trim().trim_matches(['`', '*']).to_ascii_lowercase()
}

fn parse_px_number(value: &str) -> Option<f64> {
    let number = value.trim().strip_suffix("px")?.parse::<f64>().ok()?;
    number.is_finite().then_some(number)
}

fn option_number_eq(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => (left - right).abs() < 0.001,
        (None, None) => true,
        _ => false,
    }
}

fn parse_radius_line(line: &str, label: &str) -> Option<u32> {
    let number = line.strip_prefix(label)?.trim().strip_suffix("px")?;
    (!number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| number.parse::<u32>().ok())
        .flatten()
}
