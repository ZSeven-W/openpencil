//! Scope item promises without removing actual local content instructions.

use regex::Regex;
use std::sync::LazyLock;

pub(super) fn mask_nonlocal_references(text: &str, subtask: &crate::plan::Subtask) -> String {
    let mut masked = text.as_bytes().to_vec();
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        let numeric_separator = matches!(ch, ',' | '.')
            && text[..index].ends_with(|c: char| c.is_ascii_digit())
            && text[index + ch.len_utf8()..].starts_with(|c: char| c.is_ascii_digit());
        if matches!(ch, '\n' | ';' | '；' | '。' | ',' | '，' | '.' | '!' | '！')
            && !numeric_separator
        {
            mask_clause(text, start, index, subtask, &mut masked);
            start = index + ch.len_utf8();
        }
    }
    mask_clause(text, start, text.len(), subtask, &mut masked);
    String::from_utf8(masked).unwrap_or_else(|_| text.to_owned())
}

fn mask_clause(
    text: &str,
    start: usize,
    end: usize,
    subtask: &crate::plan::Subtask,
    masked: &mut [u8],
) {
    // A conjunction starts a new scope only with an explicit local subject or
    // content instruction. "Another section has 9 cards and 3 rows" stays foreign.
    static LOCAL_RESTART: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)
            (?:\b(?:and|but|while|whereas)\s+|(?:但(?:是)?|而|并(?:且)?|同时)\s*)
            (?:本区|此区|本节|当前(?:区块|部分)|这里|显示|展示|生成|绘制|包含|保留|列出
              |(?:this\s+(?:section|block|region)|here|show|display|render|generate|include|keep|list)\b)")
        .expect("valid local ownership restart")
    });
    let mut part_start = start;
    for restart in LOCAL_RESTART.find_iter(&text[start..end]) {
        let split = start + restart.start();
        mask_reference(&text[part_start..split], part_start, subtask, masked);
        part_start = split;
    }
    mask_reference(&text[part_start..end], part_start, subtask, masked);
}

fn mask_reference(clause: &str, offset: usize, subtask: &crate::plan::Subtask, masked: &mut [u8]) {
    static FOREIGN_SECTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)
            (?:其他|其它|别的|另一个|另一|兄弟|后续|下一|上一)(?:个)?(?:区块|区域|模块|部分|章节|分区|节|页面)
            |\b(?:other|another|sibling|next|previous)\s+(?:sections?|blocks?|regions?|pages?|screens?|subtasks?)\b")
        .expect("valid foreign section reference")
    });
    static REFERENCE_START: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)
            让位给|让位于|留给|交给|移交给|仅(?:供|作(?:为)?)?参考|仅作(?:为)?(?:背景|上下文)
            |\b(?:secondary\s+to|subordinate\s+to|defer\s+to|make\s+room\s+for|leave\s+room\s+for|(?:for\s+)?reference\s+only|for\s+context\s+only)\b")
        .expect("valid non-local item reference")
    });
    static EXCLUSION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)
            无需(?:生成|显示|展示|绘制|包含)?|不(?:包含|包括|生成|显示|展示|绘制|渲染)
            |不要(?:包含|生成|显示|展示|绘制|渲染)
            |\b(?:do\s+not|does\s+not|don't|must\s+not|should\s+not)\s+(?:include|show|display|render|generate)\b")
        .expect("valid excluded-content predicate")
    });
    static OWNER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)
            由\s*(?P<cjk>.+?)\s*(?:负责|承接)
            |(?P<english>\b(?:owned\s+by|belongs?\s+to|handled\s+by|deferred\s+to)\s+)",
        )
        .expect("valid explicit item owner")
    });
    // A named owner qualifies its noun phrase, not every quantity in the
    // sentence. Coordinated neighboring phrases keep their local promises.
    for captures in OWNER.captures_iter(clause) {
        let reference = captures.get(0).unwrap();
        let (owner_start, owner_text) = captures.name("cjk").map_or_else(
            || (reference.end(), &clause[reference.end()..]),
            |owner| (owner.start(), owner.as_str()),
        );
        // Match the actual current owner before interpreting conjunctions in
        // names such as "Pricing and Plans" or "地址栏与搜索" as boundaries.
        if !is_local_owner(owner_text, subtask) {
            let from = phrase_start(clause, reference.start());
            let reference_end = if captures.name("english").is_some() {
                owner_start
                    + coordinated_phrases()
                        .find(owner_text)
                        .map_or(owner_text.len(), |separator| separator.start())
            } else {
                reference.end()
            };
            let to = phrase_end(clause, reference_end);
            masked[offset + from..offset + to].fill(b' ');
        }
    }
    let foreign = FOREIGN_SECTION.find_iter(clause).find(|reference| {
        masked[offset + reference.start()..offset + reference.end()]
            .iter()
            .any(|byte| *byte != b' ')
    });
    if let Some(reference) = foreign {
        // "9 cards in another section" refers backwards to those cards.
        // A bare spatial relation ("3x3 grid below the title") has no foreign owner.
        let before = clause[..reference.start()].trim_end();
        let follows_items = before.ends_with(['在', '于'])
            || before.split_whitespace().last().is_some_and(|word| {
                matches!(word.to_ascii_lowercase().as_str(), "in" | "of" | "from")
            });
        let (from, to) = if follows_items {
            (
                phrase_start(clause, reference.start()),
                phrase_end(clause, reference.end()),
            )
        } else {
            (reference.start(), clause.len())
        };
        masked[offset + from..offset + to].fill(b' ');
    }
    if let Some(reference) = REFERENCE_START.find(clause) {
        masked[offset + reference.start()..offset + clause.len()].fill(b' ');
    }
    for exclusion in EXCLUSION.find_iter(clause) {
        let following = clause[exclusion.end()..].trim_start().to_ascii_lowercase();
        // Negating a quantity comparison ("not fewer than 9") requires those
        // items. Preserve uncertain comparisons instead of dropping a quota.
        if ![
            "少于", "不足", "不满", "低于", "超过", "多于", "fewer", "less", "more", "under",
            "below",
        ]
        .iter()
        .any(|comparison| following.starts_with(comparison))
        {
            masked[offset + exclusion.start()..offset + clause.len()].fill(b' ');
        }
    }
}

fn coordinated_phrases() -> &'static Regex {
    static COORDINATOR: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:and|but|while|whereas)\b|以及|及|和|与|、")
            .expect("valid coordinated noun-phrase boundary")
    });
    &COORDINATOR
}

fn is_local_owner(owner: &str, subtask: &crate::plan::Subtask) -> bool {
    let owner = owner.trim().to_ascii_lowercase();
    [
        subtask.id.as_str(),
        subtask.label.as_str(),
        "本区",
        "此区",
        "本节",
        "当前区块",
        "本区块",
        "this section",
        "this block",
        "this region",
    ]
    .iter()
    .any(|local| {
        let local = local.trim().to_ascii_lowercase();
        !local.is_empty()
            && owner.strip_prefix(&local).is_some_and(|rest| {
                rest.is_empty()
                    || coordinated_phrases()
                        .find(rest)
                        .is_some_and(|separator| rest[..separator.start()].trim().is_empty())
            })
    })
}

fn phrase_start(clause: &str, reference: usize) -> usize {
    coordinated_phrases()
        .find_iter(&clause[..reference])
        .last()
        .map_or(0, |separator| separator.end())
}

fn phrase_end(clause: &str, reference: usize) -> usize {
    coordinated_phrases()
        .find(&clause[reference..])
        .map_or(clause.len(), |separator| reference + separator.start())
}
