//! Tests for Track C-1's enter-preview auto-wiring.

use super::*;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;

fn parse(json: &str) -> PenDocument {
    serde_json::from_str(json).expect("valid PenDocument")
}

fn frame_screen<'a>(doc: &'a PenDocument, id: &str) -> Option<&'a str> {
    doc.children.iter().find_map(|n| match n {
        PenNode::Frame(f) if f.base.id == id => f.screen.as_deref(),
        _ => None,
    })
}

fn two_unmarked_screens() -> &'static str {
    r#"{"version":"1.0","children":[
        {"type":"frame","id":"home","name":"Home","width":390,"height":844,
         "layout":"vertical","children":[]},
        {"type":"frame","id":"profile","name":"Profile","width":390,"height":844,
         "layout":"vertical","children":[]}
    ]}"#
}

#[test]
fn unmarked_multi_screen_doc_gets_wired() {
    let doc = parse(two_unmarked_screens());
    let wired = auto_wire_for_preview(&doc, 0).expect("no marker -> pass runs");
    assert_eq!(frame_screen(&wired, "home"), Some("/"));
    assert_eq!(frame_screen(&wired, "profile"), Some("/profile"));
}

#[test]
fn caller_document_is_never_mutated() {
    let doc = parse(two_unmarked_screens());
    let before = serde_json::to_string(&doc).unwrap();
    let _ = auto_wire_for_preview(&doc, 0);
    let after = serde_json::to_string(&doc).unwrap();
    assert_eq!(after, before, "auto_wire_for_preview must not touch `doc`");
}

#[test]
fn any_authored_marker_skips_the_whole_pass() {
    // "profile" already carries an authored marker — Track C-1's stricter
    // rule (unlike Track A's own per-node gate) skips wiring ENTIRELY, so
    // "home" is left unmarked too, not auto-assigned the entry path.
    let json = r#"{"version":"1.0","children":[
        {"type":"frame","id":"home","name":"Home","width":390,"height":844,
         "layout":"vertical","children":[]},
        {"type":"frame","id":"profile","name":"Profile","width":390,"height":844,
         "layout":"vertical","children":[],"screen":"/profile"}
    ]}"#;
    let doc = parse(json);
    assert!(
        auto_wire_for_preview(&doc, 0).is_none(),
        "an authored marker anywhere on the page must skip the whole pass"
    );
}

#[test]
fn single_screen_doc_is_untouched() {
    // Only one screen-shaped top-level frame: Track A's own gate (<2
    // candidates) makes the pass a no-op, so auto_wire's `Some` (the pass
    // DID run, unmarked) still yields a byte-identical document.
    let json = r#"{"version":"1.0","children":[
        {"type":"frame","id":"home","name":"Home","width":390,"height":844,
         "layout":"vertical","children":[]}
    ]}"#;
    let doc = parse(json);
    let wired = auto_wire_for_preview(&doc, 0).expect("no marker -> pass runs");
    assert_eq!(
        serde_json::to_string(&wired).unwrap(),
        serde_json::to_string(&doc).unwrap(),
        "single-screen doc: pass runs but is a no-op"
    );
}
