use super::*;
use std::sync::Mutex;

struct OutcomeJudge(Mutex<Vec<Vec<JudgedCandidate>>>);

impl ImageRelevanceJudge for OutcomeJudge {
    fn judge(&self, _: &str, _: &str, _: &[Vec<u8>]) -> Vec<JudgedCandidate> {
        self.0.lock().unwrap().remove(0)
    }
}

fn candidate(index: usize, verdict: RelevanceVerdict, reason: &str) -> JudgedCandidate {
    JudgedCandidate {
        index,
        verdict,
        reason: reason.into(),
    }
}

fn thumbs(count: usize) -> Vec<Vec<u8>> {
    (0..count).map(|index| vec![index as u8]).collect()
}

#[test]
fn technical_failure_is_not_a_semantic_off_or_a_weak_fallback() {
    let judge = OutcomeJudge(Mutex::new(vec![vec![
        candidate(0, RelevanceVerdict::Off, "wrong action"),
        candidate(1, RelevanceVerdict::Off, "judge unavailable"),
        candidate(2, RelevanceVerdict::Weak, "soft lighting difference"),
    ]]));
    let result = select_with_judge(&judge, "q", "brief", &[thumbs(3)]);
    assert_eq!((result.on, result.weak, result.off), (0, 1, 1));
    assert_eq!((result.unavailable, result.unreported), (1, 0));
    assert_eq!(result.picked, Some(2));
    assert_eq!(result.ordered_indices(), &[2]);
    assert_eq!(result.verdict_for(2), Some(RelevanceVerdict::Weak));
    assert_eq!(result.verdict_for(1), None);
    assert_eq!(
        format_judge_log(&result),
        "judge: k=3 verdicts=0/1/1 picked=2 rounds=1 unavailable=1 unreported=0"
    );
}

#[test]
fn a_missing_outcome_is_not_fabricated_as_model_rejection() {
    let judge = OutcomeJudge(Mutex::new(vec![vec![
        candidate(0, RelevanceVerdict::Off, "wrong subject"),
        candidate(2, RelevanceVerdict::Off, "judge unavailable"),
    ]]));
    let result = select_with_judge(&judge, "q", "brief", &[thumbs(4)]);
    assert_eq!(
        (result.off, result.unavailable, result.unreported),
        (1, 1, 2)
    );
    assert!(result.picked.is_none());
    assert!(result.ordered_indices().is_empty());
}

#[test]
fn finding_on_does_not_count_unrequested_later_batches_as_missing() {
    let judge = OutcomeJudge(Mutex::new(vec![vec![
        candidate(0, RelevanceVerdict::Off, "judge unavailable"),
        candidate(1, RelevanceVerdict::On, "matched subject"),
    ]]));
    let result = select_with_judge(&judge, "q", "brief", &[thumbs(2), thumbs(5)]);
    assert_eq!(result.rounds, 1);
    assert_eq!(result.picked, Some(1));
    assert_eq!((result.unavailable, result.unreported), (1, 0));
    assert!(judge.0.lock().unwrap().is_empty());
}

#[test]
fn diagnostics_preserve_global_indices_and_selection_across_rounds() {
    let judge = OutcomeJudge(Mutex::new(vec![
        vec![
            candidate(0, RelevanceVerdict::Off, "judge unavailable"),
            candidate(2, RelevanceVerdict::Weak, "composition preference"),
        ],
        vec![
            candidate(0, RelevanceVerdict::On, "matched subject"),
            candidate(1, RelevanceVerdict::Off, "wrong action"),
        ],
    ]));
    let result = select_with_judge(&judge, "q", "brief", &[thumbs(3), thumbs(2)]);
    assert_eq!(result.picked, Some(3));
    assert_eq!(result.ordered_indices(), &[3, 2]);
    assert_eq!(result.verdict_for(3), Some(RelevanceVerdict::On));
    assert_eq!(result.verdict_for(2), Some(RelevanceVerdict::Weak));
    assert_eq!((result.on, result.weak, result.off), (1, 1, 1));
    assert_eq!((result.unavailable, result.unreported), (1, 1));
}

#[test]
fn only_the_exact_host_sentinel_marks_unavailability() {
    assert!(candidate(0, RelevanceVerdict::Off, " judge unavailable ").is_unavailable());
    assert!(!candidate(0, RelevanceVerdict::Off, "subject unavailable in photo").is_unavailable());
    assert!(
        !candidate(0, RelevanceVerdict::Off, "judge unavailable: wrong action").is_unavailable()
    );
}

#[test]
fn malformed_indices_do_not_hide_unreported_candidates() {
    let judge = OutcomeJudge(Mutex::new(vec![vec![
        candidate(0, RelevanceVerdict::Off, "wrong action"),
        candidate(0, RelevanceVerdict::Off, "wrong subject"),
        candidate(9, RelevanceVerdict::On, "out of range"),
    ]]));
    let result = select_with_judge(&judge, "q", "brief", &[thumbs(3)]);
    assert_eq!(result.unreported, 2);
    assert!(result.picked.is_none());
    assert!(result.ordered_indices().is_empty());
}
