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

/// Footer-sink floor (weak-model insurance): a `vertical` container that
/// expresses intent to PUSH content apart on the main axis — either
/// `justifyContent: space_between|space_around|space_evenly`, or a flexible
/// spacer child (`height:"fill_container"` empty / "spacer"-named frame) — but is
/// itself `height` hug (`fit_content` / unset) has NO free space to distribute,
/// so the distribution / spacer collapses and a footer the model meant to pin to
/// the bottom floats mid-column. Promote it to `height:"fill_container"` so the
/// intent has room to act; the ancestor chain (sidebar → root) supplies the
/// definite height, and where that chain itself hugs, `fill_container` is
/// harmless (the container still hugs its content).
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
    if distributes || has_flex_spacer {
        node["height"] = json!("fill_container");
    }
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

/// Break a CIRCULAR height dependency: a `fit_content`-height container cannot be
/// sized by a `fill_container`-height child (the parent wants to hug the child;
/// the child wants to fill the parent). jian/taffy resolves the cycle by
/// COLLAPSING the child toward zero, so a KPI card's stacked icon + value + label
/// render on top of each other, and the collapsed height makes every ancestor's
/// content-height undercount (the root then never stretches — the reported "根
/// 高度撑不起来"). Pencil forbids this shape in its prompt ("a `fit_content`
/// parent cannot be pushed by `fill_container` children"); this is the
/// deterministic backstop for when a weak model emits it anyway — retarget the
/// child's height to `fit_content` so it hugs real content. Only fires on an
/// EXPLICIT `fit_content` parent, so the numeric-height root and its
/// `fill_container` sidebar chain are never touched. Whole-root, round-trip like
/// the sibling passes.
pub(crate) fn fix_circular_fill_height(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !fix_circular_recursive(&mut v) {
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

fn fix_circular_recursive(v: &mut Value) -> bool {
    let mut changed = false;
    if v.get("height").and_then(Value::as_str) == Some("fit_content") {
        if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
            for c in kids.iter_mut() {
                if c.get("height").and_then(Value::as_str) == Some("fill_container") {
                    if let Some(obj) = c.as_object_mut() {
                        obj.insert("height".into(), json!("fit_content"));
                        // A now-hug VERTICAL container can no longer distribute its
                        // children on the main (vertical) axis: `space_between`
                        // there collapses to an OVERLAP in jian (a KPI card's icon
                        // / value / label pile on top of each other). Top-pack them
                        // and give a modest gap so they read as a stack.
                        let vertical =
                            obj.get("layout").and_then(Value::as_str) == Some("vertical");
                        let distributes = matches!(
                            obj.get("justifyContent").and_then(Value::as_str),
                            Some("space_between" | "space_around" | "space_evenly")
                        );
                        if vertical && distributes {
                            obj.insert("justifyContent".into(), json!("start"));
                            let has_gap = obj
                                .get("gap")
                                .and_then(Value::as_f64)
                                .map(|g| g > 0.0)
                                .unwrap_or(false);
                            if !has_gap {
                                obj.insert("gap".into(), json!(8));
                            }
                        }
                        changed = true;
                    }
                }
            }
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= fix_circular_recursive(c);
        }
    }
    changed
}

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
            "type": "frame", "layout": "vertical", "height": "fit_content",
            "justifyContent": "space_between",
            "children": [{"type": "frame"}, {"type": "frame"}],
        });
        fix_main_axis_distribution_room(&mut node);
        assert_eq!(height_of(&node), Some("fill_container"));
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
    fn circular_fill_card_in_hug_row_is_demoted_and_undistributed() {
        // KPI shape: a fit_content row of fill_container cards that distribute with
        // space_between → the cards collapse and their stacked children overlap.
        let mut root: PenNode = serde_json::from_value(json!({
            "type": "frame", "id": "row", "name": "Key Metrics Row", "layout": "horizontal",
            "height": "fit_content", "children": [
                { "type": "frame", "id": "card", "name": "Card", "layout": "vertical",
                  "height": "fill_container", "justifyContent": "space_between", "children": [
                    { "type": "text", "id": "v", "content": "1,248" },
                    { "type": "text", "id": "l", "content": "TOTAL" }
                  ]}
            ]
        }))
        .expect("valid PenNode");
        assert!(fix_circular_fill_height(&mut root));
        let v = serde_json::to_value(&root).unwrap();
        let card = &v["children"][0];
        assert_eq!(
            card["height"],
            json!("fit_content"),
            "card demoted off fill"
        );
        assert_eq!(
            card["justifyContent"],
            json!("start"),
            "space_between neutralized"
        );
        assert!(
            card.get("gap").and_then(Value::as_f64).unwrap_or(0.0) > 0.0,
            "a gap was injected"
        );
    }

    #[test]
    fn fill_container_child_of_numeric_parent_is_left_alone() {
        // The sidebar (fill_container) under a NUMERIC-height root must NOT be
        // demoted — only an explicit `fit_content` parent is circular.
        let mut root: PenNode = serde_json::from_value(json!({
            "type": "frame", "id": "root", "layout": "horizontal", "height": 900, "children": [
                { "type": "frame", "id": "sb", "name": "Sidebar", "layout": "vertical",
                  "height": "fill_container", "children": [] }
            ]
        }))
        .expect("valid PenNode");
        assert!(
            !fix_circular_fill_height(&mut root),
            "numeric-height parent is not circular"
        );
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
