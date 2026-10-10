//! Brief-side extraction for the plan-coverage gate: the sections a brief
//! explicitly enumerates, normalised to section names, plus the provenance of
//! items that only describe a sibling. Split out of [`crate::plan_coverage`]
//! to keep the spine under the file-size cap.

use crate::plan_coverage::{han_count, SYNONYM_GROUPS, TYPE_SUFFIXES};
use crate::plan_coverage_text::{is_motion_clause, strip_emphasis, strip_leading_article};
use regex::Regex;
use std::sync::LazyLock;

static CJK_INTRO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(需要有|包含|包括|含有|分为|有)([^。；\n]+)")
        .expect("valid CJK intro-list pattern")
});
static CJK_PARTS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[0-9]+|[一二三四五六七八九十]+)\s*个\s*(?:部分|区域|模块)[：:]\s*([^。；\n]+)")
        .expect("valid CJK numbered-parts pattern")
});
static CJK_COLON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[：:]\s*([^。；\n]*、[^。；\n]+)").expect("valid CJK colon-enumeration pattern")
});
static CJK_SPLIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"、|，|以及|及|与|和").expect("valid CJK item split"));
static EN_WITH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:with|including|containing)\s+(.+?)(?:[.;\n]|$)")
        .expect("valid English with/including list")
});
static EN_SECTIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bsections?:\s*(.+?)(?:[.;\n]|$)").expect("valid English sections: list")
});
static BULLET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(?:[-*•]|[0-9]+[.)]|[①②③④⑤⑥⑦⑧⑨⑩])\s+(.+?)\s*$")
        .expect("valid bullet/numbered line")
});
static PARENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"（[^）]*）|\([^)]*\)").expect("valid parenthetical strip"));
/// `共<count>…` tail: `四组设置分组共十二行` → `四组设置分组`.
static TRAILING_SHARED_TAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"共(?:[0-9]+|[零〇一二两三四五六七八九十]+)[^\s共（）()、，。；]*$")
        .expect("valid 共-count tail")
});
/// Trailing quantity phrase: `<count>[measure][标签|带开关|带涨跌|按钮|横向卡|纵向卡]`.
/// `底部导航四个标签` → `底部导航`, `播放控制五按钮` → `播放控制`, `车型选择三档横向卡` → `车型选择`.
static TRAILING_COUNT_PHRASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:[0-9]+|[零〇一二两三四五六七八九十]+)\s*[个条张项页行位款篇栏卡组档天]?\s*(?:标签|带开关|带涨跌|按钮|横向卡|纵向卡)?$",
    )
    .expect("valid trailing count phrase")
});
/// Descriptive `带…` tail without a count: `总资产卡带涨跌` → `总资产卡`, `账户区带头像` → `账户区`.
/// Only stripped when the remainder keeps ≥3 CJK chars (guard in `strip_trailing_junk`).
static TRAILING_DESC_TAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"带[^\s（）()、，。；]{1,2}$").expect("valid trailing 带 tail"));
/// Leading `<count><measure>` prefix: `四组设置分组` → `设置分组` (remainder ≥3 CJK chars).
static LEADING_COUNT_PHRASE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[0-9]+|[零〇一二两三四五六七八九十]+)\s*[个条张项页行位款篇栏卡组档天]")
        .expect("valid leading count phrase")
});
static CJK_YOU_PARTS_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:[0-9]+|[一二三四五六七八九十]+)\s*个\s*(?:部分|区域|模块)")
        .expect("valid 有-parts skip")
});

/// One enumerated item before normalisation. `detail_of` is the raw item it
/// describes when a conjunction continued that item's descriptor clause
/// (`右侧告警列表五条带时间与等级色标` → `等级色标` describes the alert list).
struct RawItem {
    text: String,
    detail_of: Option<String>,
}

/// A brief-required section. `detail_of` names the sibling section it was
/// enumerated as a detail of; such an item is still required (and re-planned)
/// but is never appended as a subtask of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredItem {
    pub name: String,
    pub detail_of: Option<String>,
}

fn plain(items: Vec<String>) -> impl Iterator<Item = RawItem> {
    items.into_iter().map(|text| RawItem {
        text,
        detail_of: None,
    })
}

/// Extract section nouns the brief EXPLICITLY enumerates. High-precision only.
pub fn required_items(brief: &str) -> Vec<RequiredItem> {
    let brief = strip_emphasis(brief);
    let brief = brief.as_str();
    // (raw item, declared): declared items come from an explicit `N个部分：` list,
    // so even short generic nouns (头部、内容) are trustworthy sections.
    let mut raw_items: Vec<(RawItem, bool)> = Vec::new();

    for captures in CJK_PARTS.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            raw_items.extend(
                split_cjk_items(list.as_str())
                    .into_iter()
                    .map(|item| (item, true)),
            );
        }
    }

    for captures in CJK_INTRO.captures_iter(brief) {
        let Some(intro) = captures.get(1) else {
            continue;
        };
        let Some(list) = captures.get(2) else {
            continue;
        };
        if intro.as_str() == "有" {
            // Per-card copy requirements are not additional page sections.
            let prefix = brief[..intro.start()].trim_end();
            if ["每页", "每张", "每屏", "各页", "各张", "各屏"]
                .iter()
                .any(|unit| prefix.ends_with(unit))
            {
                continue;
            }
            if let Some(prev) = brief[..intro.start()].chars().last() {
                if matches!(prev, '没' | '所' | '还' | '只' | '拥' | '持' | '已') {
                    continue;
                }
            }
            let trimmed = list.as_str().trim_start();
            if CJK_YOU_PARTS_PREFIX.is_match(trimmed) {
                continue;
            }
            if !list.as_str().contains('、') {
                continue;
            }
        } else if !has_cjk_list_separator(list.as_str()) {
            continue;
        }
        raw_items.extend(
            split_cjk_items(list.as_str())
                .into_iter()
                .map(|item| (item, false)),
        );
    }

    for captures in CJK_COLON.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            raw_items.extend(
                split_cjk_items(list.as_str())
                    .into_iter()
                    .map(|item| (item, false)),
            );
        }
    }

    for captures in EN_WITH.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            let text = list.as_str();
            if has_english_list_separator(text) {
                raw_items.extend(split_english_items(text).into_iter().map(|i| (i, false)));
            }
        }
    }

    for captures in EN_SECTIONS.captures_iter(brief) {
        if let Some(list) = captures.get(1) {
            raw_items.extend(
                split_english_items(list.as_str())
                    .into_iter()
                    .map(|item| (item, false)),
            );
        }
    }

    let bullets: Vec<String> = BULLET
        .captures_iter(brief)
        .filter_map(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        .collect();
    if bullets.len() >= 2 {
        raw_items.extend(plain(bullets).map(|item| (item, false)));
    }

    dedupe_normalized(raw_items.into_iter().filter_map(|(item, declared)| {
        let name = normalize_section(&item.text, declared)?;
        let detail_of = item.detail_of.map(|parent| {
            normalize_section(&parent, declared).unwrap_or_else(|| parent.trim().to_string())
        });
        Some(RequiredItem { name, detail_of })
    }))
}

/// Normalise a raw enumerated item into a required-section name.
///
/// Quantity/descriptor tails are stripped (`商家列表五个` → `商家列表`,
/// `底部导航四个标签` → `底部导航`) but the item itself is emitted WHOLE —
/// type suffixes (`列表`/`网格`/…) are never stripped here; the suffix-stripped
/// head is only used as an extra matching alias in [`section_covered`].
fn normalize_section(raw: &str, declared: bool) -> Option<String> {
    let mut text = raw
        .trim()
        .trim_matches(|ch| {
            matches!(
                ch,
                '"' | '\'' | '“' | '”' | '‘' | '’' | '。' | '.' | '；' | ';' | '*'
            )
        })
        .to_string();
    text = strip_leading_article(&text);
    // Paren-balance gate BEFORE the strip: an item whose parentheses don't
    // pair up is the residue of a clause that item-splitting cut in half
    // (`完成屏(时长`, `连续天数统计)`, `9)：封面`) — never a section name
    // (motion50 fix 3, app-01).
    if has_unbalanced_parens(&text) {
        return None;
    }
    text = PARENS.replace_all(&text, "").into_owned();
    let text = strip_trailing_junk(text.trim());
    let text = strip_leading_count(text.trim());
    let text = text.trim();
    if text.is_empty()
        || is_negated_section(text)
        || is_optional_method_instruction(text)
        || is_motion_clause(text)
        || is_descriptor_clause(text)
        || is_sentence_length(text)
        || is_short_field_noun(text, declared)
    {
        return None;
    }
    Some(text.to_string())
}

/// `（`/`（`-class and `(`/`)`-class parens must each pair up inside the item.
fn has_unbalanced_parens(text: &str) -> bool {
    let mut full = 0i32;
    let mut half = 0i32;
    for ch in text.chars() {
        match ch {
            '（' => full += 1,
            '）' => full -= 1,
            '(' => half += 1,
            ')' => half -= 1,
            _ => {}
        }
    }
    full != 0 || half != 0
}

/// A `含`-carrying item (`每张含标题`) is a per-item descriptor clause — the
/// verb phrase describes what REPEATED content must include, not a section
/// noun. The intro-anchored captures already scope enumeration extraction to
/// 包含/含有/colon-led segments; an intro verb INSIDE an item therefore marks a
/// descriptor, and demanding it as a section used to burn a pointless
/// PlanCoverageRetry (motion50 lane0/other-03).
fn is_descriptor_clause(text: &str) -> bool {
    text.contains('含')
}

/// Strip trailing quantity/descriptor tails: `共…` tail, then `<count>[measure][tail]`,
/// then a bare `带…` tail when the remainder keeps ≥3 CJK chars.
fn strip_trailing_junk(text: &str) -> String {
    let stripped = TRAILING_SHARED_TAIL.replace(text, "").into_owned();
    let stripped = TRAILING_COUNT_PHRASE
        .replace(stripped.trim(), "")
        .into_owned();
    let trimmed = stripped.trim();
    if let Some(mat) = TRAILING_DESC_TAIL.find(trimmed) {
        let head = trimmed[..mat.start()].trim();
        if han_count(head) >= 3 {
            return head.to_string();
        }
    }
    trimmed.to_string()
}

/// Strip a leading `<count><measure>` prefix (`四组设置分组` → `设置分组`)
/// only when the remainder keeps ≥3 CJK chars.
fn strip_leading_count(text: &str) -> String {
    let Some(mat) = LEADING_COUNT_PHRASE.find(text) else {
        return text.to_string();
    };
    let rest = text[mat.end()..].trim();
    if han_count(rest) >= 3 {
        rest.to_string()
    } else {
        text.to_string()
    }
}

/// A section the brief explicitly excluded (`不要底部导航`) is never required.
fn is_negated_section(text: &str) -> bool {
    text.starts_with("不要")
        || text.starts_with("不用")
        || text.starts_with("无需")
        || text.starts_with("别加")
        || text.starts_with("别放")
        || text.starts_with("没有")
}

/// Optional ways to render a section are not extra sections. Keep capability
/// nouns such as `可编辑表格` and explicitly requested photo sections intact.
fn is_optional_method_instruction(text: &str) -> bool {
    let text = text.trim();
    // `可选用户列表` is a selectable-user section, not the verb `可选用`.
    if text.starts_with("可选用户") {
        return false;
    }
    if [
        "可使用",
        "可以使用",
        "可采用",
        "可以采用",
        "可选用",
        "可以选用",
    ]
    .iter()
    .any(|prefix| {
        text.strip_prefix(prefix)
            .is_some_and(|rest| !rest.starts_with('的'))
    }) {
        return true;
    }
    let lower = text.to_ascii_lowercase();
    [
        "may use ",
        "can use ",
        "you may use ",
        "you can use ",
        "optionally use ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

/// Very short CJK items are usually text fields, not sections (`歌名`, `时间`,
/// `搜索`). Kept when they carry a type suffix (`区块`) or are known section
/// nouns from the synonym table (`日程`), except `搜索`, which stays too
/// generic. Declared `N个部分：` items bypass this screen.
fn is_short_field_noun(text: &str, declared: bool) -> bool {
    if declared {
        return false;
    }
    let han = han_count(text);
    if han == 0 || han > 2 {
        return false;
    }
    let lower = text.to_lowercase();
    if TYPE_SUFFIXES
        .iter()
        .any(|suffix| lower != *suffix && lower.ends_with(suffix))
    {
        return false;
    }
    if text == "搜索" {
        return true;
    }
    han == 2 && SYNONYM_GROUPS.iter().any(|group| group.contains(&text))
}

fn is_sentence_length(text: &str) -> bool {
    let han = han_count(text);
    let words = text.split_whitespace().count();
    han > 12 || (han == 0 && words > 5)
}

fn has_cjk_list_separator(text: &str) -> bool {
    text.contains('、')
        || text.contains('，')
        || text.contains('和')
        || text.contains('与')
        || text.contains('及')
}

fn has_english_list_separator(text: &str) -> bool {
    text.contains(',') || text.to_ascii_lowercase().contains(" and ")
}

/// Whether a separator is a conjunction that can continue a descriptor list
/// (`带时间与等级色标`), unlike the enumeration marks `、`/`，`.
fn is_conjunction(sep: &str) -> bool {
    matches!(sep, "与" | "和" | "及" | "以及")
}

/// `右侧告警列表五条带时间` opens a `带…` descriptor: whatever a conjunction
/// adds after it is another thing the list carries, not a new section.
fn opens_descriptor(item: &str) -> bool {
    item.rsplit_once('带')
        .is_some_and(|(_, tail)| !tail.trim().is_empty())
}

/// English twin of [`opens_descriptor`]: `header with search` — an `and`
/// after it (`… and avatar`) adds to what the header carries.
fn opens_english_descriptor(item: &str) -> bool {
    item.to_ascii_lowercase().contains(" with ")
}

/// Provenance for an item that follows `prev` across `sep`: the item `prev`
/// describes when `prev` opened a descriptor clause (or itself continued one).
fn descriptor_parent(
    prev: Option<&RawItem>,
    sep: &str,
    opens: impl Fn(&str) -> bool,
) -> Option<String> {
    let prev = prev?;
    if !is_conjunction(sep) {
        return None;
    }
    if let Some(parent) = &prev.detail_of {
        return Some(parent.clone());
    }
    opens(&prev.text).then(|| prev.text.clone())
}

fn split_cjk_items(list: &str) -> Vec<RawItem> {
    // Balanced parentheticals are per-section notes (`（六张，超出屏幕裁剪）`);
    // splitting inside them cut the section in half and the paren-balance
    // screen then dropped BOTH halves (arena-m02 lost its course rail).
    let list = PARENS.replace_all(list, "");
    // A comma-delimited optional method clause can contain its own material
    // list (`可使用照片和插画`). Drop that entire clause, but resume extracting
    // required sections after the next comma, such as `报名入口`.
    let list = list
        .split('，')
        .filter(|clause| !is_optional_method_instruction(clause))
        .collect::<Vec<_>>()
        .join("，");
    let list = list.as_str();
    // Split on every separator, remembering the conjunction that followed each
    // piece: a `与/和` pair with a ≤2-char side is kept whole (they are the text
    // fields of one section, `歌名与歌手`), not split into bare nouns.
    let mut pieces: Vec<(&str, &str)> = Vec::new();
    let mut start = 0;
    for sep in CJK_SPLIT.find_iter(list) {
        pieces.push((list[start..sep.start()].trim(), sep.as_str()));
        start = sep.end();
    }
    pieces.push((list[start..].trim(), ""));

    let mut items: Vec<RawItem> = Vec::new();
    let mut index = 0;
    while index < pieces.len() {
        let sep_before = index.checked_sub(1).map_or("", |prev| pieces[prev].1);
        let (first, mut sep) = pieces[index];
        index += 1;
        // The rest of an anti-fabrication list (prices, user counts, etc.)
        // describes forbidden claims, not sections the planner must draw.
        if is_non_content_instruction(first) {
            break;
        }
        if first.is_empty() {
            continue;
        }
        let mut item = first.to_string();
        while matches!(sep, "与" | "和") && index < pieces.len() {
            let (next, next_sep) = pieces[index];
            if next.is_empty() || (han_count(&item) > 2 && han_count(next) > 2) {
                break;
            }
            item.push_str(sep);
            item.push_str(next);
            sep = next_sep;
            index += 1;
        }
        let detail_of = descriptor_parent(items.last(), sep_before, opens_descriptor);
        items.push(RawItem {
            text: item,
            detail_of,
        });
    }
    items
}

fn is_non_content_instruction(text: &str) -> bool {
    [
        "不增加",
        "不新增",
        "不添加",
        "不要编造",
        "不得编造",
        "不编造",
        "不要虚构",
        "不得虚构",
        "不虚构",
        "不要捏造",
        "不要杜撰",
    ]
    .iter()
    .any(|prefix| text.starts_with(prefix))
}

fn split_english_items(list: &str) -> Vec<RawItem> {
    let mut items: Vec<RawItem> = Vec::new();
    for comma_part in list.split(',') {
        let trimmed = comma_part
            .trim()
            .trim_start_matches("and ")
            .trim_start_matches("And ")
            .trim();
        if trimmed.is_empty() || is_optional_method_instruction(trimmed) {
            continue;
        }
        let mut sep_before = "";
        for and_part in trimmed.split(" and ") {
            let item = and_part.trim();
            if is_optional_method_instruction(item) {
                break;
            }
            if !item.is_empty() {
                let detail_of =
                    descriptor_parent(items.last(), sep_before, opens_english_descriptor);
                items.push(RawItem {
                    text: item.to_string(),
                    detail_of,
                });
            }
            sep_before = "和";
        }
    }
    items
}

/// Case-insensitive dedupe, first occurrence wins — except that an item the
/// brief ALSO names standalone is a section, not a detail.
fn dedupe_normalized(items: impl Iterator<Item = RequiredItem>) -> Vec<RequiredItem> {
    let mut out: Vec<RequiredItem> = Vec::new();
    for item in items {
        match out
            .iter_mut()
            .find(|existing| existing.name.eq_ignore_ascii_case(&item.name))
        {
            Some(existing) => {
                if item.detail_of.is_none() {
                    existing.detail_of = None;
                }
            }
            None => out.push(item),
        }
    }
    out
}
