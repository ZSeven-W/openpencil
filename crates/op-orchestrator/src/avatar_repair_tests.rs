use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

fn find_by_id<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?.iter().find_map(|c| find_by_id(c, id))
}

/// GLM-5.2 measured shape (test0711-2.op): an 88×44 pill holding a 44×44
/// image — reads as an empty circle NEXT TO a square photo. The slot is
/// squared + clipped and the image switches to fill both axes.
#[test]
fn wide_pill_avatar_slot_is_squared_and_clipped() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "hdr", "name": "GreetingHeader",
                  "width": "fill_container", "height": "fit_content", "layout": "horizontal",
                  "children": [
                    { "type": "frame", "id": "slot", "name": "Avatar",
                      "width": 88, "height": 44, "cornerRadius": 9999,
                      "children": [
                        { "type": "image", "id": "img", "name": "woman face headshot",
                          "src": "", "width": 44, "height": 44 }
                      ] }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::avatar_repair::repair_avatar_slots_for_all_roots(&mut sink);

    let root = &state.active_children()[0];
    let slot = find_by_id(root, "slot").expect("slot");
    assert_eq!(slot.width_px(), Some(44.0), "slot squared to its height");
    let slot_json = serde_json::to_value(slot).expect("slot json");
    assert_eq!(slot_json["clipContent"], serde_json::json!(true));
    let img = find_by_id(root, "img").expect("img");
    assert!(
        img.width_px().is_none() && img.height_px().is_none(),
        "image switches to fill_container on both axes"
    );
}

/// A correctly-built avatar (square + clipping + filling image) is untouched,
/// and large pill-radius hero cards are out of scope.
#[test]
fn correct_avatars_and_large_pills_are_untouched() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "good", "name": "Avatar",
                  "width": 44, "height": 44, "cornerRadius": 22, "clipContent": true,
                  "children": [
                    { "type": "image", "id": "gimg", "name": "man face headshot",
                      "src": "", "width": "fill_container", "height": "fill_container" }
                  ] },
                { "type": "frame", "id": "hero", "name": "Hero Card",
                  "width": 358, "height": 200, "cornerRadius": 100,
                  "children": [
                    { "type": "image", "id": "himg", "name": "beach sunset",
                      "src": "", "width": 358, "height": 200 }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let before = serde_json::to_string(state.active_children()).expect("snapshot");
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::avatar_repair::repair_avatar_slots_for_all_roots(&mut sink);
    assert_eq!(
        serde_json::to_string(state.active_children()).expect("snapshot"),
        before,
        "no-op on healthy avatars and non-avatar pills"
    );
}

/// The EXACT shape measured in test0711-22.op (23:07 run): slot clipped and
/// image filling, but `width: fill_container` stretched the avatar into a
/// page-wide pill. The repair squares it to its height.
#[test]
fn fill_container_wide_avatar_pill_is_squared() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Music Home",
            "width": 402, "height": 874, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "hdr", "name": "Header",
                  "width": "fill_container", "height": "fit_content", "layout": "horizontal",
                  "children": [
                    { "type": "text", "id": "t", "name": "Greeting", "content": "Good evening",
                      "width": "fit_content", "height": "fit_content" },
                    { "type": "frame", "id": "slot", "name": "Avatar",
                      "width": "fill_container", "height": 44, "cornerRadius": 22,
                      "clipContent": true,
                      "children": [
                        { "type": "image", "id": "img", "name": "man face headshot portrait",
                          "src": "", "width": "fill_container", "height": "fill_container" }
                      ] }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::avatar_repair::repair_avatar_slots_for_all_roots(&mut sink);

    let root = &state.active_children()[0];
    let slot = find_by_id(root, "slot").expect("slot");
    assert_eq!(
        slot.width_px(),
        Some(44.0),
        "fill_container avatar squared to its height"
    );
}

/// MiniPlayer art shape (test0711-22): a 44px thumb in a horizontal player
/// row given `width: fill_container` — it steals the row's flex and
/// stretches the artwork into a banner. Squared to its height. A numeric
/// wide thumb (deliberate design) stays untouched.
#[test]
fn fill_container_row_thumbnail_is_squared_but_numeric_wide_thumb_is_kept() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Music Home",
            "width": 402, "height": 874, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "player", "name": "MiniPlayer",
                  "width": "fill_container", "height": "fit_content",
                  "layout": "horizontal", "gap": 12,
                  "children": [
                    { "type": "frame", "id": "art", "name": "MPArt",
                      "width": "fill_container", "height": 44,
                      "cornerRadius": 8, "clipContent": true,
                      "children": [
                        { "type": "image", "id": "cover", "name": "album cover neon",
                          "src": "", "width": "fill_container", "height": "fill_container" }
                      ] },
                    { "type": "text", "id": "title", "name": "Title",
                      "content": "Electric Dreams", "width": "fit_content",
                      "height": "fit_content" }
                  ] },
                { "type": "frame", "id": "gallery", "name": "Gallery Row",
                  "width": "fill_container", "height": "fit_content",
                  "layout": "horizontal", "gap": 8,
                  "children": [
                    { "type": "frame", "id": "wide", "name": "Wide Thumb",
                      "width": 120, "height": 64, "cornerRadius": 8, "clipContent": true,
                      "children": [
                        { "type": "image", "id": "wimg", "name": "landscape",
                          "src": "", "width": "fill_container", "height": "fill_container" }
                      ] },
                    { "type": "text", "id": "cap", "name": "Caption",
                      "content": "Trip", "width": "fit_content", "height": "fit_content" }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::avatar_repair::repair_avatar_slots_for_all_roots(&mut sink);

    let root = &state.active_children()[0];
    let art = find_by_id(root, "art").expect("art");
    assert_eq!(
        art.width_px(),
        Some(44.0),
        "row thumb squared to its height"
    );
    let wide = find_by_id(root, "wide").expect("wide");
    assert_eq!(
        wide.width_px(),
        Some(120.0),
        "deliberate numeric wide thumb untouched"
    );
}

/// test0711-22 00:25 shape: "AvatarImg" slot authored fill×fill holding a
/// fill×300 headshot — resolved as a 42×300 strip down the screen. The
/// avatar-query NAME on the image is the contract signal; the slot becomes
/// a 44px clipped circle regardless of its own (useless) sizing.
#[test]
fn fill_by_fill_slot_with_avatar_named_image_becomes_a_circle() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Music Home",
            "width": 402, "height": 874, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "hdr", "name": "Header",
                  "width": "fill_container", "height": "fit_content", "layout": "horizontal",
                  "children": [
                    { "type": "text", "id": "t", "name": "Greeting", "content": "Good evening",
                      "width": "fit_content", "height": "fit_content" },
                    { "type": "frame", "id": "slot", "name": "AvatarImg",
                      "width": "fill_container", "height": "fill_container",
                      "children": [
                        { "type": "image", "id": "img", "name": "man face headshot portrait",
                          "src": "", "width": "fill_container", "height": 300 }
                      ] }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::avatar_repair::repair_avatar_slots_for_all_roots(&mut sink);

    let root = &state.active_children()[0];
    let slot = find_by_id(root, "slot").expect("slot");
    assert_eq!(slot.width_px(), Some(44.0));
    assert_eq!(slot.height_px(), Some(44.0));
    let img = find_by_id(root, "img").expect("img");
    assert!(
        img.height_px().is_none(),
        "300px headshot switches to fill_container"
    );
}

/// test0711-22 "Midnight Drive" shape: the model laid `[empty stub frame,
/// image]` as SIBLINGS inside the Art slot, so the photo rendered beside an
/// empty box. The stub is dropped, the image takes the slot. A scrim
/// overlay ([image, frame] order) is untouched.
#[test]
fn empty_twin_stub_beside_image_is_removed_but_scrim_overlay_is_kept() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{
            "type": "frame", "id": "root", "name": "Screen",
            "width": 402, "height": 874, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "art", "name": "Art",
                  "width": 140, "height": 140, "clipContent": true,
                  "children": [
                    { "type": "frame", "id": "stub", "width": 70, "height": 140,
                      "fill": [{ "type": "solid", "color": "#1A1A1A" }] },
                    { "type": "image", "id": "img", "name": "neon city night drive",
                      "src": "", "width": 70, "height": 140 }
                  ] },
                { "type": "frame", "id": "heroic", "name": "Hero",
                  "width": 358, "height": 200,
                  "children": [
                    { "type": "image", "id": "himg", "name": "beach", "src": "",
                      "width": "fill_container", "height": "fill_container" },
                    { "type": "frame", "id": "scrim", "width": "fill_container",
                      "height": "fill_container",
                      "fill": [{ "type": "solid", "color": "#00000080" }] }
                  ] }
            ]
        }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::avatar_repair::remove_empty_twin_stubs_beside_images_for_all_roots(&mut sink);

    let root = &state.active_children()[0];
    assert!(find_by_id(root, "stub").is_none(), "stub removed");
    let img = find_by_id(root, "img").expect("img");
    assert!(
        img.width_px().is_none() && img.height_px().is_none(),
        "image takes the slot"
    );
    assert!(find_by_id(root, "scrim").is_some(), "scrim overlay kept");
}
