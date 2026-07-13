use super::adopt_stray_images_for_all_roots;
use crate::test_support::VecDocSink;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

/// The measured deal card: an empty `DealImg` slot inside the image band, and
/// the photo hanging off the CARD below the price row.
fn deal_card_with_stray_photo() -> &'static str {
    r##"{ "version": "1.0", "children": [{
        "type": "frame", "id": "root", "name": "Screen", "width": 390, "height": 844,
        "layout": "vertical",
        "children": [{
            "type": "frame", "id": "card", "name": "Deal Card",
            "width": "fill_container", "height": "fit_content", "layout": "vertical",
            "clipContent": true,
            "children": [
                { "type": "frame", "id": "band", "width": "fill_container", "height": 160,
                  "layout": "none", "clipContent": true, "children": [
                    { "type": "frame", "id": "slot", "name": "DealImg",
                      "width": "fill_container", "height": 160, "x": 0, "y": 0 },
                    { "type": "frame", "id": "badge", "height": 28, "x": 10, "y": 10,
                      "layout": "horizontal", "children": [
                        { "type": "text", "id": "badge-text", "content": "-35%" }
                      ]}
                  ]},
                { "type": "frame", "id": "content", "width": "fill_container",
                  "height": "fit_content", "layout": "vertical", "children": [
                    { "type": "text", "id": "title", "content": "Maldives Overwater Villa" }
                  ]},
                { "type": "image", "id": "photo", "name": "maldives overwater villa ocean",
                  "width": "fill_container", "height": 300,
                  "src": "data:image/jpeg;base64,AAAA" }
            ]
        }]
    }] }"##
}

fn sink_from(json: &str) -> VecDocSink {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(json).expect("parse");
    let mut sink = VecDocSink::new();
    sink.state = op_editor_core::EditorState::from_document(doc);
    sink
}

#[test]
fn the_stray_photo_moves_into_the_empty_media_slot_and_fills_it() {
    let mut sink = sink_from(deal_card_with_stray_photo());
    adopt_stray_images_for_all_roots(&mut sink);

    let slot = op_editor_core::walkers::find_node(
        sink.state.active_children(),
        &NodeId::new("slot".to_string()),
    )
    .expect("slot survives");
    let adopted = slot.children().expect("slot has children");
    assert_eq!(adopted.len(), 1, "the photo landed in the slot");
    assert_eq!(adopted[0].id_str(), "photo");
    assert_eq!(
        adopted[0].width_px(),
        None,
        "the photo fills its slot rather than keeping its authored 300px box"
    );

    let card = op_editor_core::walkers::find_node(
        sink.state.active_children(),
        &NodeId::new("card".to_string()),
    )
    .expect("card survives");
    let card_children: Vec<&str> = card
        .children()
        .expect("card children")
        .iter()
        .map(|c| c.id_str())
        .collect();
    assert_eq!(
        card_children,
        vec!["band", "content"],
        "the photo no longer hangs below the price row"
    );
}

#[test]
fn an_image_already_inside_its_slot_is_left_alone() {
    let json = r##"{ "version": "1.0", "children": [{
        "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
        "children": [{
            "type": "frame", "id": "card", "width": "fill_container", "height": "fit_content",
            "layout": "vertical", "children": [
                { "type": "frame", "id": "band", "width": "fill_container", "height": 160,
                  "children": [
                    { "type": "image", "id": "photo", "width": "fill_container",
                      "height": "fill_container", "src": "data:image/jpeg;base64,AAAA" }
                  ]},
                { "type": "text", "id": "title", "content": "Maldives" }
            ]
        }]
    }] }"##;
    let mut sink = sink_from(json);
    adopt_stray_images_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "a correctly-parented photo is untouched: {:?}",
        sink.applied
    );
}

#[test]
fn an_ambiguous_pair_of_empty_slots_is_left_to_the_model() {
    // Two empty media boxes — moving the photo into either would be a guess.
    let json = r##"{ "version": "1.0", "children": [{
        "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
        "children": [{
            "type": "frame", "id": "card", "width": "fill_container", "height": "fit_content",
            "layout": "vertical", "children": [
                { "type": "frame", "id": "slot-a", "width": "fill_container", "height": 160 },
                { "type": "frame", "id": "slot-b", "width": "fill_container", "height": 160 },
                { "type": "image", "id": "photo", "width": "fill_container", "height": 300,
                  "src": "data:image/jpeg;base64,AAAA" }
            ]
        }]
    }] }"##;
    let mut sink = sink_from(json);
    adopt_stray_images_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "ambiguity is not a contract — echo it instead: {:?}",
        sink.applied
    );
}

#[test]
fn a_short_empty_spacer_is_not_a_media_slot() {
    let json = r##"{ "version": "1.0", "children": [{
        "type": "frame", "id": "root", "width": 390, "height": 844, "layout": "vertical",
        "children": [{
            "type": "frame", "id": "card", "width": "fill_container", "height": "fit_content",
            "layout": "vertical", "children": [
                { "type": "frame", "id": "spacer", "width": "fill_container", "height": 12 },
                { "type": "image", "id": "photo", "width": "fill_container", "height": 300,
                  "src": "data:image/jpeg;base64,AAAA" }
            ]
        }]
    }] }"##;
    let mut sink = sink_from(json);
    adopt_stray_images_for_all_roots(&mut sink);
    assert!(
        sink.applied.is_empty(),
        "a 12px divider is not a photo box: {:?}",
        sink.applied
    );
}

#[test]
fn the_move_is_idempotent() {
    let mut sink = sink_from(deal_card_with_stray_photo());
    adopt_stray_images_for_all_roots(&mut sink);
    let after_first = sink.applied.len();
    adopt_stray_images_for_all_roots(&mut sink);
    assert_eq!(
        sink.applied.len(),
        after_first,
        "a second pass finds nothing left to move"
    );
    assert!(matches!(
        sink.applied.first(),
        Some(EditorCommand::MoveNode { .. })
    ));
}
