//! Measured text-fit for fixed-width monetary / numeric amounts.
//!
//! A figure such as "¥ 268,540.32" is ONE reading token: a line break
//! between the currency symbol and the digits (or inside the digit groups)
//! is never an authored intent. The estimate path in `text_fit` only covers
//! whitespace-free ASCII tokens with a proportional-font advance guess, so a
//! 48px "DM Mono" hero amount in a 299px card (arena-m03) slipped through:
//! the layout wrapped it into "¥" + "268,540.32" and the card grew a second
//! display line. Here the amount's natural single-line width is MEASURED
//! with the same jian layout (a scratch copy of the document re-lays the
//! text as `auto` / `fit_content`), and the font shrinks just enough for the
//! token to sit on one line in the width the author gave it.

use super::*;

/// Currency symbols that may lead an amount (optionally followed by one
/// space, e.g. "¥ 268,540.32").
const CURRENCY_SYMBOLS: &[char] = &['¥', '￥', '$', '€', '£', '₩', '₹', '₽', '฿', '₫', '₺', '₱'];

/// True for a single numeric reading token: optional sign, optional currency
/// symbol (+ one space), digit groups joined by `,` `.` `'` or a thin/normal
/// space, optional trailing `%`. Letters and CJK never qualify.
pub(super) fn is_amount_token(content: &str) -> bool {
    let s = content.trim();
    if s.is_empty() || s.contains('\n') {
        return false;
    }
    let mut rest = s.trim_start_matches(['+', '-', '−', '±']);
    if let Some(stripped) = rest.strip_prefix(CURRENCY_SYMBOLS) {
        rest = stripped.strip_prefix(' ').unwrap_or(stripped);
    }
    rest = rest.trim_start_matches(['+', '-', '−']);
    let rest = rest.strip_suffix('%').unwrap_or(rest);
    let mut chars = rest.chars();
    let starts_with_digit = chars.next().is_some_and(|c| c.is_ascii_digit());
    let ends_with_digit = rest.chars().last().is_some_and(|c| c.is_ascii_digit());
    starts_with_digit
        && ends_with_digit
        && rest
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, ',' | '.' | '\'' | ' ' | '\u{2009}'))
        // Two numbers separated by a space ("12 34") are two tokens only
        // when a group is not 3 digits; a lone space inside digit groups
        // ("1 234 567") is the SI grouping. Keep the rule simple: at most
        // the grouping spaces a real figure carries.
        && rest.split([' ', '\u{2009}']).skip(1).all(|g| g.len() >= 3)
}

/// Collect fixed-width amount tokens (not in a status bar / scroller) with
/// their resolved width — the width the author gave them.
fn collect_candidates<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    skip_ids: &[String],
    in_excluded: bool,
    out: &mut Vec<(&'a Value, f64)>,
) {
    let excluded = in_excluded
        || crate::cleanup::is_status_bar_from_json(v)
        || (layout_str(v) == Some("horizontal")
            && v.get("clipContent").and_then(Value::as_bool) == Some(true));
    if !excluded && v.get("type").and_then(Value::as_str) == Some("text") {
        let fixed_width = matches!(
            v.get("textGrowth").and_then(Value::as_str),
            Some("fixed-width" | "fixed-width-height")
        );
        let id = v.get("id").and_then(Value::as_str);
        let amount = v
            .get("content")
            .and_then(Value::as_str)
            .is_some_and(is_amount_token);
        if let (true, true, Some(id)) = (fixed_width, amount, id) {
            if !skip_ids.iter().any(|s| s == id) {
                if let Some(r) = rects.get(id).filter(|r| r.w.is_finite() && r.w > 1.0) {
                    out.push((v, r.w));
                }
            }
        }
    }
    for c in children(v) {
        collect_candidates(c, rects, skip_ids, excluded, out);
    }
}

/// Font-size commands that put each wrapped / overflowing amount token back
/// on one line inside its authored width.
pub(super) fn collect_measured_amount_fixes(
    state: &EditorState,
    root: &Value,
    rects: &HashMap<String, Rect>,
    already_fixed: &[String],
) -> Vec<EditorCommand> {
    let mut candidates = Vec::new();
    collect_candidates(root, rects, already_fixed, false, &mut candidates);
    if candidates.is_empty() {
        return Vec::new();
    }
    // Re-lay the candidates as single-line hugging text on a scratch copy
    // to read their natural width from the real layout.
    let mut scratch = state.clone();
    for (text, _) in &candidates {
        if let Some(id) = text.get("id").and_then(Value::as_str) {
            scratch.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id.to_string()),
                patch_json: r#"{"textGrowth":"auto","width":"fit_content"}"#.to_string(),
                page_id: None,
            });
        }
    }
    let natural = resolved_rects(&scratch);
    let mut cmds = Vec::new();
    for (text, available) in candidates {
        let Some(id) = text.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(width) = natural.get(id).map(|r| r.w).filter(|w| w.is_finite()) else {
            continue;
        };
        if width <= available + TEXT_FIT_EPS {
            continue;
        }
        let font_size = match text.get("fontSize") {
            None | Some(Value::Null) => 14.0,
            _ => num(text, "fontSize"),
        };
        if !font_size.is_finite() || font_size <= 0.0 {
            continue;
        }
        if let Some(size) = fitted_font_size(font_size, available, width) {
            cmds.push(EditorCommand::SetNodeFontSize {
                node_id: NodeId::new(id.to_string()),
                font_size: size as f32,
            });
        }
    }
    cmds
}
