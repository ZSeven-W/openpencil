//! Shared semantics for persistent mobile actions, not ordinary trailing cards.
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;

pub(crate) fn is_action_bar(node: &PenNode) -> bool {
    let identity = [
        node.id_str(),
        node.base().name.as_deref().unwrap_or(""),
        node.base().role.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase();
    let action_bar = matches!(
        node.base().role.as_deref(),
        Some("bottom-action-bar" | "cart-bar" | "checkout-bar" | "submit-bar")
    ) || [
        "cart-bar",
        "cartbar",
        "cart bar",
        "checkout-bar",
        "bottom action",
        "bottom-action",
        "submit-bar",
        "底部购物车",
        "购物车栏",
        "底部操作栏",
        "底部提交",
    ]
    .iter()
    .any(|name| identity.contains(name));
    node.is_container() && (action_bar || explicitly_pinned_action(&identity)) && has_action(node)
}

pub(crate) fn explicitly_pinned_action(identity: &str) -> bool {
    let words: Vec<_> = identity.split(|c: char| !c.is_alphanumeric()).collect();
    let pinned = words.contains(&"pinned")
        || [
            "fixed bottom",
            "fixed-bottom",
            "bottom-fixed",
            "底部固定",
            "固定底部",
        ]
        .iter()
        .any(|name| identity.contains(name));
    let action = words
        .iter()
        .any(|word| matches!(*word, "button" | "cta" | "action"))
        || identity.contains("按钮")
        || identity.contains("操作栏");
    pinned && action
}

fn has_action(node: &PenNode) -> bool {
    matches!(node.base().role.as_deref(), Some("button" | "icon-button"))
        || crate::wire_screen_navigation::node_has_events(node)
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_action))
}
