//! Stock memoization and stale-result guards track the actual judged view.

use super::super::*;
use super::*;
use jian_ops_schema::node::ImageFitMode;
use op_image_enrich::{ImageJudgeFit, ImageJudgeView};

fn stock_target(width: f64, height: f64, fit: ImageJudgeFit) -> ImageSearchTarget {
    ImageSearchTarget {
        node_id: NodeId::new("photo"),
        query: "outdoor jogging".into(),
        prompt: Some("A runner jogging on a tree-lined park path".into()),
        mode: ImageRequestMode::Search,
        aspect_ratio: Some(ImageAspectRatio::Wide),
        width: Some(width),
        height: Some(height),
        judge_view: ImageJudgeView::new(width, height, fit),
    }
}

fn key_for(target: &ImageSearchTarget) -> SearchIntentKey {
    search_intent_key(
        &target.query,
        target.aspect_ratio,
        target.prompt.as_deref(),
        target.judge_view,
    )
}

#[test]
fn ready_memo_returns_the_photo_judged_for_each_exact_crop_and_fit() {
    let targets = [
        (stock_target(240.0, 160.0, ImageJudgeFit::Cover), "wide.jpg"),
        (stock_target(132.0, 74.0, ImageJudgeFit::Cover), "card.jpg"),
        (
            stock_target(240.0, 160.0, ImageJudgeFit::Contain),
            "contain.jpg",
        ),
    ];
    let memo = Arc::new(Mutex::new(HashMap::new()));
    // Seed the old bucket-only key too, so a regression returns an incorrect
    // cached image without accidentally starting external network work.
    let mut unknown_view = targets[0].0.clone();
    unknown_view.judge_view = None;
    memo.lock().unwrap().insert(
        key_for(&unknown_view),
        SearchMemoEntry::Ready("uncropped-legacy.jpg".into()),
    );
    for (target, url) in &targets {
        memo.lock()
            .unwrap()
            .insert(key_for(target), SearchMemoEntry::Ready((*url).into()));
    }
    assert_eq!(memo.lock().unwrap().len(), 4);
    for (target, url) in targets {
        let intent = intent_fingerprint(&target, None);
        let job = spawn_job(
            target,
            None,
            Arc::new(Mutex::new(HashSet::new())),
            Arc::clone(&memo),
            1,
            Arc::new(NoJudge),
            false,
            intent,
            None,
        );
        assert_eq!(
            job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
            JobOutcome::Search(Some(url.into()))
        );
    }
}

#[test]
fn a_different_crop_cannot_join_or_consume_another_views_search() {
    let original = stock_target(240.0, 160.0, ImageJudgeFit::Cover);
    for current in [
        stock_target(132.0, 74.0, ImageJudgeFit::Cover),
        stock_target(240.0, 160.0, ImageJudgeFit::Contain),
    ] {
        for old_ready in [false, true] {
            let old_key = key_for(&original);
            let current_key = key_for(&current);
            assert_ne!(old_key, current_key);
            let mut unknown_view = current.clone();
            unknown_view.judge_view = None;
            let memo = Arc::new(Mutex::new(HashMap::from([
                (
                    key_for(&unknown_view),
                    SearchMemoEntry::Ready("uncropped-legacy.jpg".into()),
                ),
                (
                    old_key.clone(),
                    if old_ready {
                        SearchMemoEntry::Ready("old-view.jpg".into())
                    } else {
                        SearchMemoEntry::Pending {
                            request_id: 7,
                            waiters: Vec::new(),
                        }
                    },
                ),
                (
                    current_key.clone(),
                    SearchMemoEntry::Pending {
                        request_id: 8,
                        waiters: Vec::new(),
                    },
                ),
            ])));
            let job = spawn_job(
                current.clone(),
                None,
                Arc::new(Mutex::new(HashSet::new())),
                Arc::clone(&memo),
                9,
                Arc::new(NoJudge),
                false,
                intent_fingerprint(&current, None),
                None,
            );
            assert!(matches!(job.rx.try_recv(), Err(TryRecvError::Empty)));
            if !old_ready {
                assert!(publish_search_result(
                    &memo,
                    old_key,
                    7,
                    Some("old-view.jpg".into())
                ));
                assert!(matches!(job.rx.try_recv(), Err(TryRecvError::Empty)));
            }
            assert!(publish_search_result(
                &memo,
                current_key,
                8,
                Some("current-view.jpg".into())
            ));
            assert_eq!(
                job.rx.recv_timeout(Duration::from_secs(1)).unwrap(),
                JobOutcome::Search(Some("current-view.jpg".into()))
            );
        }
    }
}

#[test]
fn changing_crop_or_fit_while_searching_discards_the_old_result() {
    for resize in [true, false] {
        let mut state = EditorState::default();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(image_node("photo", "", Some("outdoor jogging")));
        let original = collect_targets(&state, &HashSet::new()).remove(0);
        let expected = intent_fingerprint(&original, None);
        let PenNode::Image(image) = &mut state.active_children_mut()[0] else {
            panic!("image")
        };
        if resize {
            image.width = Some(SizingBehavior::Number(320.0));
        } else {
            image.object_fit = Some(ImageFitMode::Fit);
        }
        state.mark_document_changed();
        let current = collect_targets(&state, &HashSet::new()).remove(0);
        assert_eq!(original.query, current.query);
        assert_eq!(original.aspect_ratio, current.aspect_ratio);
        assert_ne!(original.judge_view, current.judge_view);
        let (tx, rx) = mpsc::channel();
        tx.send(JobOutcome::Search(Some(
            "https://stale.example/runner.jpg".into(),
        )))
        .unwrap();
        let mut session = ImageSearchSession {
            in_flight: HashSet::from(["photo".into()]),
            jobs: vec![ImageSearchJob {
                node_id: NodeId::new("photo"),
                deadline: None,
                intent: Some(expected),
                stock_view: None,
                fallback_target: None,
                rx,
            }],
            ..Default::default()
        };
        assert!(!session.poll_into(&mut state));
        let PenNode::Image(image) = &state.active_children()[0] else {
            panic!("image")
        };
        assert!(
            image.src.is_empty(),
            "a differently judged crop must not land"
        );
        assert!(session.jobs.is_empty());
        assert!(session.in_flight.is_empty());
        assert!(!session.completed.contains("photo"));
        assert_eq!(collect_targets(&state, &session.completed).len(), 1);
    }
}

#[test]
fn stock_view_does_not_change_generation_identity() {
    let mut target = stock_target(240.0, 160.0, ImageJudgeFit::Cover);
    target.mode = ImageRequestMode::Generate;
    let expected = intent_fingerprint(&target, None);
    target.judge_view = ImageJudgeView::new(240.0, 160.0, ImageJudgeFit::Contain);
    assert_eq!(intent_fingerprint(&target, None), expected);
    target.judge_view = None;
    assert_eq!(intent_fingerprint(&target, None), expected);
}

#[test]
fn generation_fallback_checks_stock_view_without_changing_generation_identity() {
    for (before, after, should_apply) in [
        (None, None, true),
        (None, Some(ImageFitMode::Fit), false),
        (Some(ImageFitMode::Tile), None, false),
    ] {
        let mut state = EditorState::default();
        state.active_children_mut().clear();
        let mut node = image_node("photo", "", None);
        let PenNode::Image(image) = &mut node else {
            panic!("image")
        };
        image.image_prompt = Some("A runner jogging on a tree-lined path".into());
        image.object_fit = before;
        state.active_children_mut().push(node);
        let original = collect_targets(&state, &HashSet::new()).remove(0);
        assert_eq!(original.mode, ImageRequestMode::Generate);
        let expected = intent_fingerprint(&original, None);
        let (tx, rx) = mpsc::channel();
        tx.send(JobOutcome::Generate(Err(ImageGenerateError::Busy {
            provider: "test",
        })))
        .unwrap();
        let mut session = ImageSearchSession {
            in_flight: HashSet::from(["photo".into()]),
            jobs: vec![ImageSearchJob {
                node_id: NodeId::new("photo"),
                deadline: None,
                intent: Some(expected.clone()),
                stock_view: None,
                fallback_target: Some(original.clone()),
                rx,
            }],
            ..Default::default()
        };
        session.search_memo.lock().unwrap().insert(
            key_for(&original),
            SearchMemoEntry::Ready("https://example.test/runner.jpg".into()),
        );
        let PenNode::Image(image) = &mut state.active_children_mut()[0] else {
            panic!("image")
        };
        image.object_fit = after;
        state.mark_document_changed();
        let current = collect_targets(&state, &HashSet::new()).remove(0);
        assert_eq!(intent_fingerprint(&current, None), expected);

        assert_eq!(session.poll_into(&mut state), should_apply);
        let PenNode::Image(image) = &state.active_children()[0] else {
            panic!("image")
        };
        assert_eq!(image.src.is_empty(), !should_apply);
        assert_eq!(session.stats().fell_back, 1);
        assert!(session.jobs.is_empty());
        assert!(session.in_flight.is_empty());
        assert!(!session.completed.contains("photo"));
    }
}
