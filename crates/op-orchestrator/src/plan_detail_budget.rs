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
// A merged section owns controls and all title/meta/badges, not just a photo.
// Leave an 80px control/breathing band above the complete summary floor;
// native measurement still decides whether the generated composition fits.
const MIN_MERGED_HERO: f64 = MIN_SUMMARY + 80.0;
const MAX_MERGED_HERO: f64 = MIN_HERO + MIN_SUMMARY;
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
        || !requests_compact_photo(&brief)
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
    let actions = matches(is_standalone_pinned_action);
    let ([hero], [list], [action]) = (heroes.as_slice(), lists.as_slice(), actions.as_slice())
    else {
        return;
    };
    let summary = match summaries.as_slice() {
        [summary] if *summary == hero + 1 && *list == summary + 1 => Some(*summary),
        [] if *list == hero + 1 && is_merged_photo_summary(&plan.subtasks[indices[*hero]]) => None,
        _ => return,
    };
    // Ambiguous ordering or mixed sections need a new composition, not an
    // automatic rewrite. Only compact navigation may precede the photo.
    if *action != list + 1
        || *action + 1 != indices.len()
        || indices[..*hero].iter().any(|&index| {
            let task = &plan.subtasks[index];
            !is_compact_navigation(task)
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
    let summary_index = summary.map(|position| indices[position]);
    let original_hero = plan.subtasks[hero_index].region.height;
    let (minimum, maximum) = if summary_index.is_some() {
        (MIN_HERO, MAX_HERO)
    } else {
        (MIN_MERGED_HERO, MAX_MERGED_HERO)
    };
    if original_hero < minimum {
        return;
    }
    let summary_height = summary_index
        .map(|index| plan.subtasks[index].region.height.max(MIN_SUMMARY))
        .unwrap_or(0.0);
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
    let hero_height = remaining.floor().min(maximum).min(original_hero);
    // Do not solve infeasibility by shrinking controls, copy or all list rows.
    if !hero_height.is_finite() || hero_height < minimum {
        return;
    }
    plan.subtasks[hero_index].region.height = hero_height;
    if let Some(summary_index) = summary_index {
        plan.subtasks[summary_index].region.height = summary_height;
        crate::first_viewport::mark_detail_tasks(plan, &[hero_index, summary_index]);
    } else {
        crate::first_viewport::mark_detail_tasks(plan, &[hero_index]);
        plan.subtasks[hero_index].elements.get_or_insert_default().push_str(
            "\nMERGED DETAIL COMPOSITION: This budget covers the entire hero section, including the back control, complete title, metadata and every difficulty badge. Recompose the photo and copy as needed; keep all supplied labels and readable type. Do not satisfy the budget by shrinking text, deleting badges, clipping overlays or forcing the parent height. The photo is supporting content, not an extra full-height section.",
        );
    }
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
    has_pinning(text)
        && has_any(text, &["bottom", "底部", "吸底"])
        && has_any(text, &["button", "cta", "action", "按钮", "操作"])
}

fn has_pinning(text: &str) -> bool {
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
}

fn is_standalone_pinned_action(task: &Subtask) -> bool {
    let identity = identity(task);
    let text = task_text(task);
    // The brief already requires a bottom action, and reserve_screen requires
    // this section to be last. A pinned back control plus a bottom scrim in
    // the hero must not compete with that standalone action section.
    (44.0..=160.0).contains(&task.region.height)
        && has_pinning(&text)
        && has_any(&text, &["button", "cta", "action", "按钮", "操作"])
        && has_any(
            &identity,
            &[
                "button",
                "cta",
                "action",
                "footer",
                "bottom bar",
                "按钮",
                "操作",
                "底栏",
                "页脚",
            ],
        )
        && !is_photo_hero(task)
        && !is_summary(task)
        && !has_list_identity(&identity)
        && !has_any(
            &identity,
            &[
                "navigation",
                "navbar",
                "sidebar",
                "导航",
                "侧栏",
                "返回",
                "叠层",
                "主图",
                "照片",
                "图片",
            ],
        )
        && !has_overlay_control_identity(task, &identity)
}

fn has_overlay_control_identity(task: &Subtask, identity: &str) -> bool {
    // Preserve camel-case boundaries before lowercasing. Planner identities
    // such as backButton/PhotoOverlayButton still name floating controls.
    let separated = format!("{} {}", task.id, task.label).chars().fold(
        String::new(),
        |mut words, character| {
            if character.is_ascii_uppercase() {
                words.push(' ');
            }
            words.extend(character.to_lowercase());
            words
        },
    );
    [identity, separated.as_str()].iter().any(|text| {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| {
                matches!(
                    word,
                    "back" | "overlay" | "hero" | "photo" | "image" | "cover"
                )
            })
    })
}

fn is_compact_navigation(task: &Subtask) -> bool {
    let identity = identity(task);
    task.region.height <= 96.0
        && has_any(&identity, &["header", "navigation", "导航", "顶栏"])
        && !has_list_identity(&identity)
        && !has_any(
            &identity,
            &[
                "hero", "photo", "image", "cover", "promo", "offer", "cta", "action", "主图",
                "照片", "图片", "推广", "促销", "操作",
            ],
        )
}

fn badges_have_other_owner(text: &str) -> bool {
    // A title overlaid on a photo does not pull a separately owned badge
    // group into that hero. Keep the ownership test local to its clause.
    static FOREIGN_BADGES: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)\b(?:badges?|chips?)\b[^,.;\n]{0,72}\b(?:belong(?:s)?\s+to|owned\s+by|live\s+in|inside|within|in|on)\b[^,.;\n]{0,40}\b(?:list|exercises?|another|other|separate|summary)\b|(?:标签|徽章)[^，。；;\n]{0,32}(?:属于|归属|位于|放在|放入|由)[^，。；;\n]{0,24}(?:列表|清单|别处|其他|另|独立)").unwrap()
    });
    FOREIGN_BADGES.is_match(text)
}

fn is_merged_photo_summary(task: &Subtask) -> bool {
    let identity = identity(task);
    let text = task_text(task);
    // Only an explicit shared owner may use a combined budget. Merely
    // mentioning another section's title or badges does not transfer them.
    has_any(&identity, &["title", "标题"])
        && has_any(&identity, &["meta", "summary", "摘要", "信息", "简介"])
        && has_any(&text, &["badge", "chip", "标签", "徽章"])
        && has_any(
            &text,
            &[
                "owned by this section",
                "owned here",
                "inside hero",
                "on hero",
                "on scrim",
                "overlaid",
                "叠加",
                "叠层",
                "主图内",
                "本区",
            ],
        )
        && !has_any(
            &text,
            &[
                "no title",
                "no overlaid title",
                "without title",
                "no meta",
                "without meta",
                "no badge",
                "without badge",
                "separate title",
                "separate meta",
                "separate badge",
                "another section",
                "无标题",
                "不含标题",
                "独立标题",
                "独立摘要",
                "独立标签",
            ],
        )
        && !badges_have_other_owner(&text)
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

fn requests_compact_photo(brief: &str) -> bool {
    // The three-row budget is an explicit-compact contract. A plain "hero
    // image" detail brief keeps the planner's authored composition (the
    // archetype's 35–45% hero with scrim title); only a brief that itself
    // asks for a compact or small photo may trade hero area for rows.
    has_any(
        brief,
        &[
            "compact",
            "small hero",
            "small photo",
            "small image",
            "thumbnail header",
            "小图",
            "紧凑",
        ],
    )
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
