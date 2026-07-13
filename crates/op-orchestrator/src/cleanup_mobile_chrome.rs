use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{first_solid_fill_hex, EditorCommand, NodeId, PenNodeExt};

/// Bottom navigation is structural chrome with exactly one legal position:
/// the mobile root's LAST child. Models "catch up" on forgotten sections by
/// appending them after the nav (measured: MiniMax-M3 appended the
/// greeting+search header as the final root child, below the tab bar —
/// test0710-1-m3.op). Nav-last is a structural CONTRACT (independent of
/// intent), so it is deterministically repaired here with one Move; where
/// the late section actually belongs is intent — the geometry echo tells
/// the model to relocate it with `M()`.
///
/// Width-only mobile gate: `fit_content`-height mobile roots resolve no
/// pixel height, so `is_mobile_root` (width AND height) would skip exactly
/// the measured case.
pub(crate) fn anchor_bottom_nav_last_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        anchor_bottom_nav_last(sink, &root_id);
    }
}

pub(super) fn anchor_bottom_nav_last(sink: &mut dyn DocSink, root_id: &str) {
    let nav_id: Option<NodeId> = {
        let Some(root) = super::find_root(sink.state(), root_id) else {
            return;
        };
        if !root.width_px().is_some_and(|w| w <= 480.0) {
            return;
        }
        let Some(children) = root.children() else {
            return;
        };
        let last_index = children.len().saturating_sub(1);
        children
            .iter()
            .enumerate()
            .find(|(index, child)| {
                *index < last_index
                    && bottom_nav_surface_target(child, *index == last_index).is_some()
            })
            .map(|(_, child)| NodeId::new(child.id_str().to_string()))
    };
    if let Some(node_id) = nav_id {
        sink.apply(EditorCommand::MoveNode {
            node_id,
            target_parent: NodeId::new(root_id.to_string()),
            page_id: None,
            index: None,
        });
    }
}

pub(crate) fn repair_mobile_structural_chrome_for_all_roots(sink: &mut dyn DocSink) {
    let root_ids: Vec<String> = sink
        .state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect();
    for root_id in root_ids {
        repair_mobile_structural_chrome(sink, &root_id);
    }
}

pub(crate) fn repair_mobile_structural_chrome(sink: &mut dyn DocSink, root_id: &str) {
    let repairs = {
        let Some(root) = super::find_root(sink.state(), root_id) else {
            return;
        };
        if !super::is_mobile_root(root) {
            return;
        }
        let root_width = root.width_px().unwrap_or(0.0);
        let Some(children) = root.children() else {
            return;
        };
        let last_index = children.len().saturating_sub(1);
        let mut repairs = MobileChromeRepairs::default();
        for (index, child) in children.iter().enumerate() {
            // The structural (unnamed) bottom-nav fallback only applies to the
            // LAST top-level section — a bottom nav sits at the page bottom, so
            // this prevents a labeled header action row at the TOP from being
            // mistaken for a nav and stretched full-width. Named / role-tagged
            // navs are still detected anywhere.
            let allow_structural = index == last_index;
            if should_strip_structural_shell(child) {
                repairs
                    .structural_shells
                    .push(NodeId::new(child.id_str().to_string()));
            }
            collect_bottom_nav_chrome_repairs(child, root_width, allow_structural, &mut repairs);
        }
        repairs
    };

    for node_id in repairs.structural_shells {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: r#"{"fill":null,"stroke":null,"effects":null,"cornerRadius":0}"#
                .to_string(),
            page_id: None,
        });
    }
    for (node_id, root_width) in repairs.bottom_nav_surfaces {
        // `x`/`y` are CLEARED, never set: any authored position reads as
        // ABSOLUTE placement in jian and yanks the nav out of flex flow —
        // the old `"x":0` patch (without y) pinned a healthy 4-tab
        // BottomTabBar to the root's top-left corner where later-painted
        // siblings buried it ("the navbar vanished at finalize",
        // test0711-22 23:34 run). The nav is the root's last FLEX child;
        // the numeric width alone gives the full-bleed span.
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: format!(
                r#"{{"x":null,"y":null,"width":{},"height":72,"layout":"horizontal","gap":0,"padding":[8,16,8,16],"justifyContent":"space_between","alignItems":"center","stroke":null,"effects":null,"cornerRadius":0}}"#,
                root_width.round()
            ),
            page_id: None,
        });
    }
    for node_id in repairs.bottom_nav_items {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json: r#"{"fill":null,"stroke":null,"effects":null,"cornerRadius":0,"width":"fill_container","height":"fill_container","layout":"vertical","gap":4,"padding":[4,0],"justifyContent":"center","alignItems":"center"}"#.to_string(),
            page_id: None,
        });
    }
}

#[derive(Default)]
struct MobileChromeRepairs {
    structural_shells: Vec<NodeId>,
    bottom_nav_surfaces: Vec<(NodeId, f64)>,
    bottom_nav_items: Vec<NodeId>,
}

fn should_strip_structural_shell(node: &PenNode) -> bool {
    if !node.is_container()
        || node
            .children()
            .map(|children| children.is_empty())
            .unwrap_or(true)
        || super::is_status_bar(node)
        || super::nav_surface_target(node).is_some()
    {
        return false;
    }

    if has_nested_filled_atomic_component(node) {
        return true;
    }
    if is_visual_surface_role(node) {
        return false;
    }

    let hay = super::node_identity_haystack(node);
    let section_like = node
        .base()
        .role
        .as_deref()
        .map(|role| role.eq_ignore_ascii_case("section"))
        .unwrap_or(false);
    (section_like || hay.contains("section"))
        && (is_search_or_category_haystack(&hay) || has_search_or_category_child(node))
}

fn is_visual_surface_role(node: &PenNode) -> bool {
    matches!(
        node.base()
            .role
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "badge"
            | "button"
            | "card"
            | "chip"
            | "feature-card"
            | "form-input"
            | "icon-button"
            | "image-card"
            | "input"
            | "pricing-card"
            | "search-bar"
            | "stat-card"
            | "tag"
    )
}

fn has_nested_filled_atomic_component(node: &PenNode) -> bool {
    let Some(parent_role) = normalized_role(node) else {
        return false;
    };
    if !is_atomic_protected_role(&parent_role) {
        return false;
    }
    node.children()
        .map(|children| {
            children.iter().any(|child| {
                let child_role = normalized_role(child);
                if child_role.as_deref() == Some(parent_role.as_str()) {
                    return true;
                }
                if child_role
                    .as_deref()
                    .map(is_primary_atomic_role)
                    .unwrap_or(false)
                    && first_solid_fill_hex(child).is_some()
                {
                    return true;
                }
                child_role.is_none()
                    && first_solid_fill_hex(child).is_some()
                    && is_input_like_haystack(&super::node_identity_haystack(child))
            })
        })
        .unwrap_or(false)
}

fn normalized_role(node: &PenNode) -> Option<String> {
    node.base()
        .role
        .as_deref()
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .map(str::to_lowercase)
}

fn is_atomic_protected_role(role: &str) -> bool {
    matches!(
        role,
        "badge"
            | "button"
            | "chip"
            | "form-input"
            | "icon-button"
            | "input"
            | "pill"
            | "search-bar"
            | "tag"
    )
}

fn is_primary_atomic_role(role: &str) -> bool {
    matches!(role, "form-input" | "input" | "search-bar")
}

fn is_input_like_haystack(hay: &str) -> bool {
    super::contains_any(hay, &["form input", "form-input", "input", "search"])
}

fn collect_bottom_nav_chrome_repairs(
    root_child: &PenNode,
    root_width: f64,
    allow_structural: bool,
    repairs: &mut MobileChromeRepairs,
) {
    let Some(nav) = bottom_nav_surface_target(root_child, allow_structural) else {
        return;
    };
    repairs
        .bottom_nav_surfaces
        .push((NodeId::new(nav.id_str().to_string()), root_width));

    let Some(children) = nav.children() else {
        return;
    };
    if children.len() < 3 {
        return;
    }
    for child in children {
        if is_bottom_nav_item(child) {
            repairs
                .bottom_nav_items
                .push(NodeId::new(child.id_str().to_string()));
        }
    }
}

pub(super) fn bottom_nav_surface_target(
    root_child: &PenNode,
    allow_structural: bool,
) -> Option<&PenNode> {
    if is_bottom_nav_surface(root_child, allow_structural) {
        if let Some(inner) = nested_bottom_nav_surface(root_child, allow_structural) {
            return Some(inner);
        }
        return Some(root_child);
    }
    // The actual nav row may be nested inside a wrapper section (e.g. a
    // "Bottom Navigation" section frame holding the horizontal tab row, or a
    // single content wrapper that holds the whole screen). Find the nav surface
    // among the direct children — but the STRUCTURAL fallback is only allowed on
    // the LAST child (the bottom-most row). Otherwise, when the page is one big
    // content wrapper, a labeled header row at the TOP of that wrapper would be
    // structurally mistaken for a nav. Named / role-tagged navs match anywhere.
    let children = root_child.children()?;
    let last = children.len().saturating_sub(1);
    children.iter().enumerate().find_map(|(i, inner)| {
        let inner_structural = allow_structural && i == last;
        is_bottom_nav_surface(inner, inner_structural).then_some(inner)
    })
}

fn nested_bottom_nav_surface(node: &PenNode, allow_structural: bool) -> Option<&PenNode> {
    let children = node.children()?;
    if children.len() < 2 {
        return None;
    }
    children.iter().find(|child| {
        child.is_container()
            && (is_bottom_nav_surface(child, allow_structural)
                || is_structural_bottom_nav_row(child))
    })
}

fn is_bottom_nav_surface(node: &PenNode, allow_structural: bool) -> bool {
    let role = normalized_role(node);
    if matches!(role.as_deref(), Some("bottom-tab-bar")) {
        return true;
    }
    let hay = super::node_identity_haystack(node);
    // Only BOTTOM-specific names match by name — "tab bar" / "navbar" /
    // "nav bar" are ambiguous (a TOP navbar carries those too) and are NOT
    // position-gated here, so including them turned top navs into bottom navs.
    // A genuinely-unnamed bottom nav is still caught by the structural fallback
    // below, which IS gated to the last/bottom section.
    if super::contains_any(
        &hay,
        &[
            "bottom nav",
            "bottom-nav",
            "bottom navigation",
            "bottom-navigation",
            "bottom tab",
            "bottom-tab",
            "底部导航",
            "底部导航栏",
            "导航栏",
            "底栏",
            "标签栏",
            "底部标签栏",
        ],
    ) {
        return true;
    }
    // Structural fallback: a horizontal row of 3-5 labeled nav tabs
    // (Home / Search / Orders / Profile…) IS a bottom nav even when the surface
    // frame wasn't named "bottom nav" or tagged role="bottom-tab-bar". Only
    // consulted for the LAST top-level section (`allow_structural`) so a labeled
    // header action row at the top is never mistaken for a nav.
    allow_structural && is_structural_bottom_nav_row(node)
}

/// A horizontal row of 3-5 children where EVERY child is a labeled nav tab —
/// the structural shape of a bottom tab bar regardless of its name/role.
///
/// Deliberately strict (every child must be an icon+label tab with a nav-ish
/// name) so it does NOT over-match: a header icon-button cluster (icons, no
/// labels) fails the label check, and a category chip row (Pizza/Burger/…)
/// fails the nav-name check.
fn is_structural_bottom_nav_row(node: &PenNode) -> bool {
    use jian_ops_schema::node::container::LayoutMode;
    let is_horizontal = match node {
        PenNode::Frame(n) => matches!(n.container.layout, Some(LayoutMode::Horizontal)),
        PenNode::Group(n) => matches!(n.container.layout, Some(LayoutMode::Horizontal)),
        _ => false,
    };
    if !is_horizontal {
        return false;
    }
    let Some(children) = node.children() else {
        return false;
    };
    if !(3..=5).contains(&children.len()) {
        return false;
    }
    children.iter().all(is_labeled_nav_tab)
}

/// Exact Pencil demo fallback for the literal trailing `Tab Bar Section`.
///
/// Pencil represents the outer section and the inner pill as intrinsic-height
/// containers, and leaves the pill's horizontal `layout` omitted while
/// providing `space_around`. Keep this predicate separate from the generic
/// structural matcher: cleanup must not rewrite every implicit flex row into a
/// fixed 72 px nav surface.
pub(super) fn is_pencil_trailing_tab_section(node: &PenNode) -> bool {
    use jian_ops_schema::node::container::{
        AlignItems, ContainerProps, JustifyContent, LayoutMode,
    };
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

    fn props(node: &PenNode) -> Option<&ContainerProps> {
        match node {
            PenNode::Frame(node) => Some(&node.container),
            PenNode::Group(node) => Some(&node.container),
            _ => None,
        }
    }
    fn is_fill_width(props: &ContainerProps) -> bool {
        matches!(
            props.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
    }
    fn is_hug_height(props: &ContainerProps) -> bool {
        matches!(
            props.height.as_ref(),
            None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
    }
    fn named(node: &PenNode, expected: &str) -> bool {
        node.base()
            .name
            .as_deref()
            .is_some_and(|name| name.trim().eq_ignore_ascii_case(expected))
    }

    let Some(outer) = props(node) else {
        return false;
    };
    if !named(node, "Tab Bar Section")
        || outer.layout.as_ref() != Some(&LayoutMode::Vertical)
        || !is_fill_width(outer)
        || !is_hug_height(outer)
    {
        return false;
    }
    let Some(children) = node.children() else {
        return false;
    };
    let [pill] = children.as_slice() else {
        return false;
    };
    let Some(pill_props) = props(pill) else {
        return false;
    };
    if !named(pill, "Tab Pill")
        || pill_props.layout.is_some()
        || !is_fill_width(pill_props)
        || !is_hug_height(pill_props)
        || !matches!(
            pill_props.justify_content,
            Some(JustifyContent::SpaceAround | JustifyContent::SpaceBetween)
        )
        || pill_props.align_items.as_ref() != Some(&AlignItems::Center)
    {
        return false;
    }
    let Some(tabs) = pill.children() else {
        return false;
    };
    (3..=5).contains(&tabs.len())
        && tabs.iter().all(|tab| {
            props(tab).is_some_and(|props| props.layout.as_ref() == Some(&LayoutMode::Vertical))
                && is_labeled_nav_tab(tab)
        })
}

/// A single bottom-nav tab: a nav-named/roled container that carries BOTH an
/// icon and a text label (the icon+label tab shape). The label requirement is
/// what separates a real tab from a header icon-button.
fn is_labeled_nav_tab(node: &PenNode) -> bool {
    if !is_bottom_nav_item(node) {
        return false;
    }
    let Some(children) = node.children() else {
        return false;
    };
    let has_icon = children.iter().any(|c| matches!(c, PenNode::IconFont(_)));
    let has_label = children.iter().any(|c| matches!(c, PenNode::Text(_)));
    has_icon && has_label
}

fn is_bottom_nav_item(node: &PenNode) -> bool {
    if !node.is_container() {
        return false;
    }
    if matches!(
        normalized_role(node).as_deref(),
        Some(
            "button"
                | "icon-button"
                | "nav-item"
                | "nav-item-active"
                | "pill"
                | "search-bar"
                | "tab"
                | "tab-active"
        )
    ) {
        return true;
    }
    let hay = super::node_identity_haystack(node);
    super::contains_any(
        &hay,
        &[
            "account", "cart", "discover", "home", "likes", "orders", "profile", "search", "tab",
        ],
    )
}

fn has_search_or_category_child(node: &PenNode) -> bool {
    node.children()
        .map(|children| {
            children
                .iter()
                .any(|child| is_search_or_category_haystack(&super::node_identity_haystack(child)))
        })
        .unwrap_or(false)
}

fn is_search_or_category_haystack(hay: &str) -> bool {
    super::contains_any(
        hay,
        &["categor", "category", "chip", "filter", "search", "sliders"],
    )
}
