//! Brief-driven fallback subtasks.
//!
//! When both planning calls fail, `build_fallback_plan` used to slice the page
//! into 1-3 blank "Section N" subtasks by prompt length. A brief that spells
//! out its own sections ("左侧栏（…）、顶部工具栏、四个 KPI 卡、…") then
//! shipped as ONE anonymous section and the sub-agent guessed the rest
//! (measured: arena d01 run 0927a/run-3 shipped only a KPI row). The brief's
//! own section list — the same one the coverage gate demands of a modelled
//! plan — is the best plan available without a model, so the fallback builds
//! one subtask per required section instead.

use crate::dashboard_columns::is_strong_sidebar_subtask;
use crate::plan::{Region, Subtask};
use crate::plan_coverage::required_sections;
use crate::plan_repair::make_safe_section_id;
use regex::Regex;
use std::sync::LazyLock;

/// Fixed left-rail width, the same 260px every app-shell path uses
/// (`scaffold::SIDEBAR_COLUMN_WIDTH`, `dashboard_columns`, `app_shell`).
const SIDEBAR_WIDTH: f64 = 260.0;
/// Default desktop section height when the request states no root height.
pub(crate) const DESKTOP_SECTION_HEIGHT: f64 = 240.0;
/// Floor for a section's share of an explicitly requested root height.
const MIN_SECTION_HEIGHT: f64 = 96.0;

/// Separators that end one item of the brief's section list (outside any
/// parenthetical).
fn is_item_separator(ch: char) -> bool {
    matches!(
        ch,
        '、' | '，' | '；' | '：' | '。' | '！' | '？' | ',' | ';' | ':' | '\n'
    )
}

/// The brief's full wording for one required section: the whole list item the
/// normalised section head was taken from, including its parenthetical detail
/// and counts ("左侧栏" → "左侧栏（品牌+六个导航项+底部用户）",
/// "下方数据表 8 行 6 列" → "下方数据表 8 行 6 列带分页"). Falls back to the
/// head itself when it cannot be located verbatim.
pub(crate) fn section_wording(brief: &str, head: &str) -> String {
    let lower_brief;
    let lower_head;
    let (haystack, needle) = if brief.contains(head) {
        (brief, head)
    } else {
        // Only case-folded when folding keeps byte offsets (ASCII-safe).
        lower_brief = brief.to_lowercase();
        lower_head = head.to_lowercase();
        if lower_brief.len() != brief.len() {
            return head.to_string();
        }
        (lower_brief.as_str(), lower_head.as_str())
    };
    let Some(start) = haystack.find(needle) else {
        return head.to_string();
    };

    // Walk back to the previous separator at paren depth 0.
    let mut item_start = 0usize;
    let mut depth = 0i32;
    for (index, ch) in brief[..start].char_indices().rev() {
        match ch {
            ')' | '）' => depth += 1,
            '(' | '（' => {
                if depth == 0 {
                    // Inside an enclosing parenthetical: the item starts after it.
                    item_start = index + ch.len_utf8();
                    break;
                }
                depth -= 1;
            }
            _ if depth == 0 && is_item_separator(ch) => {
                item_start = index + ch.len_utf8();
                break;
            }
            _ => {}
        }
    }

    // Walk forward to the next separator at paren depth 0.
    let mut item_end = brief.len();
    let mut depth = 0i32;
    for (offset, ch) in brief[start..].char_indices() {
        let index = start + offset;
        match ch {
            '(' | '（' => depth += 1,
            ')' | '）' => {
                if depth == 0 {
                    item_end = index;
                    break;
                }
                depth -= 1;
            }
            _ if depth == 0 && is_item_separator(ch) => {
                item_end = index;
                break;
            }
            _ => {}
        }
    }

    let wording = brief[item_start..item_end].trim();
    if wording.is_empty() {
        head.to_string()
    } else {
        wording.to_string()
    }
}

/// A CJK left-rail head ("左侧栏", "侧边栏", "侧边导航", "边栏"). The English
/// forms are already recognised by `dashboard_columns`' strong sidebar
/// keywords through the ASCII id; CJK heads have no ASCII id, so they are
/// given the canonical `sidebar` id and then go through the same detector.
fn is_cjk_sidebar_head(head: &str) -> bool {
    head.contains("侧栏") || head.contains("侧边") || head.contains("边栏")
}

fn unique_id(base: String, taken: &mut Vec<String>) -> String {
    let mut id = base.clone();
    let mut n = 2usize;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    taken.push(id.clone());
    id
}

static SCREEN_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([0-9]+|[一二两三四五六七八九十]+)\s*(?:个\s*)?(?:屏|页|screens?\b|pages?\b)")
        .expect("valid explicit screen-count pattern")
});

static EN_SINGLE_OR_NEGATED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:single[-\s]+(?:page|screen)|do\s+not|don['’]t|no\s+need)\b")
        .expect("valid explicit English single-page or negation phrases")
});

/// Preserve byte offsets while removing quoted copy and parenthetical detail
/// from the declaration/list grammar. Their contents are not screen counts or
/// top-level separators (but remain intact in each worker's `elements`).
fn screen_declaration_mask(brief: &str) -> String {
    let mut masked = String::with_capacity(brief.len());
    let mut depth = 0usize;
    let mut quote_end = None;
    for ch in brief.chars() {
        let hidden = if let Some(end) = quote_end {
            if ch == end {
                quote_end = None;
            }
            true
        } else {
            match ch {
                '"' | '`' | '“' | '「' | '『' | '‘' => {
                    quote_end = Some(match ch {
                        '“' => '”',
                        '「' => '」',
                        '『' => '』',
                        '‘' => '’',
                        other => other,
                    });
                    true
                }
                '(' | '（' => {
                    depth += 1;
                    true
                }
                ')' | '）' => {
                    depth = depth.saturating_sub(1);
                    true
                }
                _ => depth > 0,
            }
        };
        if hidden {
            masked.push_str(&" ".repeat(ch.len_utf8()));
        } else {
            masked.push(ch);
        }
    }
    masked
}

fn screen_count(text: &str) -> Option<usize> {
    text.parse().ok().or_else(|| {
        ["一", "二", "三", "四", "五", "六", "七", "八", "九", "十"]
            .iter()
            .position(|digit| *digit == text)
            .map(|index| index + 1)
            .or_else(|| (text == "两").then_some(2))
    })
}

fn ends_declaration(ch: char) -> bool {
    matches!(ch, '。' | '.' | ';' | '；' | '\n' | '!' | '！' | '?' | '？')
}

/// Only upgrade an explicit N-screen declaration whose following list is
/// exactly the already-extracted N full-page items: `App 两屏: A(...), B(...)`.
/// A count elsewhere in the brief cannot turn ordinary sections into screens.
fn explicit_screen_items(brief: &str, heads: &[String]) -> Option<Vec<String>> {
    if !(2..=10).contains(&heads.len()) {
        return None;
    }
    let masked = screen_declaration_mask(brief);
    // Normal-mode's explicit delivery choice outranks page counts repeated
    // in the user's original brief (notably Single + an old two-screen brief).
    if SCREEN_COUNT.captures_iter(&masked).any(|captures| {
        let before = masked[..captures.get(0).unwrap().start()].trim_end();
        before.ends_with("交付") && screen_count(&captures[1]) != Some(heads.len())
    }) {
        return None;
    }
    for (colon, ch) in masked
        .char_indices()
        .filter(|(_, ch)| matches!(ch, ':' | '：'))
    {
        let header_start = masked[..colon]
            .char_indices()
            .rev()
            .find(|(_, ch)| ends_declaration(*ch))
            .map_or(0, |(index, ch)| index + ch.len_utf8());
        let header = &masked[header_start..colon];
        let lower = header.to_lowercase();
        // A single-page instruction or a negated multi-page example is not
        // affirmative screen ownership. Ambiguous clauses keep section mode.
        if ["单页", "单屏", "不要", "不需要", "无需", "禁止"]
            .iter()
            .any(|word| header.contains(word))
            || EN_SINGLE_OR_NEGATED.is_match(header)
        {
            continue;
        }
        if [
            "文案", "标题", "示例", "例如", "参考", "caption", "headline", "example",
        ]
        .iter()
        .any(|word| lower.contains(word))
        {
            continue;
        }
        let counts: Vec<_> = SCREEN_COUNT
            .captures_iter(header)
            .filter(|captures| {
                !header[..captures.get(0).unwrap().start()]
                    .trim_end()
                    .ends_with('第')
            })
            .filter_map(|captures| screen_count(&captures[1]))
            .collect();
        if counts.is_empty() || counts.iter().any(|count| *count != heads.len()) {
            continue;
        }
        let start = colon + ch.len_utf8();
        let end = masked[start..]
            .find(ends_declaration)
            .map_or(masked.len(), |offset| start + offset);
        let mut item_start = start;
        let mut items = Vec::new();
        for (offset, ch) in masked[start..end].char_indices() {
            if matches!(ch, '、' | ',' | '，') {
                items.push(brief[item_start..start + offset].trim());
                item_start = start + offset + ch.len_utf8();
            }
        }
        items.push(brief[item_start..end].trim());
        if items.len() == heads.len()
            && items.iter().zip(heads).all(|(item, head)| {
                let Some(open) = item.find(['(', '（']) else {
                    return false;
                };
                item[..open].trim() == head
                    && item.ends_with([')', '）'])
                    && !item[open..]
                        .trim_matches(['(', '（', ')', '）'])
                        .trim()
                        .is_empty()
            })
        {
            return Some(items.into_iter().map(str::to_string).collect());
        }
    }
    None
}

/// One subtask per section the brief explicitly enumerates, or `None` when it
/// enumerates none (callers then keep their generic skeleton).
///
/// * `label` and `covers` carry the required section verbatim, so the
///   coverage gate (`plan_coverage::missing_sections`) accepts the plan.
/// * `elements` carries the brief's full wording for the section plus an
///   instruction to build exactly that, with its stated counts.
/// * On desktop (`narrow_sidebar`), a strong sidebar section gets the 260px
///   rail width and the full root height; the remaining sections take the
///   main-column width and share the requested height (or a default each).
///
/// Explicit full-page lists retain separate screen ownership and each gets
/// the full requested viewport. Returns one viewport height in that case;
/// otherwise returns the summed height of stacked (non-rail) sections.
pub(crate) fn brief_section_subtasks(
    brief: &str,
    root_width: f64,
    requested_height: Option<f64>,
    narrow_sidebar: bool,
    parent_frame_id: Option<&str>,
) -> Option<(Vec<Subtask>, f64)> {
    let heads = required_sections(brief);
    if heads.is_empty() {
        return None;
    }
    let screen_items = explicit_screen_items(brief, &heads);

    let mut taken: Vec<String> = Vec::new();
    let mut subtasks: Vec<Subtask> = heads
        .iter()
        .enumerate()
        .map(|(index, head)| {
            let base = if is_cjk_sidebar_head(head) {
                "sidebar".to_string()
            } else {
                make_safe_section_id(head, index)
            };
            let id = unique_id(base, &mut taken);
            let wording = screen_items.as_ref().map_or_else(
                || section_wording(brief, head),
                |items| items[index].clone(),
            );
            Subtask {
                id: id.clone(),
                label: head.clone(),
                region: Region {
                    width: root_width,
                    height: DESKTOP_SECTION_HEIGHT,
                },
                bleed_hero: false,
                id_prefix: id,
                parent_frame_id: parent_frame_id.map(str::to_string),
                insert_after_sibling_id: None,
                elements: Some(format!(
                    "{wording} — build exactly what this names, with every count it states; \
                     do not add content that belongs to the brief's other sections"
                )),
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
                // The brief's own wording, verbatim: the coverage gate trusts
                // `covers` by normalized equality.
                covers: Some(vec![head.clone()]),
                retry_feedback: None,
            }
        })
        .collect();

    if screen_items.is_some() {
        let height = requested_height.unwrap_or(DESKTOP_SECTION_HEIGHT);
        for st in &mut subtasks {
            st.screen = Some(st.label.clone());
            st.region.height = height;
        }
        return Some((subtasks, height));
    }

    let is_rail = |st: &Subtask| narrow_sidebar && is_strong_sidebar_subtask(st);
    let has_rail = subtasks.iter().any(is_rail);
    let stacked = subtasks.iter().filter(|st| !is_rail(st)).count().max(1);
    let section_height = requested_height
        .map(|height| (height / stacked as f64).floor().max(MIN_SECTION_HEIGHT))
        .unwrap_or(DESKTOP_SECTION_HEIGHT);
    let stacked_total = section_height * stacked as f64;
    let rail_height = requested_height.unwrap_or(stacked_total);
    let main_width = if has_rail {
        (root_width - SIDEBAR_WIDTH).max(320.0)
    } else {
        root_width
    };

    for st in &mut subtasks {
        if is_rail(st) {
            st.region = Region {
                width: SIDEBAR_WIDTH,
                height: rail_height,
            };
        } else {
            st.region = Region {
                width: main_width,
                height: section_height,
            };
        }
    }
    Some((subtasks, stacked_total))
}

#[cfg(test)]
#[path = "plan_fallback_brief_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "plan_fallback_brief_screens_tests.rs"]
mod screens_tests;
