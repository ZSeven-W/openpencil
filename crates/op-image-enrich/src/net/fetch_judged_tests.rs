use super::*;
use crate::net::JudgedCandidate;

#[derive(Default)]
struct RecordingJudge {
    briefs: Mutex<Vec<(String, String)>>,
}

impl ImageRelevanceJudge for RecordingJudge {
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
    fn judge(&self, query: &str, intent: &str, thumbs: &[Vec<u8>]) -> Vec<JudgedCandidate> {
        self.briefs
            .lock()
            .unwrap()
            .push((query.into(), intent.into()));
        thumbs
            .iter()
            .enumerate()
            .map(|(index, bytes)| JudgedCandidate {
                index,
                verdict: match bytes[0] {
                    0 => RelevanceVerdict::Off,
                    1 => RelevanceVerdict::Weak,
                    _ => RelevanceVerdict::On,
                },
                reason: String::new(),
            })
            .collect()
    }
}

fn candidate(src: &str, grade: u8) -> PreparedJudgeCandidate {
    PreparedJudgeCandidate {
        attribution: "CC BY 4.0".into(),
        original: None,
        title: src.into(),
        data_url: src.into(),
        thumb_jpeg: vec![grade],
        identity: None,
    }
}

#[test]
fn same_artwork_cannot_win_again_with_different_resolution_bytes() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "cream",
        intent: "cream",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut first = candidate("preview-600", 2);
    first.identity = Some("openverse:artwork-1".into());
    let mut second = candidate("original-2048", 2);
    second.identity = first.identity.clone();
    let mut fallback = WeakFallback::new(&used, None);
    let chosen = stage(&slot, &used, &mut fallback, "cream", &[first]).unwrap();
    assert_eq!(chosen.attribution, "CC BY 4.0");
    assert!(stage(&slot, &used, &mut fallback, "cream", &[second]).is_none());
}

fn stage(
    slot: &JudgeSlot<'_>,
    used: &Mutex<HashSet<String>>,
    fallback: &mut WeakFallback<'_>,
    query: &str,
    candidates: &[PreparedJudgeCandidate],
) -> Option<ResolvedImage> {
    let thumbnails = candidates
        .iter()
        .map(|candidate| candidate.thumb_jpeg.clone())
        .collect();
    let selection = slot.select(&[thumbnails]);
    claim_ranked_image(&selection, candidates, used, fallback.is_empty())
        .and_then(|image| fallback.consider("openverse", query, image))
}

#[test]
fn weak_primary_does_not_hide_on_in_a_later_tier() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "amber skincare serum bottle",
        intent: "product card",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut fallback = WeakFallback::new(&used, None);
    let mut weak = candidate("weak", 1);
    weak.identity = Some("wikimedia:123".into());
    assert!(stage(&slot, &used, &mut fallback, slot.query, &[weak]).is_none());
    let original_window = fallback.delivery_deadline();
    let winner = stage(
        &slot,
        &used,
        &mut fallback,
        "serum bottle",
        &[candidate("on", 2)],
    )
    .unwrap();
    assert_eq!(winner.src, "on");
    assert_eq!(winner.provider_query, "serum bottle");
    assert_eq!(
        fallback.delivery_deadline(),
        original_window,
        "winning On cannot restart the original-download budget"
    );
    assert!(fallback.finish().is_none());
    assert!(!used.lock().unwrap().contains("wikimedia:123"));
    assert!(
        claim_unused_image_src(&used, "weak"),
        "provisional fallback must be released"
    );
    assert!(
        !claim_unused_image_src(&used, "on"),
        "final winner stays reserved"
    );
}

#[test]
fn finishing_weak_preserves_its_original_opportunity_deadline_for_delivery() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "bottle",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let outer = Instant::now() + Duration::from_secs(120);
    let mut fallback = WeakFallback::new(&used, Some(outer));
    assert!(stage(
        &slot,
        &used,
        &mut fallback,
        "serum",
        &[candidate("weak", 1)]
    )
    .is_none());
    let original_window = fallback.deadline().unwrap();
    let (winner, delivery_deadline) = fallback.finish_with_deadline().unwrap();
    assert_eq!(winner.src, "weak");
    assert_eq!(delivery_deadline, Some(original_window));
    assert!(original_window < outer);
}

#[test]
fn rewritten_search_is_judged_against_the_original_slot_brief() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "minimal amber skincare serum bottle on cream",
        intent: "serum product photo, not a drink",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut fallback = WeakFallback::new(&used, None);
    for query in [slot.query, "serum bottle", "amber bottle", "skincare serum"] {
        assert!(stage(&slot, &used, &mut fallback, query, &[candidate(query, 0)]).is_none());
    }
    assert!(fallback.finish().is_none());
    let briefs = judge.briefs.lock().unwrap();
    assert_eq!(briefs.len(), 4);
    assert!(briefs
        .iter()
        .all(|(query, intent)| query == slot.query && intent == slot.intent));
}

#[test]
fn all_weak_tiers_keep_the_first_usable_fallback_without_claiming_more() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "product photo",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut fallback = WeakFallback::new(&used, None);
    for query in ["serum", "amber serum", "bottle"] {
        assert!(stage(&slot, &used, &mut fallback, query, &[candidate(query, 1)]).is_none());
    }
    let winner = fallback.finish().unwrap();
    assert_eq!(winner.src, "serum");
    assert_eq!(judge.briefs.lock().unwrap().len(), 3);
    assert_eq!(used.lock().unwrap().len(), 1);
    assert!(!claim_unused_image_src(&used, "serum"));
    assert!(claim_unused_image_src(&used, "bottle"));
}

#[test]
fn on_that_is_already_used_does_not_upgrade_a_weak_fallback() {
    let used = Mutex::new(HashSet::new());
    assert!(claim_unused_image_src(&used, "duplicate"));
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "product",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut fallback = WeakFallback::new(&used, None);
    assert!(stage(
        &slot,
        &used,
        &mut fallback,
        "serum",
        &[candidate("duplicate", 2), candidate("weak", 1)]
    )
    .is_none());
    assert_eq!(fallback.finish().unwrap().src, "weak");
}

#[test]
fn off_is_never_reserved_or_used_as_a_fallback() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "product",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut fallback = WeakFallback::new(&used, None);
    assert!(stage(
        &slot,
        &used,
        &mut fallback,
        "serum",
        &[candidate("unrelated", 0)]
    )
    .is_none());
    assert!(fallback.finish().is_none());
    assert!(used.lock().unwrap().is_empty());
}

#[test]
fn cancellation_releases_only_the_provisional_fallback_claims() {
    let used = Mutex::new(HashSet::new());
    used.lock().unwrap().insert("other-slot".into());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "product",
        judge: &judge,
        view: None,
        deadline: None,
    };
    {
        let mut fallback = WeakFallback::new(&used, None);
        let mut weak = candidate("weak", 1);
        weak.identity = Some("wikimedia:123".into());
        assert!(stage(&slot, &used, &mut fallback, "serum", &[weak]).is_none());
        assert_eq!(used.lock().unwrap().len(), 3);
    }
    assert_eq!(*used.lock().unwrap(), HashSet::from(["other-slot".into()]));
}

#[test]
fn a_stalled_extra_tier_delivers_the_retained_weak_before_the_outer_deadline() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "product",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let started = Instant::now();
    let outer_deadline = started + Duration::from_millis(300);
    let mut fallback = WeakFallback::new(&used, Some(outer_deadline));
    assert!(stage(
        &slot,
        &used,
        &mut fallback,
        "serum",
        &[candidate("weak", 1)]
    )
    .is_none());
    assert!(fallback.can_continue(&judge));
    let expired = crate::net::block_on_image_runtime(before_deadline(
        fallback.deadline(),
        std::future::pending::<Option<JudgedImage>>(),
    ));
    assert!(expired.is_err());
    let winner = fallback
        .finish()
        .expect("deadline must deliver, not drop, the saved Weak");
    assert_eq!(winner.src, "weak");
    assert!(Instant::now() < outer_deadline);
    assert!(
        !claim_unused_image_src(&used, "weak"),
        "delivered image keeps its claim"
    );
}

#[test]
fn a_late_slot_does_not_restart_the_global_deadline() {
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "product",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut fallback = WeakFallback::new(&used, Some(Instant::now() + Duration::from_millis(100)));
    assert!(stage(
        &slot,
        &used,
        &mut fallback,
        "serum",
        &[candidate("weak", 1)]
    )
    .is_none());
    assert!(
        !fallback.can_continue(&judge),
        "reserve delivery time before the shared outer cutoff"
    );
    assert_eq!(fallback.finish().unwrap().src, "weak");
}

#[test]
fn an_unbounded_judge_skips_extra_retrieval_as_soon_as_weak_is_available() {
    struct UnboundedJudge;
    impl ImageRelevanceJudge for UnboundedJudge {
        fn judge(&self, _: &str, _: &str, _: &[Vec<u8>]) -> Vec<JudgedCandidate> {
            panic!("unsupported follow-up judge must never run");
        }
    }
    let used = Mutex::new(HashSet::new());
    let judge = RecordingJudge::default();
    let slot = JudgeSlot {
        query: "serum",
        intent: "product",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let mut fallback = WeakFallback::new(&used, None);
    assert!(
        fallback.can_continue(&UnboundedJudge),
        "first no-Weak search is unchanged"
    );
    assert!(stage(
        &slot,
        &used,
        &mut fallback,
        "serum",
        &[candidate("weak", 1)]
    )
    .is_none());
    assert!(!fallback.can_continue(&UnboundedJudge));
    assert_eq!(fallback.finish().unwrap().src, "weak");
}
