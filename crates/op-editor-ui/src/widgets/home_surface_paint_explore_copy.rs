//! Fitting the explore cards' copy column.
//!
//! The copy column sits LEFT of the card's art, so every string painted
//! there has a hard right edge. The Chinese descriptions were written as
//! two comma clauses and used to be split on `，`; English and the other
//! long-text locales have no such comma and painted as one line straight
//! under the art. Lines are now measured against the column: clauses
//! still break at `，`, each clause wraps greedily (by word where the text
//! has spaces, by character otherwise), and whatever does not fit in the
//! line budget ends in an ellipsis.

use crate::widgets::file_menu::truncate_to_width_measured;

/// The Chinese clause separator the zh descriptions are written around.
const CLAUSE_BREAK: char = '，';

/// `desc` as at most `max_lines` lines, each no wider than `max_w`
/// under `measure`.
pub(crate) fn fit_lines(
    desc: &str,
    max_w: f32,
    max_lines: usize,
    mut measure: impl FnMut(&str) -> f32,
) -> Vec<String> {
    if max_lines == 0 || max_w <= 0.0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for clause in desc.split(CLAUSE_BREAK) {
        let clause = clause.trim();
        if !clause.is_empty() {
            wrap_clause(clause, max_w, &mut measure, &mut lines);
        }
    }
    if lines.len() > max_lines {
        let rest = lines.split_off(max_lines);
        let last = lines.pop().unwrap_or_default();
        let joined = format!("{last} {}", rest.join(" "));
        lines.push(truncate_to_width_measured(&joined, max_w, &mut measure));
    }
    lines
}

/// Greedy wrap of one clause, appending to `lines`.
fn wrap_clause(
    clause: &str,
    max_w: f32,
    measure: &mut impl FnMut(&str) -> f32,
    lines: &mut Vec<String>,
) {
    let by_word = clause.contains(' ');
    let pieces: Vec<&str> = if by_word {
        clause.split_inclusive(' ').collect()
    } else {
        clause
            .char_indices()
            .map(|(i, c)| &clause[i..i + c.len_utf8()])
            .collect()
    };
    let mut current = String::new();
    for piece in pieces {
        let probe = format!("{current}{piece}");
        if measure(probe.trim_end()) <= max_w || current.is_empty() {
            current = probe;
            continue;
        }
        lines.push(truncate_to_width_measured(
            current.trim_end(),
            max_w,
            &mut *measure,
        ));
        current = piece.to_string();
    }
    let current = current.trim_end();
    if !current.is_empty() {
        // A single word wider than the column is ellipsized rather than
        // left to run under the art.
        lines.push(truncate_to_width_measured(current, max_w, &mut *measure));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 7 px per char — Latin at 12 px is close to this.
    fn advance(s: &str) -> f32 {
        s.chars().count() as f32 * 7.0
    }

    fn fits(lines: &[String], max_w: f32) {
        for line in lines {
            assert!(advance(line) <= max_w, "{line:?} is wider than {max_w}");
        }
    }

    #[test]
    fn english_wraps_by_word_inside_the_column() {
        let lines = fit_lines("Turn complex ideas into clear visuals.", 150.0, 2, advance);
        assert_eq!(lines, vec!["Turn complex ideas", "into clear visuals."]);
        fits(&lines, 150.0);
    }

    #[test]
    fn chinese_still_breaks_at_the_clause_comma() {
        let lines = fit_lines("把复杂内容，整理成清晰的图文。", 150.0, 2, advance);
        assert_eq!(lines, vec!["把复杂内容", "整理成清晰的图文。"]);
    }

    #[test]
    fn overflow_past_the_line_budget_ends_in_an_ellipsis() {
        let desc = "Цельный визуальный стиль для вашего события.";
        let lines = fit_lines(desc, 110.0, 2, advance);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'), "{lines:?}");
        fits(&lines, 110.0);
    }

    #[test]
    fn a_word_wider_than_the_column_is_cut_not_overdrawn() {
        let lines = fit_lines("Die Bildschirmfotoanleitungen", 70.0, 2, advance);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'), "{lines:?}");
        fits(&lines, 70.0);
    }
}
