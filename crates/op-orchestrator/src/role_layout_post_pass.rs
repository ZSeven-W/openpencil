use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

/// Read a width/height as a pixel number (port of TS `toSizeNumber`).
pub(crate) fn size_number(node: &Value, key: &str) -> f64 {
    match node.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn gap_number(node: &Value) -> f64 {
    match node.get("gap") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn padding_lr(node: &Value) -> (f64, f64) {
    match node.get("padding") {
        Some(Value::Number(n)) => {
            let v = n.as_f64().unwrap_or(0.0);
            (v, v)
        }
        Some(Value::String(s)) => {
            let v = s.parse::<f64>().unwrap_or(0.0);
            (v, v)
        }
        Some(Value::Array(a)) if a.len() == 2 => (
            a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
            a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
        ),
        Some(Value::Array(a)) if a.len() >= 4 => (
            a.get(3).and_then(Value::as_f64).unwrap_or(0.0),
            a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
        ),
        _ => (0.0, 0.0),
    }
}

pub(crate) fn fix_horizontal_overflow(node: &mut Value, canvas_width: f64) {
    // Summing child widths as a ROW is only valid for a row layout. A
    // `vertical` column stacks its children (widths don't sum — the max
    // applies) and a `none` container positions them absolutely; running the
    // row-sum on those wrongly widens them — e.g. a narrow vertical sidebar of N
    // `fill_container` items (each counted as 80px) sums to ~80*N and gets
    // re-widened to a fraction of the canvas, undoing the app-shell pass. A
    // frame with no explicit `layout` defaults to a row, so only the explicit
    // column / absolute cases are skipped.
    if matches!(
        node.get("layout").and_then(Value::as_str),
        Some("vertical") | Some("none")
    ) {
        return;
    }
    let parent_w = size_number(node, "width");
    if parent_w <= 0.0 {
        return;
    }
    let (pad_l, pad_r) = padding_lr(node);
    let avail_w = parent_w - pad_l - pad_r;
    let gap = gap_number(node);
    let Some(children) = node.get("children").and_then(Value::as_array) else {
        return;
    };
    if children.len() < 2 {
        return;
    }
    let gap_total = gap * (children.len().saturating_sub(1) as f64);
    let mut total_w = gap_total
        + children
            .iter()
            .map(|c| {
                let w = size_number(c, "width");
                if c.get("width").and_then(Value::as_f64).is_some() && w > 0.0 {
                    w
                } else {
                    80.0
                }
            })
            .sum::<f64>();
    if total_w <= avail_w {
        return;
    }
    for try_gap in [8.0, 4.0] {
        if gap > try_gap {
            let reduced = total_w - gap_total + try_gap * (children.len() - 1) as f64;
            if reduced <= avail_w {
                node["gap"] = json!(try_gap);
                total_w = reduced;
                break;
            }
        }
    }
    if total_w > avail_w {
        let needed_w = (total_w + pad_l + pad_r).round();
        if needed_w > parent_w && needed_w <= canvas_width {
            node["width"] = json!(needed_w);
        } else if needed_w > canvas_width * 0.8 {
            // Content exceeds the viewport — widening can't make the children fit
            // (their sum already overflows the canvas). Span the viewport and clip
            // the overflow at the row edge so it reads as a scroll row cut at the
            // screen, instead of chips spilling off-canvas into the void.
            // overflow.md mandates a `clipContent` wrapper for scroll rows; weak
            // models (e.g. glm-5.2) routinely emit a bare horizontal frame without
            // it, so this is the deterministic floor that keeps off-screen children
            // from rendering outside the device frame.
            node["width"] = json!("fill_container");
            node["clipContent"] = json!(true);
        }
    }
}

/// Footer-sink floor (weak-model insurance): a `vertical` container with a
/// flexible spacer, or an explicitly named viewport/sidebar/work-surface that
/// distributes children on the main axis, needs definite free space. Promote
/// that narrow, explicit shape from Hug to Full Height. `space_between` alone
/// is not enough: ordinary cards and content columns use it internally while
/// remaining content-sized.
///
/// Weak models (glm-5.2) emit this even WITH the sidebar footer-sink contract
/// loaded — they group a Top cluster + footer correctly but leave the wrapper
/// `fit_content`, so the prompt teaching alone does not carry it. Self-contained
/// (reads only this node's own props), so it is safe at both the per-subtask role
/// pass and the whole-doc loop finalize.
pub(crate) fn fix_main_axis_distribution_room(node: &mut Value) {
    if node.get("layout").and_then(Value::as_str) != Some("vertical") {
        return;
    }
    let hugs_height = match node.get("height") {
        None => true,
        Some(Value::String(s)) => s == "fit_content",
        _ => false,
    };
    if !hugs_height {
        return;
    }
    let distributes = matches!(
        node.get("justifyContent").and_then(Value::as_str),
        Some("space_between") | Some("space_around") | Some("space_evenly")
    );
    let has_flex_spacer = node
        .get("children")
        .and_then(Value::as_array)
        .is_some_and(|kids| kids.iter().any(is_flex_main_axis_spacer));
    if has_flex_spacer || (distributes && is_explicit_main_axis_remainder_consumer(node)) {
        node["height"] = json!("fill_container");
    }
}

fn is_explicit_main_axis_remainder_consumer(node: &Value) -> bool {
    let role = node
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if matches!(
        role.as_str(),
        "sidebar"
            | "navigation-rail"
            | "main"
            | "scroll"
            | "scroll-area"
            | "viewport"
            | "workspace"
            | "work-surface"
    ) {
        return true;
    }

    let name = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    [
        "sidebar",
        "navigation rail",
        "scroll viewport",
        "workspace",
        "work surface",
        "work-surface",
        "侧边栏",
        "导航栏",
        "滚动视口",
        "工作区",
    ]
    .iter()
    .any(|needle| name.contains(needle))
}

/// A flexible vertical spacer: a child asking to fill the main axis
/// (`height:"fill_container"`) with no content of its own (empty / absent
/// children) or an explicit "spacer" name — the model's stand-in for "eat the
/// remaining space". A content-bearing `fill_container`-height child is NOT a
/// spacer, so a real fill panel does not trip the promote.
fn is_flex_main_axis_spacer(child: &Value) -> bool {
    if child.get("height").and_then(Value::as_str) != Some("fill_container") {
        return false;
    }
    let name = child
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    if name.contains("spacer") {
        return true;
    }
    child
        .get("children")
        .and_then(Value::as_array)
        .map(|c| c.is_empty())
        .unwrap_or(true)
}

/// Whole-root driver for [`fix_main_axis_distribution_room`], registered as a
/// `run_cleanup_passes` root transform so it runs on the FULLY ASSEMBLED tree
/// (after scaffolding wraps each subtask section into its column, and after the
/// per-subtask role pass). The per-subtask pass sees a section root in isolation
/// — too early, because the sidebar column's definite height comes from the
/// assembled `[sidebar | content]` shell, not the bare section — so the promote
/// has to re-run here where the hierarchy + its space_between / spacer signals
/// are final. Returns whether any node was promoted (the caller commits only on
/// change). Round-trips through `Value` like the sibling structural passes; a bad
/// (de)serialize leaves the root untouched.
pub(crate) fn sink_main_axis_distribution(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !promote_distribution_recursive(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(new_root) => {
            *root = new_root;
            true
        }
        Err(_) => false,
    }
}

/// Apply [`fix_main_axis_distribution_room`] to every node in the subtree,
/// reporting whether any `height` changed.
fn promote_distribution_recursive(v: &mut Value) -> bool {
    let before = v.get("height").and_then(Value::as_str).map(str::to_owned);
    fix_main_axis_distribution_room(v);
    let mut changed = before != v.get("height").and_then(Value::as_str).map(str::to_owned);
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            if promote_distribution_recursive(c) {
                changed = true;
            }
        }
    }
    changed
}

// NOTE: the tree-shape circular-height demoter (`fix_circular_fill_height`)
// that lived here is retired. The layout engine now resolves a fill-height
// child of a hugging parent to its content size on BOTH axes (vertical main
// axis → grow, horizontal cross axis → stretch), so the collapse it guessed at
// no longer exists; a real resolved-~0 collapse is caught by
// `geometry_validation::collect_collapse_fixes` against the actual layout.

pub(crate) fn fix_text_heights(node: &mut Value) {
    let Some(children) = node.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    for child in children {
        if child.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        let explicit_height = child.get("height").and_then(Value::as_f64).is_some();
        let fixed_height =
            child.get("textGrowth").and_then(Value::as_str) == Some("fixed-width-height");
        if explicit_height && !fixed_height {
            if let Some(obj) = child.as_object_mut() {
                obj.remove("height");
            }
        }
    }
}

#[cfg(test)]
mod distribution_room_tests {
    use super::*;
    use serde_json::json;

    fn height_of(v: &Value) -> Option<&str> {
        v.get("height").and_then(Value::as_str)
    }

    #[test]
    fn space_between_hug_column_is_promoted_to_fill() {
        // glm shape A: sidebar nav uses space_between but the column hugs, so
        // the distribution has no room and the footer floats mid-rail.
        let mut node = json!({
            "type": "frame", "name": "Sidebar Navigation",
            "layout": "vertical", "height": "fit_content",
            "justifyContent": "space_between",
            "children": [{"type": "frame"}, {"type": "frame"}],
        });
        fix_main_axis_distribution_room(&mut node);
        assert_eq!(height_of(&node), Some("fill_container"));
    }

    #[test]
    fn ordinary_space_between_content_column_keeps_hug_height() {
        let mut node = json!({
            "type": "frame", "name": "Offer Card Content",
            "layout": "vertical", "height": "fit_content",
            "justifyContent": "space_between",
            "children": [{"type": "text"}, {"type": "frame", "role": "button"}],
        });
        fix_main_axis_distribution_room(&mut node);
        assert_eq!(height_of(&node), Some("fit_content"));
    }

    #[test]
    fn flex_spacer_in_hug_column_is_promoted_to_fill() {
        // glm shape B: a fill_container spacer between a top group and a footer,
        // but the wrapper hugs so the spacer collapses to 0 — promote the wrapper.
        let mut node = json!({
            "type": "frame", "layout": "vertical", "height": "fit_content",
            "children": [
                {"type": "frame", "name": "Top Group", "height": "fit_content"},
                {"type": "frame", "name": "Spacer", "height": "fill_container", "children": []},
                {"type": "frame", "name": "User Card", "height": "fit_content"},
            ],
        });
        fix_main_axis_distribution_room(&mut node);
        assert_eq!(height_of(&node), Some("fill_container"));
    }

    #[test]
    fn absent_height_column_with_spacer_is_promoted() {
        let mut node = json!({
            "type": "frame", "layout": "vertical",
            "children": [{"type": "frame", "height": "fill_container", "children": []}],
        });
        fix_main_axis_distribution_room(&mut node);
        assert_eq!(height_of(&node), Some("fill_container"));
    }

    #[test]
    fn already_fill_or_numeric_height_is_left_alone() {
        let mut fill = json!({
            "type": "frame", "layout": "vertical", "height": "fill_container",
            "justifyContent": "space_between", "children": [{}, {}],
        });
        fix_main_axis_distribution_room(&mut fill);
        assert_eq!(height_of(&fill), Some("fill_container"));

        let mut numeric = json!({
            "type": "frame", "layout": "vertical", "height": 600,
            "justifyContent": "space_between", "children": [{}, {}],
        });
        fix_main_axis_distribution_room(&mut numeric);
        assert_eq!(numeric.get("height").and_then(Value::as_f64), Some(600.0));
    }

    #[test]
    fn horizontal_and_plain_columns_are_not_touched() {
        // Horizontal space_between distributes on the WIDTH axis — height is
        // irrelevant; never promote a row's height.
        let mut row = json!({
            "type": "frame", "layout": "horizontal", "height": "fit_content",
            "justifyContent": "space_between", "children": [{}, {}],
        });
        fix_main_axis_distribution_room(&mut row);
        assert_eq!(height_of(&row), Some("fit_content"));

        // A plain hug column with no distribution intent must keep hugging.
        let mut plain = json!({
            "type": "frame", "layout": "vertical", "height": "fit_content",
            "children": [{"type": "text"}, {"type": "text"}],
        });
        fix_main_axis_distribution_room(&mut plain);
        assert_eq!(height_of(&plain), Some("fit_content"));
    }

    #[test]
    fn whole_doc_driver_promotes_nested_sidebar_nav() {
        // Assembled shell shape: a horizontal root → [Sidebar(fill) → Sidebar
        // Navigation(hug, space_between, two groups), Main]. The driver must
        // recurse and promote the NESTED nav column (the per-subtask pass can't,
        // because the column's definite height only exists after assembly).
        let v = json!({
            "type": "frame", "id": "root", "layout": "horizontal", "width": 1200,
            "height": 900,
            "children": [
                {"type": "frame", "id": "sb", "name": "Sidebar", "layout": "vertical",
                 "width": 260, "height": "fill_container", "children": [
                    {"type": "frame", "id": "nav", "name": "Sidebar Navigation",
                     "layout": "vertical", "width": "fill_container", "height": "fit_content",
                     "justifyContent": "space_between", "children": [
                        {"type": "frame", "id": "top", "name": "Top Group", "height": "fit_content"},
                        {"type": "frame", "id": "bot", "name": "Bottom Group", "height": "fit_content"},
                     ]},
                 ]},
                {"type": "frame", "id": "main", "name": "Main", "layout": "vertical",
                 "width": "fill_container", "height": "fit_content", "children": []},
            ],
        });
        let mut root: PenNode = serde_json::from_value(v).expect("valid PenNode");
        assert!(
            sink_main_axis_distribution(&mut root),
            "driver should report a change"
        );
        let out = serde_json::to_value(&root).unwrap();
        let nav = &out["children"][0]["children"][0];
        assert_eq!(
            nav["height"].as_str(),
            Some("fill_container"),
            "nested sidebar nav must be promoted; got {:?}",
            nav["height"]
        );
        // The outer Sidebar (already fill) and Main (a plain hug column) are
        // untouched.
        assert_eq!(out["children"][1]["height"].as_str(), Some("fit_content"));
    }

    #[test]
    fn content_bearing_fill_child_is_not_a_spacer() {
        // A fill_container-height child that HAS content is a real panel, not a
        // spacer — it must not trip the promote.
        let mut node = json!({
            "type": "frame", "layout": "vertical", "height": "fit_content",
            "children": [
                {"type": "frame", "name": "Body", "height": "fill_container",
                 "children": [{"type": "text"}]},
            ],
        });
        fix_main_axis_distribution_room(&mut node);
        assert_eq!(height_of(&node), Some("fit_content"));
    }
}
