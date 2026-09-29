//! Drop "umbrella" subtasks: a planned section that owns no content of its
//! own and only wraps other subtasks of the same plan.
//!
//! Plans are a flat list, so a subtask like `main (主内容区) elements="(container)"`
//! or `main (Main Column) elements="container for header, filter bar, table,
//! pagination"` is handed to its own sub-agent, which dutifully draws the
//! whole main area — and then each real sibling appends its section again.
//! The result is a dashboard with the toolbar, KPIs, charts and table twice
//! (GLM-5.3-Flash arena-d01/d02, 0926–0927).
//!
//! A false drop loses a real section, which is worse than the duplicate, so a
//! subtask is dropped only when it is provably redundant:
//! - its elements, once container/layout framing is stripped, are either a
//!   bare marker (`(container)`) or a list whose every item names a sibling;
//! - at least two other subtasks share its screen (never across screens);
//! - it is not navigation chrome (sidebar / nav / header / toolbar);
//! - every `covers` entry it carries finds a sibling to take it.

use crate::design_type::contains_word;
use crate::plan::{OrchestratorPlan, Subtask};
use crate::plan_coverage::is_han;
use crate::plan_coverage_text::{contains_term, strip_leading_article};
use regex::Regex;
use std::sync::LazyLock;

/// Words that, standing alone, describe structure rather than content.
const FRAMING_WORDS: &[&str] = &[
    "container",
    "main container",
    "content container",
    "wrapper",
    "layout",
    "vertical layout",
    "horizontal layout",
    "stack",
    "vertical stack",
    "column",
    "容器",
    "主容器",
    "内容容器",
    "布局",
    "垂直布局",
    "水平布局",
    "纵向布局",
    "横向布局",
    "纵向排列",
    "横向排列",
    "垂直排列",
    "水平排列",
];

/// Leading phrases that introduce the wrapped children (`container for …`).
const FRAMING_LEADS: &[&str] = &[
    "container for",
    "container of",
    "container with",
    "wrapper for",
    "wrapper of",
    "wraps",
    "wrapping",
    "contains",
    "containing",
    "holds",
    "包含",
    "包裹",
    "容纳",
];

/// A `<head>: <list>` prefix is framing when its head carries one of these
/// (`垂直布局：…`, `layout: …`, `Main container: …`).
const FRAMING_HEAD_CUES: &[&str] = &[
    "布局",
    "排列",
    "容器",
    "包含",
    "结构",
    "layout",
    "container",
    "wrapper",
    "stack",
    "contains",
    "structure",
];

/// Navigation chrome is never dropped: its elements are its own content even
/// when the planner also split parts of it into siblings.
const CHROME_CUES_ASCII: &[&str] = &[
    "sidebar",
    "side",
    "nav",
    "navbar",
    "navigation",
    "menu",
    "header",
    "topbar",
    "toolbar",
    "top bar",
    "tab bar",
    "tabbar",
    "footer",
    "rail",
];
const CHROME_CUES_CJK: &[&str] = &[
    "导航",
    "侧栏",
    "侧边",
    "页头",
    "顶栏",
    "菜单",
    "工具栏",
    "标签栏",
    "页脚",
    "底栏",
];

/// Generic section suffixes set aside before comparing names, so `KPI 卡行`
/// names `KPI 卡片行` and `图表行` names `图表区`.
const GENERIC_SUFFIXES_CJK: &[&str] = &[
    "区域", "区块", "板块", "模块", "部分", "卡片", "卡", "行", "区", "组", "栏",
];
const GENERIC_SUFFIXES_ASCII: &[&str] = &[
    "section", "sections", "row", "rows", "area", "region", "block", "group", "bar", "strip",
    "panel", "module", "zone",
];

/// Remove umbrella subtasks in place. Returns the number removed.
pub(crate) fn drop_umbrella_subtasks(plan: &mut OrchestratorPlan) -> usize {
    let mut removed = 0;
    // One at a time against the current plan: two umbrellas naming each
    // other must not both vanish on the strength of the other's presence.
    while let Some(index) = (0..plan.subtasks.len()).find(|&i| is_umbrella(plan, i)) {
        if !transfer_covers(plan, index) {
            // Unreachable in practice (`is_umbrella` already checked every
            // entry has a home); keep the subtask rather than open a hole.
            break;
        }
        let dropped = plan.subtasks.remove(index);
        eprintln!(
            "[PLAN] normalize: dropped umbrella subtask {}({}) elements={:?}",
            dropped.id,
            dropped.label,
            dropped.elements.as_deref().unwrap_or_default()
        );
        removed += 1;
    }
    removed
}

fn same_screen(a: &Subtask, b: &Subtask) -> bool {
    let screen = |st: &Subtask| {
        st.screen
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
    };
    screen(a) == screen(b)
}

fn siblings(plan: &OrchestratorPlan, index: usize) -> Vec<&Subtask> {
    let target = &plan.subtasks[index];
    plan.subtasks
        .iter()
        .enumerate()
        .filter(|(i, st)| *i != index && same_screen(st, target))
        .map(|(_, st)| st)
        .collect()
}

fn is_umbrella(plan: &OrchestratorPlan, index: usize) -> bool {
    let st = &plan.subtasks[index];
    // An absent / blank `elements` is how old models leave a real section.
    let Some(elements) = st.elements.as_deref().filter(|e| !e.trim().is_empty()) else {
        return false;
    };
    let siblings = siblings(plan, index);
    if siblings.len() < 2 {
        return false;
    }
    // The planner saying outright that the subtask holds nothing of its own
    // ("无独立内容，仅作为顶部栏与看板的纵向容器") is the plainest proof there is;
    // arena w01 still drew the whole main column from it, then appended the
    // bar and board again beside it and starved the board to 50px.
    if declares_no_content(elements) {
        return covers_have_home(st, &[], &siblings);
    }
    let items = content_items(elements);
    if !items.is_empty() && is_chrome(st) {
        return false;
    }
    let all_named = items
        .iter()
        .all(|item| siblings.iter().any(|sib| names_subtask(item, sib)));
    all_named && covers_have_home(st, &items, &siblings)
}

fn is_chrome(st: &Subtask) -> bool {
    let identity = format!("{} {}", st.id.replace(['-', '_'], " "), st.label).to_lowercase();
    CHROME_CUES_ASCII
        .iter()
        .any(|cue| contains_word(&identity, cue))
        || CHROME_CUES_CJK.iter().any(|cue| identity.contains(cue))
}

/// The sibling that would take a `covers` entry: one already claiming it, one
/// whose name it is, or the sibling named by the umbrella item it names.
fn covers_home<'a>(entry: &str, items: &[String], siblings: &[&'a Subtask]) -> Option<&'a Subtask> {
    let key = compact(&normalize_name(entry));
    if key.is_empty() {
        return None;
    }
    let claimed = siblings.iter().find(|sib| {
        sib.covers
            .iter()
            .flatten()
            .any(|c| compact(&normalize_name(c)) == key)
    });
    claimed
        .or_else(|| siblings.iter().find(|sib| names_subtask(entry, sib)))
        .or_else(|| {
            items
                .iter()
                .filter(|item| names_match(entry, item))
                .find_map(|item| siblings.iter().find(|sib| names_subtask(item, sib)))
        })
        .copied()
}

fn covers_have_home(st: &Subtask, items: &[String], siblings: &[&Subtask]) -> bool {
    st.covers
        .iter()
        .flatten()
        .filter(|entry| !entry.trim().is_empty())
        .all(|entry| covers_home(entry, items, siblings).is_some())
}

/// Move the umbrella's `covers` onto the siblings that take them, so the
/// coverage diagnostics never see a brief section vanish with it.
fn transfer_covers(plan: &mut OrchestratorPlan, index: usize) -> bool {
    let umbrella = &plan.subtasks[index];
    let items = content_items(umbrella.elements.as_deref().unwrap_or_default());
    let entries: Vec<String> = umbrella
        .covers
        .iter()
        .flatten()
        .filter(|entry| !entry.trim().is_empty())
        .cloned()
        .collect();
    let mut moves: Vec<(String, String)> = Vec::new();
    {
        let siblings = siblings(plan, index);
        for entry in entries {
            let Some(home) = covers_home(&entry, &items, &siblings) else {
                return false;
            };
            moves.push((home.id.clone(), entry));
        }
    }
    for (home_id, entry) in moves {
        let Some(home) = plan
            .subtasks
            .iter_mut()
            .enumerate()
            .find(|(i, st)| *i != index && st.id == home_id)
            .map(|(_, st)| st)
        else {
            return false;
        };
        let covers = home.covers.get_or_insert_with(Vec::new);
        let key = compact(&normalize_name(&entry));
        if !covers.iter().any(|c| compact(&normalize_name(c)) == key) {
            covers.push(entry);
        }
    }
    true
}

/// The content items of an `elements` string with container/layout framing
/// stripped. Empty for a bare marker such as `(container)`.
pub(crate) fn content_items(elements: &str) -> Vec<String> {
    let mut text = strip_wrapping_brackets(elements.trim()).to_string();
    if let Some((head, tail)) = text.split_once([':', '：']) {
        let head_lower = head.to_lowercase();
        let head_is_list = head.contains([',', '，', '、', ';', '；']);
        if !head_is_list && FRAMING_HEAD_CUES.iter().any(|cue| head_lower.contains(cue)) {
            text = tail.to_string();
        }
    }
    split_items(&text)
        .into_iter()
        .filter_map(|item| clean_item(&item))
        .collect()
}

fn split_items(text: &str) -> Vec<String> {
    let mut spaced = format!(" {text} ");
    for joiner in [" and ", " AND ", " And "] {
        spaced = spaced.replace(joiner, ",");
    }
    spaced
        .split([
            '、', '，', ',', '；', ';', '/', '+', '＋', '&', '和', '与', '及',
        ])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn clean_item(raw: &str) -> Option<String> {
    let mut item = strip_wrapping_brackets(raw.trim()).trim().to_string();
    loop {
        let lower = item.to_lowercase();
        let lead = FRAMING_LEADS.iter().find(|lead| {
            lower.starts_with(*lead)
                && lower[lead.len()..]
                    .chars()
                    .next()
                    .is_none_or(|ch| !ch.is_ascii_alphanumeric())
        });
        // Lowercasing may change byte offsets outside ASCII; slice only on a
        // boundary both strings agree on.
        let rest = lead.and_then(|lead| item.get(lead.len()..));
        match rest {
            Some(rest) => item = rest.trim_start_matches([' ', ':', '：']).to_string(),
            None => break,
        }
    }
    for tail in ["等", "etc.", "etc"] {
        if let Some(head) = item.strip_suffix(tail) {
            item = head.trim().to_string();
        }
    }
    // A whole-item framing phrase ("垂直容器") must be recognised before its
    // tail is stripped, or it would survive as a bare modifier ("垂直").
    if is_framing_phrase(&normalize_name(item.trim())) {
        return None;
    }
    item = strip_framing_tail(item.trim()).to_string();
    item = strip_leading_article(item.trim());
    let normalized = normalize_name(&item);
    if normalized.is_empty()
        || FRAMING_WORDS.contains(&normalized.as_str())
        || is_framing_phrase(&normalized)
    {
        return None;
    }
    Some(item)
}

/// Modifiers that only say how a wrapper is laid out, never what it holds.
const FRAMING_MODIFIERS_ASCII: &[&str] = &[
    "vertical",
    "horizontal",
    "main",
    "content",
    "flex",
    "flexbox",
    "column",
    "columns",
    "row",
    "full",
    "width",
    "full-width",
    "outer",
    "inner",
    "page",
    "scrollable",
    "scroll",
];
/// Nouns that make a phrase framing: the wrapper itself.
const FRAMING_CORES_ASCII: &[&str] = &["container", "wrapper", "stack", "layout"];
const FRAMING_MODIFIERS_CJK: &[&str] = &[
    "垂直", "水平", "纵向", "横向", "竖向", "主", "内容", "堆叠", "弹性", "滚动", "外层", "整体",
];
const FRAMING_CORES_CJK: &[&str] = &["容器", "布局", "排列"];

/// A phrase built only from layout modifiers around a wrapper noun
/// ("vertical container", "main content wrapper", "垂直堆叠容器") names
/// structure, not content — the combinations are too many to list whole.
fn is_framing_phrase(normalized: &str) -> bool {
    let tokens: Vec<&str> = normalized
        .split(|ch: char| ch.is_whitespace() || ch == '-' || ch == '_')
        .filter(|token| !token.is_empty())
        .collect();
    if !tokens.is_empty() && tokens.iter().all(|token| token.is_ascii()) {
        return tokens
            .iter()
            .any(|token| FRAMING_CORES_ASCII.contains(token))
            && tokens.iter().all(|token| {
                FRAMING_CORES_ASCII.contains(token) || FRAMING_MODIFIERS_ASCII.contains(token)
            });
    }
    let compacted: String = normalized
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    let Some(core) = FRAMING_CORES_CJK
        .iter()
        .find(|core| compacted.ends_with(**core))
    else {
        return false;
    };
    let mut rest = &compacted[..compacted.len() - core.len()];
    'strip: while !rest.is_empty() {
        for modifier in FRAMING_MODIFIERS_CJK.iter().chain(FRAMING_CORES_CJK) {
            if let Some(shorter) = rest.strip_prefix(modifier) {
                rest = shorter;
                continue 'strip;
            }
        }
        return false;
    }
    true
}

/// Strip a trailing "what this list is" tail from the last item: the planner
/// closes the list with it ("…、告警列表三块的布局容器"), so without this the
/// last sibling name never matches and the umbrella survives (arena d03).
/// Explicit "this subtask has no content of its own" declarations.
const NO_CONTENT_CUES: &[&str] = &[
    "无独立内容",
    "没有独立内容",
    "无自身内容",
    "无实际内容",
    "不含内容",
    "不包含内容",
    "no content of its own",
    "no own content",
    "has no content",
    "holds no content",
    "contains no content",
];

fn declares_no_content(elements: &str) -> bool {
    let lower = elements.to_lowercase();
    NO_CONTENT_CUES.iter().any(|cue| lower.contains(cue))
}

fn strip_framing_tail(item: &str) -> &str {
    static TAIL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:[0-9零一二两三四五六七八九十]+\s*(?:块|个|部分|大块|区块|模块))?\s*的?\s*(?:整体|外层|主)?\s*(?:布局容器|容器|布局)$",
        )
        .expect("valid framing-tail pattern")
    });
    let Some(found) = TAIL.find(item) else {
        return item;
    };
    let head = item[..found.start()].trim_end();
    // A lone "容器" item is handled as a framing word; keep the tail when
    // stripping it would leave too little to name a sibling.
    if head.chars().filter(|ch| is_han(*ch)).count() < 2 && !head.is_ascii() {
        return item;
    }
    if head.is_empty() {
        return item;
    }
    head
}

fn strip_wrapping_brackets(mut text: &str) -> &str {
    const PAIRS: [(char, char); 4] = [('(', ')'), ('（', '）'), ('[', ']'), ('【', '】')];
    loop {
        let trimmed = text.trim();
        let wrapped = PAIRS.iter().find(|(open, close)| {
            trimmed.starts_with(*open)
                && trimmed.ends_with(*close)
                && trimmed[open.len_utf8()..trimmed.len() - close.len_utf8()]
                    .find([*open, *close])
                    .is_none()
        });
        match wrapped {
            Some((open, close)) => {
                text = &trimmed[open.len_utf8()..trimmed.len() - close.len_utf8()];
            }
            None => return trimmed,
        }
    }
}

/// Lowercase, keep letters / digits / Han, collapse everything else to one
/// space.
fn normalize_name(text: &str) -> String {
    let lowered = strip_leading_article(text.trim()).to_lowercase();
    let mut out = String::new();
    for ch in lowered.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.trim().to_string()
}

fn compact(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// The name with generic section suffixes and a plural `s` set aside.
fn head(normalized: &str) -> String {
    let mut words: Vec<&str> = normalized.split(' ').filter(|w| !w.is_empty()).collect();
    while words.len() > 1 && GENERIC_SUFFIXES_ASCII.contains(words.last().unwrap_or(&"")) {
        words.pop();
    }
    let mut head = words.concat();
    loop {
        let stripped = GENERIC_SUFFIXES_CJK.iter().find_map(|suffix| {
            head.strip_suffix(suffix)
                .filter(|rest| !rest.is_empty())
                .map(str::to_string)
        });
        match stripped {
            Some(rest) => head = rest,
            None => break,
        }
    }
    if head.is_ascii() && head.len() >= 4 && head.ends_with('s') && !head.ends_with("ss") {
        head.pop();
    }
    head
}

/// Enough of a name to identify a section on its own: two Han characters or
/// three ASCII letters/digits.
fn is_substantial(text: &str) -> bool {
    let han = text.chars().filter(|ch| is_han(*ch)).count();
    let ascii = text.chars().filter(char::is_ascii_alphanumeric).count();
    han >= 2 || ascii >= 3 || (han >= 1 && ascii >= 2)
}

/// Whether two section names denote the same section.
fn names_match(a: &str, b: &str) -> bool {
    let (a, b) = (normalize_name(a), normalize_name(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if compact(&a) == compact(&b) {
        return true;
    }
    let (head_a, head_b) = (head(&a), head(&b));
    if is_substantial(&head_a) && head_a == head_b {
        return true;
    }
    (is_substantial(&a) && contains_term(&b, &a)) || (is_substantial(&b) && contains_term(&a, &b))
}

/// Generic names for the page's top bar. A list item that is only one of
/// these ("顶部栏") names whichever sibling is the top bar, whatever the
/// planner labelled it: arena w01 planned `topbar (顶部标签切换)` beside an
/// umbrella "垂直排列: 顶部栏与看板".
const TOP_BAR_ALIASES: &[&str] = &[
    "顶部栏",
    "顶栏",
    "顶部导航",
    "顶部导航栏",
    "顶部工具栏",
    "top bar",
    "topbar",
    "header",
    "toolbar",
];

fn names_subtask(item: &str, st: &Subtask) -> bool {
    let id = st.id.replace(['-', '_'], " ");
    if names_match(item, &st.label) || names_match(item, &id) {
        return true;
    }
    let item = normalize_name(item);
    TOP_BAR_ALIASES.contains(&item.as_str()) && {
        let identity = format!("{} {}", id, st.label).to_lowercase();
        TOP_BAR_ALIASES.iter().any(|alias| identity.contains(alias))
    }
}

#[cfg(test)]
#[path = "plan_normalize_umbrella_tests.rs"]
mod tests;
