//! Fixture replay of the 2026-09-27 arena-l01 incident (skincare landing
//! page, unjudged op-smoke fill, square Openverse filter) plus an ignored
//! live trace of the real ladder.

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Mutex;

use super::{fenced_openverse_list_with, fetch_first_image_resolved_blocking, ResolvedImage};
use crate::net::providers::{fence_rejection, retain_relevant_hits_for_intent, RawHit};
use crate::ImageAspectRatio;

const SERUM_QUERY: &str = "serum oil bottle";
const SERUM_INTENT: &str = "minimal product photography of serum dropper bottle on a warm \
     neutral background, soft natural light";
const CLEANSER_QUERY: &str = "cleanser tube white";
const CLEANSER_INTENT: &str = "minimal product photography of cleanser tube on a warm neutral \
     background, soft natural light";

fn hit(id: &str, title: &str, tags: &[&str]) -> RawHit {
    RawHit {
        id: id.to_string(),
        thumb_url: format!("https://example.invalid/{id}/thumb/"),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: std::iter::once(title)
            .chain(tags.iter().copied())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Openverse `q=cleanser tube&aspect_ratio=square` — the only hit, which
/// shipped as the n257 "Product Photo".
fn tattoo_cover_up() -> RawHit {
    hit(
        "31401797",
        "Cover up for my friend Julio Done with disposable tubes by @kingpintattoosupply \
         cleaned with @officialarticyn cleanser #tattoo #inked #coverup #miamibeach \
         @salvationtattoolounge",
        &[
            "instagramapp",
            "iphoneography",
            "square",
            "adult",
            "art",
            "artistic",
            "blood",
            "body",
            "design",
            "face",
            "human",
            "illustration",
            "man",
            "painting",
            "people",
            "person",
            "portrait",
            "science",
            "tattoo",
            "woman",
        ],
    )
}

/// Openverse `q=serum oil&aspect_ratio=square` rank 1 — shipped as n246.
fn snake_oil_mural() -> RawHit {
    hit(
        "3a6171aa",
        "The snake oil serum",
        &[
            "apo",
            "belton",
            "chicago",
            "cmk",
            "graffiti",
            "meetingofstyles2013",
            "molotow",
            "mos",
            "serum",
            "serumgraffiti",
            "smear",
            "snakeoil",
        ],
    )
}

fn dropper_bottle() -> RawHit {
    hit(
        "dropper",
        "Dropper bottle, blank retro label",
        &[
            "alternative medicine",
            "dropper",
            "medicine bottle",
            "oil bottle",
            "serum",
            "serum bottle",
            "skincare",
        ],
    )
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("test runtime")
        .block_on(future)
}

#[test]
fn product_photo_intent_fences_the_tattoo_caption_hit() {
    // The caption mentions both subject words ("tubes", "cleanser"), so the
    // query-only fence has no reason to drop it — the photo contract lives
    // in the image prompt.
    assert_eq!(
        fence_rejection(&tattoo_cover_up(), CLEANSER_QUERY, ""),
        None,
        "query-only contract cannot see the product-photo request"
    );
    assert_eq!(
        fence_rejection(&tattoo_cover_up(), CLEANSER_QUERY, CLEANSER_INTENT),
        Some("photo requested but metadata names a non-photo medium")
    );
    assert!(retain_relevant_hits_for_intent(
        vec![tattoo_cover_up()],
        CLEANSER_QUERY,
        CLEANSER_INTENT
    )
    .is_empty());
}

#[test]
fn product_photo_intent_fences_the_graffiti_mural_hit() {
    assert_eq!(
        fence_rejection(&snake_oil_mural(), SERUM_QUERY, SERUM_INTENT),
        Some("photo requested but metadata names a non-photo medium")
    );
    // A genuine product shot for the same slot still survives.
    let kept = retain_relevant_hits_for_intent(
        vec![snake_oil_mural(), dropper_bottle()],
        SERUM_QUERY,
        SERUM_INTENT,
    );
    let titles: Vec<&str> = kept.iter().map(|hit| hit.title.as_str()).collect();
    assert_eq!(titles, vec!["Dropper bottle, blank retro label"]);
}

#[test]
fn illustration_intent_keeps_artwork_hits() {
    // The medium fence only engages for photographic intents.
    let kept = retain_relevant_hits_for_intent(
        vec![snake_oil_mural()],
        "snake oil serum",
        "street art graffiti of a snake oil serum bottle",
    );
    assert_eq!(kept.len(), 1);
}

#[test]
fn empty_square_answers_leave_the_fenced_list_empty_instead_of_a_head_noun_less_retry() {
    // Incident replay: both square primaries answered zero hits. The old
    // ladder retried with the FIRST two words ("serum oil", "cleanser
    // tube") and took the best raw-overlap result, falling back to the
    // first result at zero overlap. The fenced list judges every retry
    // against the authored query + intent.
    for (query, intent, retry_answer) in [
        (SERUM_QUERY, SERUM_INTENT, vec![snake_oil_mural()]),
        (CLEANSER_QUERY, CLEANSER_INTENT, vec![tattoo_cover_up()]),
    ] {
        let requests = RefCell::new(Vec::new());
        let list = block_on(fenced_openverse_list_with(query, intent, |candidate| {
            requests.borrow_mut().push(candidate.clone());
            let answer = if candidate == query {
                Vec::new()
            } else {
                retry_answer.clone()
            };
            async move { Some(answer) }
        }))
        .expect("catalogue answered");
        assert!(list.hits.is_empty(), "{query}: {:?}", requests.borrow());
        // A three-word subject phrase has no shorter concrete tail, so no
        // head-noun-less "first two words" request is ever sent.
        assert_eq!(requests.into_inner(), vec![query.to_string()]);
    }
}

#[test]
fn a_longer_query_retries_with_its_concrete_tail_under_the_authored_fence() {
    let query = "minimal warm serum dropper bottle";
    let requests = RefCell::new(Vec::new());
    let list = block_on(fenced_openverse_list_with(
        query,
        SERUM_INTENT,
        |candidate| {
            requests.borrow_mut().push(candidate.clone());
            let answer = if candidate == query {
                Vec::new()
            } else {
                vec![snake_oil_mural(), dropper_bottle()]
            };
            async move { Some(answer) }
        },
    ))
    .expect("catalogue answered");
    assert_eq!(
        requests.into_inner(),
        vec![query.to_string(), "serum dropper bottle".to_string()]
    );
    assert_eq!(list.provider, "openverse-retry");
    assert_eq!(list.provider_query, "serum dropper bottle");
    let titles: Vec<&str> = list.hits.iter().map(|hit| hit.title.as_str()).collect();
    assert_eq!(titles, vec!["Dropper bottle, blank retro label"]);
}

#[test]
fn resolved_image_description_is_one_line_and_url_free() {
    let resolved = ResolvedImage {
        src: "data:image/jpeg;base64,AAAA".to_string(),
        provider: "openverse",
        provider_query: "serum oil bottle".to_string(),
        title: "Dropper \"bottle\"\nblank".to_string(),
    };
    assert_eq!(
        resolved.describe(),
        "openverse \"Dropper 'bottle' blank\" (via \"serum oil bottle\")"
    );
}

/// Live per-stage trace of the unjudged ladder against the public Openverse
/// and Wikimedia endpoints (no credentials). Run with
/// `cargo test -p op-image-enrich --features net live_image_search_trace
/// -- --ignored --nocapture`; override the slot with
/// `OP_IMAGE_TRACE_QUERY` / `OP_IMAGE_TRACE_INTENT` / `OP_IMAGE_TRACE_ASPECT`
/// (`wide|tall|square|any`).
#[test]
#[ignore = "live network trace of the Openverse/Wikimedia ladder"]
fn live_image_search_trace() {
    crate::net::search_trace::force_search_trace(true);
    let slots: Vec<(String, String)> = match std::env::var("OP_IMAGE_TRACE_QUERY") {
        Ok(query) => vec![(
            query,
            std::env::var("OP_IMAGE_TRACE_INTENT").unwrap_or_default(),
        )],
        Err(_) => vec![
            (SERUM_QUERY.to_string(), SERUM_INTENT.to_string()),
            (CLEANSER_QUERY.to_string(), CLEANSER_INTENT.to_string()),
        ],
    };
    let aspects: Vec<Option<ImageAspectRatio>> =
        match std::env::var("OP_IMAGE_TRACE_ASPECT").as_deref() {
            Ok("wide") => vec![Some(ImageAspectRatio::Wide)],
            Ok("tall") => vec![Some(ImageAspectRatio::Tall)],
            Ok("square") => vec![Some(ImageAspectRatio::Square)],
            Ok("any") => vec![None],
            _ => vec![Some(ImageAspectRatio::Square), None],
        };
    for (query, intent) in &slots {
        for aspect in &aspects {
            let used = Mutex::new(HashSet::new());
            let resolved = fetch_first_image_resolved_blocking(query, intent, *aspect, None, &used);
            eprintln!(
                "[IMAGE-TRACE] RESULT query=\"{query}\" aspect={:?} -> {}\n",
                aspect,
                resolved
                    .as_ref()
                    .map_or("none (fallback tile)".to_string(), ResolvedImage::describe)
            );
        }
    }
}
