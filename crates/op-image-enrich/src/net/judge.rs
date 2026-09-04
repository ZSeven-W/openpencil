//! Visual relevance judging and the provider-independent re-ranking policy.

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

/// Judge a set of thumbnails for one image slot. Implementations may issue
/// calls concurrently, but must return one entry per input thumbnail.
pub trait ImageRelevanceJudge: Send + Sync {
    fn judge(&self, query: &str, intent: &str, thumbs_jpeg: &[Vec<u8>]) -> Vec<JudgedCandidate>;
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
}

/// Judge batches in order, stopping after the first batch with an On or Weak
/// candidate. A caller can provide at most two batches to enforce the image
/// search cost bound.
pub fn select_with_judge(
    judge: &dyn ImageRelevanceJudge,
    query: &str,
    intent: &str,
    batches: &[Vec<Vec<u8>>],
) -> JudgeSelection {
    let mut selection = JudgeSelection {
        picked: None,
        rounds: 0,
        on: 0,
        weak: 0,
        off: 0,
        k: 0,
        ordered: Vec::new(),
    };
    let mut offset = 0;

    for batch in batches {
        if batch.is_empty() {
            continue;
        }
        if selection.rounds == 0 {
            selection.k = batch.len();
        }
        selection.rounds += 1;
        let judged = judge.judge(query, intent, batch);
        let mut on = Vec::new();
        let mut weak = Vec::new();
        for candidate in judged {
            if candidate.index >= batch.len() {
                continue;
            }
            match candidate.verdict {
                RelevanceVerdict::On => {
                    selection.on += 1;
                    on.push(offset + candidate.index);
                }
                RelevanceVerdict::Weak => {
                    selection.weak += 1;
                    weak.push(offset + candidate.index);
                }
                RelevanceVerdict::Off => selection.off += 1,
            }
        }

        if !on.is_empty() {
            selection.picked = on.first().copied();
            selection.ordered = on;
            selection.ordered.extend(weak);
            return selection;
        }
        if !weak.is_empty() {
            selection.picked = weak.first().copied();
            selection.ordered = weak;
            return selection;
        }
        offset += batch.len();
    }

    selection
}

/// Format the stable enrichment log line used by the desktop/headless
/// callers. Verdict counts are ordered On/Weak/Off.
pub fn format_judge_log(selection: &JudgeSelection) -> String {
    format!(
        "judge: k={} verdicts={}/{}/{} picked={} rounds={}",
        selection.k,
        selection.on,
        selection.weak,
        selection.off,
        selection
            .picked
            .map_or_else(|| "none".to_string(), |index| index.to_string()),
        selection.rounds
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
            "judge: k=2 verdicts=1/1/0 picked=1 rounds=1"
        );
    }
}
