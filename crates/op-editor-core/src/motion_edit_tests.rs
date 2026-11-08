use super::*;

#[test]
fn errors_name_the_node_and_the_rule() {
    let node_id = NodeId::new("card");
    assert_eq!(
        MotionEditError::AnimationMissing {
            node_id: node_id.clone(),
            index: 3
        }
        .to_string(),
        "node card has no animation 3"
    );
    assert_eq!(
        MotionEditError::Invalid {
            node_id,
            path: "animations[0].keyframes".into(),
            reason: "first keyframe offset must be 0".into(),
        }
        .to_string(),
        "node card: animations[0].keyframes: first keyframe offset must be 0"
    );
}
