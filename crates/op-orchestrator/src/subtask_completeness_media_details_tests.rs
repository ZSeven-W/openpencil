//! The same merchant remains one item when its metadata is arranged differently.

use super::*;

const RETAINED: [&str; 2] = [
    include_str!("../tests/fixtures/subtask_completeness/merchant-list-1019-attempt2.js"),
    include_str!("../tests/fixtures/subtask_completeness/merchant-list-1019-attempt3.js"),
];

fn business_card_ids(nodes: &[PenNode], out: &mut Vec<String>) {
    fn content(node: &PenNode) -> (bool, usize) {
        node.children().into_iter().flatten().fold(
            (
                matches!(node, PenNode::Text(_)),
                usize::from(matches!(node, PenNode::Image(_))),
            ),
            |(text, image), child| {
                let (t, i) = content(child);
                (text || t, image + i)
            },
        )
    }
    for node in nodes {
        let children = node.children().map(Vec::as_slice).unwrap_or(&[]);
        if children.iter().any(|child| content(child).1 == 1)
            && children.iter().any(|child| content(child) == (true, 0))
        {
            out.push(node.id_str().to_owned());
        } else {
            business_card_ids(children, out);
        }
    }
}

#[test]
fn retained_merchant_metadata_variants_are_five_real_siblings() {
    let task: Subtask = serde_json::from_str(include_str!(
        "../tests/fixtures/subtask_completeness/merchant-list-1019-task.json"
    ))
    .unwrap();
    assert_eq!(expected_item_count(&task), Some(5));
    assert!(media_items::has_local_media_card_contract(
        &local_item_text(&task),
        5
    ));
    for script in RETAINED {
        let (sink, outcome) = inserted(script);
        let mut cards = Vec::new();
        business_card_ids(sink.state.active_children(), &mut cards);
        assert_eq!(cards.len(), 5);
        assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 4);
        assert_eq!(
            media_items::delivered_media_card_count(&sink, &outcome.inserted_root_ids),
            Some(5)
        );
        assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
        // Removing any actual merchant must still fail the five-item promise.
        for card in cards {
            let mut missing = VecDocSink::new();
            missing.state = EditorState::from_document(sink.state.doc.clone());
            assert!(missing.apply(EditorCommand::DeleteNode {
                node_id: NodeId::new(&card),
                page_id: None,
            }));
            let failure = incomplete_attempt(&missing, &task, &outcome).unwrap();
            assert_eq!((failure.expected, failure.delivered), (5, 4));
        }
    }
}

#[test]
fn retained_six_dishes_with_one_signature_badge_are_not_five() {
    let script = include_str!("../tests/fixtures/subtask_completeness/dish-list-1019-attempt1.js");
    let task: Subtask = serde_json::from_str(include_str!(
        "../tests/fixtures/subtask_completeness/dish-list-1019-task.json"
    ))
    .unwrap();
    let (sink, outcome) = inserted(script);
    assert_eq!(expected_item_count(&task), Some(6));
    assert!(media_items::has_local_media_card_contract(
        &local_item_text(&task),
        6
    ));
    assert_eq!(delivered_item_count(&sink, &outcome.inserted_root_ids), 5);
    assert_eq!(
        media_items::delivered_media_card_count(&sink, &outcome.inserted_root_ids),
        Some(6)
    );
    assert!(incomplete_attempt(&sink, &task, &outcome).is_none());
    let mut cards = Vec::new();
    business_card_ids(sink.state.active_children(), &mut cards);
    assert_eq!(cards.len(), 6);
    for card in cards {
        let mut missing = VecDocSink::new();
        missing.state = EditorState::from_document(sink.state.doc.clone());
        assert!(missing.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(&card),
            page_id: None
        }));
        let failure = incomplete_attempt(&missing, &task, &outcome).unwrap();
        assert_eq!((failure.expected, failure.delivered), (6, 5));
    }
}
