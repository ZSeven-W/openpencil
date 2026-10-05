//! Replacement words are data for routing, including unquoted novice input.
//! Only explicit text-property replacements are masked. Independent commands
//! following punctuation or conjunctions remain visible to the scope resolver.

use super::{contains_phrase, requests_edit};

const REPLACEMENTS: &[&str] = &[
    "改成",
    "改为",
    "改為",
    "换成",
    "換成",
    "替换为",
    "替換為",
    "变成",
    "變成",
    " to ",
    " with ",
];
const TEXT_FIELDS: &[&str] = &[
    "标题",
    "標題",
    "文字",
    "文案",
    "内容",
    "內容",
    "名称",
    "名稱",
    "标签",
    "標籤",
    "说明",
    "說明",
    "描述",
    "title",
    "heading",
    "label",
    "text",
    "copy",
    "caption",
    "content",
    "name",
    "button",
    "cta",
    "subtitle",
    "description",
    "body",
    "按钮",
    "按鈕",
];
const JOINS: &[&str] = &[
    " and ", " then ", "然后", "然後", "并且", "並且", "同时", "同時", "并", "並", "再把", "再将",
    "再將",
];

fn starts_word(text: &str, word: &str) -> bool {
    text.strip_prefix(word).is_some_and(|tail| {
        tail.chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
    })
}

fn statement_start(text: &str) -> bool {
    let mut text = text.trim_start();
    // Polite and sequential prefixes do not make a second action copy data.
    for _ in 0..4 {
        let next = [
            "please ", "and ", "then ", "请", "請", "然后", "然後", "同时", "同時", "并且", "並且",
            "并", "並", "再",
        ]
        .iter()
        .find_map(|prefix| text.strip_prefix(prefix));
        match next {
            Some(next) => text = next.trim_start(),
            None => break,
        }
    }
    [
        "change",
        "modify",
        "update",
        "adjust",
        "resize",
        "move",
        "restyle",
        "refine",
        "fix",
        "tweak",
        "edit",
        "replace",
        "remove",
        "delete",
        "add",
        "set",
        "rename",
        "create",
        "generate",
        "design",
        "draw",
        "build",
        "regenerate",
        "redesign",
        "go",
        "why",
        "how",
        "what",
    ]
    .iter()
    .any(|word| starts_word(text, word))
        || [
            "不要",
            "不修改",
            "只提供建议",
            "只給建議",
            "只给建议",
            "新增",
            "添加",
            "新建",
            "重新生成",
            "重新画",
            "重新設計",
            "重新设计",
            "修改",
            "调整",
            "調整",
            "改",
            "删除",
            "刪除",
            "移除",
            "为什么",
            "為什麼",
            "怎么",
            "怎麼",
            "如何",
        ]
        .iter()
        .any(|word| text.starts_with(word))
        || (text.starts_with('把') || text.starts_with('将') || text.starts_with('將'))
            && requests_edit(text)
        || preservation_start(text)
        || text.starts_with("do not ")
        || text.starts_with("don't ")
}

fn preservation_start(text: &str) -> bool {
    let text = text.trim_start();
    (text.starts_with("keep ") || text.starts_with("leave "))
        && ["unchanged", "the same", "as is", "other", "everything else"]
            .iter()
            .any(|term| text.contains(term))
        || (text.starts_with("保留") || text.starts_with("保持"))
            && [
                "不变", "不變", "原样", "原樣", "其他", "其它", "其余", "其餘", "原稿",
            ]
            .iter()
            .any(|term| text.contains(term))
}

fn control_boundaries(text: &str, start: usize, end: usize) -> Vec<usize> {
    let mut boundaries = Vec::new();
    for (relative, ch) in text[start..end].char_indices() {
        let offset = start + relative;
        if matches!(
            ch,
            ',' | '，' | ';' | '；' | '。' | '\n' | '?' | '？' | '!' | '！'
        ) && statement_start(&text[offset + ch.len_utf8()..end])
        {
            boundaries.push(offset);
        }
    }
    for join in JOINS {
        for (relative, _) in text[start..end].match_indices(join) {
            let offset = start + relative;
            // 再把 / 再将 include the next action's prefix.
            let tail = if join.starts_with('再') {
                &text[offset..end]
            } else {
                &text[offset + join.len()..end]
            };
            if statement_start(tail) {
                boundaries.push(offset);
            }
        }
    }
    for marker in [" keep ", " leave ", "保留", "保持"] {
        for (relative, _) in text[start..end].match_indices(marker) {
            let offset = start + relative;
            if preservation_start(text[offset..end].trim_start()) {
                boundaries.push(offset);
            }
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
}

pub(super) fn mask_replacement_copy(text: &str) -> String {
    let mut cursor = 0;
    let mut ranges = Vec::new();
    while cursor < text.len() {
        let Some((offset, marker)) = REPLACEMENTS
            .iter()
            .filter_map(|marker| {
                text[cursor..]
                    .find(marker)
                    .map(|offset| (cursor + offset, *marker))
            })
            .min_by_key(|(offset, _)| *offset)
        else {
            break;
        };
        let prefix_start = control_boundaries(text, cursor, offset)
            .last()
            .copied()
            .unwrap_or(cursor);
        let prefix = &text[prefix_start..offset];
        let value_start = offset + marker.len();
        let text_property = TEXT_FIELDS
            .iter()
            .any(|field| contains_phrase(prefix, field))
            || contains_phrase(prefix, "rename");
        if !(requests_edit(prefix) || requests_edit(marker)) || !text_property {
            cursor = value_start;
            continue;
        }
        let value_end = control_boundaries(text, value_start, text.len())
            .into_iter()
            .find(|boundary| *boundary > value_start)
            .unwrap_or(text.len());
        if value_end > value_start {
            ranges.push(value_start..value_end);
        }
        cursor = value_end.max(value_start);
    }
    let mut masked = String::new();
    let mut previous = 0;
    for range in ranges {
        masked.push_str(&text[previous..range.start]);
        masked.push(' ');
        previous = range.end;
    }
    masked.push_str(&text[previous..]);
    masked
}
