//! Resolve ordinary-workspace follow-ups to real, existing board ids before
//! a broad design classifier can mistake a page reference for a new design.

use op_editor_core::{EditorState, PageEditTarget, PenNodeExt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEditScope {
    NotApplicable,
    Target(PageEditTarget),
    /// An edit was requested, but its scope is missing, stale or ambiguous.
    NeedsTarget,
}

/// Replacement copy is data, not an instruction or a second board reference.
fn routing_text(text: &str) -> String {
    let mut result = String::new();
    let mut closing_quote = None;
    let mut replacement_copy = false;
    let mut characters = text.chars().peekable();
    while let Some(ch) = characters.next() {
        if let Some(end) = closing_quote {
            if ch == end {
                closing_quote = None;
                result.push(' ');
            } else if !replacement_copy {
                result.push(ch);
            }
            continue;
        }
        closing_quote = match ch {
            '“' => Some('”'),
            '「' => Some('」'),
            '『' => Some('』'),
            '"' => Some('"'),
            _ => None,
        };
        if closing_quote.is_none() {
            // A sentence boundary separates an edit from a following
            // preservation clause. Decimal points and quoted names stay data.
            if ch == '.' && characters.peek().is_none_or(|next| next.is_whitespace()) {
                result.push(';');
            } else {
                result.push(ch);
            }
        } else {
            let prefix = result
                .rsplit([',', '，', ';', '；', '。', '\n'])
                .next()
                .unwrap_or("")
                .to_lowercase();
            replacement_copy = ["改成", "改为", "换成", "替换为", "变成", " to ", " with "]
                .iter()
                .any(|marker| prefix.contains(marker));
        }
    }
    result.trim().to_lowercase()
}

/// A question about editing is not itself an instruction to edit.
pub fn is_workspace_question(state: &EditorState, text: &str) -> bool {
    if !state.editor_ui.workspace.visible {
        return false;
    }
    let text = routing_text(text);
    // A global no-edit request is distinct from protecting a sibling page or
    // one property while requesting a real edit. Require a complete clause.
    let advice_only = text
        .split([',', '，', ';', '；', '。', '\n'])
        .any(|clause| {
            let clause = clause.trim().trim_end_matches(['.', '?', '!', '？', '！']);
            [
                "不要修改设计",
                "暂时不要修改设计",
                "不修改设计",
                "不要改动作品",
                "只提供建议",
                "只给建议",
                "without editing the design",
                "without changing the design",
                "不要修改設計",
                "暫時不要修改設計",
                "不修改設計",
                "只提供建議",
                "只給建議",
                "do not edit the design",
                "do not change anything",
            ]
            .iter()
            .any(|signal| clause.ends_with(signal))
        });
    advice_only
        || [
            "为什么",
            "怎么",
            "如何",
            "是什么",
            "what is",
            "why ",
            "how do",
            "how can",
            "how to",
        ]
        .iter()
        .any(|question| text.contains(question))
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(offset, _)| {
        let before = text[..offset].chars().next_back();
        let after = text[offset + phrase.len()..].chars().next();
        let word = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
        !phrase.starts_with(|ch: char| ch.is_ascii_alphanumeric())
            || (!before.is_some_and(word) && !after.is_some_and(word))
    })
}

/// Deliberately narrow: new-page requests retain the host's normal generation
/// route, even if the same prompt also mentions changes or an old reader scope.
fn creates_new_work(text: &str) -> bool {
    crate::screen_sets::requests_listed_whole_screens(text)
        || [
            "重新生成",
            "重新画",
            "重做",
            "新增页面",
            "新增一页",
            "加一页",
            "加一个页面",
            "添加页面",
            "添加一页",
            "新建",
            "再做",
            "再画",
            "再生成",
            "生成一个",
            "做一个",
            "画一个",
        ]
        .iter()
        .any(|signal| text.contains(signal))
        || [
            "regenerate",
            "redesign",
            "create",
            "generate",
            "design",
            "draw",
            "build",
        ]
        .iter()
        .any(|signal| contains_phrase(text, signal))
        || [
            "add a page",
            "add a new",
            "add another",
            "new page",
            "new screen",
        ]
        .iter()
        .any(|signal| text.contains(signal))
}

fn requests_edit(text: &str) -> bool {
    [
        "改",
        "调整",
        "調整",
        "修复",
        "替换",
        "换成",
        "变成",
        "删除",
        "移除",
        "加个",
        "加一个",
        "加上",
        "加粗",
        "小一点",
        "大一点",
        "优化",
        "優化",
    ]
    .iter()
    .any(|signal| text.contains(signal))
        || [
            "change", "modify", "update", "adjust", "resize", "move", "restyle", "refine", "fix",
            "tweak", "edit", "replace", "remove", "delete", "add", "smaller", "larger",
        ]
        .iter()
        .any(|signal| contains_phrase(text, signal))
}

fn protects_clause(text: &str) -> bool {
    let text = text.trim();
    [
        "不要",
        "不许",
        "别改",
        "勿改",
        "不改",
        "不修改",
        "不更改",
        "do not",
        "don't",
        "unchanged",
    ]
    .iter()
    .any(|signal| text.contains(signal))
        || ["保留", "保持", "keep ", "leave "]
            .iter()
            .any(|signal| text.starts_with(signal))
}

/// Resolve only the ordinary view. Professional mode keeps its explicit
/// canvas-selection semantics. Never fall back to the last generated board.
pub fn resolve_workspace_edit_scope(state: &EditorState, text: &str) -> WorkspaceEditScope {
    use WorkspaceEditScope::*;
    let workspace = &state.editor_ui.workspace;
    let lower = routing_text(text);
    let affirmative = lower
        .split([',', '，', ';', '；', '。', '\n'])
        .filter(|clause| !protects_clause(clause))
        .collect::<Vec<_>>()
        .join("; ");
    if !workspace.visible
        || lower.is_empty()
        || is_workspace_question(state, text)
        || creates_new_work(&affirmative)
    {
        return NotApplicable;
    }
    if workspace.page_edit.is_none() && !requests_edit(&lower) {
        return NotApplicable;
    }
    if requests_edit(&lower) && !requests_edit(&affirmative) {
        return NeedsTarget;
    }
    let boards = op_editor_core::preview_slideshow::active_page_boards(state);
    let target = |index: usize| PageEditTarget {
        board_id: boards[index].clone(),
        index,
    };
    // Collect every affirmative clause: a second requested page must not be
    // silently discarded, and a protected page is never an edit target.
    let subjects = affirmative
        .split(';')
        .filter(|clause| requests_edit(clause))
        .collect::<Vec<_>>()
        .join("; ");
    let subject = subjects.as_str();
    let mut candidates = Vec::new();
    let mut mentions_target = subject.contains('页')
        || subject.contains('頁')
        || subject.contains("界面")
        || contains_phrase(subject, "page")
        || contains_phrase(subject, "screen");
    let home = subject.contains("首页")
        || contains_phrase(subject, "home")
        || contains_phrase(subject, "homepage");
    let current = [
        "这一页",
        "這一頁",
        "這頁",
        "當前頁",
        "這個頁面",
        "这页",
        "当前页",
        "这个页面",
        "this page",
        "current page",
        "this screen",
    ]
    .iter()
    .any(|reference| subject.contains(reference));
    if current {
        mentions_target = true;
        if workspace.selected < boards.len() {
            candidates.push(workspace.selected);
        }
    }
    let chinese = ["一", "二", "三", "四", "五", "六", "七", "八", "九", "十"];
    for (index, id) in boards.iter().enumerate() {
        let ordinal = subject.contains(&format!("第{}页", index + 1))
            || subject.contains(&format!("第 {} 页", index + 1))
            || contains_phrase(subject, &format!("page {}", index + 1))
            || chinese
                .get(index)
                .is_some_and(|number| subject.contains(&format!("第{number}页")))
            || (index == 0 && subject.contains("first page"));
        let Some(board) = state
            .active_children()
            .iter()
            .find(|node| node.id_str() == id)
        else {
            continue;
        };
        let name = board.base().name.as_deref().unwrap_or("").to_lowercase();
        let name = name.trim_start_matches(|ch: char| {
            ch.is_ascii_digit() || ch.is_whitespace() || matches!(ch, '-' | '_' | '.' | '、' | '·')
        });
        let named = !name.is_empty() && contains_phrase(subject, name);
        let home_board = home
            && (name.contains("首页")
                || contains_phrase(name, "home")
                || contains_phrase(name, "homepage"));
        if ordinal || named || home_board {
            mentions_target = true;
            candidates.push(index);
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    // A reader binding is explicit but cannot override contradictory page
    // references. Missing targets never degrade into unrestricted edits.
    if let Some(staged) = &workspace.page_edit {
        let Some(index) = boards.iter().position(|id| id == &staged.board_id) else {
            return NeedsTarget;
        };
        return match candidates.as_slice() {
            [] if !mentions_target => Target(target(index)),
            [resolved] if *resolved == index => Target(target(index)),
            _ => NeedsTarget,
        };
    }
    match candidates.as_slice() {
        [index] => Target(target(*index)),
        [] if !mentions_target && !home && boards.len() == 1 => Target(target(0)),
        _ => NeedsTarget,
    }
}

/// Short, actionable copy shared by native hosts and the web endpoint.
pub fn scope_unavailable_message(locale: op_editor_core::Locale) -> &'static str {
    if matches!(
        locale,
        op_editor_core::Locale::ZhCn | op_editor_core::Locale::ZhTw
    ) {
        "请先选择要修改的页面，或在要求中写清页面名称（例如“把首页标题改成……”）。其他页面会保持不变。"
    } else {
        "Choose the page to edit, or name it in your request (for example, “Change the Home title…”). Other pages will stay unchanged."
    }
}

#[cfg(test)]
#[path = "workspace_edit_tests.rs"]
mod tests;
