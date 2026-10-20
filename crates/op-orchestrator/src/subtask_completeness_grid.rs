//! Grid-shape promises: read the planned rows × columns, never cell sizes.

use super::parse_count;
use regex::Regex;
use std::sync::LazyLock;

/// Read the promised grid shape from a subtask's local text.
///
/// Explicit rows/columns wording is authoritative; otherwise fall back to the
/// largest bare a×b pair that is not a cell or tile pixel size.
pub(super) fn grid_dimensions(text: &str) -> Option<(usize, usize)> {
    if text.contains("九宫格") {
        return Some((3, 3));
    }
    if !text.to_ascii_lowercase().contains("grid") && !text.contains("网格") {
        return None;
    }
    if let Some(dimensions) = explicit_dimensions(text) {
        return Some(dimensions);
    }
    static GRID: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)(\d[\d,]*)\s*[x×]\s*(\d[\d,]*)").expect("valid grid-count pattern")
    });
    GRID.captures_iter(text)
        .filter(|captures| !is_size_pair(text, captures.get(0).unwrap()))
        .filter_map(|captures| {
            Some((
                parse_count(captures.get(1)?.as_str())?,
                parse_count(captures.get(2)?.as_str())?,
            ))
        })
        .filter(|&(rows, columns)| (1..=50).contains(&rows) && (1..=50).contains(&columns))
        .max_by_key(|&(rows, columns)| rows * columns)
}

/// "6 行 × 7 列", "6行7列", "6 rows × 7 columns", "7 columns × 6 rows": the
/// number attached to the row/column noun names the shape in either order,
/// and beats every bare a×b pair in the same text (those may be cell sizes).
fn explicit_dimensions(text: &str) -> Option<(usize, usize)> {
    static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        [
            r"(?P<rows>\d[\d,]*)\s*行\s*(?:[x×]\s*)?(?P<columns>\d[\d,]*)\s*列",
            r"(?P<columns>\d[\d,]*)\s*列\s*(?:[x×]\s*)?(?P<rows>\d[\d,]*)\s*行",
            r"(?i)(?P<rows>\d[\d,]*)\s*rows?\s*(?:[x×]\s*)?(?P<columns>\d[\d,]*)\s*columns?\b",
            r"(?i)(?P<columns>\d[\d,]*)\s*columns?\s*(?:[x×]\s*)?(?P<rows>\d[\d,]*)\s*rows?\b",
        ]
        .into_iter()
        .map(|pattern| Regex::new(pattern).expect("valid explicit rows-columns pattern"))
        .collect()
    });
    PATTERNS
        .iter()
        .flat_map(|pattern| pattern.captures_iter(text))
        .filter_map(|captures| {
            let rows = captures.name("rows")?;
            let columns = captures.name("columns")?;
            Some((parse_count(rows.as_str())?, parse_count(columns.as_str())?))
        })
        .filter(|&(rows, columns)| (1..=50).contains(&rows) && (1..=50).contains(&columns))
        .max_by_key(|&(rows, columns)| rows * columns)
}

/// A unit suffix ("34×36px") or a size noun within the 8 characters before
/// the pair ("单元格约 34×36", "cells 44×44") marks a pixel size, not a shape.
fn is_size_pair(text: &str, whole: regex::Match<'_>) -> bool {
    static SIZE_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^\s*(?:px|pt|dp|pixels?)\b").expect("valid size-suffix pattern")
    });
    if SIZE_SUFFIX.is_match(&text[whole.end()..]) {
        return true;
    }
    const SIZE_HINTS: [&str; 11] = [
        "单元格",
        "格子",
        "尺寸",
        "大小",
        "约",
        "宽",
        "高",
        "cell",
        "size",
        "tile",
        "@",
    ];
    let prefix = &text[..whole.start()];
    let window_start = prefix
        .char_indices()
        .rev()
        .nth(7)
        .map_or(0, |(index, _)| index);
    let window = text[window_start..whole.start()].to_ascii_lowercase();
    SIZE_HINTS.iter().any(|hint| window.contains(hint))
}
