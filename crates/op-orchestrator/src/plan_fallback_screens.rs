//! Fallback screens for briefs that declare N screens and then describe them
//! sentence by sentence: "App 两屏（…）：<home sections>。详情页：<detail
//! sections>".
//!
//! `explicit_screen_items` only recognises a declaration whose colon is
//! followed by exactly N full-page items ("两屏：商品页(…)、购物车页(…)").
//! Most briefs instead list the first screen's sections after the colon and
//! open each later screen with its own "<name>页：" sentence. When both
//! planning calls failed, such a brief was cut into unlabelled sections and
//! every screen was stacked into one frame (astra bonsai run, 2026-10-23).

use crate::plan::{Region, Subtask};
use crate::plan_fallback_brief::{
    delivery_count_conflicts, header_screen_counts, screen_declaration_mask, unique_id,
};
use crate::plan_repair::make_safe_section_id;

/// Longest leading "<name>：" that can still name a screen rather than open
/// an ordinary sentence.
const MAX_SCREEN_NAME_CHARS: usize = 16;

/// One declared screen: its name and the brief's own wording for it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeclaredScreen {
    pub(crate) name: String,
    pub(crate) wording: String,
}

fn ends_sentence(ch: char) -> bool {
    matches!(ch, '。' | '\n' | '!' | '！' | '?' | '？')
}

/// The screens an affirmative "N 屏/页" declaration describes sentence by
/// sentence, or `None` unless exactly N screens can be told apart.
pub(crate) fn declared_screens(brief: &str) -> Option<Vec<DeclaredScreen>> {
    let masked = screen_declaration_mask(brief);
    for (colon, ch) in masked
        .char_indices()
        .filter(|(_, ch)| matches!(ch, ':' | '：'))
    {
        let header_start = masked[..colon]
            .char_indices()
            .rev()
            .find(|(_, ch)| ends_sentence(*ch) || matches!(ch, ';' | '；'))
            .map_or(0, |(index, ch)| index + ch.len_utf8());
        let header = &masked[header_start..colon];
        // Normal mode's wrapper ("交付 2 页完整界面：…") restates a delivery
        // count; the user's own declaration follows it.
        if header.contains("交付") {
            continue;
        }
        let Some(counts) = header_screen_counts(header) else {
            continue;
        };
        let count = counts[0];
        if !(2..=10).contains(&count)
            || counts.iter().any(|other| *other != count)
            || delivery_count_conflicts(&masked, count)
        {
            continue;
        }
        if let Some(screens) = split_screens(brief, &masked, colon + ch.len_utf8(), count) {
            return Some(screens);
        }
    }
    None
}

/// A short leading "<label>：" of a sentence: (label, body start offset).
/// `masked` and `sentence` share byte offsets.
fn leading_label<'a>(masked: &str, sentence: &'a str) -> Option<(&'a str, usize)> {
    let (colon, ch) = masked
        .char_indices()
        .find(|(_, ch)| matches!(ch, ':' | '：'))?;
    let label = sentence[..colon].trim();
    let separators = ['，', '、', ',', '；', ';'];
    (!label.is_empty()
        && label.chars().count() <= MAX_SCREEN_NAME_CHARS
        && !label.contains(separators))
    .then_some((label, colon + ch.len_utf8()))
}

fn is_screen_name(label: &str) -> bool {
    let lower = label.to_lowercase();
    ["页", "屏", "界面"].iter().any(|end| label.ends_with(end))
        || ["page", "screen"].iter().any(|end| lower.ends_with(end))
}

fn first_screen_name(wording: &str) -> String {
    if wording.contains("首页") {
        "首页".into()
    } else if wording.to_lowercase().contains("home") {
        "Home".into()
    } else {
        "Screen 1".into()
    }
}

fn split_screens(
    brief: &str,
    masked: &str,
    start: usize,
    count: usize,
) -> Option<Vec<DeclaredScreen>> {
    let mut screens: Vec<DeclaredScreen> = Vec::new();
    // After a non-screen note ("交互：…") unlabelled sentences belong to the
    // note, not to the screen before it.
    let mut in_note = false;
    let mut sentence_start = start;
    let ends = masked[start..]
        .char_indices()
        .filter(|(_, ch)| ends_sentence(*ch))
        .map(|(offset, ch)| (start + offset, ch.len_utf8()))
        .chain(std::iter::once((masked.len(), 0)));
    for (end, width) in ends {
        let (from, to) = (sentence_start, end);
        sentence_start = end + width;
        let sentence = &brief[from..to];
        if sentence.trim().is_empty() {
            continue;
        }
        let label = leading_label(&masked[from..to], sentence);
        match label {
            Some((name, body)) if is_screen_name(name) => {
                in_note = false;
                screens.push(DeclaredScreen {
                    name: name.to_string(),
                    wording: sentence[body..].trim().to_string(),
                });
            }
            Some(_) if !screens.is_empty() => in_note = true,
            _ if screens.is_empty() => screens.push(DeclaredScreen {
                name: first_screen_name(sentence),
                wording: sentence.trim().to_string(),
            }),
            _ if !in_note => {
                let last = screens.last_mut().expect("non-empty");
                last.wording.push('。');
                last.wording.push_str(sentence.trim());
            }
            _ => {}
        }
    }
    let distinct = screens.iter().enumerate().all(|(index, screen)| {
        screens[..index]
            .iter()
            .all(|other| other.name != screen.name)
    });
    (screens.len() == count && distinct).then_some(screens)
}

/// One full-viewport subtask per declared screen, shaped like the explicit
/// full-page path in `brief_section_subtasks`: the screen name is its label,
/// `screen` and `covers`, so normalisation scaffolds one root per screen.
pub(crate) fn screen_subtasks(
    screens: &[DeclaredScreen],
    width: f64,
    height: f64,
    parent_frame_id: Option<&str>,
) -> (Vec<Subtask>, f64) {
    let mut taken = Vec::new();
    let subtasks = screens
        .iter()
        .enumerate()
        .map(|(index, screen)| {
            let id = unique_id(make_safe_section_id(&screen.name, index), &mut taken);
            Subtask {
                id: id.clone(),
                label: screen.name.clone(),
                region: Region { width, height },
                bleed_hero: false,
                id_prefix: id,
                parent_frame_id: parent_frame_id.map(str::to_string),
                insert_after_sibling_id: None,
                elements: Some(format!(
                    "the complete {} screen: {} — build exactly what this names, with every \
                     count it states; do not add content that belongs to the brief's other screens",
                    screen.name, screen.wording
                )),
                screen: Some(screen.name.clone()),
                generated_root_id: None,
                existing_section_labels: None,
                covers: Some(vec![screen.name.clone()]),
                retry_feedback: None,
            }
        })
        .collect();
    (subtasks, height)
}

#[cfg(test)]
#[path = "plan_fallback_screens_tests.rs"]
mod tests;
