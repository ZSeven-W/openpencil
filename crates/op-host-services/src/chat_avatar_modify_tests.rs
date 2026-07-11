use crate::chat_canvas_tools::apply_design_modification;
use op_editor_core::{walkers::find_node, EditorState, NodeId, PenNodeExt};

#[test]
fn apply_modification_replaces_avatar_image_fill_without_adding_border() {
    let mut state = EditorState::new();
    state.active_children_mut().clear();
    state.active_children_mut().push(
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": "page-1",
            "name": "Health Screen",
            "width": 390.0,
            "height": 844.0,
            "children": [{
                "type": "frame",
                "id": "n156",
                "name": "Avatar",
                "role": "avatar",
                "width": 44.0,
                "height": 44.0,
                "layout": "horizontal",
                "justifyContent": "center",
                "alignItems": "center",
                "clipContent": true,
                "cornerRadius": 22.0,
                "fill": [{"type": "solid", "color": "$color-surface"}],
                "children": [{
                    "type": "text",
                    "id": "n157",
                    "name": "Avatar Initials",
                    "content": "A",
                    "fontSize": 22.0
                }]
            }]
        }))
        .expect("valid page with avatar"),
    );
    let nodes = vec![(
        "null".to_string(),
        serde_json::json!({
            "id": "n156",
            "type": "frame",
            "name": "Avatar",
            "role": "avatar",
            "width": 44.0,
            "height": 44.0,
            "layout": "horizontal",
            "justifyContent": "center",
            "alignItems": "center",
            "clipContent": true,
            "cornerRadius": 22.0,
            "fill": {"type": "solid", "color": "$color-surface"},
            "stroke": {
                "thickness": 1,
                "fill": {"type": "solid", "color": "$color-border"}
            },
            "children": [{
                "type": "rectangle",
                "name": "Avatar Image",
                "width": "fill_container",
                "height": "fill_container",
                "cornerRadius": 22.0,
                "fill": {
                    "type": "image",
                    "imageSearchQuery": "professional headshot portrait person face",
                    "fit": "cover"
                }
            }]
        }),
    )];

    let (count, mutated) = apply_design_modification(&mut state, &nodes);

    assert_eq!(count, 1);
    assert!(mutated);
    let avatar = find_node(state.active_children(), &NodeId::new("n156"))
        .expect("avatar remains at the same id");
    let avatar_json = serde_json::to_value(avatar).expect("avatar serializes");
    assert!(avatar_json.get("stroke").is_none());
    let kids = avatar.children().expect("avatar children");
    assert_eq!(kids.len(), 1);
    assert_eq!(kids[0].base().name.as_deref(), Some("Avatar Image"));
    let image_json = serde_json::to_value(&kids[0]).expect("image child serializes");
    assert_eq!(image_json["fill"][0]["type"], serde_json::json!("image"));
    assert_eq!(image_json["fill"][0]["url"], serde_json::json!(""));
    assert!(!serde_json::to_string(&state.doc)
        .unwrap()
        .contains("Avatar Initials"));
}
