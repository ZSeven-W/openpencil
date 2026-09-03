//! Reference-driven generation intent — a prompt-embedded URL reference
//! that the user wants a new design modelled on.

/// A reference the user wants a new design modelled on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceIntent {
    /// A web page to fetch and mine for structure + style tokens.
    Url(String),
}

/// Trigger words (CJK substrings / ASCII whole words, case-insensitive) that
/// turn a URL in a prompt into a reference. Single source of truth — tests
/// iterate it.
pub(crate) const REFERENCE_TRIGGER_WORDS: &[&str] = &[
    "参考",
    "照着",
    "像这个",
    "像这样",
    "复刻",
    "仿照",
    "仿",
    "风格",
    "对标",
    "借鉴",
    "reference",
    "like this",
    "similar to",
    "inspired by",
    "clone",
    "replicate",
    "match the style",
    "in the style of",
    "based on",
    "modeled on",
    "modelled on",
];

/// Detect a reference intent: the prompt carries an http(s) URL AND one of the
/// trigger words. Returns the FIRST URL (trailing punctuation `.,;:!?)]}>"'`
/// and CJK punctuation `。，；：！？）】》` stripped). A URL with no trigger word
/// is NOT a reference (the user may just be naming a product site) → None.
pub fn detect_reference_intent(prompt: &str) -> Option<ReferenceIntent> {
    // Check if any trigger word is present
    let lower = prompt.to_lowercase();
    let has_trigger = REFERENCE_TRIGGER_WORDS.iter().any(|word| {
        if word.is_ascii() && word.contains(' ') {
            // Multi-word ASCII trigger: check for exact phrase
            lower.contains(word)
        } else if word.is_ascii() {
            // Single ASCII word: use word boundary check
            crate::design_type::contains_word(&lower, word)
        } else {
            // CJK substring: direct contains check
            prompt.contains(word)
        }
    });

    if !has_trigger {
        return None;
    }

    // Extract the first URL
    let url = extract_first_url(prompt)?;
    Some(ReferenceIntent::Url(url))
}

/// Extract the first http(s) URL from the prompt, stripping trailing
/// punctuation. Trailing punctuation removed: `.,;:!?)]}>"'` (ASCII) and
/// `。，；：！？）】》` (CJK).
fn extract_first_url(prompt: &str) -> Option<String> {
    const ASCII_TRAILING: &[char] = &['.', ',', ';', ':', '!', '?', ')', ']', '}', '>', '"', '\''];
    const CJK_TRAILING: &[char] = &['。', '，', '；', '：', '！', '？', '）', '】', '》'];

    // Find http:// or https://
    let start = match (prompt.find("http://"), prompt.find("https://")) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return None,
    };

    // Find the end: stop at whitespace or non-ASCII character (URLs are ASCII-only)
    let remainder = &prompt[start..];
    let end_offset = remainder
        .char_indices()
        .find(|(_, c)| c.is_whitespace() || !c.is_ascii())
        .map(|(i, _)| i)
        .unwrap_or(remainder.len());

    let mut url = remainder[..end_offset].to_string();

    // Strip trailing punctuation
    while let Some(last_char) = url.chars().last() {
        if ASCII_TRAILING.contains(&last_char) || CJK_TRAILING.contains(&last_char) {
            url.pop();
        } else {
            break;
        }
    }

    if url.is_empty() || !url.starts_with("http") {
        None
    } else {
        Some(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_on_url_plus_trigger_in_either_language() {
        assert_eq!(
            detect_reference_intent("参考 https://example.com/landing 做一个我们产品的官网"),
            Some(ReferenceIntent::Url(
                "https://example.com/landing".to_string()
            ))
        );
        assert_eq!(
            detect_reference_intent("Build a landing page like this: https://stripe.com/."),
            Some(ReferenceIntent::Url("https://stripe.com/".to_string()))
        );
    }

    #[test]
    fn a_bare_url_is_not_a_reference() {
        assert_eq!(
            detect_reference_intent("帮我把 https://example.com 的链接放进页脚"),
            None
        );
    }

    #[test]
    fn trigger_without_url_is_not_a_reference() {
        assert_eq!(detect_reference_intent("参考 Stripe 的风格做定价页"), None);
    }

    #[test]
    fn every_trigger_word_fires() {
        for word in REFERENCE_TRIGGER_WORDS {
            let prompt = format!("{word} https://example.com/x page");
            assert_eq!(
                detect_reference_intent(&prompt),
                Some(ReferenceIntent::Url("https://example.com/x".to_string())),
                "trigger word {word:?} should fire"
            );
        }
    }

    #[test]
    fn returns_the_earliest_url_regardless_of_scheme() {
        assert_eq!(
            detect_reference_intent("参考 https://first.example/x 和 http://second.example/y"),
            Some(ReferenceIntent::Url("https://first.example/x".to_string()))
        );
    }

    #[test]
    fn strips_cjk_trailing_punctuation() {
        assert_eq!(
            detect_reference_intent("照着 https://a.b/c。做首页"),
            Some(ReferenceIntent::Url("https://a.b/c".to_string()))
        );
    }
}
