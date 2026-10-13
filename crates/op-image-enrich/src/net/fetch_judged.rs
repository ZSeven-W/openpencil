//! Selection state shared by every tier of one visual image search.

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::{
    claim_unused_image_src, image_content_identity, resolved, PreparedJudgeCandidate, ResolvedImage,
};
use crate::net::judge::select_with_judge_before;
use crate::net::search_trace::trace_line;
use crate::net::{ImageRelevanceJudge, JudgeSelection, RelevanceVerdict};

const WEAK_UPGRADE_WINDOW: Duration = Duration::from_secs(20);
const FALLBACK_DELIVERY_MARGIN: Duration = Duration::from_millis(250);

/// Search rewrites may broaden retrieval, but cannot change what the image
/// must depict. All provider tiers judge against this original slot brief.
pub(super) struct JudgeSlot<'a> {
    pub query: &'a str,
    pub intent: &'a str,
    pub judge: &'a dyn ImageRelevanceJudge,
    pub deadline: Option<Instant>,
}

impl JudgeSlot<'_> {
    pub fn select(&self, batches: &[Vec<Vec<u8>>]) -> JudgeSelection {
        select_with_judge_before(self.judge, self.query, self.intent, batches, self.deadline)
    }
}

pub(super) struct JudgedImage {
    pub title: String,
    pub src: String,
    pub verdict: RelevanceVerdict,
    pub identity: Option<String>,
}

pub(super) fn claim_ranked_image(
    selection: &JudgeSelection,
    candidates: &[PreparedJudgeCandidate],
    used_urls: &Mutex<HashSet<String>>,
    allow_weak: bool,
) -> Option<JudgedImage> {
    for index in selection.ordered_indices() {
        let verdict = selection.verdict_for(*index)?;
        if verdict == RelevanceVerdict::Weak && !allow_weak {
            continue;
        }
        let Some(candidate) = candidates.get(*index) else {
            continue;
        };
        if let Some(identity) = &candidate.identity {
            if !used_urls.lock().unwrap().insert(identity.clone()) {
                continue;
            }
        }
        if claim_unused_image_src(used_urls, &candidate.data_url) {
            return Some(JudgedImage {
                title: candidate.title.clone(),
                src: candidate.data_url.clone(),
                verdict,
                identity: candidate.identity.clone(),
            });
        }
        // Duplicate content keeps its page identity claimed, as in the
        // unjudged provider path, while the next ranked candidate is tried.
    }
    None
}

/// Reserve just one Weak image while the existing finite ladder searches
/// for On. This retains the usable fallback without keeping whole provider
/// batches in memory or letting another slot claim the same fallback.
pub(super) struct WeakFallback<'a> {
    used_urls: &'a Mutex<HashSet<String>>,
    image: Option<(ResolvedImage, Option<String>)>,
    outer_deadline: Option<Instant>,
    deadline: Option<Instant>,
}

impl<'a> WeakFallback<'a> {
    pub fn new(used_urls: &'a Mutex<HashSet<String>>, outer_deadline: Option<Instant>) -> Self {
        Self {
            used_urls,
            image: None,
            outer_deadline,
            deadline: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.image.is_none()
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub fn can_continue(&self, judge: &dyn ImageRelevanceJudge) -> bool {
        if self.image.is_none() {
            return true;
        }
        let allowed = judge.supports_deadline_judging()
            && self
                .deadline
                .is_some_and(|deadline| Instant::now() < deadline);
        if !allowed {
            eprintln!(
                "[ENRICH] weak-upgrade: stop (deadline exhausted or bounded judge unavailable)"
            );
        }
        allowed
    }

    pub fn consider(
        &mut self,
        provider: &'static str,
        query: &str,
        image: JudgedImage,
    ) -> Option<ResolvedImage> {
        let result = resolved(provider, query, (image.title, image.src));
        if image.verdict == RelevanceVerdict::On {
            self.release();
            eprintln!("[ENRICH] image-selected: verdict=On {}", result.describe());
            trace_line(format_args!("chosen: {} (judge=On)", result.describe()));
            return Some(result);
        }
        debug_assert_eq!(image.verdict, RelevanceVerdict::Weak);
        debug_assert!(self.image.is_none());
        trace_line(format_args!(
            "retained fallback: {} (judge=Weak)",
            result.describe()
        ));
        self.image = Some((result, image.identity));
        let window_end = Instant::now() + WEAK_UPGRADE_WINDOW;
        self.deadline = Some(self.outer_deadline.map_or(window_end, |deadline| {
            window_end.min(
                deadline
                    .checked_sub(FALLBACK_DELIVERY_MARGIN)
                    .unwrap_or(deadline),
            )
        }));
        None
    }

    pub fn finish(mut self) -> Option<ResolvedImage> {
        self.image.take().map(|(image, _)| {
            eprintln!(
                "[ENRICH] image-selected: verdict=Weak fallback=true {}",
                image.describe()
            );
            trace_line(format_args!(
                "chosen: {} (judge=Weak fallback)",
                image.describe()
            ));
            image
        })
    }

    fn release(&mut self) {
        if let Some((image, identity)) = self.image.take() {
            let mut used = self.used_urls.lock().unwrap();
            used.remove(&image_content_identity(&image.src));
            if let Some(identity) = identity {
                used.remove(&identity);
            }
        }
    }
}

/// Catalogue fetch, pacing, retries and thumbnail downloads share the same
/// opportunity deadline. Bounded judges enforce it inside their sync bridge.
#[derive(Debug)]
pub(super) struct SearchDeadlineElapsed;

pub(super) async fn before_deadline<F: std::future::Future>(
    deadline: Option<Instant>,
    future: F,
) -> Result<F::Output, SearchDeadlineElapsed> {
    let Some(deadline) = deadline else {
        return Ok(future.await);
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(SearchDeadlineElapsed);
    }
    tokio::time::timeout(remaining, future).await.map_err(|_| {
        eprintln!("[ENRICH] weak-upgrade: deadline reached; deliver retained fallback");
        SearchDeadlineElapsed
    })
}

impl Drop for WeakFallback<'_> {
    fn drop(&mut self) {
        // Release a provisional reservation on cancellation as well as when
        // a later tier wins. `finish` transfers the claim to the caller.
        self.release();
    }
}

#[cfg(test)]
#[path = "fetch_judged_tests.rs"]
mod tests;
