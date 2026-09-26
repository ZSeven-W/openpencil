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
/// Returns the subtasks and the summed height of the stacked (non-rail)
/// sections.
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
            let wording = section_wording(brief, head);
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
