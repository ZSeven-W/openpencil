//! Regression cover for adding a template to a document that already exists.

use super::*;
use crate::scene_template_catalog::{scene_template_catalogue, scene_template_document};
use crate::test_support::rect;
use jian_ops_schema::page::PenPage;

fn boards_for(template_id: &str) -> TemplateBoards {
    let source = scene_template_document(template_id).expect("the template ships");
    template_boards(source, template_id).expect("the shipped asset parses")
}

fn top_level_ids(state: &EditorState) -> Vec<String> {
    state
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect()
}

/// The trap this whole path exists to avoid: inserting boards one at a time
/// lets the empty-root swap consume the previous one, so N appends leave one
/// board. Every shipped template goes in as a unit and every board survives.
#[test]
fn every_template_lands_with_all_of_its_boards() {
    for template in scene_template_catalogue() {
        let mut state = EditorState::starter();
        let before = state.active_children().len();
        let boards = boards_for(&template.id);
        let expected = boards.nodes.len();

        assert!(state.append_template_boards(boards), "{}", template.id);
        assert_eq!(
            state.active_children().len(),
            before + expected,
            "{} lost boards on the way in",
            template.id
        );
        assert_eq!(
            expected, template.frames as usize,
            "{} disagrees with its catalogue frame count",
            template.id
        );
    }
}

/// The starter frame is a document, not a placeholder to be swallowed: the
/// append path must leave whatever was on the page exactly where it was.
#[test]
fn existing_boards_are_untouched_and_the_template_lands_to_their_right() {
    let mut state = EditorState::starter();
    let existing = top_level_ids(&state);
    assert_eq!(existing.len(), 1);

    assert!(state.append_template_boards(boards_for("slide-deck")));

    let after = top_level_ids(&state);
    assert_eq!(after.len(), 7, "one starter frame plus six slides");
    assert_eq!(&after[..1], &existing[..], "the starter kept its identity");

    // The starter is 1200 wide at x=0, so every appended board starts past
    // its right edge plus the gap.
    let floor = 1200.0 + TEMPLATE_APPEND_GAP;
    for node in state.active_children().iter().skip(1) {
        assert!(
            node.base().x.unwrap_or(0.0) >= floor,
            "a board landed on top of the existing content"
        );
    }
}

/// The replace-vs-append decision, from both sides. This is the rule the web
/// host applies directly and the one the desktop branches on before taking
/// its longer starter road, so the outcomes are pinned here rather than in
/// either host.
#[test]
fn a_blank_starter_is_taken_over_and_real_work_is_added_to() {
    let mut fresh = EditorState::starter();
    assert!(fresh.adopt_template_boards(boards_for("slide-deck")));
    assert_eq!(
        fresh.active_children().len(),
        6,
        "the starter frame stepped aside for the deck"
    );

    let mut working = EditorState::starter();
    assert!(working.append_template_boards(boards_for("knowledge-card-vertical")));
    assert!(working.append_template_boards(boards_for("knowledge-card-square")));
    let existing = top_level_ids(&working);
    assert_eq!(existing.len(), 3, "starter plus two cards is the fixture");

    assert!(working.adopt_template_boards(boards_for("slide-deck")));
    let after = top_level_ids(&working);
    assert_eq!(after.len(), 9, "three boards kept, six added");
    assert_eq!(&after[..3], &existing[..], "the original boards are intact");
}

/// A one-board template is the case the empty-root swap fires on. Appending
/// it beside an empty frame must add a board, not trade one for the other.
#[test]
fn a_single_board_template_does_not_consume_an_empty_frame() {
    let mut state = EditorState::starter();
    let before = top_level_ids(&state);

    assert!(state.append_template_boards(boards_for("knowledge-card-vertical")));

    let after = top_level_ids(&state);
    assert_eq!(after.len(), 2);
    assert_eq!(&after[..1], &before[..]);
}

/// Boards keep their relative layout: a six-slide deck stays a row of six
/// evenly spaced slides after it moves.
#[test]
fn boards_move_as_a_block() {
    let source = scene_template_document("slide-deck").expect("ships");
    let authored = template_boards(source, "slide-deck").expect("parses");
    let spacings: Vec<f64> = authored
        .nodes
        .windows(2)
        .map(|pair| pair[1].base().x.unwrap_or(0.0) - pair[0].base().x.unwrap_or(0.0))
        .collect();

    let mut state = EditorState::starter();
    assert!(state.append_template_boards(boards_for("slide-deck")));

    let landed: Vec<f64> = state
        .active_children()
        .iter()
        .skip(1)
        .collect::<Vec<_>>()
        .windows(2)
        .map(|pair| pair[1].base().x.unwrap_or(0.0) - pair[0].base().x.unwrap_or(0.0))
        .collect();
    assert_eq!(spacings, landed);
}

/// Every `$ref` in an appended board resolves to a variable the document now
/// carries. A dangling reference paints as a missing fill, which is the
/// failure mode a naive "skip the variable table" merge produces.
#[test]
fn appended_boards_leave_no_dangling_variable_reference() {
    for template in scene_template_catalogue() {
        let mut state = EditorState::starter();
        assert!(state.append_template_boards(boards_for(&template.id)));

        let declared = state.doc.variables.clone().unwrap_or_default();
        let serialized =
            serde_json::to_string(state.active_children()).expect("the tree serializes");
        for reference in variable_references(&serialized) {
            assert!(
                declared.contains_key(&reference),
                "{} references ${reference}, which the document does not declare",
                template.id
            );
        }
    }
}

/// Two templates that both name `c-bg` must both keep their own colour.
#[test]
fn two_templates_keep_their_own_palettes() {
    let mut state = EditorState::starter();
    assert!(state.append_template_boards(boards_for("slide-deck")));
    assert!(state.append_template_boards(boards_for("pitch-deck-dark")));

    let variables = state.doc.variables.clone().unwrap_or_default();
    let light = variables
        .get("slide-deck--c-bg")
        .expect("the light deck kept its background");
    let dark = variables
        .get("pitch-deck-dark--c-bg")
        .expect("the dark deck kept its background");
    assert_ne!(light.value, dark.value);
}

/// Appending the same template twice is a no-op on the variable table: the
/// names are derived from the template id, so the second pass matches the
/// first instead of growing a suffix chain.
#[test]
fn appending_the_same_template_twice_does_not_multiply_variables() {
    let mut state = EditorState::starter();
    assert!(state.append_template_boards(boards_for("slide-deck")));
    let after_first = state.doc.variables.clone().unwrap_or_default().len();

    assert!(state.append_template_boards(boards_for("slide-deck")));
    assert_eq!(
        state.doc.variables.clone().unwrap_or_default().len(),
        after_first
    );
    assert_eq!(state.active_children().len(), 13, "boards still doubled");
}

/// One undo puts the document back, whatever the board count — the append is
/// a single transaction, not one entry per board.
#[test]
fn one_undo_removes_the_whole_template() {
    let mut state = EditorState::starter();
    let before = top_level_ids(&state);

    assert!(state.append_template_boards(boards_for("minimal-keynote")));
    assert_eq!(state.active_children().len(), 10);

    assert!(state.undo());
    assert_eq!(top_level_ids(&state), before);
}

/// The scene cache is keyed on the document revision, so an append that did
/// not bump it paints as nothing happening until some later edit forces a
/// rebuild.
#[test]
fn appending_bumps_the_document_revision() {
    let mut state = EditorState::starter();
    let before = state.revision;
    assert!(state.append_template_boards(boards_for("slide-deck")));
    assert_ne!(state.revision, before);
}

/// Max-munch on the identifier after `$`, and a `$` that names nothing is
/// left alone — body copy quoting a price must survive the rewrite.
#[test]
fn reference_rewriting_takes_the_longest_name_and_ignores_the_rest() {
    let renames = BTreeMap::from([
        ("c-accent".to_string(), "t--c-accent".to_string()),
        ("c-accent-soft".to_string(), "t--c-accent-soft".to_string()),
    ]);
    assert_eq!(
        rewrite_refs_in_text("$c-accent-soft", &renames).as_deref(),
        Some("$t--c-accent-soft")
    );
    assert_eq!(
        rewrite_refs_in_text("$c-accent", &renames).as_deref(),
        Some("$t--c-accent")
    );
    assert_eq!(rewrite_refs_in_text("售价 $99 起", &renames), None);
    assert_eq!(rewrite_refs_in_text("no dollar here", &renames), None);
}

/// Collect the `$name` tokens a serialized tree references.
///
/// A token has to start with a letter to count. `$` is also just a currency
/// sign, and a pricing table full of `$49` is not a document with three
/// dangling variables — the production rewriter already draws the same line
/// (`rewrite_refs_in_text("售价 $99 起", …)` returns `None`), so this keeps
/// the check and the behaviour it checks in agreement.
fn variable_references(serialized: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = serialized;
    while let Some(offset) = rest.find('$') {
        let after = &rest[offset + 1..];
        let end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(after.len());
        if after[..end].starts_with(|c: char| c.is_ascii_alphabetic()) {
            found.push(after[..end].to_string());
        }
        rest = &after[end..];
    }
    found
}

#[test]
fn adopting_a_catalogue_template_by_command_brings_its_boards_and_palette() {
    // The command exists because boards and palette must land together:
    // applying it has to leave both, or the frames resolve against a
    // palette that is not there.
    let template = crate::scene_template_catalog::scene_template_catalogue()
        .iter()
        .find(|template| template.style_guide.is_some())
        .expect("catalogue ships at least one template carrying a palette");

    let mut state = EditorState::new();
    let variables_before = state.doc.variables.as_ref().map_or(0, BTreeMap::len);
    let changed = state.apply(crate::EditorCommand::AdoptSceneTemplate {
        template_id: template.id.clone(),
    });

    assert!(changed, "adopting {} changed nothing", template.id);
    assert!(
        !state.active_children().is_empty(),
        "adopting {} left no boards",
        template.id
    );
    assert!(
        state.doc.variables.as_ref().map_or(0, BTreeMap::len) > variables_before,
        "adopting {} brought no palette",
        template.id
    );
}

#[test]
fn adopting_an_unknown_template_id_leaves_the_document_alone() {
    let mut state = EditorState::new();
    let changed = state.apply(crate::EditorCommand::AdoptSceneTemplate {
        template_id: "no-such-template".to_owned(),
    });
    assert!(!changed);
    assert!(state.active_children().is_empty());
}

/// A saved template is adopted through the same command and the same
/// `template_boards` path as a shipped one — the command only resolves the
/// document from a different half of the catalogue.
#[test]
fn adopting_a_saved_template_brings_its_boards() {
    let _guard = crate::user_scene_templates::exclusive_registry_for_tests();
    crate::user_scene_templates::load_user_scene_template(
        crate::user_scene_templates::UserSceneTemplate {
            id: "user:my-deck".to_string(),
            name: "my-deck".to_string(),
            frames: 1,
            frame_width: 100,
            frame_height: 100,
            document: r##"{"version":"1.0.0","children":[
            {"type":"frame","id":"f1","name":"封面","x":0,"y":0,"width":100,"height":100,
             "fill":[{"type":"solid","color":"#ff0000"}]}
        ]}"##
                .to_string(),
            preview_jpeg: Vec::new(),
        },
    )
    .expect("saves");

    let mut state = EditorState::new();
    let changed = state.apply(crate::EditorCommand::AdoptSceneTemplate {
        template_id: "user:my-deck".to_owned(),
    });
    assert!(changed, "adopting the saved template changed nothing");
    assert_eq!(state.active_children().len(), 1);
}

#[test]
fn raw_thumbnail_extraction_is_local_and_schema_bounded() {
    let max_bytes = jian_ops_schema::image_thumbs::MAX_THUMB_BYTES;
    let accepted = vec![0x51; max_bytes];
    let oversized = vec![0x52; max_bytes + 1];
    let mut document = serde_json::json!({
        "version": "1.0.0",
        "imageThumbs": {
            "41": base64::engine::general_purpose::STANDARD.encode(&accepted),
            "42": base64::engine::general_purpose::STANDARD.encode(&oversized),
            "43": "not base64!",
            "not-decimal": "/9j/2Q=="
        },
        "children": []
    });

    let extracted = take_raw_template_thumbnails(&mut document);

    assert!(
        document.get("imageThumbs").is_none(),
        "the compat loader must never attach a template-only pending seed"
    );
    assert_eq!(extracted.len(), 1);
    assert_eq!(
        extracted.get(&41).map(AsRef::as_ref),
        Some(accepted.as_slice())
    );
    assert!(!extracted.contains_key(&42), "decoded-size cap is enforced");
    assert!(!extracted.contains_key(&43), "invalid base64 is skipped");
}

/// Exercise the same streaming canonical writer used by Save As Template.
/// The saved wire shape has no root boards: the selected page owns the image,
/// whose large data URL and blur-up JPEG live in top-level side tables.
#[test]
fn canonical_saved_active_page_and_image_round_trip_through_parse_and_adopt() {
    let _registry_guard = crate::user_scene_templates::exclusive_registry_for_tests();
    let image_source = format!(
        "data:image/png;base64,scene-template-round-trip{}",
        "A".repeat(4_096)
    );
    let image_paint_id = jian_ops_schema::node::image_src::paint_image_id(image_source.as_str());
    let image_thumbnail = vec![0xff, 0xd8, 0x51, 0xd9];

    // The writer snapshots the live registry. Restore the pre-template state
    // immediately after writing so the parse/adopt assertions prove the
    // thumbnail came back from canonical `imageThumbs`, not this setup.
    let retained_source = "data:image/png;base64,scene-template-retained-thumb";
    let retained_paint_id = jian_ops_schema::node::image_src::paint_image_id(retained_source);
    let retained_thumbnail = vec![0xff, 0xd8, 0x52, 0xd9];
    let concurrent_source = "data:image/png;base64,scene-template-concurrent-thumb";
    let concurrent_paint_id = jian_ops_schema::node::image_src::paint_image_id(concurrent_source);
    let concurrent_thumbnail = vec![0xff, 0xd8, 0x53, 0xd9];
    jian_ops_schema::image_thumbs::store_thumb(retained_paint_id, retained_thumbnail.clone());
    let registry_without_template = jian_ops_schema::image_thumbs::capture_snapshot();
    jian_ops_schema::image_thumbs::store_thumb(image_paint_id, image_thumbnail.clone());

    let image: PenNode = serde_json::from_value(serde_json::json!({
        "type": "image",
        "id": "active-image",
        "name": "Active image",
        "x": 10,
        "y": 20,
        "width": 320,
        "height": 180,
        "src": image_source,
    }))
    .expect("image fixture");
    let mut document = EditorState::new().doc;
    document.pages = Some(vec![
        PenPage {
            id: "inactive-page".into(),
            name: "Inactive".into(),
            children: vec![rect("inactive-board", "Inactive", 0.0, 0.0, 100.0, 100.0)],
            background_color: None,
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "active-page".into(),
            name: "Active".into(),
            children: vec![image],
            background_color: None,
            state: None,
            lifecycle: None,
        },
    ]);
    document.children.clear();

    let mut saved = Vec::new();
    let stats = jian_ops_schema::image_table::write_document_with_extension(
        &mut saved,
        &document,
        &jian_ops_schema::image_thumbs::capture_snapshot(),
        "editorMeta",
        &serde_json::json!({
            "activePageIndex": 1,
            "preserveAuthoredGeometry": false,
        }),
    )
    .expect("canonical writer");
    assert!(stats.wrote_images_table);
    assert!(stats.wrote_image_thumbs);
    let saved = String::from_utf8(saved).expect("canonical UTF-8 JSON");
    let wire: serde_json::Value = serde_json::from_str(&saved).expect("canonical JSON");
    assert_eq!(wire["editorMeta"]["activePageIndex"], 1);
    assert!(
        wire["pages"][1]["children"][0]["src"]
            .as_str()
            .is_some_and(|src| src.starts_with("op-image:")),
        "the writer externalized the active page's image"
    );
    assert!(wire.get("images").is_some());
    assert!(wire
        .get("imageThumbs")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|table| table.contains_key(&image_paint_id.to_string())));

    jian_ops_schema::image_thumbs::restore_snapshot(registry_without_template);
    assert!(jian_ops_schema::image_thumbs::thumb_for(image_paint_id).is_none());

    // Model a thumbnail decode landing in the still-open destination while
    // the saved template is parsed. Parsing must never replace that active
    // registry, even transiently.
    let decode_start = std::sync::Arc::new(std::sync::Barrier::new(2));
    let worker_start = std::sync::Arc::clone(&decode_start);
    let worker_thumbnail = concurrent_thumbnail.clone();
    let decode_worker = std::thread::spawn(move || {
        worker_start.wait();
        jian_ops_schema::image_thumbs::store_thumb(concurrent_paint_id, worker_thumbnail);
    });
    decode_start.wait();
    let parsed = template_boards(&saved, "user:canonical-round-trip")
        .expect("canonical saved template parses");
    decode_worker.join().expect("thumbnail decode worker");
    assert_eq!(
        parsed.nodes.len(),
        1,
        "only the saved active page is adopted"
    );
    assert_eq!(parsed.nodes[0].id_str(), "active-image");
    let PenNode::Image(parsed_image) = &parsed.nodes[0] else {
        panic!("the active image survived as an image node");
    };
    assert_eq!(parsed_image.src.as_str(), image_source);
    assert_eq!(
        jian_ops_schema::image_thumbs::thumb_for(retained_paint_id).as_deref(),
        Some(retained_thumbnail.as_slice()),
        "parsing preserved the current document's thumbnail registry"
    );
    assert_eq!(
        jian_ops_schema::image_thumbs::thumb_for(concurrent_paint_id).as_deref(),
        Some(concurrent_thumbnail.as_slice()),
        "template extraction must not overwrite a decode landing in the active registry"
    );
    assert!(
        jian_ops_schema::image_thumbs::thumb_for(image_paint_id).is_none(),
        "parsing alone does not publish template thumbnails"
    );

    crate::user_scene_templates::load_user_scene_template(
        crate::user_scene_templates::UserSceneTemplate {
            id: "user:canonical-round-trip".into(),
            name: "Canonical round trip".into(),
            frames: 1,
            frame_width: 320,
            frame_height: 180,
            document: saved,
            preview_jpeg: Vec::new(),
        },
    )
    .expect("register canonical template");
    let mut state = EditorState::new();
    assert!(state.apply(crate::EditorCommand::AdoptSceneTemplate {
        template_id: "user:canonical-round-trip".into(),
    }));
    assert_eq!(state.active_children().len(), 1);
    let PenNode::Image(adopted_image) = &state.active_children()[0] else {
        panic!("adopted node is an image");
    };
    assert_eq!(adopted_image.src.as_str(), image_source);
    assert_eq!(
        jian_ops_schema::image_thumbs::thumb_for(image_paint_id).as_deref(),
        Some(image_thumbnail.as_slice()),
        "successful adopt merged the template thumbnail"
    );
    assert_eq!(
        jian_ops_schema::image_thumbs::thumb_for(retained_paint_id).as_deref(),
        Some(retained_thumbnail.as_slice()),
        "adopt preserved the current document's thumbnail"
    );
    assert_eq!(
        jian_ops_schema::image_thumbs::thumb_for(concurrent_paint_id).as_deref(),
        Some(concurrent_thumbnail.as_slice()),
        "adopt preserved the independently decoded thumbnail"
    );
    assert!(state.undo(), "adopt is one undoable transaction");
    assert!(state.active_children().is_empty());
}

/// A `user:` id the registry does not know leaves the document alone — the
/// same silent no-op as an unknown shipped id; the MCP tool layer validates
/// ids before issuing the command.
#[test]
fn adopting_an_unknown_user_template_id_leaves_the_document_alone() {
    let _guard = crate::user_scene_templates::exclusive_registry_for_tests();
    let mut state = EditorState::new();
    let changed = state.apply(crate::EditorCommand::AdoptSceneTemplate {
        template_id: "user:never-saved".to_owned(),
    });
    assert!(!changed);
    assert!(state.active_children().is_empty());
}

/// The two-source document resolver answers shipped ids from the embedded
/// asset and `user:` ids from the runtime registry.
#[test]
fn template_document_for_resolves_both_halves() {
    let _guard = crate::user_scene_templates::exclusive_registry_for_tests();
    let shipped = crate::scene_template_catalog::scene_template_catalogue()
        .first()
        .expect("catalogue ships templates");
    assert!(
        template_document_for(&shipped.id).is_some(),
        "shipped id resolves"
    );
    assert!(template_document_for("user:never-saved").is_none());
    assert!(template_document_for("no-such-template").is_none());
}
