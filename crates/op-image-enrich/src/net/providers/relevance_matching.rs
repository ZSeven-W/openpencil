//! Match catalogue spelling variants without rewriting the authored query.

use super::{canonicalize_word, lexical_words};

/// Negative metadata fences must see the same compound constituents as
/// positive matching. Keep the original token too, and retain raw spelling
/// for brands. Explicit query opt-ins use this view symmetrically; provider
/// query generation remains unchanged. This is not positive isolation
/// evidence: non-isolated must never become an isolated-photo assertion.
pub(super) fn fence_words(value: &str) -> Vec<String> {
    lexical_words(value)
        .into_iter()
        .flat_map(|word| {
            let mut words = vec![word.clone()];
            if word.contains('-') {
                words.extend(word.split('-').map(str::to_owned));
            }
            words
        })
        .collect()
}

pub(super) fn normalized_fence_words(value: &str) -> Vec<String> {
    fence_words(value)
        .iter()
        .map(|word| canonicalize_word(word))
        .collect()
}

pub(super) struct Evidence {
    pub overlap: usize,
    pub unmatched: Vec<String>,
}

struct Word {
    canonical: String,
    action_alias: Option<String>,
}

impl Word {
    fn new(word: &str) -> Self {
        Self {
            canonical: canonicalize_word(word),
            action_alias: action_alias(word).map(str::to_owned),
        }
    }

    fn matches(&self, other: &Self) -> bool {
        self.canonical == other.canonical
            || self.action_alias.as_ref() == Some(&other.canonical)
            || other.action_alias.as_ref() == Some(&self.canonical)
            || self
                .action_alias
                .as_ref()
                .zip(other.action_alias.as_ref())
                .is_some_and(|(left, right)| left == right)
    }
}

/// Only a narrow regular inflection class: stretch/stretching/stretches,
/// wash/washing/washes, press/pressing/presses. No general -ing stripping,
/// doubled-consonant inference, silent-e restoration, or short stems. The
/// original spelling remains intact in provider queries and metadata fences.
fn action_alias(word: &str) -> Option<&str> {
    let stem = word
        .strip_suffix("ing")
        .or_else(|| word.strip_suffix("es"))?;
    (stem.len() >= 4
        && stem.bytes().all(|byte| byte.is_ascii_alphabetic())
        && ["ch", "sh", "ss"]
            .iter()
            .any(|suffix| stem.ends_with(suffix)))
    .then_some(stem)
}

fn parts(value: &str) -> Vec<Word> {
    lexical_words(value)
        .iter()
        .flat_map(|word| {
            if !word.contains('-') {
                return vec![Word::new(word)];
            }
            // Keep the old whole-word plural normalization too: push-ups
            // becomes push-up even though the short part "ups" stays intact
            // when it is an independent word.
            let canonical = canonicalize_word(word);
            word.split('-')
                .zip(canonical.split('-'))
                .map(|(original, canonical)| {
                    let mut word = Word::new(original);
                    word.canonical = canonicalize_word(canonical);
                    word
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn contains_parts(whole: &[Word], part: &[Word]) -> bool {
    !part.is_empty()
        && whole.windows(part.len()).any(|window| {
            part.iter()
                .zip(window)
                .all(|(left, right)| left.matches(right))
        })
}

/// Repeated spellings/aliases cannot supply independent subject evidence.
/// A compound also already requires its parts: blue/t-shirt/shirt still
/// needs blue and the complete t-shirt, rather than counting shirt twice.
pub(super) fn distinct_subjects(core: Vec<String>) -> Vec<String> {
    let mut distinct: Vec<String> = Vec::new();
    for subject in core {
        let subject_parts = parts(&subject);
        if distinct
            .iter()
            .any(|other| contains_parts(&parts(other), &subject_parts))
        {
            continue;
        }
        distinct.retain(|other| !contains_parts(&subject_parts, &parts(other)));
        distinct.push(subject);
    }
    distinct
}

/// Each original query condition contributes at most one match. A candidate
/// compound can support separate query terms (serum-bottle -> serum, bottle),
/// but a query compound needs all its adjacent parts (push-up != up).
pub(super) fn evidence(core: &[String], text: &str) -> Evidence {
    let candidate = parts(text);
    let mut covered = vec![false; candidate.len()];
    let mut overlap = 0;
    for subject in distinct_subjects(core.to_vec()) {
        let subject = parts(&subject);
        if subject.is_empty() {
            continue;
        }
        let mut matched = false;
        for (start, window) in candidate.windows(subject.len()).enumerate() {
            if subject
                .iter()
                .zip(window)
                .all(|(left, right)| left.matches(right))
            {
                covered[start..start + subject.len()].fill(true);
                matched = true;
            }
        }
        overlap += usize::from(matched);
    }
    Evidence {
        overlap,
        unmatched: candidate
            .into_iter()
            .zip(covered)
            .filter_map(|(word, covered)| (!covered).then_some(word.canonical))
            .collect(),
    }
}

#[cfg(test)]
#[path = "relevance_matching_tests.rs"]
mod tests;
