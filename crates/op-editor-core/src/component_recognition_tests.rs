use super::*;
use serde_json::json;

fn card(id: &str, title: &str, body: &str) -> Value {
    json!({
        "type": "frame", "id": id, "name": "div", "width": 330.0,
        "height": "fit_content", "layout": "vertical", "gap": 8.0, "padding": 24.0,
        "cornerRadius": 12.0,
        "fill": [{"type": "solid", "color": "#ffffff"}],
        "stroke": {"thickness": 1.0, "fill": [{"type": "solid", "color": "#e7e5e4"}]},
        "children": [
            {"type": "text", "id": format!("{id}-t"), "name": "Text", "content": title,
             "fontSize": 20.0, "fill": [{"type": "solid", "color": "#1c1917"}]},
            {"type": "text", "id": format!("{id}-b"), "name": "Text", "content": body,
             "fontSize": 14.0, "fill": [{"type": "solid", "color": "#57534e"}]}
        ]
    })
}

fn page(children: Vec<Value>) -> Vec<PenNode> {
    vec![serde_json::from_value(json!({
        "type": "frame", "id": "page", "name": "Page", "width": 1440.0,
        "height": "fit_content", "layout": "vertical",
        "children": [{
            "type": "frame", "id": "grid", "name": "section", "layout": "horizontal",
            "gap": 24.0, "children": children
        }]
    }))
    .unwrap()]
}

fn find<'a>(roots: &'a [PenNode], id: &str) -> &'a PenNode {
    crate::walkers::find_node(roots, &crate::NodeId::new(id)).expect(id)
}

fn texts_after_render(roots: &[PenNode]) -> Vec<String> {
    let doc: PenDocument = serde_json::from_value(json!({
        "version": "1.0", "children": serde_json::to_value(roots).unwrap()
    }))
    .unwrap();
    let rendered = crate::ref_resolve::resolve_refs_for_canvas(&doc);
    let mut out = Vec::new();
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if let PenNode::Text(text) = node {
                out.push(serde_json::to_value(&text.content).unwrap().to_string());
            }
            if let Some(children) = node.children() {
                walk(children, out);
            }
        }
    }
    walk(&rendered.children, &mut out);
    out
}

#[test]
fn identical_cards_become_one_master_and_ref_instances() {
    let mut roots = page(vec![
        card("c1", "Fast", "Ship in days."),
        card("c2", "Safe", "Audited code."),
        card("c3", "Open", "MIT licensed."),
    ]);
    let before = texts_after_render(&roots);
    let created = componentize_repeated_structures(&mut roots);
    assert_eq!(created.len(), 1, "{created:?}");
    let component = &created[0];
    assert_eq!(component.master_id, "c1");
    assert_eq!(component.kind, ComponentKind::Card);
    assert_eq!(component.name, "Card");
    assert_eq!(component.instance_ids, vec!["c2", "c3"]);

    let PenNode::Frame(master) = find(&roots, "c1") else {
        panic!("master stays a frame");
    };
    assert_eq!(master.reusable, Some(true));
    let PenNode::Ref(instance) = find(&roots, "c2") else {
        panic!("copy became a ref");
    };
    assert_eq!(instance.target, "c1");
    let overrides = instance.descendants.as_ref().expect("content overrides");
    assert_eq!(overrides["c1-t"]["content"], "Safe");
    assert_eq!(overrides["c1-b"]["content"], "Audited code.");
    // Nothing the page showed is lost: the rendered text is unchanged.
    assert_eq!(texts_after_render(&roots), before);
}

#[test]
fn a_style_difference_keeps_the_copies_apart() {
    let mut odd = card("c2", "Safe", "Audited code.");
    odd["cornerRadius"] = json!(4.0);
    let mut roots = page(vec![card("c1", "Fast", "Ship in days."), odd]);
    let before = serde_json::to_value(&roots).unwrap();
    assert!(componentize_repeated_structures(&mut roots).is_empty());
    assert_eq!(serde_json::to_value(&roots).unwrap(), before);
}

#[test]
fn a_structure_difference_keeps_the_copies_apart() {
    let mut extra = card("c2", "Safe", "Audited code.");
    extra["children"].as_array_mut().unwrap().push(json!({
        "type": "text", "id": "c2-x", "name": "Text", "content": "More",
        "fontSize": 14.0
    }));
    let mut roots = page(vec![card("c1", "Fast", "Ship in days."), extra]);
    assert!(componentize_repeated_structures(&mut roots).is_empty());
}

#[test]
fn a_content_field_present_on_only_one_copy_is_not_overridable() {
    let mut roots = page(vec![
        json!({"type": "frame", "id": "a", "layout": "vertical",
               "fill": [{"type": "solid", "color": "#eeeeee"}],
               "children": [{"type": "text", "id": "a-t", "name": "Title", "content": "A"}]}),
        json!({"type": "frame", "id": "b", "layout": "vertical",
               "fill": [{"type": "solid", "color": "#eeeeee"}],
               "children": [{"type": "text", "id": "b-t", "content": "B"}]}),
    ]);
    assert!(componentize_repeated_structures(&mut roots).is_empty());
}

#[test]
fn image_sources_become_overrides() {
    let tile = |id: &str, src: &str| {
        json!({"type": "frame", "id": id, "layout": "vertical",
        "cornerRadius": 8.0,
        "children": [
            {"type": "image", "id": format!("{id}-i"), "src": src,
             "width": 120.0, "height": 80.0},
            {"type": "text", "id": format!("{id}-t"), "content": id}
        ]})
    };
    let mut roots = page(vec![tile("t1", "one.png"), tile("t2", "two.png")]);
    let created = componentize_repeated_structures(&mut roots);
    assert_eq!(created.len(), 1);
    let PenNode::Ref(instance) = find(&roots, "t2") else {
        panic!("ref");
    };
    let overrides = instance.descendants.as_ref().unwrap();
    assert_eq!(overrides["t1-i"]["src"], "two.png");
    assert_eq!(overrides["t1-t"]["content"], "t2");
}

#[test]
fn buttons_anywhere_on_the_page_share_one_component() {
    let button = |id: &str, label: &str| {
        json!({"type": "frame", "id": id, "name": "a", "layout": "vertical",
               "padding": [12.0, 20.0, 12.0, 20.0], "cornerRadius": 10.0,
               "fill": [{"type": "solid", "color": "#0e7c66"}],
               "children": [{"type": "text", "id": format!("{id}-t"), "name": "Text",
                             "content": label,
                             "fill": [{"type": "solid", "color": "#ffffff"}]}]})
    };
    let mut roots = vec![serde_json::from_value(json!({
        // The two wrappers are identical too, but a bare one-child wrapper
        // is plumbing: the buttons are the part.
        "type": "frame", "id": "page", "layout": "vertical", "children": [
            {"type": "frame", "id": "hero", "layout": "vertical",
             "children": [button("b1", "Get started")]},
            {"type": "frame", "id": "footer", "layout": "vertical",
             "children": [button("b2", "Contact sales")]}
        ]
    }))
    .unwrap()];
    let created = componentize_repeated_structures(&mut roots);
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].kind, ComponentKind::Button);
    assert_eq!(created[0].name, "Button");
    assert!(matches!(find(&roots, "b2"), PenNode::Ref(_)));
}

#[test]
fn larger_structures_claim_their_parts_first() {
    // Each card holds a button; the cards group first, so the buttons
    // inside them are not turned into a second, nested master.
    let with_button = |id: &str, title: &str| {
        let mut value = card(id, title, "Body");
        value["children"].as_array_mut().unwrap().push(json!({
            "type": "frame", "id": format!("{id}-btn"), "layout": "vertical",
            "cornerRadius": 6.0, "fill": [{"type": "solid", "color": "#111111"}],
            "children": [{"type": "text", "id": format!("{id}-btn-t"), "content": "Go"}]
        }));
        value
    };
    let mut roots = page(vec![with_button("c1", "One"), with_button("c2", "Two")]);
    let created = componentize_repeated_structures(&mut roots);
    assert_eq!(created.len(), 1, "{created:?}");
    assert_eq!(created[0].master_id, "c1");
}

#[test]
fn page_roots_refs_and_contentless_boxes_are_never_candidates() {
    let spacer = |id: &str| {
        json!({"type": "frame", "id": id, "layout": "vertical", "children": [
            {"type": "rectangle", "id": format!("{id}-r"), "width": 10.0, "height": 10.0}
        ]})
    };
    let mut roots = page(vec![spacer("s1"), spacer("s2")]);
    assert!(componentize_repeated_structures(&mut roots).is_empty());

    // Two identical page roots are boards, not parts.
    let board = |id: &str| -> PenNode {
        serde_json::from_value(json!({"type": "frame", "id": id, "children": [
            {"type": "text", "id": format!("{id}-t"), "content": "Hi"}
        ]}))
        .unwrap()
    };
    let mut boards = vec![board("p1"), board("p2")];
    assert!(componentize_repeated_structures(&mut boards).is_empty());
}

#[test]
fn second_component_of_a_kind_gets_a_numbered_name() {
    let pill = |id: &str, color: &str, label: &str| {
        json!({"type": "frame", "id": id, "layout": "vertical", "cornerRadius": 99.0,
               "fill": [{"type": "solid", "color": color}],
               "children": [{"type": "text", "id": format!("{id}-t"), "content": label}]})
    };
    let mut roots = page(vec![
        pill("a1", "#ff0000", "A"),
        pill("a2", "#ff0000", "B"),
        pill("b1", "#00ff00", "C"),
        pill("b2", "#00ff00", "D"),
    ]);
    let mut names: Vec<String> = componentize_repeated_structures(&mut roots)
        .into_iter()
        .map(|c| c.name)
        .collect();
    names.sort();
    assert_eq!(names, vec!["Button", "Button 2"]);
}
