//! Cache hit / miss / dedupe / supersede logic, no network.

use op_editor_core::generator::{GeneratorSpec, GENERATOR_STARTERS};
use serde_json::json;

use super::*;

fn frame() -> PenNode {
    GENERATOR_STARTERS[0].frame(0.0, 0.0)
}

fn spec_with_title(title: &str) -> GeneratorSpec {
    let mut spec = GENERATOR_STARTERS[0].spec();
    let mut values = serde_json::Map::new();
    values.insert("title".into(), json!(title));
    spec.apply_param_values(&values).expect("title");
    spec
}

fn rect() -> PenNode {
    serde_json::from_value(json!({"type": "rectangle", "id": "r", "width": 4, "height": 4}))
        .expect("rect")
}

fn ok_reply() -> GeneratorRunReply {
    GeneratorRunReply {
        result: Ok(vec![rect()]),
        transient: false,
    }
}

fn run(cache: &mut GeneratorCache, id: &str, spec: &GeneratorSpec, frame: &PenNode) -> bool {
    let request = GeneratorRequest {
        generator_id: id,
        spec,
        frame,
    };
    match cache.run(&request) {
        Err(GeneratorError::Pending) => false,
        Ok(_) => true,
        Err(other) => panic!("unexpected {other}"),
    }
}

fn ask(cache: &mut GeneratorCache, spec: &GeneratorSpec) -> Result<Vec<PenNode>, GeneratorError> {
    let frame = frame();
    cache.run(&GeneratorRequest {
        generator_id: "g1",
        spec,
        frame: &frame,
    })
}

#[test]
fn a_miss_queues_once_and_a_completed_reply_is_a_hit() {
    let mut cache = GeneratorCache::default();
    let spec = spec_with_title("A");
    assert!(!run(&mut cache, "g1", &spec, &frame()));
    assert!(!run(&mut cache, "g1", &spec, &frame()), "still pending");
    let queued = cache.take_queued();
    assert_eq!(queued.len(), 1, "identical requests are deduped");
    assert!(
        !run(&mut cache, "g1", &spec, &frame()),
        "in flight: not asked again"
    );
    assert!(cache.take_queued().is_empty());

    let job = queued.into_iter().next().unwrap();
    assert!(
        job.wire.frame.base().explain.is_none(),
        "explain is stripped"
    );
    cache.complete(job.clone(), ok_reply());
    assert_eq!(cache.take_ready(), vec![job]);
    assert!(run(&mut cache, "g1", &spec, &frame()), "hit");
}

#[test]
fn the_key_ignores_output_hash_and_frame_explain_but_not_params_or_id() {
    let mut cache = GeneratorCache::default();
    let spec = spec_with_title("A");
    assert!(!run(&mut cache, "g1", &spec, &frame()));
    let job = cache.take_queued().remove(0);
    cache.complete(job, ok_reply());

    let mut hashed = spec.clone();
    hashed.output_hash = Some("abc".into());
    let mut explained = frame();
    explained.base_mut().explain = Some(hashed.to_explain());
    assert!(
        run(&mut cache, "g1", &hashed, &explained),
        "same program input"
    );

    assert!(!run(&mut cache, "g1", &spec_with_title("B"), &frame()));
    assert!(
        !run(&mut cache, "g2", &spec, &frame()),
        "id seeds the output"
    );
    assert_eq!(cache.take_queued().len(), 2);
}

#[test]
fn an_older_reply_is_cached_but_not_reapplied_over_a_newer_request() {
    let mut cache = GeneratorCache::default();
    assert!(!run(&mut cache, "g1", &spec_with_title("old"), &frame()));
    assert!(!run(&mut cache, "g1", &spec_with_title("new"), &frame()));
    let mut jobs = cache.take_queued();
    let newer = jobs.pop().unwrap();
    let older = jobs.pop().unwrap();
    cache.complete(older, ok_reply());
    assert!(cache.take_ready().is_empty(), "superseded");
    assert!(run(&mut cache, "g1", &spec_with_title("old"), &frame()));
    cache.complete(newer.clone(), ok_reply());
    // The hit above made "old" the latest ask again, so the newer reply is
    // now the stale one.
    assert!(cache.take_ready().is_empty());
    assert!(run(&mut cache, "g1", &spec_with_title("new"), &frame()));
}

#[test]
fn deterministic_errors_stick_and_transient_ones_are_served_once() {
    let mut cache = GeneratorCache::default();
    let failing = spec_with_title("fails");
    assert!(!run(&mut cache, "g1", &failing, &frame()));
    let job = cache.take_queued().remove(0);
    let error = GeneratorError::Remote("generator program failed: boom".into());
    cache.complete(
        job,
        GeneratorRunReply {
            result: Err(error.clone()),
            transient: false,
        },
    );
    assert_eq!(ask(&mut cache, &failing), Err(error.clone()));
    assert_eq!(ask(&mut cache, &failing), Err(error), "sticky");

    let flaky = spec_with_title("flaky");
    assert!(!run(&mut cache, "g1", &flaky, &frame()));
    let job = cache.take_queued().remove(0);
    let offline = GeneratorError::Remote("unreachable".into());
    cache.complete(
        job,
        GeneratorRunReply {
            result: Err(offline.clone()),
            transient: true,
        },
    );
    assert_eq!(ask(&mut cache, &flaky), Err(offline), "surfaced once");
    assert_eq!(
        ask(&mut cache, &flaky),
        Err(GeneratorError::Pending),
        "then asked again"
    );
    assert_eq!(cache.take_queued().len(), 1);
}

#[test]
fn the_cache_is_bounded() {
    let mut cache = GeneratorCache::default();
    for index in 0..CACHE_CAPACITY + 5 {
        let spec = spec_with_title(&format!("t{index}"));
        assert!(!run(&mut cache, "g1", &spec, &frame()));
        let job = cache.take_queued().remove(0);
        cache.complete(job, ok_reply());
    }
    assert_eq!(cache.cached_len(), CACHE_CAPACITY);
    assert!(
        !run(&mut cache, "g1", &spec_with_title("t0"), &frame()),
        "the oldest entry was evicted"
    );
    let last = format!("t{}", CACHE_CAPACITY + 4);
    assert!(run(&mut cache, "g1", &spec_with_title(&last), &frame()));
}
