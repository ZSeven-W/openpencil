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

/// A fixed-width amount token queued for the measured fit.
struct Candidate<'a> {
    text: &'a Value,
    /// The width the author actually gave the token (see
    /// [`allotted::allotted_width`]).
    available: f64,
    /// The immediate parent the token must end inside after the fit, with
    /// that parent's right padding.
    parent: Option<(String, f64)>,
}

/// Collect fixed-width amount tokens (not in a status bar / scroller) with
/// the width the author gave them.
fn collect_candidates<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    skip_ids: &[String],
    in_excluded: bool,
    ancestors: &mut Vec<&'a Value>,
    out: &mut Vec<Candidate<'a>>,
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
            let resolved = rects.get(id).is_some_and(|r| r.w.is_finite() && r.w > 1.0);
            if resolved && !skip_ids.iter().any(|s| s == id) {
                ancestors.push(v);
                let available = allotted::allotted_width(ancestors, rects);
                ancestors.pop();
                let parent = ancestors.last().and_then(|p| {
                    let pid = p.get("id").and_then(Value::as_str)?;
                    let right = numeric_padding_sides(p).map_or(0.0, |[_, r, _, _]| r);
                    Some((pid.to_string(), right))
                });
                if let Some(available) = available.filter(|w| *w > 1.0) {
                    out.push(Candidate {
                        text: v,
                        available,
                        parent,
                    });
                }
            }
        }
    }
    ancestors.push(v);
    for c in children(v) {
        collect_candidates(c, rects, skip_ids, excluded, ancestors, out);
    }
    ancestors.pop();
}

/// Font-size commands that put each wrapped / overflowing amount token back
/// on one line inside the width the author gave it.
pub(super) fn collect_measured_amount_fixes(
    state: &EditorState,
    root: &Value,
    rects: &HashMap<String, Rect>,
    already_fixed: &[String],
) -> Vec<EditorCommand> {
    let mut candidates = Vec::new();
    collect_candidates(
        root,
        rects,
        already_fixed,
        false,
        &mut Vec::new(),
        &mut candidates,
    );
    if candidates.is_empty() {
        return Vec::new();
    }
    // Re-lay the candidates as single-line hugging text on a scratch copy
    // to read their natural width from the real layout.
    let mut scratch = state.clone();
    for Candidate { text, .. } in &candidates {
        if let Some(id) = text.get("id").and_then(Value::as_str) {
            scratch.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(id.to_string()),
                patch_json: r#"{"textGrowth":"auto","width":"fit_content"}"#.to_string(),
                page_id: None,
            });
        }
    }
    let natural = resolved_rects(&scratch);
    let mut fits: Vec<Fit> = Vec::new();
    for Candidate {
        text,
        available,
        parent,
    } in candidates
    {
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
            fits.push(Fit {
                id: id.to_string(),
                size,
                available,
                original: font_size,
                parent,
            });
        }
    }
    verify_fitted_sizes(state, &mut scratch, &mut fits);
    fits.into_iter()
        .map(|fit| EditorCommand::SetNodeFontSize {
            node_id: NodeId::new(fit.id),
            font_size: fit.size as f32,
        })
        .collect()
}

/// A font-size fit under verification.
struct Fit {
    id: String,
    size: f64,
    available: f64,
    original: f64,
    parent: Option<(String, f64)>,
}

/// The proportional estimate assumes width scales with the font size, but
/// letter spacing is a fixed per-glyph offset, fallback faces do not scale
/// linearly, and the platform line breaker may wrap a token whose natural
/// width nominally fits. So check the real condition: at the chosen size the
/// text, laid out in its AUTHORED fixed width, must be exactly as tall as the
/// same text laid out on one line, and must end inside its parent's inner box
/// (a text that spills past it paints over the parent's next sibling). Step
/// each still-failing token down a pixel at a time (bounded, never below the
/// estimate's floor).
fn verify_fitted_sizes(state: &EditorState, single_line: &mut EditorState, fits: &mut [Fit]) {
    const MAX_STEPS: usize = 16;
    let mut authored = state.clone();
    for _ in 0..MAX_STEPS {
        for fit in fits.iter() {
            for doc in [&mut *single_line, &mut authored] {
                doc.apply(EditorCommand::SetNodeFontSize {
                    node_id: NodeId::new(fit.id.clone()),
                    font_size: fit.size as f32,
                });
            }
        }
        let one_line = resolved_rects(single_line);
        let laid_out = resolved_rects(&authored);
        let mut stepped = false;
        for fit in fits.iter_mut() {
            let minimum = if fit.original >= 32.0 { 24.0 } else { 12.0 };
            let (Some(one), Some(real)) = (one_line.get(&fit.id), laid_out.get(&fit.id)) else {
                continue;
            };
            let too_wide = one.w.is_finite() && one.w > fit.available + TEXT_FIT_EPS;
            let wrapped = real.h.is_finite() && one.h.is_finite() && real.h > one.h + 0.5;
            let spills = fit.parent.as_ref().is_some_and(|(parent_id, right_pad)| {
                laid_out.get(parent_id).is_some_and(|parent| {
                    real.x + real.w > parent.x + parent.w - right_pad + TEXT_FIT_EPS
                })
            });
            if (too_wide || wrapped || spills) && fit.size - 1.0 >= minimum {
                fit.size -= 1.0;
                stepped = true;
            }
        }
        if !stepped {
            break;
        }
    }
}

#[path = "text_fit_allotted.rs"]
mod allotted;
