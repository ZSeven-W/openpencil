//! Bounded retrieval refinements from the authored image brief.

use super::*;

/// Style and staging words stripped from an intent-derived rewrite. The
/// judged-ladder rewrite wants the subject of the image prompt, not its art
/// direction. Multi-word forms are matched per token because the tokenizer
/// splits on non-alphanumerics ("close-up" → "close up", "high quality" →
/// "high" + "quality").
pub(super) const INTENT_STYLE_WORDS: &[&str] = &[
    "photo",
    "photography",
    "professional",
    "shot",
    "close",
    "closeup",
    "image",
    "stock",
    "high",
    "quality",
    "4k",
    "hd",
    "cinematic",
    "lighting",
    "background",
    "minimal",
];

/// Up to two rewritten queries for the judged Openverse ladder, in attempt
/// order. The visual judge can reject every primary candidate (k=0 after the
/// lexical filter, or all Off verdicts) while a looser phrasing still has
/// good hits — the two-keyword retry inside the list fetch only fires on a
/// zero-hit response, so judge-rejected slots used to fall straight to the
/// Wikimedia ladder.
///
/// 1. The query's subject words with style words ([`INTENT_STYLE_WORDS`])
///    and generic framing nouns ([`GENERIC_FRAMING_WORDS`]) dropped — or
///    nothing at all when three or more subject words remain, per the 0930
///    three-word rule (see [`two_keyword_rewrite`]).
/// 2. Subject words from the intent anchored to the original query, with
///    nearby concrete context when useful, capped at four words.
///
/// Candidates that are empty, equal to the original query, or equal to an
/// earlier rewrite are dropped, so the caller's cost bound is at most two
/// extra Openverse list calls per slot.
pub(crate) fn rewrite_queries(query: &str, intent: &str) -> Vec<String> {
    let mut rewrites: Vec<String> = Vec::new();
    let mut push = |rewrite: Option<String>| {
        if let Some(rewrite) = rewrite {
            if !rewrite.is_empty() && rewrite != query && !rewrites.contains(&rewrite) {
                rewrites.push(rewrite);
            }
        }
    };
    push(two_keyword_rewrite(query));
    push(intent_rewrite(query, intent));
    rewrites
}

/// Genre/framing/occasion nouns: they name the kind of picture wanted, not
/// the subject in it, so the two-keyword rewrite drops them ("kettlebell
/// swing workout" → "kettlebell swing", not "swing workout"). They only
/// leave while at least two subject words survive, and they never license
/// shortening an already concrete three-word subject.
const GENERIC_FRAMING_WORDS: &[&str] = &[
    "workout",
    "exercise",
    "routine",
    "session",
    "training",
    "portrait",
    "scene",
    "lifestyle",
    "concept",
    "setup",
    "view",
    "detail",
    "texture",
    "illustration",
    "banner",
    "hero",
    "shot",
    "moment",
    "vibe",
    "aesthetic",
];

/// At most two surviving subject words of the query, or `None`. Style words
/// ([`INTENT_STYLE_WORDS`], canonicalized like [`two_keyword_retry`]) and
/// generic framing nouns ([`GENERIC_FRAMING_WORDS`]) are dropped first;
/// framing nouns only while at least two subject words remain ("workout
/// routine" keeps both words). Three or more surviving subject words return
/// no rewrite, per the rule measured 2026-09-30 in [`two_keyword_retry`]:
/// "No shorter retry for a three-word subject: dropping its leading word
/// loses the domain (rose serum bottle → 1960s medical serum, camellia
/// cream jar → cake in a jar). An empty slot is better than a wrong
/// picture."
fn two_keyword_rewrite(query: &str) -> Option<String> {
    let core: Vec<String> = concrete_query_words(query)
        .into_iter()
        .filter(|word| !INTENT_STYLE_WORDS.contains(&canonicalize_word(word).as_str()))
        .collect();
    let stripped: Vec<&str> = core
        .iter()
        .filter(|word| !GENERIC_FRAMING_WORDS.contains(&canonicalize_word(word).as_str()))
        .map(|word| word.as_str())
        .collect();
    // Stripping must not eat the whole subject: an all-framing phrase such
    // as "workout routine" retries with its framing words kept.
    let subject: Vec<&str> = if stripped.len() >= 2 {
        stripped
    } else {
        core.iter().map(|word| word.as_str()).collect()
    };
    (!subject.is_empty() && subject.len() <= 2).then(|| subject.join(" "))
}

/// Find the authored subject before selecting a short retrieval phrase. Rich
/// prompts often start with an arbitrary art-direction prefix; taking its
/// first three tokens can search for staging adjectives instead of the subject.
fn intent_rewrite(query: &str, intent: &str) -> Option<String> {
    let mut anchors = Vec::new();
    for word in core_query_words(query) {
        if !IMAGE_SEARCH_STOP_WORDS.contains(&word.as_str())
            && !INTENT_STYLE_WORDS.contains(&word.as_str())
            && !anchors.contains(&word)
        {
            anchors.push(word);
        }
    }
    if anchors.is_empty() {
        return None;
    }
    let words: Vec<(String, usize)> = intent
        .split([',', ';', '.', '!', '?', '\n'])
        .enumerate()
        .flat_map(|(clause, text)| {
            lexical_words(text)
                .into_iter()
                .map(move |word| (word, clause))
        })
        .collect();
    let mut subject = Vec::new();
    let mut first_match = None;
    let mut last_match = None;
    for (index, (word, _)) in words.iter().enumerate() {
        let canonical = canonicalize_word(word);
        if anchors.contains(&canonical) && !subject.contains(&canonical) {
            subject.push(canonical);
            first_match.get_or_insert(index);
            last_match = Some(index);
            if subject.len() == 4 {
                break;
            }
        }
    }
    // One generic overlap ("bottle", "model") cannot establish that a
    // multi-word prompt describes the same subject. Keep the existing query
    // ladder instead of inventing a rewrite from unrelated intent wording.
    if subject.len() < anchors.len().min(2) {
        return None;
    }
    let last_match = last_match?;
    let clause = words[last_match].1;
    // Preserve a simple location phrase such as "in a clinic". Arbitrary
    // trailing praise is not subject context; ambiguous longer phrases stay
    // with the visual judge rather than becoming new retrieval keywords.
    if subject.len() == 2
        && words
            .get(last_match + 1)
            .is_some_and(|(word, _)| matches!(word.as_str(), "in" | "at" | "inside" | "within"))
    {
        let context: Vec<_> = words
            .iter()
            .skip(last_match + 2)
            .take_while(|(_, next_clause)| *next_clause == clause)
            .map(|(word, _)| word.as_str())
            .collect();
        let context = match context.as_slice() {
            ["a" | "an" | "the", rest @ ..] => rest,
            rest => rest,
        };
        if let [word] = context {
            let canonical = canonicalize_word(word);
            if !IMAGE_SEARCH_STOP_WORDS.contains(word)
                && !INTENT_STYLE_WORDS.contains(word)
                && !IMAGE_SEARCH_DESCRIPTORS.contains(word)
                && !subject.contains(&canonical)
            {
                subject.push(canonical);
            }
        }
    }
    // A literal color/material directly before the subject is useful stock
    // metadata. Positive recognition avoids a growing blacklist of arbitrary
    // taste adjectives (premium, elegant, etc.) and never displaces a subject.
    if subject.len() < 4 {
        let first_match = first_match?;
        if let Some((word, modifier_clause)) = first_match.checked_sub(1).map(|i| &words[i]) {
            let canonical = canonicalize_word(word);
            if *modifier_clause == words[first_match].1
                && concrete_visual_modifier(&canonical)
                && !subject.contains(&canonical)
            {
                subject.insert(0, canonical);
            }
        }
    }
    Some(subject.join(" "))
}

fn concrete_visual_modifier(word: &str) -> bool {
    matches!(
        word,
        "white"
            | "black"
            | "gray"
            | "grey"
            | "red"
            | "green"
            | "blue"
            | "yellow"
            | "pink"
            | "purple"
            | "orange"
            | "cyan"
            | "amber"
            | "brown"
            | "ivory"
            | "beige"
            | "gold"
            | "silver"
            | "glass"
            | "ceramic"
            | "porcelain"
            | "wood"
            | "steel"
            | "metal"
            | "aluminum"
            | "brass"
            | "copper"
            | "chrome"
            | "plastic"
            | "paper"
            | "linen"
            | "leather"
            | "fabric"
            | "wicker"
            | "stone"
            | "concrete"
            | "bamboo"
    )
}

/// Refine retrieval before spending a visual batch on an ambiguous query.
/// The judge retains the original query/brief. The original retrieval query
/// occupies one existing rewrite slot; the plan never adds request tiers.
pub(crate) fn retrieval_plan(query: &str, intent: &str) -> (String, Vec<String>) {
    let primary = human_activity_query(query, intent)
        .or_else(|| {
            let refined = intent_rewrite(query, intent)?;
            let original = normalized_words(query);
            normalized_words(&refined)
                .iter()
                .any(|word| concrete_visual_modifier(word) && !original.contains(word))
                .then_some(refined)
        })
        .unwrap_or_else(|| query.to_string());
    let mut retries = Vec::new();
    if primary != query {
        retries.push(query.to_string());
    }
    for rewrite in rewrite_queries(query, intent) {
        if rewrite != primary && !retries.contains(&rewrite) {
            retries.push(rewrite);
        }
    }
    retries.truncate(2);
    (primary, retries)
}

/// Recognize only an explicit, positive human activity clause. Unsupported
/// syntax/languages keep the original query. A person mentioned incidentally
/// in a background, negation, or non-photo description cannot redirect it.
fn human_activity(intent: &str) -> Option<Vec<String>> {
    let clause = intent.split([',', ';', '.', '!', '?', '\n']).next()?;
    let mut words = lexical_words(clause);
    if words
        .iter()
        .any(|word| matches!(word.as_str(), "no" | "not" | "without" | "avoid" | "never"))
    {
        return None;
    }
    if matches!(words.first()?.as_str(), "a" | "an" | "the") {
        words.remove(0);
    }
    if !is_human_word(words.first()?) {
        return None;
    }
    words.remove(0);
    if !matches!(
        words.first()?.as_str(),
        "doing" | "performing" | "practicing" | "stretching" | "lifting" | "jogging" | "running"
    ) {
        return None;
    }
    Some(words)
}

fn is_human_word(word: &str) -> bool {
    matches!(
        word,
        "person"
            | "people"
            | "woman"
            | "women"
            | "man"
            | "men"
            | "runner"
            | "runners"
            | "athlete"
            | "athletes"
            | "girl"
            | "girls"
            | "boy"
            | "boys"
    )
}

fn human_activity_query(query: &str, intent: &str) -> Option<String> {
    let activity = human_activity(intent)?;
    let core = matching::distinct_subjects(core_query_words(query));
    if core.is_empty() || core.iter().any(|word| is_human_word(word)) {
        return None;
    }
    // A generic fitness label can be made concrete by an explicitly authored
    // exercise. Specific subjects (dog training, aircraft model, etc.) need
    // their original anchors and cannot be replaced by an unrelated brief.
    let generic_fitness = core.iter().all(|word| {
        matches!(
            word.as_str(),
            "core" | "training" | "fitness" | "exercise" | "workout" | "routine"
        )
    });
    let subject = if generic_fitness {
        let subject: Vec<_> = activity
            .iter()
            .take_while(|word| {
                !matches!(
                    word.as_str(),
                    "on" | "in" | "at" | "with" | "while" | "against" | "near" | "under"
                )
            })
            .filter(|word| {
                !IMAGE_SEARCH_STOP_WORDS.contains(&word.as_str())
                    && !matches!(
                        word.as_str(),
                        "doing" | "performing" | "practicing" | "their" | "his" | "her"
                    )
            })
            .cloned()
            .collect();
        if subject.is_empty()
            || subject.len() > 4
            || !subject.iter().any(|word| {
                matches!(
                    word.as_str(),
                    "exercise" | "workout" | "plank" | "push-up" | "push-ups" | "squat" | "squats"
                )
            })
        {
            return None;
        }
        subject.join(" ")
    } else {
        if core.len() < 2 || matching::evidence(&core, &activity.join(" ")).overlap < core.len() {
            return None;
        }
        query.to_string()
    };
    Some(format!("person {subject}"))
}

/// Positive context is a stable ranking preference, never permission to
/// bypass photo/brand/subject fences or the original visual judgment. Sparse
/// metadata stays eligible, and tied hits retain the existing title ranking.
pub(super) fn rank_context(mut hits: Vec<RawHit>, query: &str, intent: &str) -> Vec<RawHit> {
    let human = human_activity_query(query, intent).is_some()
        || (human_activity(intent).is_some()
            && lexical_words(query).iter().any(|word| is_human_word(word)));
    let original = normalized_words(query);
    let mut modifiers = Vec::new();
    for word in normalized_words(&intent_rewrite(query, intent).unwrap_or_default()) {
        if concrete_visual_modifier(&word)
            && !original.contains(&word)
            && !modifiers.contains(&word)
        {
            modifiers.push(word);
        }
    }
    if !human && modifiers.is_empty() {
        return hits;
    }
    hits.sort_by_cached_key(|hit| {
        let words = matching::fence_words(&without_hashtags(&hit.relevance_metadata));
        let person = human && words.iter().any(|word| is_human_word(word));
        let physical = modifiers.iter().filter(|word| words.contains(word)).count();
        std::cmp::Reverse((person, physical))
    });
    hits
}

#[cfg(test)]
#[path = "relevance_intent_tests.rs"]
mod tests;
