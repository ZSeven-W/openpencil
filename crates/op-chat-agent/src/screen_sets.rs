//! Strong intent signal for continuation prompts that list sibling screens.
//!
//! Generic words such as `完成` and `界面` are individually ambiguous: they
//! can describe finishing the current screen or one of its sections.  A
//! delimited list before a shared whole-screen noun is much stronger, e.g.
//! `继续完成 explore/profile界面`.  Keep this parser narrow so ordinary
//! append/modify requests retain their existing behavior.

/// CJK deictics naming the CURRENT screen — moved here with this module
/// from `op-host-services::chat_intent` (which re-imports it) so the
/// listed-screen parser stays a leaf.
pub const EXISTING_SCREEN_CTX_CJK: &[&str] =
    &["这个", "这一", "当前", "现有", "此页", "这页", "这屏"];

const LIST_VERBS_CJK: &[&str] = &["完成", "补齐", "补完", "生成", "设计", "画", "绘", "做"];
const SCREEN_NOUNS_CJK: &[&str] = &["界面", "页面", "屏幕", "页", "屏"];
const LIST_VERBS_EN: &[&str] = &[
    "generate",
    "generating",
    "design",
    "designing",
    "create",
    "creating",
    "build",
    "building",
    "draw",
    "drawing",
    "complete",
    "completing",
    "finish",
    "finishing",
];
const SCREEN_NOUNS_EN: &[&str] = &[
    "interface",
    "interfaces",
    "page",
    "pages",
    "screen",
    "screens",
];
const NEGATION_EN: &[&str] = &["not", "never", "don't", "dont", "without", "stop", "avoid"];
const EXISTING_SCREEN_CTX_EN: &[&str] = &[
    "current", "existing", "same", "this", "that", "these", "those",
];

fn has_name_chars(part: &str) -> bool {
    let letters: Vec<char> = part.chars().filter(|c| c.is_alphabetic()).collect();
    letters.iter().any(|c| !c.is_ascii()) || letters.len() >= 3
}

/// A segment that shed a declared count (`My 2个`) proved it belongs to a
/// counted list, so the generic 3-letter floor is not needed to tell it apart
/// from `16/9` or `UI/UX`. Two letters keep short real names (`My`, `Me`) while
/// still rejecting a bare count (`2个`) or a stray initial.
fn has_counted_name_chars(part: &str) -> bool {
    part.chars().filter(|c| c.is_alphabetic()).count() >= 2
}

fn normalize_target_name(raw: &str) -> Option<String> {
    let mut name = raw
        .trim()
        .trim_matches(|c: char| matches!(c, ',' | '，' | ':' | '：'))
        .to_string();

    // A shared CJK screen noun commonly carries the declared count after the
    // final name: `星图、观测计划、我的3个界面`. The count describes the list; it is
    // not part of the last screen's name.
    let mut shed_declared_count = false;
    if name.ends_with('个') {
        name.pop();
        name = name.trim_end().to_string();
        while name
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_digit() || "零一二三四五六七八九十两几".contains(c))
        {
            name.pop();
        }
        name = name
            .trim_end()
            .trim_end_matches('这')
            .trim_end()
            .to_string();
        shed_declared_count = true;
    }

    if let Some(without_article) = name
        .strip_prefix("the ")
        .or_else(|| name.strip_prefix("The "))
    {
        name = without_article.trim().to_string();
    }

    // The list is all-or-nothing on purpose (`16/9` must not half-parse), so a
    // short trailing name would otherwise kill an otherwise well-formed
    // request: `Home、My 2个页面` collapsed to no names at all because `My` is
    // two letters. Falling back to the RAW segment was rejected — it would
    // ship `My 2个` as a screen name into the continuation contract, which is
    // worse than not matching.
    (has_name_chars(&name) || (shed_declared_count && has_counted_name_chars(&name)))
        .then_some(name)
}

fn named_list_with(targets: &str, separator: char) -> Option<Vec<String>> {
    let parts = targets
        .split(separator)
        .map(normalize_target_name)
        .collect::<Option<Vec<_>>>()?;
    (parts.len() >= 2).then_some(parts)
}

fn named_list_names(targets: &str) -> Option<Vec<String>> {
    named_list_with(targets, '／')
        .or_else(|| named_list_with(targets, '、'))
        .or_else(|| {
            (!targets.contains("://") && !targets.contains("//"))
                .then(|| named_list_with(targets, '/'))
                .flatten()
        })
}

fn is_clause_end(tail: &str) -> bool {
    tail.trim_start().chars().next().is_none_or(|c| {
        matches!(
            c,
            ',' | '，' | '.' | '。' | ';' | '；' | '!' | '！' | '?' | '？' | '\n'
        )
    })
}

fn crosses_clause_boundary(targets: &str) -> bool {
    targets.chars().any(|c| {
        matches!(
            c,
            ',' | '，' | '.' | '。' | ';' | '；' | '!' | '！' | '?' | '？' | '\n'
        )
    })
}

fn completion_is_negated(prompt: &str, verb_pos: usize, verb: &str, after_verb: &str) -> bool {
    let prefix = prompt[..verb_pos].trim_end();
    let recent_start = prefix
        .char_indices()
        .rev()
        .nth(7)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let recent_prefix = &prefix[recent_start..];
    let explicit_negation = ["不要", "无需", "不用", "不必", "禁止", "别"]
        .iter()
        .any(|marker| recent_prefix.contains(marker));
    explicit_negation
        || (verb == "完成"
            && (prefix
                .chars()
                .next_back()
                .is_some_and(|c| matches!(c, '未' | '没' | '已'))
                || after_verb.starts_with('度')))
}

fn targets_reference_existing_screen(targets: &str) -> bool {
    EXISTING_SCREEN_CTX_CJK
        .iter()
        .any(|marker| targets.contains(marker))
}

fn is_ascii_word_match(text: &str, start: usize, word: &str) -> bool {
    let end = start + word.len();
    let is_word_char = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text[..start]
        .chars()
        .next_back()
        .is_none_or(|c| !is_word_char(c))
        && text[end..].chars().next().is_none_or(|c| !is_word_char(c))
}

fn contains_ascii_word(text: &str, word: &str) -> bool {
    text.match_indices(word)
        .any(|(start, _)| is_ascii_word_match(text, start, word))
}

fn english_completion_is_negated(lower: &str, verb_pos: usize) -> bool {
    let prefix = &lower[..verb_pos];
    let clause = prefix
        .rsplit([',', '.', ';', '!', '?', '\n'])
        .next()
        .unwrap_or(prefix);
    NEGATION_EN
        .iter()
        .any(|marker| contains_ascii_word(clause, marker))
}

fn targets_reference_existing_screen_en(targets: &str) -> bool {
    EXISTING_SCREEN_CTX_EN
        .iter()
        .any(|marker| contains_ascii_word(targets, marker))
}

fn listed_whole_screen_names_en(prompt: &str) -> Option<Vec<String>> {
    let lower = prompt.to_ascii_lowercase();
    for &verb in LIST_VERBS_EN {
        for (verb_pos, _) in lower.match_indices(verb) {
            if !is_ascii_word_match(&lower, verb_pos, verb)
                || english_completion_is_negated(&lower, verb_pos)
            {
                continue;
            }
            let after_verb = &lower[verb_pos + verb.len()..];
            let after_verb_original = &prompt[verb_pos + verb.len()..];
            for &noun in SCREEN_NOUNS_EN {
                for (noun_pos, _) in after_verb.match_indices(noun) {
                    if !is_ascii_word_match(after_verb, noun_pos, noun) {
                        continue;
                    }
                    let targets = after_verb_original[..noun_pos].trim();
                    // The "current / these / …" guard compares against a
                    // lowercase marker table, so it must read the LOWERCASED
                    // slice (`the Current explore/profile interfaces` is the
                    // same request as the all-lowercase one). Name extraction
                    // keeps the original casing. `to_ascii_lowercase` is
                    // byte-length preserving, so both slices share indices.
                    let targets_lower = after_verb[..noun_pos].trim();
                    let tail = &after_verb_original[noun_pos + noun.len()..];
                    if !crosses_clause_boundary(targets)
                        && !targets_reference_existing_screen_en(targets_lower)
                        && is_clause_end(tail)
                    {
                        if let Some(names) = named_list_names(targets) {
                            return Some(names);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Exact sibling-screen names promised by a listed continuation request.
///
/// This is the structured counterpart to the routing predicate below: the
/// desktop host attaches these names to `DesignRequest`, allowing a planning
/// fallback to create one real top-level screen per promise instead of a
/// generic `Section 1` board.
pub fn listed_whole_screen_names(prompt: &str) -> Vec<String> {
    for &verb in LIST_VERBS_CJK {
        for (verb_pos, _) in prompt.match_indices(verb) {
            let after_verb = &prompt[verb_pos + verb.len()..];
            if completion_is_negated(prompt, verb_pos, verb, after_verb) {
                continue;
            }
            // Try every noun occurrence rather than the earliest one: screen
            // names themselves can contain 页/屏 ("首页/profile页面").
            for &noun in SCREEN_NOUNS_CJK {
                for (noun_pos, _) in after_verb.match_indices(noun) {
                    let targets = after_verb[..noun_pos].trim();
                    let tail = &after_verb[noun_pos + noun.len()..];
                    if !crosses_clause_boundary(targets)
                        && !targets_reference_existing_screen(targets)
                        && is_clause_end(tail)
                    {
                        if let Some(names) = named_list_names(targets) {
                            return names;
                        }
                    }
                }
            }
        }
    }
    listed_whole_screen_names_en(prompt).unwrap_or_default()
}

pub fn requests_listed_whole_screens(prompt: &str) -> bool {
    !listed_whole_screen_names(prompt).is_empty()
}

#[cfg(test)]
mod tests {
    use super::{listed_whole_screen_names, requests_listed_whole_screens};

    #[test]
    fn extracts_promised_screen_names_and_drops_the_declared_count() {
        assert_eq!(
            listed_whole_screen_names("继续生成 星图、观测计划、我的3个界面"),
            ["星图", "观测计划", "我的"]
        );
        assert_eq!(
            listed_whole_screen_names("Continue generating the explore/profile interface"),
            ["explore", "profile"]
        );
    }

    /// A declared count must not disqualify the name it follows. The list is
    /// all-or-nothing, so a short trailing latin name used to take the whole
    /// request down with it.
    #[test]
    fn keeps_short_latin_names_that_carried_the_declared_count() {
        assert_eq!(
            listed_whole_screen_names("继续生成 Home、My 2个页面"),
            ["Home", "My"]
        );
        assert_eq!(
            listed_whole_screen_names("继续生成 Home、Me 两个页面"),
            ["Home", "Me"]
        );
        assert_eq!(
            listed_whole_screen_names("继续生成 探索、Me 两个界面"),
            ["探索", "Me"]
        );
        // The relaxation is scoped to the counted segment: an uncounted
        // two-letter pair stays out, so `UI/UX` is still not a screen list.
        assert!(listed_whole_screen_names("继续完成 UI/UX界面").is_empty());
        // A bare count is not a name.
        assert!(listed_whole_screen_names("继续生成 Home、2个页面").is_empty());
    }

    /// The English existing-screen guard reads a lowercase marker table, so it
    /// has to be fed the lowercased slice. Reading the original-cased one let
    /// `Current` through AND leaked `Current explore` as a screen name.
    #[test]
    fn english_existing_screen_guard_is_case_insensitive() {
        for prompt in [
            "Continue generating the Current explore/profile interfaces",
            "Continue generating THE CURRENT explore/profile interfaces",
            "Continue generating These explore/profile interfaces",
            "Continue generating Those explore/profile interfaces",
            "Continue generating the Same explore/profile interfaces",
        ] {
            assert!(
                !requests_listed_whole_screens(prompt),
                "expected an in-place request: {prompt}"
            );
            assert!(
                listed_whole_screen_names(prompt).is_empty(),
                "an in-place request must promise no screen names: {prompt}"
            );
        }
    }

    #[test]
    fn recognizes_listed_follow_on_screens_with_shared_cjk_noun() {
        for prompt in [
            "继续完成 explore/profile界面",
            "继续完成 Explore ／ Profile 界面",
            "接着补齐 Search、Orders 两个页面",
            "继续生成 cart/checkout屏幕，保持首页的视觉风格",
            "基于当前首页继续完成 explore/profile界面",
            "继续完成 explore/profile界面，沿用当前设计",
            "继续完成 首页/profile页面",
            "继续完成 首屏/profile界面",
        ] {
            assert!(
                requests_listed_whole_screens(prompt),
                "expected listed whole screens: {prompt}"
            );
        }
    }

    #[test]
    fn recognizes_listed_follow_on_screens_with_english_interface_noun() {
        for prompt in [
            "Continue generating the explore/profile interface",
            "continue generating Explore / Profile interfaces",
            "Continue designing search/checkout screens.",
        ] {
            assert!(
                requests_listed_whole_screens(prompt),
                "expected listed English whole screens: {prompt}"
            );
        }
    }

    #[test]
    fn rejects_current_screen_and_subobject_requests() {
        for prompt in [
            "继续完成这个界面",
            "继续完成当前界面的推荐区块",
            "继续完成 profile 界面的 header",
            "继续完成 explore/profile 界面之间的跳转",
            "继续把 explore/profile 两个界面改成深色",
            "继续在首页加入 explore/profile 入口",
            "继续修改未完成的 explore/profile界面",
            "不要完成 explore/profile界面",
            "无需完成 explore/profile界面",
            "请不要再完成 explore/profile界面",
            "继续完成 16/9页面",
            "继续完成 UI/UX界面",
            "Continue modifying the explore/profile interface",
            "Do not continue generating the explore/profile interface",
            "Continue generating the explore/profile interface transitions",
            "Continue generating the UI/UX interface",
            "Don't generate the explore/profile interfaces",
            "Stop generating explore/profile interfaces",
            "Continue generating the current explore/profile interfaces",
            "Continue generating the explore/profile interface header",
            "Continue generating the explore/profile interface's navigation",
            "Continue generating the profile interface",
            "Continue generating 16/9 interfaces",
            "Continue regenerating the explore/profile interface",
        ] {
            assert!(
                !requests_listed_whole_screens(prompt),
                "expected an in-place request: {prompt}"
            );
        }
    }
}
