use crate::{EditorState, NodeId, Viewport};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;

#[test]
fn sized_image_preserves_aspect_ratio_centers_selects_and_undoes() {
    let mut state = EditorState::sample();
    state.viewport = Viewport {
        pan_x: -200.0,
        pan_y: -100.0,
        zoom: 2.0,
    };

    let id = state
        .insert_image_node_at_viewport_sized(
            "Clipboard image",
            "data:image/png;base64,AAAA",
            400,
            200,
        )
        .expect("image inserted");

    assert_eq!(id, NodeId::new("n15"));
    assert_eq!(state.selection.anchor, id);
    assert_eq!(state.active_children().len(), 2);
    let PenNode::Image(image) = &state.active_children()[0] else {
        panic!("inserted node should be an Image");
    };
    assert_eq!(image.base.name.as_deref(), Some("Clipboard image"));
    assert_eq!(image.base.x, Some(-50.0));
    assert_eq!(image.base.y, Some(-25.0));
    assert_eq!(image.width, Some(SizingBehavior::Number(300.0)));
    assert_eq!(image.height, Some(SizingBehavior::Number(150.0)));
    assert_eq!(image.src, "data:image/png;base64,AAAA");

    assert!(state.undo());
    assert_eq!(state.active_children().len(), 1);
    assert_eq!(state.selection.anchor, NodeId::new("n11"));
}

#[test]
fn sized_image_rejects_zero_pixel_dimensions_without_history() {
    for (pixel_width, pixel_height) in [(0, 100), (100, 0), (0, 0)] {
        let mut state = EditorState::sample();

        assert!(state
            .insert_image_node_at_viewport_sized(
                "Invalid image",
                "data:image/png;base64,AAAA",
                pixel_width,
                pixel_height,
            )
            .is_none());
        assert_eq!(state.active_children().len(), 1);
        assert_eq!(state.selection.anchor, NodeId::new("n11"));
        assert!(!state.history.can_undo());
    }
}

#[test]
fn sized_image_keeps_small_bitmap_at_original_size_and_centers_it() {
    let mut state = EditorState::new();
    state.viewport = Viewport {
        pan_x: -200.0,
        pan_y: -100.0,
        zoom: 2.0,
    };

    let id = state
        .insert_image_node_at_viewport_sized(
            "Small clipboard image",
            "data:image/png;base64,AAAA",
            64,
            32,
        )
        .expect("small image inserted");

    let PenNode::Image(image) = state.selected_node().expect("inserted image selected") else {
        panic!("inserted node should be an Image");
    };
    assert_eq!(state.selection.anchor, id);
    assert_eq!(image.width, Some(SizingBehavior::Number(64.0)));
    assert_eq!(image.height, Some(SizingBehavior::Number(32.0)));
    assert_eq!(image.base.x, Some(68.0));
    assert_eq!(image.base.y, Some(34.0));
}
