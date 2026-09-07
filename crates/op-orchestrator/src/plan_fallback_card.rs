//! Fallback plan for a social card series.
//!
//! The deck rung learned this the hard way: a design type with a fixed board
//! needs its OWN fallback branch, because the generic one builds a 1200-wide
//! scrolling page and every downstream size contract is gone before anything
//! is drawn (`build_fallback_deck_plan`'s reason for existing). A card series
//! has the same shape — N independent boards, not N sections of one page — so
//! every subtask carries a `screen` tag and `plan_normalize`'s screen-group
//! pass turns each into its own top-level root.
//!
//! Sizes come from `card-system-0808.md` §5 through the design-type preset;
//! the running order comes from §4.2 (cover → body → closing).

use crate::plan::{OrchestratorPlan, PlanFill, Region, RootFrameSpec, Subtask};
use crate::types::DesignRequest;
use crate::DesignTypePreset;

/// How many cards a prompt asks for, when it says so.
///
/// Separate from the deck's `explicit_slide_count` because the units differ —
/// a card series is counted in 张 / 图 / cards, never in 页.
fn explicit_card_count(prompt: &str) -> Option<usize> {
    const UNITS: [&str; 6] = ["cards", "card", "张", "張", "图", "圖"];
    let lower = prompt.to_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let mut unit_start = i;
        while unit_start < bytes.len() && matches!(bytes[unit_start], b' ' | b'-' | b'_' | b'\t') {
            unit_start += 1;
        }
        if UNITS
            .iter()
            .any(|unit| lower[unit_start..].starts_with(unit))
        {
            if let Ok(count) = lower[start..i].parse::<usize>() {
                if (2..=20).contains(&count) {
                    return Some(count);
                }
            }
        }
    }
    None
}

/// Cover, body, closing — §4.2's arrangement rule, sized to the count.
fn card_titles(count: usize) -> Vec<String> {
    let mut titles = vec!["封面".to_string()];
    let body = count.saturating_sub(2).max(1);
    for index in 1..=body {
        titles.push(format!("要点 {index}"));
    }
    if count >= 3 {
        titles.push("收尾".to_string());
    }
    titles.truncate(count.max(2));
    titles
}

/// What each card should carry, by position in the run.
fn card_elements(title: &str) -> String {
    if title == "封面" {
        "the hook: one short headline that makes someone stop scrolling, plus a one-line \
         subtitle. No body copy."
            .to_string()
    } else if title == "收尾" {
        "the closing action: a one-line takeaway and a single call to action.".to_string()
    } else {
        format!("{title}: one idea, stated as a short heading plus 2-4 supporting lines.")
    }
}

/// A card series' fallback plan — one screen-tagged subtask per card.
pub(crate) fn build_fallback_card_plan(
    req: &DesignRequest,
    preset: DesignTypePreset,
) -> OrchestratorPlan {
    let count = explicit_card_count(&req.prompt).unwrap_or(match req.prompt.chars().count() {
        0..=80 => 5,
        81..=200 => 6,
        _ => 8,
    });

    let subtasks = card_titles(count)
        .into_iter()
        .enumerate()
        .map(|(index, title)| {
            let id = format!("card-{}", index + 1);
            Subtask {
                id: id.clone(),
                label: title.clone(),
                region: Region {
                    width: preset.width,
                    height: preset.root_height,
                },
                bleed_hero: false,
                id_prefix: id,
                // `plan_normalize` rewrites this per screen group.
                parent_frame_id: None,
                insert_after_sibling_id: None,
                elements: Some(card_elements(&title)),
                screen: Some(title),
                generated_root_id: None,
                existing_section_labels: None,
                retry_feedback: None,
            }
        })
        .collect();

    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "card".into(),
            name: "Card".into(),
            width: preset.width,
            height: preset.root_height,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: Some(0.0),
            fill: Some(vec![PlanFill {
                kind: "solid".into(),
                color: "#FFFFFF".into(),
            }]),
        },
        subtasks,
        style_guide_name: None,
    }
}

#[cfg(test)]
#[path = "plan_fallback_card_tests.rs"]
mod tests;
