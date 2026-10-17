//! Visual relevance judging and the provider-independent re-ranking policy.

use std::time::Instant;

/// Coarse visual relevance for one candidate thumbnail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelevanceVerdict {
    On,
    Weak,
    Off,
}

/// One candidate's visual verdict. `index` addresses the input thumbnail
/// slice supplied to the judge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgedCandidate {
    pub index: usize,
    pub verdict: RelevanceVerdict,
    pub reason: String,
}

impl JudgedCandidate {
    /// Technical failures stay ineligible, but are not completed visual Off
    /// judgments. This sentinel is owned by the host, never model prose.
    pub fn is_unavailable(&self) -> bool {
        self.reason.trim() == "judge unavailable"
    }
}

/// Judge a set of thumbnails for one image slot. Implementations may issue
/// calls concurrently, but must return one entry per input thumbnail.
pub trait ImageRelevanceJudge: Send + Sync {
    fn judge(&self, query: &str, intent: &str, thumbs_jpeg: &[Vec<u8>]) -> Vec<JudgedCandidate>;

    /// Only opt in when the implementation can stop its transport by the
    /// deadline, including silent providers and retries. A synchronous judge
    /// cannot be bounded by wrapping its caller in an async timeout.
    fn supports_deadline_judging(&self) -> bool {
        false
    }

    /// Return completed verdicts by the deadline. Unsupported judges do not
    /// start an unbounded extra call when a usable Weak fallback exists.
    fn judge_before(
        &self,
        _query: &str,
        _intent: &str,
        _thumbs_jpeg: &[Vec<u8>],
        _deadline: Instant,
    ) -> Vec<JudgedCandidate> {
        Vec::new()
    }
}

/// The default keeps the pre-judge behavior: every candidate is eligible and
/// the provider's existing order remains authoritative.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoJudge;

impl ImageRelevanceJudge for NoJudge {
    fn judge(&self, _query: &str, _intent: &str, thumbs_jpeg: &[Vec<u8>]) -> Vec<JudgedCandidate> {
        thumbs_jpeg
            .iter()
            .enumerate()
            .map(|(index, _)| JudgedCandidate {
                index,
                verdict: RelevanceVerdict::On,
                reason: String::new(),
            })
            .collect()
    }

    fn supports_deadline_judging(&self) -> bool {
        true
    }

    fn judge_before(
        &self,
        query: &str,
        intent: &str,
        thumbs: &[Vec<u8>],
        deadline: Instant,
    ) -> Vec<JudgedCandidate> {
        if Instant::now() >= deadline {
            Vec::new()
        } else {
            self.judge(query, intent, thumbs)
        }
    }
}

/// The accumulated outcome of one or more judge rounds. Candidate indices are
/// global across the supplied batches, so the provider can map the selected
/// thumbnail back to its downloaded candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeSelection {
    pub picked: Option<usize>,
    pub rounds: usize,
    pub on: usize,
    pub weak: usize,
    pub off: usize,
    /// Returned technical failures, excluded from the semantic Off count.
    pub unavailable: usize,
    /// Candidates submitted in a judge call without a returned outcome. This
    /// does not imply that their transport never started.
    pub unreported: usize,
    k: usize,
    ordered: Vec<usize>,
}

impl JudgeSelection {
    pub fn k(&self) -> usize {
        self.k
    }

    /// On candidates in provider order, followed by Weak candidates in
    /// provider order. The provider can advance through this list when an
    /// already-used content digest wins the visual ranking.
    pub fn ordered_indices(&self) -> &[usize] {
        &self.ordered
    }

    /// Preserve the winning candidate's grade across provider tiers. An On
    /// count alone is insufficient when every On image is already used and
    /// the provider advances to a Weak candidate in the same result set.
    pub fn verdict_for(&self, index: usize) -> Option<RelevanceVerdict> {
        self.ordered
            .iter()
            .position(|item| *item == index)
            .map(|rank| {
                if rank < self.on {
                    RelevanceVerdict::On
                } else {
                    RelevanceVerdict::Weak
                }
            })
    }
}

/// Judge batches in order, stopping at the first batch with an On candidate.
/// A batch that only yields Weak candidates is remembered as the fallback and
/// the next batch is still judged: the 2026-09-05 A/B run showed 63% of judge
/// calls settling on a Weak first-round pick without ever looking at the
/// second batch, and the external audit rated a fifth of those picks Off.
/// A caller can provide at most two batches to enforce the image search cost
/// bound, and the thumbnails are already downloaded, so the extra round is one
/// judge call.
pub fn select_with_judge(
    judge: &dyn ImageRelevanceJudge,
    query: &str,
    intent: &str,
    batches: &[Vec<Vec<u8>>],
) -> JudgeSelection {
    select_with_judge_before(judge, query, intent, batches, None)
}

pub(crate) fn select_with_judge_before(
    judge: &dyn ImageRelevanceJudge,
    query: &str,
    intent: &str,
    batches: &[Vec<Vec<u8>>],
    deadline: Option<Instant>,
) -> JudgeSelection {
    let mut selection = JudgeSelection {
        picked: None,
        rounds: 0,
        on: 0,
        weak: 0,
        off: 0,
        unavailable: 0,
        unreported: 0,
        k: 0,
        ordered: Vec::new(),
    };
    let mut offset = 0;
    let mut weak_fallback: Vec<usize> = Vec::new();

    for batch in batches {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            break;
        }
        if batch.is_empty() {
            continue;
        }
        if selection.rounds == 0 {
            selection.k = batch.len();
        }
        selection.rounds += 1;
        let judged = match deadline {
            Some(deadline) => judge.judge_before(query, intent, batch, deadline),
            None => judge.judge(query, intent, batch),
        };
        let mut on = Vec::new();
        let mut weak = Vec::new();
        let mut reported = vec![false; batch.len()];
        for candidate in judged {
            if candidate.index >= batch.len() {
                continue;
            }
            reported[candidate.index] = true;
            match candidate.verdict {
                RelevanceVerdict::On => {
                    selection.on += 1;
                    on.push(offset + candidate.index);
                }
                RelevanceVerdict::Weak => {
                    selection.weak += 1;
                    weak.push(offset + candidate.index);
                }
                RelevanceVerdict::Off if candidate.is_unavailable() => {
                    selection.unavailable += 1;
                }
                RelevanceVerdict::Off => selection.off += 1,
            }
        }
        selection.unreported += reported.iter().filter(|reported| !**reported).count();

        if !on.is_empty() {
            selection.picked = on.first().copied();
            selection.ordered = on;
            // Earlier Weak candidates outrank this batch's Weak ones: they
            // came first in provider relevance order.
            selection.ordered.append(&mut weak_fallback);
            selection.ordered.extend(weak);
            return selection;
        }
        weak_fallback.extend(weak);
        offset += batch.len();
    }

    if !weak_fallback.is_empty() {
        selection.picked = weak_fallback.first().copied();
        selection.ordered = weak_fallback;
    }
    selection
}

/// Format the stable enrichment log line used by the desktop/headless
/// callers. Verdict counts are ordered On/Weak/semantic Off; technical
/// failures and missing outcomes are reported separately.
pub fn format_judge_log(selection: &JudgeSelection) -> String {
    format!(
        "judge: k={} verdicts={}/{}/{} picked={} rounds={} unavailable={} unreported={}",
        selection.k,
        selection.on,
        selection.weak,
        selection.off,
        selection
            .picked
            .map_or_else(|| "none".to_string(), |index| index.to_string()),
        selection.rounds,
        selection.unavailable,
        selection.unreported
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct ScriptedJudge {
        rounds: Mutex<Vec<Vec<RelevanceVerdict>>>,
    }

    impl ScriptedJudge {
        fn new(rounds: Vec<Vec<RelevanceVerdict>>) -> Self {
            Self {
                rounds: Mutex::new(rounds),
            }
        }
    }

    impl ImageRelevanceJudge for ScriptedJudge {
        fn judge(
            &self,
            _query: &str,
            _intent: &str,
            thumbs_jpeg: &[Vec<u8>],
        ) -> Vec<JudgedCandidate> {
            let verdicts = self.rounds.lock().expect("scripted judge lock").remove(0);
            thumbs_jpeg
                .iter()
                .enumerate()
                .map(|(index, _)| JudgedCandidate {
                    index,
                    verdict: verdicts[index],
                    reason: "scripted".into(),
                })
                .collect()
        }
    }

    fn thumbs(count: usize) -> Vec<Vec<u8>> {
        (0..count).map(|index| vec![index as u8]).collect()
    }

    #[test]
    fn no_judge_marks_every_thumbnail_on() {
        let result = NoJudge.judge("cat", "cat photo", &thumbs(3));
        assert_eq!(result.len(), 3);
        assert!(result.iter().all(|candidate| {
            candidate.verdict == RelevanceVerdict::On && candidate.reason.is_empty()
        }));
    }

    #[test]
    fn on_beats_an_earlier_weak_candidate() {
        let judge = ScriptedJudge::new(vec![vec![RelevanceVerdict::Weak, RelevanceVerdict::On]]);
        let result = select_with_judge(&judge, "chair", "a chair", &[thumbs(2)]);

        assert_eq!(result.picked, Some(1));
        assert_eq!(result.ordered_indices(), &[1, 0]);
        assert_eq!(result.rounds, 1);
    }

    #[test]
    fn all_off_advances_to_the_second_round() {
        let judge = ScriptedJudge::new(vec![
            vec![RelevanceVerdict::Off, RelevanceVerdict::Off],
            vec![RelevanceVerdict::On],
        ]);
        let result = select_with_judge(&judge, "vase", "ceramic vase", &[thumbs(2), thumbs(1)]);

        assert_eq!(result.picked, Some(2));
        assert_eq!(result.rounds, 2);
        assert_eq!((result.on, result.weak, result.off), (1, 0, 2));
    }

    #[test]
    fn weak_only_first_round_still_judges_the_second_batch() {
        let judge = ScriptedJudge::new(vec![
            vec![RelevanceVerdict::Weak, RelevanceVerdict::Off],
            vec![RelevanceVerdict::Off, RelevanceVerdict::On],
        ]);
        let result = select_with_judge(&judge, "vase", "ceramic vase", &[thumbs(2), thumbs(2)]);

        assert_eq!(result.picked, Some(3));
        assert_eq!(result.ordered_indices(), &[3, 0]);
        assert_eq!(result.rounds, 2);
        assert_eq!((result.on, result.weak, result.off), (1, 1, 2));
    }

    #[test]
    fn weak_only_rounds_fall_back_to_the_earliest_weak_candidate() {
        let judge = ScriptedJudge::new(vec![
            vec![RelevanceVerdict::Off, RelevanceVerdict::Weak],
            vec![RelevanceVerdict::Weak],
        ]);
        let result = select_with_judge(&judge, "vase", "ceramic vase", &[thumbs(2), thumbs(1)]);

        assert_eq!(result.picked, Some(1));
        assert_eq!(result.ordered_indices(), &[1, 2]);
        assert_eq!(result.rounds, 2);
    }

    #[test]
    fn two_all_off_rounds_are_unresolved_without_a_fallback() {
        let judge = ScriptedJudge::new(vec![
            vec![RelevanceVerdict::Off, RelevanceVerdict::Off],
            vec![RelevanceVerdict::Off],
        ]);
        let result = select_with_judge(&judge, "vase", "ceramic vase", &[thumbs(2), thumbs(1)]);

        assert_eq!(result.picked, None);
        assert_eq!(result.rounds, 2);
        assert_eq!((result.on, result.weak, result.off), (0, 0, 3));
    }

    #[test]
    fn judge_log_has_the_stable_shape() {
        let judge = ScriptedJudge::new(vec![vec![RelevanceVerdict::Weak, RelevanceVerdict::On]]);
        let result = select_with_judge(&judge, "chair", "a chair", &[thumbs(2)]);

        assert_eq!(
            format_judge_log(&result),
            "judge: k=2 verdicts=1/1/0 picked=1 rounds=1 unavailable=0 unreported=0"
        );
    }
}

#[cfg(test)]
#[path = "judge_diagnostics_tests.rs"]
mod diagnostics_tests;
