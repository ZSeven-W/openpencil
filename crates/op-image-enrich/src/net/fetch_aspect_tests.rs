use super::*;
use crate::net::{JudgedCandidate, NoJudge, RelevanceVerdict};

fn image(src: &str, verdict: RelevanceVerdict) -> JudgedImage {
    JudgedImage {
        title: src.into(),
        src: src.into(),
        verdict,
        identity: None,
        attribution: String::new(),
        original: None,
    }
}

#[test]
fn empty_square_search_tries_the_same_subject_once_without_geometry() {
    crate::net::block_on_image_runtime(async {
        let used = Mutex::new(HashSet::new());
        let mut fallback = WeakFallback::new(&used, None);
        let mut calls = Vec::new();
        let result = search_original_aspects(
            "face cream jar",
            Some(ImageAspectRatio::Square),
            &NoJudge,
            None,
            &mut fallback,
            |aspect, deadline, _| {
                calls.push((aspect, deadline));
                std::future::ready(Some(
                    aspect.is_none().then(|| image("jar", RelevanceVerdict::On)),
                ))
            },
        )
        .await
        .unwrap()
        .0
        .unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], (Some(ImageAspectRatio::Square), None));
        assert_eq!(calls[1].0, None);
        assert!(calls[1].1.is_some());
        assert_eq!(result.provider_query, "face cream jar");
        assert_eq!(result.provider, "openverse-aspect");
    });
}

#[test]
fn an_on_winner_or_an_already_unfiltered_search_never_adds_a_tier() {
    crate::net::block_on_image_runtime(async {
        for (aspect, verdict) in [
            (Some(ImageAspectRatio::Tall), Some(RelevanceVerdict::On)),
            (None, None),
        ] {
            let used = Mutex::new(HashSet::new());
            let mut fallback = WeakFallback::new(&used, None);
            let mut calls = 0;
            let _ = search_original_aspects(
                "subject",
                aspect,
                &NoJudge,
                None,
                &mut fallback,
                |_, _, _| {
                    calls += 1;
                    std::future::ready(Some(verdict.map(|grade| image("subject", grade))))
                },
            )
            .await
            .unwrap();
            assert_eq!(calls, 1);
        }
    });
}

#[test]
fn a_transport_failure_does_not_trigger_an_aspect_retry() {
    crate::net::block_on_image_runtime(async {
        let used = Mutex::new(HashSet::new());
        let mut fallback = WeakFallback::new(&used, None);
        let mut calls = 0;
        let result = search_original_aspects(
            "subject",
            Some(ImageAspectRatio::Square),
            &NoJudge,
            None,
            &mut fallback,
            |_, _, _| {
                calls += 1;
                std::future::ready(None)
            },
        )
        .await
        .unwrap();
        assert!(result.0.is_none());
        assert_eq!(calls, 1);
    });
}

#[test]
fn an_unbounded_judge_does_not_start_the_added_opportunity() {
    struct UnboundedJudge;
    impl ImageRelevanceJudge for UnboundedJudge {
        fn judge(&self, _: &str, _: &str, _: &[Vec<u8>]) -> Vec<JudgedCandidate> {
            vec![]
        }
    }
    crate::net::block_on_image_runtime(async {
        let used = Mutex::new(HashSet::new());
        let mut fallback = WeakFallback::new(&used, None);
        let mut calls = 0;
        let result = search_original_aspects(
            "subject",
            Some(ImageAspectRatio::Square),
            &UnboundedJudge,
            None,
            &mut fallback,
            |_, _, _| {
                calls += 1;
                std::future::ready(Some(None))
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.1, Some(ImageAspectRatio::Square));
    });
}

#[test]
fn an_expired_caller_deadline_never_starts_a_request() {
    crate::net::block_on_image_runtime(async {
        let used = Mutex::new(HashSet::new());
        let deadline = Instant::now();
        let mut fallback = WeakFallback::new(&used, Some(deadline));
        let result = search_original_aspects(
            "subject",
            Some(ImageAspectRatio::Square),
            &NoJudge,
            Some(deadline),
            &mut fallback,
            |_, _, _| {
                panic!("expired requests must not be constructed");
                #[allow(unreachable_code)]
                std::future::ready(Some(None))
            },
        )
        .await;
        assert!(result.is_err());
    });
}

#[test]
fn stalled_relaxed_search_preserves_weak_and_its_original_deadline() {
    crate::net::block_on_image_runtime(async {
        let used = Mutex::new(HashSet::new());
        let outer = Instant::now() + Duration::from_millis(330);
        let mut fallback = WeakFallback::new(&used, Some(outer));
        let mut deadlines = Vec::new();
        let result = search_original_aspects(
            "serum bottle",
            Some(ImageAspectRatio::Square),
            &NoJudge,
            Some(outer),
            &mut fallback,
            |aspect, deadline, allow_weak| {
                deadlines.push(deadline);
                async move {
                    if aspect.is_some() {
                        assert!(allow_weak);
                        Some(Some(image("weak", RelevanceVerdict::Weak)))
                    } else {
                        assert!(!allow_weak);
                        std::future::pending().await
                    }
                }
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(deadlines.len(), 2);
        assert_eq!(deadlines[0], Some(outer));
        assert_eq!(deadlines[1], outer.checked_sub(Duration::from_millis(250)));
        assert_eq!(fallback.finish().unwrap().src, "weak");
        assert!(
            Instant::now() < outer,
            "fallback is ready before the CLI's last poll"
        );
    });
}

#[test]
fn caller_deadline_cancels_a_slow_primary_even_without_weak() {
    crate::net::block_on_image_runtime(async {
        let used = Mutex::new(HashSet::new());
        let outer = Instant::now() + Duration::from_millis(20);
        let mut fallback = WeakFallback::new(&used, Some(outer));
        let mut calls = 0;
        let result = search_original_aspects(
            "subject",
            Some(ImageAspectRatio::Square),
            &NoJudge,
            Some(outer),
            &mut fallback,
            |_, deadline, _| {
                assert_eq!(deadline, Some(outer));
                calls += 1;
                std::future::pending()
            },
        )
        .await;
        assert!(result.is_err());
        assert!(fallback.finish().is_none());
        assert_eq!(calls, 1);
    });
}

#[test]
fn candidate_dedup_is_per_slot_and_leaves_new_hits_available() {
    fn hit(id: &str) -> RawHit {
        RawHit {
            id: id.into(),
            thumb_url: format!("https://example.com/{id}"),
            attribution: String::new(),
            title: id.into(),
            relevance_metadata: id.into(),
            original: None,
        }
    }
    let seen = Mutex::new(HashSet::from(["openverse:square-hit".to_string()]));
    let filtered = unseen_hits(
        vec![hit("square-hit"), hit("new-hit"), hit("new-hit")],
        &seen,
    );
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "new-hit");
    // Only the caller's completed judgement records an identity. A failed
    // download/judge can supply this candidate again on another tier.
    assert_eq!(unseen_hits(vec![hit("new-hit")], &seen).len(), 1);
    assert_eq!(
        unseen_hits(vec![hit("square-hit")], &Mutex::new(HashSet::new())).len(),
        1
    );
}

#[test]
fn a_legacy_judge_still_selects_the_original_brief_with_a_caller_deadline() {
    struct LegacyJudge;
    impl ImageRelevanceJudge for LegacyJudge {
        fn judge(&self, query: &str, intent: &str, thumbs: &[Vec<u8>]) -> Vec<JudgedCandidate> {
            assert_eq!(query, "face cream jar");
            assert_eq!(intent, "white ceramic jar on linen");
            assert_eq!(thumbs.len(), 1);
            vec![JudgedCandidate {
                index: 0,
                verdict: RelevanceVerdict::On,
                reason: "correct subject".into(),
            }]
        }
    }
    let caller_deadline = Some(Instant::now() + Duration::from_secs(1));
    let slot = JudgeSlot {
        query: "face cream jar",
        intent: "white ceramic jar on linen",
        judge: &LegacyJudge,
        view: None,
        deadline: judging_deadline(&LegacyJudge, caller_deadline),
    };
    let selection = slot.select(&[vec![vec![1]]]);
    assert_eq!(selection.picked, Some(0));
    assert_eq!(selection.on, 1);
    assert_eq!(judging_deadline(&NoJudge, caller_deadline), caller_deadline);
}

#[test]
fn unavailable_off_stays_retryable_but_a_completed_off_is_deduplicated() {
    use std::sync::atomic::AtomicUsize;
    struct RetryJudge(AtomicUsize);
    impl ImageRelevanceJudge for RetryJudge {
        fn judge(&self, _: &str, _: &str, _: &[Vec<u8>]) -> Vec<JudgedCandidate> {
            let attempt = self.0.fetch_add(1, Ordering::Relaxed);
            vec![JudgedCandidate {
                index: 0,
                verdict: if attempt == 1 {
                    RelevanceVerdict::On
                } else {
                    RelevanceVerdict::Off
                },
                reason: if attempt == 0 {
                    "judge unavailable"
                } else {
                    "completed visual judgment"
                }
                .into(),
            }]
        }
    }
    let judge = RetryJudge(AtomicUsize::new(0));
    let slot = JudgeSlot {
        query: "jar",
        intent: "white ceramic jar",
        judge: &judge,
        view: None,
        deadline: None,
    };
    let seen = Mutex::new(HashSet::new());
    let candidates = vec![PreparedJudgeCandidate {
        title: "jar".into(),
        data_url: "data:image/jpeg;base64,AQ==".into(),
        thumb_jpeg: vec![1],
        identity: Some("openverse:jar".into()),
        attribution: String::new(),
        original: None,
    }];
    let batches = vec![vec![vec![1]]];
    let unavailable = select_and_record(&slot, &batches, &candidates, &seen);
    assert_eq!(unavailable.off, 0);
    assert_eq!(unavailable.unavailable, 1);
    assert_eq!(unavailable.unreported, 0);
    assert!(seen.lock().unwrap().is_empty());
    let retry = select_and_record(&slot, &batches, &candidates, &seen);
    assert_eq!(retry.on, 1);
    assert!(seen.lock().unwrap().contains("openverse:jar"));
    seen.lock().unwrap().clear();
    let off = select_and_record(&slot, &batches, &candidates, &seen);
    assert_eq!(off.off, 1);
    assert!(seen.lock().unwrap().contains("openverse:jar"));
}

#[test]
fn a_legacy_off_after_the_caller_deadline_stops_before_any_later_request() {
    use std::sync::atomic::AtomicUsize;
    struct SlowLegacyJudge;
    impl ImageRelevanceJudge for SlowLegacyJudge {
        fn judge(&self, _: &str, _: &str, _: &[Vec<u8>]) -> Vec<JudgedCandidate> {
            std::thread::sleep(Duration::from_millis(30));
            vec![JudgedCandidate {
                index: 0,
                verdict: RelevanceVerdict::Off,
                reason: "wrong subject".into(),
            }]
        }
    }
    crate::net::block_on_image_runtime(async {
        for aspect in [Some(ImageAspectRatio::Square), None] {
            let used = Mutex::new(HashSet::new());
            let deadline = Some(Instant::now() + Duration::from_millis(10));
            let mut fallback = WeakFallback::new(&used, deadline);
            let requests = AtomicUsize::new(0);
            let result = search_original_aspects(
                "face cream jar",
                aspect,
                &SlowLegacyJudge,
                deadline,
                &mut fallback,
                |_, deadline, _| {
                    let requests = &requests;
                    async move {
                        requests.fetch_add(1, Ordering::Relaxed);
                        let slot = JudgeSlot {
                            query: "face cream jar",
                            intent: "white ceramic jar",
                            judge: &SlowLegacyJudge,
                            view: None,
                            deadline: judging_deadline(&SlowLegacyJudge, deadline),
                        };
                        let selection = slot.select(&[vec![vec![1]]]);
                        assert_eq!(selection.off, 1);
                        Some(None)
                    }
                },
            )
            .await;
            assert!(
                result.is_err(),
                "an exhausted original tier must stop the caller's ladder"
            );
            assert_eq!(requests.load(Ordering::Relaxed), 1);
        }
    });
}

#[test]
fn existing_subject_rewrites_keep_any_aspect_and_the_original_judgment_contract() {
    use std::sync::atomic::AtomicUsize;
    const QUERY: &str = "luxury serum bottle";
    const INTENT: &str =
        "editorial product photography of an amber glass serum bottle on ivory linen";
    struct BriefJudge {
        calls: AtomicUsize,
        weak_first: bool,
    }
    impl ImageRelevanceJudge for BriefJudge {
        fn judge(&self, query: &str, intent: &str, _: &[Vec<u8>]) -> Vec<JudgedCandidate> {
            assert_eq!(
                query, QUERY,
                "retrieval rewrites must not weaken the visual contract"
            );
            assert_eq!(intent, INTENT);
            let first = self.calls.fetch_add(1, Ordering::Relaxed) == 0;
            vec![JudgedCandidate {
                index: 0,
                verdict: if first && self.weak_first {
                    RelevanceVerdict::Weak
                } else {
                    RelevanceVerdict::Off
                },
                reason: "fixture assessment".into(),
            }]
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
            assert!(Instant::now() < deadline);
            self.judge(query, intent, thumbs)
        }
    }
    crate::net::block_on_image_runtime(async {
        for weak_first in [false, true] {
            let judge = BriefJudge {
                calls: AtomicUsize::new(0),
                weak_first,
            };
            let used = Mutex::new(HashSet::new());
            let outer = Some(Instant::now() + Duration::from_secs(2));
            let mut fallback = WeakFallback::new(&used, outer);
            let mut calls = Vec::new();
            let mut search = |query: &str, aspect, deadline, allow_weak| {
                calls.push((query.to_string(), aspect, deadline, allow_weak));
                let slot = JudgeSlot {
                    query: QUERY,
                    intent: INTENT,
                    judge: &judge,
                    deadline,
                    view: None,
                };
                let selection = slot.select(&[vec![vec![1]]]);
                let selected = selection.verdict_for(0).map(|grade| image("serum", grade));
                std::future::ready(Some(selected))
            };
            let (winner, rewrite_aspect) = search_original_aspects(
                QUERY,
                Some(ImageAspectRatio::Square),
                &judge,
                outer,
                &mut fallback,
                |aspect, deadline, allow_weak| search(QUERY, aspect, deadline, allow_weak),
            )
            .await
            .unwrap();
            assert!(winner.is_none());
            assert_eq!(rewrite_aspect, None);
            let tier_deadline = earlier_deadline(outer, fallback.deadline());
            for rewrite in crate::net::providers::rewrite_queries(QUERY, INTENT) {
                super::super::judged::before_deadline(
                    tier_deadline,
                    search(&rewrite, rewrite_aspect, tier_deadline, fallback.is_empty()),
                )
                .await
                .unwrap();
            }
            assert_eq!(
                calls
                    .iter()
                    .map(|call| (call.0.as_str(), call.1))
                    .collect::<Vec<_>>(),
                vec![
                    (QUERY, Some(ImageAspectRatio::Square)),
                    (QUERY, None),
                    ("serum bottle", None),
                    ("glass serum bottle", None),
                ]
            );
            assert_eq!(calls.len(), 4, "geometry does not add more rewrite tiers");
            for call in &calls[2..] {
                assert_eq!(
                    call.2, tier_deadline,
                    "rewrites keep the absolute caller/Weak deadline"
                );
                assert_eq!(call.3, !weak_first);
            }
        }
    });
}
