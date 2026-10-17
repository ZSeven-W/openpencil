//! Reserve usable content before an optional photo on action-oriented details.

use crate::{
    plan::{OrchestratorPlan, Subtask},
    types::DesignRequest,
};
use regex::Regex;
use std::sync::LazyLock;

// Prefer the roomier budget when it fits, but a usable 120px photo must yield
// to complete metadata, normal list rows and the authored bottom action.
const MIN_HERO: f64 = 120.0;
const MAX_HERO: f64 = 208.0;
const MIN_SUMMARY: f64 = 160.0;
const LIST_HEADING: f64 = 40.0;
const VISIBLE_ROWS: f64 = 3.0;
const ROW_HEIGHT: f64 = 72.0;
const SAFETY_INSET: f64 = 24.0;

pub(super) fn reserve_supporting_content(plan: &mut OrchestratorPlan, req: &DesignRequest) {
    let brief = req.prompt.to_lowercase();
    if req.append_context.is_some()
        || req.continuation_context.is_some()
        || req.reference_skeleton.is_some()
        || !(320.0..=super::MOBILE_MAX_WIDTH).contains(&plan.root_frame.width)
        || !(640.0..=1000.0).contains(&plan.root_frame.height)
        || plan
            .root_frame
            .layout
            .as_deref()
            .is_some_and(|layout| layout != "vertical")
        || !has_any(&brief, &["detail", "详情"])
        || !has_photo_identity(&brief)
        || !has_list_identity(&brief)
        || !has_pinned_action(&brief)
        || preserves_photo_composition(&brief)
    {
        return;
    }
    let mut groups = crate::screen_groups::group_subtasks_by_screen(&plan.subtasks);
    if groups.is_empty() {
        groups.push(crate::screen_groups::ScreenGroup {
            screen: "page".into(),
            indices: (0..plan.subtasks.len()).collect(),
        });
    }
    for group in groups {
        reserve_screen(plan, &group.indices);
    }
}

fn reserve_screen(plan: &mut OrchestratorPlan, indices: &[usize]) {
    // Explicit above-the-fold requests and existing section budgets retain
    // precedence; do not layer a three-row contract over an all-content one.
    if indices
        .iter()
        .any(|&index| crate::first_viewport::owns(&plan.subtasks[index]))
    {
        return;
    }
    let matches = |predicate: fn(&Subtask) -> bool| -> Vec<usize> {
        indices
            .iter()
            .enumerate()
            .filter_map(|(position, &index)| predicate(&plan.subtasks[index]).then_some(position))
            .collect()
    };
    let heroes = matches(is_photo_hero);
    let summaries = matches(is_summary);
    let lists = matches(is_repeated_list);
    let actions = matches(|task| has_pinned_action(&task_text(task)));
    let ([hero], [summary], [list], [action]) = (
        heroes.as_slice(),
        summaries.as_slice(),
        lists.as_slice(),
        actions.as_slice(),
    ) else {
        return;
    };
    // Ambiguous ordering or mixed sections need a new composition, not an
    // automatic rewrite. Only compact navigation may precede the photo.
    if *summary != hero + 1
        || *list != summary + 1
        || *action != list + 1
        || *action + 1 != indices.len()
        || indices[..*hero].iter().any(|&index| {
            let task = &plan.subtasks[index];
            task.region.height > 96.0
                || !has_any(&identity(task), &["header", "navigation", "导航", "顶栏"])
        })
        || indices.iter().any(|&index| {
            let size = &plan.subtasks[index].region;
            !size.height.is_finite()
                || size.height <= 0.0
                || !size.width.is_finite()
                || size.width <= 0.0
        })
    {
        return;
    }
    let hero_index = indices[*hero];
    let summary_index = indices[*summary];
    let original_hero = plan.subtasks[hero_index].region.height;
    if original_hero < MIN_HERO {
        return;
    }
    let summary_height = plan.subtasks[summary_index].region.height.max(MIN_SUMMARY);
    let preceding: f64 = indices[..*hero]
        .iter()
        .map(|&i| plan.subtasks[i].region.height)
        .sum();
    let gap = plan.root_frame.gap.unwrap_or(0.0);
    let padding = plan.root_frame.padding.unwrap_or(0.0);
    if !gap.is_finite() || !padding.is_finite() {
        return;
    }
    let remaining = plan.root_frame.height
        - crate::scaffold::STATUS_BAR_HEIGHT
        - padding.max(0.0) * 2.0
        - gap.max(0.0) * indices.len() as f64
        - preceding
        - plan.subtasks[indices[*action]].region.height
        - summary_height
        - LIST_HEADING
        - VISIBLE_ROWS * ROW_HEIGHT
        - SAFETY_INSET;
    let hero_height = remaining.floor().min(MAX_HERO).min(original_hero);
    // Do not solve infeasibility by shrinking controls, copy or all list rows.
    if !hero_height.is_finite() || hero_height < MIN_HERO {
        return;
    }
    plan.subtasks[hero_index].region.height = hero_height;
    plan.subtasks[summary_index].region.height = summary_height;
    crate::first_viewport::mark_detail_tasks(plan, &[hero_index, summary_index]);
}

fn identity(task: &Subtask) -> String {
    format!("{} {}", task.id, task.label).to_lowercase()
}

fn task_text(task: &Subtask) -> String {
    format!(
        "{} {}",
        identity(task),
        task.elements.as_deref().unwrap_or("")
    )
    .to_lowercase()
}

fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}

fn has_photo_identity(text: &str) -> bool {
    has_any(
        text,
        &[
            "hero image",
            "hero photo",
            "cover image",
            "cover photo",
            "主图",
            "头图",
            "首图",
            "封面图",
        ],
    )
}

fn has_list_identity(text: &str) -> bool {
    has_any(text, &["list", "rows", "列表", "清单", "条目"])
}

fn has_pinned_action(text: &str) -> bool {
    !has_any(
        text,
        &[
            "not pinned",
            "not fixed",
            "not sticky",
            "不要固定",
            "不固定",
            "非固定",
        ],
    ) && has_any(text, &["pinned", "fixed", "sticky", "固定", "吸底"])
        && has_any(text, &["bottom", "底部", "吸底"])
        && has_any(text, &["button", "cta", "action", "按钮", "操作"])
}

fn is_photo_hero(task: &Subtask) -> bool {
    let identity = identity(task);
    let elements = task.elements.as_deref().unwrap_or("").to_lowercase();
    has_any(&identity, &["hero", "主图", "头图", "首图", "封面"])
        && has_any(
            &elements,
            &["photo", "image", "照片", "图片", "主图", "头图"],
        )
        && !has_list_identity(&identity)
}

fn is_summary(task: &Subtask) -> bool {
    let identity = identity(task);
    has_any(&identity, &["title", "标题"])
        && has_any(&identity, &["meta", "summary", "摘要", "信息", "简介"])
        && !has_list_identity(&identity)
        && !has_any(&identity, &["hero", "主图", "头图"])
}

fn is_repeated_list(task: &Subtask) -> bool {
    if !has_list_identity(&identity(task)) {
        return false;
    }
    // Reuse the existing quantity grammar after normalizing English words.
    // The normalized clone is for classification only; promised rows stay intact.
    static WORD_COUNT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\b(three|four|five|six|seven|eight|nine|ten|eleven|twelve)\b").unwrap()
    });
    let text = task_text(task);
    let normalized = WORD_COUNT.replace_all(&text, |caps: &regex::Captures<'_>| match &caps[0] {
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        "ten" => "10",
        "eleven" => "11",
        _ => "12",
    });
    let mut count_task = task.clone();
    count_task.label = normalized.into_owned();
    count_task.elements = None;
    crate::subtask_completeness::expected_item_count(&count_task).is_some_and(|count| count >= 3)
}

fn preserves_photo_composition(brief: &str) -> bool {
    if has_any(
        brief,
        &[
            "full-screen",
            "fullscreen",
            "full screen",
            "全屏",
            "gallery",
            "图库",
            "画廊",
            "photo album",
            "相册",
            "carousel",
            "轮播",
            "editorial",
            "narrative",
            "storytelling",
            "叙事",
            "故事",
            "杂志",
            "参考",
            "复刻",
            "reference",
            "replicate",
            "large hero",
            "large photo",
            "large image",
            "oversized hero",
            "oversized photo",
            "big hero",
            "big photo",
            "大图展示",
            "超大主图",
            "大幅照片",
            "大幅主图",
            "half screen",
            "half the screen",
            "half-screen",
            "半屏",
            "占一半",
        ],
    ) {
        return true;
    }
    static PHOTO_SIZE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)(?:hero|photo|image|主图|头图|首图|封面图|图片|照片)[^,;。\n]{0,40}(?:\d+(?:\.\d+)?\s*(?:%|％|px|pixels?|percent|像素)|\d+\s*[:：/×x]\s*\d+|(?:height|高)\D{0,8}\d+)").unwrap()
    });
    static SIZE_BEFORE_PHOTO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)(?:\d+(?:\.\d+)?\s*(?:px|pixels?|像素|%|％)|\d+\s*[:：/×x]\s*\d+)[^,;。:\n]{0,20}(?:hero|photo|image|主图|头图|首图|封面图|照片)").unwrap()
    });
    static PROMINENT_PHOTO: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)\b(?:large|oversized|tall|dominant|big|panoramic)\b[^,;.\n]{0,20}(?:hero|photo|image|cover)|(?:主图|头图|照片).{0,6}(?:要大|大幅|占主导|突出展示)").unwrap()
    });
    PHOTO_SIZE.is_match(brief)
        || SIZE_BEFORE_PHOTO.is_match(brief)
        || PROMINENT_PHOTO.is_match(brief)
}

#[cfg(test)]
#[path = "plan_detail_budget_tests.rs"]
mod tests;
