//! Query simplification and relevance filtering for the image-search
//! providers: keyword extraction, the two-keyword retry, and the
//! photo/scene/isolation fences over provider metadata. Carved out of the
//! `providers.rs` spine to keep it under the 800-line cap; pure code motion.

use super::RawHit;

/// Design-artifact words that are pure noise against a photo corpus (see
/// the desktop `image_search_session.rs` for the measurement notes).
const IMAGE_SEARCH_ARTIFACT_WORDS: &[&str] = &[
    "album",
    "cover",
    "playlist",
    "artwork",
    "poster",
    "thumbnail",
    "logo",
    "icon",
    "banner",
    "mockup",
    "screenshot",
    "wallpaper",
];

const IMAGE_SEARCH_STOP_WORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "and",
    "or",
    "but",
    "in",
    "on",
    "at",
    "to",
    "for",
    "of",
    "with",
    "by",
    "from",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "could",
    "should",
    "may",
    "might",
    "shall",
    "can",
    "that",
    "this",
    "these",
    "those",
    "it",
    "its",
    "very",
    "really",
    "just",
    "also",
    "about",
    "above",
    "after",
    "before",
    "between",
    "into",
    "through",
    "during",
    "each",
    "some",
    "such",
    "no",
    "not",
    "only",
    "same",
    "so",
    "than",
    "too",
    "up",
    "out",
    "if",
    "then",
    "once",
    "here",
    "there",
    "when",
    "where",
    "how",
    "all",
    "both",
    "few",
    "more",
    "most",
    "other",
    "any",
    "as",
    "while",
    "using",
    "showing",
    "featuring",
    "looking",
    "style",
    "styled",
    "inspired",
    "based",
];

/// Presentation adjectives and staging words that make a zero-result retry
/// less searchable. They stay in the primary query; only the shortened retry
/// removes them so the provider receives the concrete subject phrase.
const IMAGE_SEARCH_DESCRIPTORS: &[&str] = &[
    "minimal",
    "minimalist",
    "warm",
    "modern",
    "neutral",
    "tone",
    "surface",
    "beige",
    "background",
    "shelf",
    "product",
    "photo",
    "photography",
    "isolated",
    "studio",
];

/// Metadata markers that make a catalogue hit an explicitly non-photographic
/// result. They are only a fence when the authored query itself asks for a
/// photo/studio result; illustration searches must keep working unchanged.
const NON_PHOTO_RESULT_WORDS: &[&str] = &[
    "illustration",
    "illustrated",
    "drawing",
    "engraving",
    "painting",
    "diagram",
    "sketch",
    "poster",
    "catalog",
    "catalogue",
];

/// Result themes that are usually adjacent catalogue noise rather than the
/// requested product. They remain valid when the authored query explicitly
/// asks for that theme (for example "plush toy studio photo").
const OFF_SUBJECT_RESULT_GROUPS: &[&[&str]] = &[
    &["toy", "plush", "teddy", "doll", "figurine", "stuffed"],
    &["collage", "montage", "puzzle", "pattern"],
];

/// Metadata that usually describes a staged room rather than an isolated
/// catalogue subject. Product-photo prompts reject these results unless the
/// authored query explicitly asks for a room/interior scene.
pub(super) const SCENE_HEAVY_RESULT_WORDS: &[&str] = &[
    "room", "house", "interior", "hotel", "bedroom", "kitchen", "dining", "hallway", "lounge",
];

/// Query words that explicitly opt into a staged scene. `lounge` is omitted:
/// it is also a product descriptor in phrases such as "lounge chair". A
/// genuine scene request still opts in through `room`, `interior`, or the
/// explicit `hotel lounge` phrase handled by [`query_requests_scene`].
const SCENE_QUERY_OPT_IN_WORDS: &[&str] = &[
    "room", "house", "interior", "bedroom", "kitchen", "dining", "hallway",
];

/// Positive catalogue evidence that a result is presented independently from
/// a room scene. An authored `isolated` query is stricter than a generic
/// `studio photo` query and requires one of these signals (or an equivalent
/// white/plain/transparent-background phrase).
const ISOLATION_RESULT_WORDS: &[&str] = &["isolated", "isolate", "isolation", "cutout"];

pub(super) fn two_keyword_retry(query: &str) -> Option<String> {
    let core = concrete_query_words(query);
    // Product prompts commonly put the searchable subject noun near the end
    // (for example "... table lamp terracotta"). Keeping the concrete tail
    // avoids truncating that noun while preserving the existing behavior for
    // short two- and three-word subject phrases.
    let core_tail = &core[core.len().saturating_sub(3)..];
    let retry = core_tail.join(" ");
    let primary = lexical_words(query).join(" ");
    (!retry.is_empty() && retry != primary).then_some(retry)
}

pub(super) fn core_query_words(query: &str) -> Vec<String> {
    concrete_query_words(query)
        .into_iter()
        .map(|word| canonicalize_word(&word))
        .collect()
}

fn concrete_query_words(query: &str) -> Vec<String> {
    lexical_words(query)
        .into_iter()
        .filter(|word| {
            let canonical = canonicalize_word(word);
            !IMAGE_SEARCH_DESCRIPTORS.contains(&canonical.as_str())
        })
        .collect()
}

fn lexical_words(value: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(value.len());
    for ch in value.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }
    normalized.split_whitespace().map(str::to_string).collect()
}

fn normalized_words(value: &str) -> Vec<String> {
    lexical_words(value)
        .into_iter()
        .map(|word| canonicalize_word(&word))
        .collect()
}

fn canonicalize_word(word: &str) -> String {
    match word {
        "knit" | "knitted" | "knitting" => return "knit".to_string(),
        "wood" | "wooden" => return "wood".to_string(),
        _ => {}
    }

    if word.len() > 4 && word.ends_with("ies") {
        return format!("{}y", &word[..word.len() - 3]);
    }
    if word.len() > 4
        && ["ches", "shes", "xes", "zes"]
            .iter()
            .any(|suffix| word.ends_with(suffix))
    {
        return word[..word.len() - 2].to_string();
    }
    if word.len() > 3
        && word.ends_with('s')
        && !word.ends_with("ss")
        && !word.ends_with("us")
        && !word.ends_with("is")
    {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// Keep only provider hits whose title/tags mention at least one concrete
/// subject word. When a query contains no concrete words (for example an
/// all-descriptor prompt), preserve the provider response rather than making
/// an unprovable relevance decision.
pub(super) fn retain_relevant_hits(hits: Vec<RawHit>, query: &str) -> Vec<RawHit> {
    let strict = retain_relevant_hits_enforcing(hits.clone(), query, true);
    if !strict.is_empty() {
        return strict;
    }
    // Isolation evidence ("isolated", "white background", …) is sparse in
    // provider metadata, and treating it as a hard fence empties the result
    // set for perfectly good product queries — the slot then publishes as a
    // gray placeholder. When the strict pass keeps nothing, degrade the
    // isolation requirement to a preference and keep the subject fence.
    retain_relevant_hits_enforcing(hits, query, false)
}

fn retain_relevant_hits_enforcing(
    hits: Vec<RawHit>,
    query: &str,
    enforce_isolation: bool,
) -> Vec<RawHit> {
    let core = core_query_words(query);
    let requires_photo = query_requests_photo(query);
    let requires_isolation = enforce_isolation && query_requests_isolation(query);
    if core.is_empty() {
        return hits
            .into_iter()
            .filter(|hit| {
                !metadata_is_off_subject(&hit.relevance_metadata, query)
                    && (!requires_photo
                        || !metadata_is_explicitly_non_photo(&hit.relevance_metadata))
                    && (!requires_isolation
                        || metadata_has_isolation_evidence(&hit.relevance_metadata))
            })
            .collect();
    }
    // Multi-word product subjects need more than one token of evidence. A
    // single generic overlap such as "lamp" previously accepted
    // "Photography lamp setup" for "ceramic table lamp studio photo", which
    // is technically an image but visibly the wrong product. Single-word
    // subjects still use one match so common queries such as "armchair" keep
    // their useful recall.
    let minimum_overlap = if core.len() >= 2 { 2 } else { 1 };
    let mut ranked: Vec<(usize, usize, usize, usize, RawHit)> = hits
        .into_iter()
        .filter_map(|hit| {
            if metadata_is_off_subject(&hit.relevance_metadata, query)
                || (requires_photo && metadata_is_explicitly_non_photo(&hit.relevance_metadata))
                || (requires_isolation && !metadata_has_isolation_evidence(&hit.relevance_metadata))
                || (requires_photo
                    && metadata_is_scene_heavy(&hit.relevance_metadata)
                    && !query_requests_scene(query))
            {
                return None;
            }
            let title = normalized_words(&hit.title);
            let metadata = normalized_words(&hit.relevance_metadata);
            let title_overlap = overlap_count(&core, &title);
            if requires_photo && title_overlap == 0 {
                return None;
            }
            let total_overlap = overlap_count(&core, &metadata);
            if total_overlap < minimum_overlap {
                return None;
            }
            let title_extra_tokens = title
                .iter()
                .filter(|word| {
                    !core.contains(word)
                        && !IMAGE_SEARCH_STOP_WORDS.contains(&word.as_str())
                        && !IMAGE_SEARCH_DESCRIPTORS.contains(&word.as_str())
                })
                .count();
            let title_subject_tokens = title_overlap + title_extra_tokens;
            Some((
                title_overlap,
                title_subject_tokens,
                title_extra_tokens,
                total_overlap,
                hit,
            ))
        })
        .collect();
    // Prefer a subject-dense title before raw overlap: a concise "Ceramic
    // vase" is a safer product match than a long archaeological title that
    // happens to contain both words. Then prefer title evidence, concision,
    // and finally title + tag evidence. `sort_by` is stable, so exact ties
    // retain provider order.
    ranked.sort_by(|left, right| {
        let density = (right.0 * left.1).cmp(&(left.0 * right.1));
        density
            .then_with(|| right.0.cmp(&left.0))
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| right.3.cmp(&left.3))
    });
    ranked.into_iter().map(|(_, _, _, _, hit)| hit).collect()
}

fn overlap_count(core: &[String], candidate: &[String]) -> usize {
    core.iter().filter(|word| candidate.contains(word)).count()
}

fn query_requests_photo(query: &str) -> bool {
    normalized_words(query).iter().any(|word| {
        matches!(
            word.as_str(),
            "photo" | "photograph" | "photography" | "studio" | "isolated"
        )
    })
}

fn metadata_is_explicitly_non_photo(metadata: &str) -> bool {
    normalized_words(metadata)
        .iter()
        .any(|word| NON_PHOTO_RESULT_WORDS.contains(&word.as_str()))
}

fn metadata_is_off_subject(metadata: &str, query: &str) -> bool {
    let metadata = normalized_words(metadata);
    let query = normalized_words(query);
    OFF_SUBJECT_RESULT_GROUPS.iter().any(|group| {
        let query_requests_group = query.iter().any(|word| group.contains(&word.as_str()));
        !query_requests_group && metadata.iter().any(|word| group.contains(&word.as_str()))
    })
}

pub(super) fn query_requests_scene(query: &str) -> bool {
    let words = normalized_words(query);
    words
        .iter()
        .any(|word| SCENE_QUERY_OPT_IN_WORDS.contains(&word.as_str()))
        || contains_adjacent_words(&words, "hotel", "lounge")
}

pub(super) fn metadata_is_scene_heavy(metadata: &str) -> bool {
    let words = normalized_words(metadata);
    words
        .iter()
        .any(|word| word != "lounge" && SCENE_HEAVY_RESULT_WORDS.contains(&word.as_str()))
        || words.iter().enumerate().any(|(index, word)| {
            word == "lounge" && words.get(index + 1).is_none_or(|next| next != "chair")
        })
}

fn query_requests_isolation(query: &str) -> bool {
    normalized_words(query).iter().any(|word| {
        matches!(
            word.as_str(),
            "isolated" | "isolate" | "isolation" | "cutout"
        )
    })
}

fn metadata_has_isolation_evidence(metadata: &str) -> bool {
    let words = normalized_words(metadata);
    words
        .iter()
        .any(|word| ISOLATION_RESULT_WORDS.contains(&word.as_str()))
        || contains_adjacent_words(&words, "cut", "out")
        || contains_adjacent_words(&words, "white", "background")
        || contains_adjacent_words(&words, "white", "backdrop")
        || contains_adjacent_words(&words, "plain", "background")
        || contains_adjacent_words(&words, "transparent", "background")
        || contains_adjacent_words(&words, "on", "white")
}

fn contains_adjacent_words(words: &[String], first: &str, second: &str) -> bool {
    words
        .windows(2)
        .any(|pair| pair[0] == first && pair[1] == second)
}

/// Simplify a verbose prompt into provider keywords. Shared by the desktop
/// image pipeline and the web daemon route.
pub fn simplify_search_query(prompt: &str) -> String {
    let mut normalized = String::with_capacity(prompt.len());
    for ch in prompt.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '-' {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }
    let keywords: Vec<&str> = normalized
        .split_whitespace()
        .filter(|word| word.len() > 2 && !IMAGE_SEARCH_STOP_WORDS.contains(word))
        .take(6)
        .collect();
    // Drop artifact words ONLY when aesthetic words remain — "logo" alone
    // must not become an empty query.
    let non_artifact: Vec<&str> = keywords
        .iter()
        .copied()
        .filter(|word| !IMAGE_SEARCH_ARTIFACT_WORDS.contains(word))
        .collect();
    let keywords: Vec<&str> = if non_artifact.is_empty() {
        keywords
    } else {
        non_artifact
    }
    .into_iter()
    .take(4)
    .collect();
    if keywords.is_empty() {
        prompt.chars().take(30).collect()
    } else {
        keywords.join(" ")
    }
}
