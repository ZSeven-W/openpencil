//! Page-scroll contract: the page root is the implicit `$scroll` source,
//! authored pins are the pinned candidates, and paint-only translate
//! bindings shift subtrees. Split from `tests_binding_overlay.rs` to keep
//! it under the 800-line cap.

#![cfg(test)]

use super::tests_binding_overlay::{enter, find};

/// Page-scroll contract: a root without its own `events.onScroll` is
/// the scroll source for every `$scroll` reference beneath it.
fn page_scroll_doc() -> jian_ops_schema::PenDocument {
    let source = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "page",
        "app": { "name": "page", "version": "1", "id": "page" },
        "children": [
            { "type": "frame", "id": "landing", "x": 0, "y": 0,
              "width": 400, "height": 1200,
              "children": [
                  { "type": "rectangle", "id": "nav", "pin": true,
                    "x": 0, "y": 0, "width": 400, "height": 64,
                    "fill": [{ "type": "solid", "color": "#111111" }] },
                  { "type": "rectangle", "id": "hero",
                    "x": 0, "y": 64, "width": 400, "height": 500,
                    "opacity": 1,
                    "fill": [{ "type": "solid", "color": "#eeeeee" }],
                    "bindings": { "opacity": "$scroll.progress" } },
                  { "type": "frame", "id": "inner", "x": 0, "y": 600,
                    "width": 400, "height": 200, "clipContent": true,
                    "events": { "onScroll": [ { "set": { "$app.seen": "true" } } ] },
                    "children": [
                        { "type": "rectangle", "id": "inner-body",
                          "x": 0, "y": 0, "width": 400, "height": 600,
                          "opacity": 1,
                          "fill": [{ "type": "solid", "color": "#cccccc" }],
                          "bindings": { "opacity": "$scroll.progress" } }
                    ] }
              ] }
        ],
        "state": { "seen": { "type": "bool", "default": false } }
    }"##;
    jian_ops_schema::load_str(source)
        .expect("parse page scroll document")
        .value
}

#[test]
fn page_scroll_feeds_root_scope_and_explicit_scrollers_shadow_it() {
    let document = page_scroll_doc();
    let mut session = enter(&document);
    assert!(
        session.warnings().is_empty(),
        "page-scoped bindings raise no diagnostics: {:?}",
        session.warnings()
    );
    let before = session.preview_scene_for_test();
    assert_eq!(
        find(&before, "hero").opacity,
        0.0,
        "unscrolled page → progress 0"
    );
    let hero_y = find(&before, "hero").bounds.origin.y;

    assert!(
        session.set_page_scroll(150.0, 300.0, -20.0),
        "a page scroll that moves a bound value reports a change"
    );
    let scrolled = session.preview_scene_for_test();
    assert!(
        (find(&scrolled, "hero").opacity - 0.5).abs() < 1e-4,
        "hero opacity tracks $scroll.progress of the page: {}",
        find(&scrolled, "hero").opacity
    );
    assert_eq!(
        find(&scrolled, "hero").bounds.origin.y,
        hero_y,
        "the host scrolls the page; the overlay must not translate page children too"
    );
    assert_eq!(
        find(&scrolled, "inner-body").opacity,
        0.0,
        "an explicit scroller shadows the page scope for its subtree"
    );
    assert!(
        !session.set_page_scroll(150.0, 300.0, -20.0),
        "an unchanged position reports no change"
    );
    assert!(
        session.set_page_scroll(150.0, 600.0, 0.0),
        "a new max changes progress even at the same offset"
    );
}

#[test]
fn translate_bindings_shift_the_subtree_as_a_paint_only_offset() {
    let source = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "parallax",
        "app": { "name": "parallax", "version": "1", "id": "parallax" },
        "children": [
            { "type": "frame", "id": "page", "x": 0, "y": 0,
              "width": 400, "height": 1600,
              "children": [
                  { "type": "frame", "id": "hero-bg", "x": 0, "y": 0,
                    "width": 400, "height": 500,
                    "fill": [{ "type": "solid", "color": "#eeeeee" }],
                    "bindings": { "translateY": "$scroll.offset * -0.5" },
                    "children": [
                        { "type": "rectangle", "id": "blob", "x": 40, "y": 60,
                          "width": 80, "height": 80,
                          "fill": [{ "type": "solid", "color": "#88aaff" }] }
                    ] },
                  { "type": "rectangle", "id": "bar", "x": 0, "y": 0,
                    "width": 400, "height": 4,
                    "fill": [{ "type": "solid", "color": "#ff0000" }],
                    "bindings": { "translateX": "-(1 - $scroll.progress) * 400" } }
              ] }
        ]
    }"##;
    let document = jian_ops_schema::load_str(source).expect("parse").value;
    let mut session = enter(&document);
    assert!(
        session.warnings().is_empty(),
        "translate is PaintOnly, so $scroll may drive it: {:?}",
        session.warnings()
    );
    let before = session.preview_scene_for_test();
    let bg_y = find(&before, "hero-bg").bounds.origin.y;
    let blob_y = find(&before, "blob").bounds.origin.y;
    assert_eq!(
        find(&before, "bar").bounds.origin.x,
        -400.0,
        "an unscrolled page parks the progress bar fully off-canvas"
    );

    assert!(session.set_page_scroll(200.0, 800.0, -20.0));
    let scrolled = session.preview_scene_for_test();
    assert_eq!(
        find(&scrolled, "hero-bg").bounds.origin.y,
        bg_y - 100.0,
        "the bound subtree root shifts by the evaluated offset"
    );
    assert_eq!(
        find(&scrolled, "blob").bounds.origin.y,
        blob_y - 100.0,
        "descendants ride along with the visual offset"
    );
    assert_eq!(
        find(&scrolled, "bar").bounds.origin.x,
        -300.0,
        "translateX tracks $scroll.progress"
    );
}

#[test]
fn authored_pin_on_a_page_root_is_the_pinned_candidate_on_every_device() {
    let document = page_scroll_doc();
    let session = enter(&document);
    let (top_id, _) = session
        .pinned_status_bar_candidate(false)
        .expect("a pin:true child flush with the top edge pins on desktop");
    assert_eq!(top_id, "nav");
    let (phone_id, _) = session
        .pinned_status_bar_candidate(true)
        .expect("the authored pin also wins on phone");
    assert_eq!(phone_id, "nav");
    assert!(
        session.pinned_nav_candidate(false).is_none(),
        "nothing is pinned to the bottom edge"
    );
}

/// Generated sections wrap their nav in a fit-content shell and put the
/// `pin: true` on the inner frame; the pin bubbles up through the
/// single-child chain so the shell (the page root's direct child) pins.
#[test]
fn a_pin_inside_a_single_child_wrapper_pins_the_wrapper() {
    let source = r##"{
        "version": "1.1", "formatVersion": "1.1", "id": "wrapped",
        "app": { "name": "wrapped", "version": "1", "id": "wrapped" },
        "children": [
            { "type": "frame", "id": "landing", "x": 0, "y": 0, "width": 400, "height": 1400,
              "layout": "vertical",
              "children": [
                  { "type": "frame", "id": "nav-section", "width": 400, "height": "fit_content",
                    "layout": "vertical",
                    "children": [
                        { "type": "frame", "id": "nav-bar", "pin": true, "width": 400, "height": 64,
                          "fill": [{ "type": "solid", "color": "#111111" }] }
                    ] },
                  { "type": "frame", "id": "two-kids", "width": 400, "height": "fit_content",
                    "layout": "vertical",
                    "children": [
                        { "type": "rectangle", "id": "a", "pin": true, "width": 400, "height": 40 },
                        { "type": "rectangle", "id": "b", "width": 400, "height": 900 }
                    ] }
              ] }
        ]
    }"##;
    let document = jian_ops_schema::load_str(source).expect("parse").value;
    let session = enter(&document);
    let (top, _) = session
        .pinned_status_bar_candidate(false)
        .expect("the wrapped nav pins through its shell");
    assert_eq!(top, "nav-section");
    assert!(
        session.pinned_nav_candidate(false).is_none(),
        "a pin inside a multi-child section does not bubble up"
    );
}
