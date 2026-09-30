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
    // Camera / editorial framing, not the subject: "cheeseburger close-up"
    // made "close-up" a second required subject word that Openverse tags
    // spell "closeup" or "close", and burger photos failed the 2-word bar.
    "editorial",
    "closeup",
    "close-up",
    "macro",
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
    // Painted/drawn media that Openverse files as ordinary photo uploads: a
    // street mural of a "snake oil serum" bottle is artwork, not a product
    // photo.
    "graffiti",
    "mural",
    "cartoon",
    "clipart",
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

pub(crate) fn two_keyword_retry(query: &str) -> Option<String> {
    // Camera/style words ("closeup", "shot") go before any subject word:
    // keeping the tail of "skincare cream texture closeup" used to drop the
    // domain word and search "cream texture closeup", which ranked a child
    // eating vanilla ice cream as a skincare hero (arena l01, 2026-09-28).
    let core: Vec<String> = concrete_query_words(query)
        .into_iter()
        .filter(|word| !INTENT_STYLE_WORDS.contains(&canonicalize_word(word).as_str()))
        .collect();
    // Product prompts commonly put the searchable subject noun near the end
    // (for example "... table lamp terracotta"). Keeping the concrete tail
    // avoids truncating that noun while preserving the existing behavior for
    // short two- and three-word subject phrases.
    let core_tail = &core[core.len().saturating_sub(3)..];
    let retry = core_tail.join(" ");
    let primary = lexical_words(query).join(" ");
    // No shorter retry for a three-word subject: dropping its leading word
    // loses the domain ("rose serum bottle" → a 1960s medical serum,
    // "camellia cream jar" → cake in a jar; measured 2026-09-30). An empty
    // slot is better than a wrong picture.
    (!retry.is_empty() && retry != primary).then_some(retry)
}

/// Style and staging words stripped from an intent-derived rewrite. The
/// judged-ladder rewrite wants the subject of the image prompt, not its art
/// direction. Multi-word forms are matched per token because the tokenizer
/// splits on non-alphanumerics ("close-up" → "close up", "high quality" →
/// "high" + "quality").
const INTENT_STYLE_WORDS: &[&str] = &[
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
/// 1. The last two content words of the query.
/// 2. The leading subject words of the intent (the image prompt) with stop
///    words and style words removed, capped at three words.
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
    push(intent_rewrite(intent));
    rewrites
}

/// The last two content words of the query — tighter than
/// [`two_keyword_retry`]'s three-word tail, which equals the primary query
/// for the common three-word subject phrase and therefore never fires there.
fn two_keyword_rewrite(query: &str) -> Option<String> {
    let core = concrete_query_words(query);
    let tail = &core[core.len().saturating_sub(2)..];
    (!tail.is_empty()).then(|| tail.join(" "))
}

/// The leading subject words of the intent with stop words and style words
/// removed, capped at three words. `None` when nothing searchable remains.
fn intent_rewrite(intent: &str) -> Option<String> {
    let words: Vec<String> = lexical_words(intent)
        .into_iter()
        .filter(|word| {
            !IMAGE_SEARCH_STOP_WORDS.contains(&word.as_str())
                && !INTENT_STYLE_WORDS.contains(&word.as_str())
        })
        .take(3)
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
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

/// Lower-cased ASCII word tokens. A hyphen joining two alphanumerics stays
/// inside the word ("push-up", "t-shirt"): splitting it used to turn the
/// rewrite of "push-up exercise" into "up exercise".
fn lexical_words(value: &str) -> Vec<String> {
    let lower = value.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut normalized = String::with_capacity(value.len());
    for (index, &ch) in chars.iter().enumerate() {
        let joins_word = ch == '-'
            && index > 0
            && chars[index - 1].is_ascii_alphanumeric()
            && chars
                .get(index + 1)
                .is_some_and(char::is_ascii_alphanumeric);
        if ch.is_ascii_alphanumeric() || joins_word {
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
pub(crate) fn retain_relevant_hits(hits: Vec<RawHit>, query: &str) -> Vec<RawHit> {
    retain_relevant_hits_for_intent(hits, query, "")
}

/// [`retain_relevant_hits`] with the slot's authored intent (its image
/// prompt). Search queries are terse ("cleanser tube white") while the
/// photographic contract usually lives only in the prompt ("minimal product
/// photography of …"); without it the photo fence never engaged and an
/// Instagram tattoo shot that merely mentions "cleanser" in its caption
/// passed as a product photo. The intent only ever tightens the photo
/// contract — subject words still come from the query.
pub(crate) fn retain_relevant_hits_for_intent(
    hits: Vec<RawHit>,
    query: &str,
    intent: &str,
) -> Vec<RawHit> {
    let contract = RelevanceContract::new(query, intent);
    let strict = retain_relevant_hits_enforcing(hits.clone(), &contract, true);
    if !strict.is_empty() {
        return strict;
    }
    // Isolation evidence ("isolated", "white background", …) is sparse in
    // provider metadata, and treating it as a hard fence empties the result
    // set for perfectly good product queries — the slot then publishes as a
    // gray placeholder. When the strict pass keeps nothing, degrade the
    // isolation requirement to a preference and keep the subject fence.
    retain_relevant_hits_enforcing(hits, &contract, false)
}

/// Why the fence would drop `hit` for `query`/`intent`, or `None` when it
/// survives the lenient (isolation-as-preference) pass. Diagnostics only —
/// the search trace prints it next to every provider hit.
pub(crate) fn fence_rejection(hit: &RawHit, query: &str, intent: &str) -> Option<&'static str> {
    fence_verdict(hit, &RelevanceContract::new(query, intent), false).err()
}

/// The query/intent-derived facts every hit is fenced against.
struct RelevanceContract<'a> {
    query: &'a str,
    core: Vec<String>,
    /// The QUERY itself asks for a photo ("… studio photo"): the title must
    /// name the subject and staged rooms are off.
    requires_photo: bool,
    /// Only the slot's image PROMPT asks for a photo ("minimal product
    /// photography of …"): artwork media are rejected, nothing more. Holding
    /// these slots to the title rule too turned 1 in 9 skincare / food slots
    /// into grey tiles — Openverse titles photos "[139/365]" or "Day 330" and
    /// names the subject only in tags.
    photo_medium_only: bool,
    requires_isolation: bool,
    minimum_overlap: usize,
}

impl<'a> RelevanceContract<'a> {
    fn new(query: &'a str, intent: &str) -> Self {
        let core = core_query_words(query);
        // Multi-word product subjects need more than one token of evidence.
        // A single generic overlap such as "lamp" previously accepted
        // "Photography lamp setup" for "ceramic table lamp studio photo",
        // which is technically an image but visibly the wrong product.
        // Single-word subjects still use one match so common queries such as
        // "armchair" keep their useful recall.
        let minimum_overlap = if core.len() >= 2 { 2 } else { 1 };
        Self {
            query,
            requires_photo: query_requests_photo(query),
            photo_medium_only: !query_requests_photo(query) && query_requests_photo(intent),
            requires_isolation: query_requests_isolation(query),
            core,
            minimum_overlap,
        }
    }
}

/// Ranking key of a hit that passed the fence: (title overlap, title subject
/// tokens, title extra tokens, total overlap).
type RankKey = (usize, usize, usize, usize);

fn fence_verdict(
    hit: &RawHit,
    contract: &RelevanceContract<'_>,
    enforce_isolation: bool,
) -> Result<RankKey, &'static str> {
    let metadata = &hit.relevance_metadata;
    let query = contract.query;
    if metadata_is_off_subject(metadata, query) {
        return Err("off-subject theme or unrequested brand");
    }
    if (contract.requires_photo || contract.photo_medium_only)
        && metadata_is_explicitly_non_photo(metadata)
    {
        return Err("photo requested but metadata names a non-photo medium");
    }
    if enforce_isolation
        && contract.requires_isolation
        && !metadata_has_isolation_evidence(metadata)
    {
        return Err("isolation requested but no isolation evidence");
    }
    if contract.core.is_empty() {
        return Ok((0, 0, 0, 0));
    }
    if contract.requires_photo && metadata_is_scene_heavy(metadata) && !query_requests_scene(query)
    {
        return Err("photo requested but metadata describes a staged room");
    }
    let core = &contract.core;
    // Hashtag soup in a title ("… #beauty #skincare #model") is the uploader's
    // reach bait, not a description: an event crowd shot titled that way
    // passed as a "skincare model" photo (2026-09-30). Only prose words and
    // the provider's own tags count as subject evidence.
    let title = normalized_words(&without_hashtags(&hit.title));
    let metadata = normalized_words(&without_hashtags(metadata));
    let title_overlap = overlap_count(core, &title);
    if contract.requires_photo && title_overlap == 0 {
        return Err("photo requested but no subject word in the title");
    }
    let total_overlap = overlap_count(core, &metadata);
    if total_overlap < contract.minimum_overlap {
        return Err("too few subject words in title/tags");
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
    Ok((
        title_overlap,
        title_subject_tokens,
        title_extra_tokens,
        total_overlap,
    ))
}

fn without_hashtags(text: &str) -> String {
    text.split_whitespace()
        .filter(|word| !word.starts_with('#'))
        .collect::<Vec<_>>()
        .join(" ")
}

fn retain_relevant_hits_enforcing(
    hits: Vec<RawHit>,
    contract: &RelevanceContract<'_>,
    enforce_isolation: bool,
) -> Vec<RawHit> {
    let mut ranked: Vec<(RankKey, RawHit)> = hits
        .into_iter()
        .filter_map(|hit| {
            fence_verdict(&hit, contract, enforce_isolation)
                .ok()
                .map(|key| (key, hit))
        })
        .collect();
    if contract.core.is_empty() {
        // No concrete subject: keep provider order among the survivors.
        return ranked.into_iter().map(|(_, hit)| hit).collect();
    }
    // Prefer a subject-dense title before raw overlap: a concise "Ceramic
    // vase" is a safer product match than a long archaeological title that
    // happens to contain both words. Then prefer title evidence, concision,
    // and finally title + tag evidence. `sort_by` is stable, so exact ties
    // retain provider order.
    ranked.sort_by(|(left, _), (right, _)| {
        let density = (right.0 * left.1).cmp(&(left.0 * right.1));
        density
            .then_with(|| right.0.cmp(&left.0))
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| right.3.cmp(&left.3))
    });
    ranked.into_iter().map(|(_, hit)| hit).collect()
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
    // Brand names are matched on raw lexical words: canonicalisation would
    // turn "starbucks" into "starbuck" and miss the list.
    if super::brand_fence::names_unrequested_brand(&lexical_words(metadata), &lexical_words(query))
    {
        return true;
    }
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

#[cfg(test)]
mod rewrite_queries_tests {
    use super::rewrite_queries;

    #[test]
    fn two_keyword_rewrite_precedes_the_intent_rewrite() {
        assert_eq!(
            rewrite_queries(
                "female dermatologist portrait",
                "professional photo of a female dermatologist in a clinic",
            ),
            vec![
                "dermatologist portrait".to_string(),
                "female dermatologist clinic".to_string(),
            ]
        );
    }

    #[test]
    fn intent_rewrite_strips_style_words() {
        assert_eq!(
            rewrite_queries(
                "fitness model gym workout",
                "professional photo of a fitness model in a gym, cinematic lighting, 4k hd",
            ),
            vec!["gym workout".to_string(), "fitness model gym".to_string(),]
        );
    }

    #[test]
    fn empty_intent_keeps_only_the_two_keyword_rewrite() {
        assert_eq!(
            rewrite_queries("oak desk lamp", ""),
            vec!["desk lamp".to_string()]
        );
    }

    #[test]
    fn rewrites_matching_the_query_or_each_other_are_dropped() {
        // Both rewrites resolve to the primary query itself: nothing to retry.
        assert!(rewrite_queries(
            "dermatologist portrait",
            "photo of a dermatologist portrait"
        )
        .is_empty());
        // The intent rewrite duplicates the two-keyword rewrite: deduped.
        assert_eq!(
            rewrite_queries(
                "dermatologist clinic portrait",
                "professional photo clinic portrait",
            ),
            vec!["clinic portrait".to_string()]
        );
    }
}
