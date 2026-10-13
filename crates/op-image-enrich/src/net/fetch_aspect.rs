//! One bounded opportunity to preserve the subject while relaxing crop geometry.

use std::collections::HashSet;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::judged::{JudgedImage, SearchDeadlineElapsed, WeakFallback};
use super::{JudgeSlot, PreparedJudgeCandidate, RawHit, ResolvedImage};
use crate::net::{ImageRelevanceJudge, JudgeSelection, JudgedCandidate};
use crate::ImageAspectRatio;

const ASPECT_RELAX_WINDOW: Duration = Duration::from_secs(20);

pub(super) fn earlier_deadline(a: Option<Instant>, b: Option<Instant>) -> Option<Instant> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// Legacy synchronous judges keep their existing path. Added opportunities
/// require an explicit bounded implementation before any candidates download.
pub(super) fn judging_deadline(
    judge: &dyn ImageRelevanceJudge,
    deadline: Option<Instant>,
) -> Option<Instant> {
    deadline.filter(|_| judge.supports_deadline_judging())
}

/// Transport/parse failures are represented by the host as an unavailable Off.
/// Observe those verdicts before selection discards the reasons, so transient
/// failures never become a negative cache. Incomplete batches remain retryable.
struct CompletionTrackedJudge<'a> {
    inner: &'a dyn ImageRelevanceJudge,
    complete: AtomicBool,
}

impl CompletionTrackedJudge<'_> {
    fn record(&self, verdicts: Vec<JudgedCandidate>, expected: usize) -> Vec<JudgedCandidate> {
        let indices: HashSet<_> = verdicts.iter().map(|verdict| verdict.index).collect();
        if indices.len() != expected
            || indices.iter().any(|index| *index >= expected)
            || verdicts
                .iter()
                .any(|verdict| verdict.reason.trim() == "judge unavailable")
        {
            self.complete.store(false, Ordering::Relaxed);
        }
        verdicts
    }
}

impl ImageRelevanceJudge for CompletionTrackedJudge<'_> {
    fn judge(&self, query: &str, intent: &str, thumbs: &[Vec<u8>]) -> Vec<JudgedCandidate> {
        self.record(self.inner.judge(query, intent, thumbs), thumbs.len())
    }

    fn supports_deadline_judging(&self) -> bool {
        self.inner.supports_deadline_judging()
    }

    fn judge_before(
        &self,
        query: &str,
        intent: &str,
        thumbs: &[Vec<u8>],
        deadline: Instant,
    ) -> Vec<JudgedCandidate> {
        self.record(
            self.inner.judge_before(query, intent, thumbs, deadline),
            thumbs.len(),
        )
    }
}

pub(super) fn select_and_record(
    slot: &JudgeSlot<'_>,
    batches: &[Vec<Vec<u8>>],
    candidates: &[PreparedJudgeCandidate],
    seen: &Mutex<HashSet<String>>,
) -> JudgeSelection {
    let tracker = CompletionTrackedJudge {
        inner: slot.judge,
        complete: AtomicBool::new(true),
    };
    let tracked_slot = JudgeSlot {
        judge: &tracker,
        ..*slot
    };
    let selection = tracked_slot.select(batches);
    if tracker.complete.load(Ordering::Relaxed)
        && selection.on + selection.weak + selection.off == candidates.len()
    {
        seen.lock()
            .unwrap()
            .extend(candidates.iter().filter_map(|hit| hit.identity.clone()));
    }
    selection
}

/// A slot gets at most two original-query tiers: preferred aspect, then any
/// aspect. Each tier retains the existing two-list / ten-candidate ceiling.
/// A transport failure does not trigger another catalogue request here.
pub(super) async fn search_original_aspects<F, Fut>(
    query: &str,
    aspect: Option<ImageAspectRatio>,
    judge: &dyn ImageRelevanceJudge,
    outer_deadline: Option<Instant>,
    fallback: &mut WeakFallback<'_>,
    mut search: F,
) -> Result<(Option<ResolvedImage>, Option<ImageAspectRatio>), SearchDeadlineElapsed>
where
    F: FnMut(Option<ImageAspectRatio>, Option<Instant>, bool) -> Fut,
    Fut: Future<Output = Option<Option<JudgedImage>>>,
{
    // Once a geometry preference has been relaxed, existing query rewrites
    // must not silently restore it and lose the same candidates again.
    let mut rewrite_aspect = aspect;
    for (index, aspect) in std::iter::once(aspect)
        .chain(aspect.is_some().then_some(None))
        .enumerate()
    {
        if !fallback.can_continue(judge) {
            return Err(SearchDeadlineElapsed);
        }
        let mut deadline = earlier_deadline(outer_deadline, fallback.deadline());
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(SearchDeadlineElapsed);
        }
        if index > 0 {
            // An added opportunity may never start an unbounded sync judge.
            if !judge.supports_deadline_judging() {
                return Ok((None, rewrite_aspect));
            }
            deadline = earlier_deadline(deadline, Some(Instant::now() + ASPECT_RELAX_WINDOW));
            eprintln!("[ENRICH] aspect-relax: original query={query:?} aspect=any");
        }
        rewrite_aspect = aspect;
        let future = search(aspect, deadline, fallback.is_empty());
        let result = if let Some(deadline) = deadline {
            match tokio::time::timeout_at(deadline.into(), future).await {
                Ok(result) => result,
                Err(_) => {
                    eprintln!("[ENRICH] aspect-search: deadline reached");
                    if !fallback.is_empty()
                        || outer_deadline.is_some_and(|outer| Instant::now() >= outer)
                    {
                        return Err(SearchDeadlineElapsed);
                    }
                    return Ok((None, rewrite_aspect));
                }
            }
        } else {
            future.await
        };
        let transport_failed = result.is_none();
        if let Some(Some(image)) = result {
            let provider = if index == 0 {
                "openverse"
            } else {
                "openverse-aspect"
            };
            if let Some(winner) = fallback.consider(provider, query, image) {
                return Ok((Some(winner), rewrite_aspect));
            }
        }
        // A legacy synchronous judge may finish after the async deadline.
        // Keep completed winners, but never start a new tier after expiry.
        if earlier_deadline(outer_deadline, fallback.deadline())
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(SearchDeadlineElapsed);
        }
        if transport_failed {
            return Ok((None, rewrite_aspect));
        }
    }
    Ok((None, rewrite_aspect))
}

pub(super) fn hit_identity(hit: &RawHit) -> String {
    if hit.id.trim().is_empty() {
        format!("openverse:url:{}", hit.thumb_url)
    } else {
        format!("openverse:{}", hit.id)
    }
}

/// Only completely judged batches enter this per-slot set. Transport failures
/// remain retryable, and different slots still judge their own authored brief.
pub(super) fn unseen_hits(hits: Vec<RawHit>, seen: &Mutex<HashSet<String>>) -> Vec<RawHit> {
    let seen = seen.lock().unwrap();
    let mut in_batch = HashSet::new();
    hits.into_iter()
        .filter(|hit| {
            let identity = hit_identity(hit);
            !seen.contains(&identity) && in_batch.insert(identity)
        })
        .collect()
}

#[cfg(test)]
#[path = "fetch_aspect_tests.rs"]
mod tests;
