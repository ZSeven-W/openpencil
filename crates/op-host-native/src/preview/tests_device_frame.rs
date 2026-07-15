//! Device-frame session-layer tests: framed root selection and bottom
//! nav detection (semantic + heuristic + validity gates).

#![cfg(test)]

use super::PreviewSession;

fn theme() -> std::collections::BTreeMap<String, String> {
    std::collections::BTreeMap::new()
}

fn load(src: &str) -> jian_ops_schema::PenDocument {
    jian_ops_schema::load_str(src)
        .expect("parse test doc")
        .value
}

fn enter(doc: &jian_ops_schema::PenDocument) -> PreviewSession {
    PreviewSession::enter(doc, (1200.0, 800.0), &theme(), 0, false).expect("enter preview")
}

/// 390-wide phone root, 800 tall, with a bottom nav frame flush to the
/// bottom (y=740, h=60, full width) and a sibling content block.
fn phone_doc_with_bottom_nav(nav_extra: &str) -> jian_ops_schema::PenDocument {
    load(&format!(
        r##"{{
        "version": "1.0.0",
        "children": [
            {{ "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{{"type":"solid","color":"#ffffff"}}],
              "children": [
                {{ "type": "rectangle", "id": "content", "x": 0, "y": 0,
                  "width": 390, "height": 700,
                  "fill": [{{"type":"solid","color":"#eeeeee"}}] }},
                {{ "type": "frame", "id": "nav", "x": 0, "y": 740,
                  "width": 390, "height": 60{nav_extra},
                  "fill": [{{"type":"solid","color":"#222222"}}] }}
              ] }}
        ]
    }}"##
    ))
}

#[test]
fn framed_root_returns_first_page_root_scene_rect() {
    let doc = phone_doc_with_bottom_nav("");
    let session = enter(&doc);
    let (id, rect) = session.framed_root().expect("framed root");
    assert_eq!(id, "screen");
    assert!((rect.size.x - 390.0).abs() < 0.5);
    assert!((rect.size.y - 800.0).abs() < 0.5);
}

#[test]
fn framed_root_app_mode_returns_mounted_screen_root() {
    let doc = load(
        r##"{
        "version": "1.1",
        "formatVersion": "1.1",
        "id": "x",
        "app": { "name": "x", "version": "1", "id": "x" },
        "pages": [
            { "id": "canvas", "name": "Canvas", "children": [
                { "type": "frame", "id": "home", "x": 0, "y": 0,
                  "width": 390, "height": 800, "screen": "/",
                  "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
                { "type": "frame", "id": "detail", "x": 500, "y": 0,
                  "width": 390, "height": 800, "screen": "/detail",
                  "fill": [{"type":"solid","color":"#ffffff"}], "children": [] }
            ] }
        ]
    }"##,
    );
    let session = enter(&doc);
    assert!(session.is_app_mode(), "screen-marked doc enters app mode");
    let (id, rect) = session.framed_root().expect("entry screen root");
    assert_eq!(id, "home");
    assert!((rect.size.x - 390.0).abs() < 0.5);
}

#[test]
fn zero_height_semantic_nav_is_rejected() {
    let doc = load(
        r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "nav", "x": 0, "y": 800,
                  "width": 390, "height": 0,
                  "semantics": { "role": "nav" },
                  "fill": [{"type":"solid","color":"#222222"}] }
              ] }
        ]
    }"##,
    );
    assert!(enter(&doc).pinned_nav_candidate(true).is_none());
}

#[test]
fn framed_root_none_for_empty_page() {
    let doc = load(r##"{ "version": "1.0.0", "children": [] }"##);
    let session = enter(&doc);
    assert!(session.framed_root().is_none());
}

#[test]
fn heuristic_detects_flush_full_width_nav() {
    let doc = phone_doc_with_bottom_nav("");
    let session = enter(&doc);
    let (id, rect) = session.pinned_nav_candidate(true).expect("nav pinned");
    assert_eq!(id, "nav");
    assert!((rect.size.y - 60.0).abs() < 0.5);
}

#[test]
fn desktop_never_pins() {
    let doc = phone_doc_with_bottom_nav("");
    let session = enter(&doc);
    assert!(session.pinned_nav_candidate(false).is_none());
}

#[test]
fn semantic_nav_pins_even_when_narrow() {
    let doc = load(
        r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "nav", "x": 80, "y": 740,
                  "width": 230, "height": 60,
                  "semantics": { "role": "nav" },
                  "fill": [{"type":"solid","color":"#222222"}] }
              ] }
        ]
    }"##,
    );
    let session = enter(&doc);
    let (id, _) = session.pinned_nav_candidate(true).expect("semantic nav");
    assert_eq!(id, "nav");
}

#[test]
fn top_nav_with_semantic_role_does_not_pin() {
    let doc = load(
        r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "topnav", "x": 0, "y": 0,
                  "width": 390, "height": 60,
                  "semantics": { "role": "nav" },
                  "fill": [{"type":"solid","color":"#222222"}] }
              ] }
        ]
    }"##,
    );
    let session = enter(&doc);
    assert!(session.pinned_nav_candidate(true).is_none());
}

#[test]
fn hidden_nav_is_rejected() {
    let doc = phone_doc_with_bottom_nav(r#", "visible": false"#);
    let session = enter(&doc);
    assert!(session.pinned_nav_candidate(true).is_none());
}

#[test]
fn heuristic_rejects_each_missing_gate() {
    let doc = load(
        r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "nav", "x": 80, "y": 740,
                  "width": 230, "height": 60,
                  "fill": [{"type":"solid","color":"#222222"}] }
              ] }
        ]
    }"##,
    );
    assert!(enter(&doc).pinned_nav_candidate(true).is_none());

    let doc = load(
        r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "nav", "x": 0, "y": 670,
                  "width": 390, "height": 130,
                  "fill": [{"type":"solid","color":"#222222"}] }
              ] }
        ]
    }"##,
    );
    assert!(enter(&doc).pinned_nav_candidate(true).is_none());

    let doc = load(
        r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "nav", "x": 0, "y": 730,
                  "width": 390, "height": 60,
                  "fill": [{"type":"solid","color":"#222222"}] }
              ] }
        ]
    }"##,
    );
    assert!(enter(&doc).pinned_nav_candidate(true).is_none());
}

#[test]
fn oversized_or_hidden_candidates_are_rejected() {
    let doc = load(
        r##"{
        "version": "1.0.0",
        "children": [
            { "type": "frame", "id": "screen", "x": 0, "y": 0,
              "width": 390, "height": 800,
              "fill": [{"type":"solid","color":"#ffffff"}],
              "children": [
                { "type": "frame", "id": "bignav", "x": 0, "y": 500,
                  "width": 390, "height": 300,
                  "semantics": { "role": "nav" },
                  "fill": [{"type":"solid","color":"#222222"}] }
              ] }
        ]
    }"##,
    );
    let session = enter(&doc);
    assert!(session.pinned_nav_candidate(true).is_none());
}
