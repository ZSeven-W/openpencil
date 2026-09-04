//! Image/G() placement tests for the multi-op batch-design executor.

use super::*;

#[test]
fn image_op_accepts_binding_or_quoted_parent_syntax() {
    // Best-effort policy: the deliberately-bad third line must be
    // dropped (not roll back the batch) so the G() parse + binding
    // resolution of the good lines stays observable.
    let mut state = sample();
    let program = r##"wrap=I(null, {"type":"frame","name":"Wrap","x":500,"y":0,"width":400,"height":300})
img=G(wrap, "search", "sunset photo")
G(wrap, "search")"##;
    let (envelope, cmd) = call_operations_best_effort(&state, program);
    let errors = envelope["errors"].as_array().expect("errors");
    assert_eq!(errors.len(), 1, "{envelope}");
    assert!(
        errors[0]["error"]
            .as_str()
            .unwrap()
            .starts_with("Invalid G() syntax:"),
        "{envelope}"
    );
    let wrap_id = binding_id(&envelope, "wrap");
    let img_id = binding_id(&envelope, "img");

    assert!(state.apply(cmd.expect("command")));
    let wrap = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&wrap_id))
        .expect("wrap");
    let img = wrap
        .children()
        .expect("children")
        .iter()
        .find(|c| c.id_str() == img_id)
        .expect("image under wrap");
    let jian_ops_schema::node::PenNode::Image(image) = img else {
        panic!("expected image")
    };
    assert_eq!(img.base().name.as_deref(), Some("sunset photo"));
    assert_eq!(image.image_search_query.as_deref(), Some("sunset photo"));
    assert_eq!(image.image_prompt, None);
}

#[test]
fn image_op_preserves_generate_mode_on_the_node() {
    let mut state = sample();
    let program = r##"slot=I(null, {"type":"frame","name":"Hero","x":500,"y":0,"width":160,"height":90,"layout":"none"})
img=G(slot, "generate", "cinematic sunset coast")"##;
    let (envelope, cmd) = call_operations(&state, program);
    let image_id = binding_id(&envelope, "img");

    assert!(state.apply(cmd.expect("command")));
    let image = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(image_id))
        .expect("image");
    let jian_ops_schema::node::PenNode::Image(image) = image else {
        panic!("expected image")
    };
    assert_eq!(
        image.image_prompt.as_deref(),
        Some("cinematic sunset coast")
    );
    assert_eq!(image.image_search_query, None);
}

#[test]
fn image_op_rejects_a_populated_flow_row_and_accepts_its_explicit_slot() {
    // Exact live-QA failure shape: the row already owns a cover slot, text
    // column, and play control. G(row, ...) used to append the image as a
    // fourth sibling, leaving the cover slot empty. The contract gate uses
    // only explicit layout/parent structure; it does not infer intent from
    // these fixture names or their 56px dimensions.
    let row: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame", "id": "n42", "name": "Daily Mix 1",
        "layout": "horizontal", "width": "fill_container", "height": "fit_content",
        "children": [
            {"type": "frame", "id": "n43", "name": "Daily Mix 1 Cover", "layout": "none", "width": 56, "height": 56},
            {"type": "frame", "id": "n44", "name": "Daily Mix 1 Text", "layout": "vertical", "width": "fill_container", "height": "fit_content"},
            {"type": "icon_font", "id": "n47", "iconFontFamily": "lucide", "iconFontName": "play", "width": 20, "height": 20}
        ]
    }))
    .unwrap();
    let mut state = state_with(vec![row]);

    let (rejected, command) = call_operations(
        &state,
        r##"img=G("n42", "search", "green forest ambient")"##,
    );
    assert!(
        command.is_none(),
        "the bad parent must not mutate: {rejected}"
    );
    assert_eq!(rejected["applied"], Value::Bool(false));
    let message = rejected["errors"][0]["error"]
        .as_str()
        .expect("G target error");
    assert!(
        message.contains("slot target n42 must be empty"),
        "{message}"
    );
    assert!(message.contains("[n43, n44, n47]"), "{message}");
    assert!(message.contains("exact empty"), "{message}");

    let (accepted, command) = call_operations(
        &state,
        r##"img=G("n43", "search", "green forest ambient")"##,
    );
    let image_id = binding_id(&accepted, "img");
    assert!(state.apply(command.expect("slot-targeted G command")));
    let slot = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n43"))
        .expect("explicit slot");
    assert!(
        slot.children()
            .is_some_and(|children| children.iter().any(|child| child.id_str() == image_id)),
        "the accepted image must land inside the explicit slot"
    );
    let row = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n42"))
        .expect("row");
    assert_eq!(
        row.children().map(Vec::len),
        Some(3),
        "no fourth row sibling"
    );

    let (duplicate, command) =
        call_operations(&state, r##"duplicate=G("n43", "search", "second cover")"##);
    assert!(command.is_none(), "populated slot must reject: {duplicate}");
    let message = duplicate["errors"][0]["error"]
        .as_str()
        .expect("populated slot error");
    assert!(
        message.contains("slot target n43 must be empty"),
        "{message}"
    );
    assert!(message.contains(&image_id), "{message}");

    let (overlay, command) = call_operations(
        &state,
        r##"overlay=G("n43", "search", "overlay", "append")"##,
    );
    assert!(
        command.is_none(),
        "layout-none append must reject: {overlay}"
    );
    let message = overlay["errors"][0]["error"]
        .as_str()
        .expect("append layout error");
    assert!(message.contains("must declare layout"), "{message}");
    assert!(message.contains("got none"), "{message}");
    assert!(message.contains("never an absolute overlay"), "{message}");

    let (unsized_result, command) = call_operations(
        &state,
        r##"img=G("n42", "search", "unsized gallery artwork", "append")"##,
    );
    assert!(
        command.is_none(),
        "unsized append must reject: {unsized_result}"
    );
    assert_eq!(
        unsized_result["applied"],
        Value::Bool(false),
        "{unsized_result}"
    );
    let message = unsized_result["errors"][0]["error"]
        .as_str()
        .expect("append sizing error");
    assert!(
        message.contains("followed in the same batch by U"),
        "{message}"
    );
    assert!(message.contains("positive number"), "{message}");
    assert!(message.contains("fill_container"), "{message}");

    let (appended, command) = call_operations(
        &state,
        r##"img=G("n42", "search", "new gallery artwork", "append")
U(img, {"width":72,"height":72})"##,
    );
    let appended_id = binding_id(&appended, "img");
    assert!(state.apply(command.expect("explicit append command")));
    let row = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new("n42"))
        .expect("row");
    let appended = row
        .children()
        .expect("row children")
        .iter()
        .find(|child| child.id_str() == appended_id)
        .expect("explicit image sibling");
    assert_eq!(appended.width_px(), Some(72.0));
    assert_eq!(appended.height_px(), Some(72.0));
    assert_eq!(row.children().map(Vec::len), Some(4));
}

#[test]
fn bindless_image_with_equals_in_prompt_cannot_bypass_populated_flow_slot_gate() {
    let row: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame", "id": "n42", "name": "Event Rail",
        "layout": "horizontal", "width": "fill_container", "height": "fit_content",
        "children": [
            {"type": "frame", "id": "n43", "name": "Cover Slot", "layout": "none", "width": 168, "height": 112},
            {"type": "frame", "id": "n44", "name": "Event Details", "layout": "vertical", "width": "fill_container", "height": "fit_content"}
        ]
    }))
    .unwrap();
    let state = state_with(vec![row]);

    // The '=' is data inside the quoted prompt, not a result-binding
    // delimiter. This bindless call must still route through the snapshot
    // program validator instead of the geometry-blind direct parser.
    let (rejected, command) = call_operations(
        &state,
        r##"G("n42", "search", "festival crowd ratio=16:9")"##,
    );

    assert!(command.is_none(), "populated row must reject: {rejected}");
    assert_eq!(rejected["applied"], Value::Bool(false), "{rejected}");
    let message = rejected["errors"][0]["error"]
        .as_str()
        .expect("strict G slot error");
    assert!(
        message.contains("slot target n42 must be empty"),
        "{message}"
    );
    assert!(message.contains("[n43, n44]"), "{message}");
}

#[test]
fn image_op_inserts_under_live_target_resolved_from_slash_path_and_authored_alias() {
    let mut state = op_editor_core::EditorState::new();
    let program = r##"root=I(null, {"type":"frame","id":"auth-root","name":"Root","width":390,"height":844,"layout":"vertical","children":[{"type":"frame","id":"auth-slot","name":"Hero Slot","width":160,"height":90,"layout":"none"}]})
img=G(root+"/auth-slot", "search", "sunset coast ratio=16:9")"##;

    let (envelope, command) = call_operations(&state, program);
    assert!(envelope.get("errors").is_none(), "{envelope}");
    let root_id = binding_id(&envelope, "root");
    let image_id = binding_id(&envelope, "img");
    assert!(state.apply(command.expect("slash-path G command")));

    let root = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(&root_id))
        .expect("root");
    let slot = root
        .children()
        .expect("root children")
        .iter()
        .find(|child| child.base().name.as_deref() == Some("Hero Slot"))
        .expect("authored slot remapped to a live child");
    assert_ne!(slot.id_str(), "auth-slot", "authored id must be remapped");
    assert!(
        slot.children()
            .is_some_and(|children| children.iter().any(|child| child.id_str() == image_id)),
        "image must be inserted under the resolved live slot id"
    );
}

#[test]
fn image_op_rejects_populated_layout_omitted_slot_and_omitted_append_parent() {
    let populated = sample();
    let (slot_result, command) =
        call_operations(&populated, r##"img=G("n10", "search", "product photo")"##);
    assert!(
        command.is_none(),
        "populated slot must reject: {slot_result}"
    );
    let message = slot_result["errors"][0]["error"]
        .as_str()
        .expect("slot error");
    assert!(
        message.contains("slot target n10 must be empty"),
        "{message}"
    );
    assert!(message.contains("[n11, n12]"), "{message}");

    let empty_omitted = state_with(vec![frame("slot", "Slot", 0.0, 0.0, 100.0, 80.0, vec![])]);
    let (append_result, command) = call_operations(
        &empty_omitted,
        r##"img=G("slot", "search", "product photo", "append")"##,
    );
    assert!(
        command.is_none(),
        "omitted-layout append must reject: {append_result}"
    );
    let message = append_result["errors"][0]["error"]
        .as_str()
        .expect("append error");
    assert!(message.contains("must declare layout"), "{message}");
    assert!(message.contains("got omitted"), "{message}");
}

#[test]
fn image_op_rejects_a_missing_target_for_slot_and_append() {
    let state = sample();
    for operation in [
        r##"img=G(null, "search", "photo")"##,
        r##"img=G(null, "search", "photo", "append")"##,
    ] {
        let (envelope, command) = call_operations(&state, operation);
        assert!(command.is_none(), "null target must reject: {envelope}");
        assert!(envelope["errors"][0]["error"]
            .as_str()
            .is_some_and(|message| message.contains("explicit frame/rectangle target id")));
    }
}

#[test]
fn image_op_rejects_an_unknown_placement() {
    let state = sample();
    let (envelope, command) = call_operations(
        &state,
        r##"img=G("n10", "search", "product photo", "guess")"##,
    );
    assert!(command.is_none(), "transaction rolls back: {envelope}");
    assert!(envelope["errors"][0]["error"]
        .as_str()
        .is_some_and(|message| message.contains("placement must be")));
}

#[test]
fn image_op_fills_an_absolute_slot_without_importing_400x300_geometry() {
    let mut state = sample();
    let program = r##"slot=I(null, {"type":"frame","name":"Hero Image","x":500,"y":0,"width":160,"height":90,"layout":"none","clipContent":true})
img=G("slot", "search", "sunset coast")"##;
    let (envelope, cmd) = call_operations(&state, program);
    let slot_id = binding_id(&envelope, "slot");
    let img_id = binding_id(&envelope, "img");

    assert!(state.apply(cmd.expect("command")));
    let slot = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(slot_id))
        .expect("slot");
    let image = slot
        .children()
        .expect("children")
        .iter()
        .find(|node| node.id_str() == img_id)
        .expect("image");
    assert_eq!(image.width_px(), Some(160.0));
    assert_eq!(image.height_px(), Some(90.0));
    assert_eq!(image.base().x, Some(0.0));
    assert_eq!(image.base().y, Some(0.0));
}

#[test]
fn image_op_uses_resolved_width_for_fill_sized_absolute_slot() {
    let mut state = sample();
    let program = r##"root=I(null, {"type":"frame","name":"Travel","x":500,"y":0,"width":390,"height":844,"layout":"vertical"})
slot=I(root, {"type":"frame","name":"Hero Image","width":"fill_container","height":140,"layout":"none","clipContent":true})
img=G(slot, "search", "sunset coast")"##;
    let (envelope, cmd) = call_operations(&state, program);
    let slot_id = binding_id(&envelope, "slot");
    let img_id = binding_id(&envelope, "img");

    assert!(state.apply(cmd.expect("command")));
    let slot = op_editor_core::walkers::find_node(state.active_children(), &NodeId::new(slot_id))
        .expect("slot");
    let image = slot
        .children()
        .expect("children")
        .iter()
        .find(|node| node.id_str() == img_id)
        .expect("image");
    assert_eq!(image.width_px(), Some(390.0));
    assert_eq!(image.height_px(), Some(140.0));
    assert_eq!(image.base().x, Some(0.0));
    assert_eq!(image.base().y, Some(0.0));
}

#[test]
fn image_op_rejects_an_unsized_absolute_slot() {
    let state = sample();
    let program = r##"slot=I(null, {"type":"frame","name":"Hero Image","layout":"none"})
img=G("slot", "search", "sunset coast")"##;
    let (envelope, cmd) = call_operations(&state, program);

    assert!(cmd.is_none(), "transaction rolls back: {envelope}");
    assert!(envelope["errors"].as_array().is_some_and(|errors| errors
        .iter()
        .any(|error| error["error"]
            .as_str()
            .is_some_and(|message| message.contains("declared width and height")))));
}

/// A batch that frees an id with `D()` must not hand that id to a later
/// insert: the caller still holds a binding for the deleted node, and
/// reissuing its id makes that binding silently resolve to the replacement.
#[test]
fn an_id_freed_in_this_batch_is_never_reissued() {
    let state = sample();
    let program = r##"card=I(null, {"type":"frame","name":"Card","x":900,"y":0,"width":400,"height":300,"layout":"none"})
badge=I(card, {"type":"frame","name":"Badge","width":52,"height":28,"x":10,"y":10})
D(badge)
img=G(card, "search", "desert dunes epic film poster")"##;
    let (envelope, _cmd) = call_operations(&state, program);
    let badge = binding_id(&envelope, "badge");
    let img = binding_id(&envelope, "img");
    assert_ne!(
        badge, img,
        "the deleted badge's id was reissued to the image: {envelope}"
    );
}

/// The rolled-back hint is the only line some hosts render, so it has to
/// name the first offending line and its reason.
#[test]
fn rollback_hint_names_the_first_failing_line_and_reason() {
    let state = sample();
    let program = r##"card=I(null, {"type":"frame","name":"Card","x":900,"y":0,"width":400,"height":300,"layout":"none"})
badge=I(card, {"type":"frame","name":"Badge","width":52,"height":28,"x":10,"y":10})
img=G(card, "search", "desert dunes epic film poster")"##;
    let (rolled_back, _) = call_operations(&state, program);
    let hint = rolled_back["hint"].as_str().expect("hint");
    assert!(hint.contains("First failure"), "{hint}");
    assert!(
        hint.contains("img=G(card"),
        "hint must name the line: {hint}"
    );
    assert!(
        hint.contains("must be empty"),
        "hint must carry the reason: {hint}"
    );
}
