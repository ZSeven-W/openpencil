//! None-stack inset — rewrite a `fill_container` child that is ALSO pinned at
//! a positive offset inside a `layout: "none"` stack into the numeric size the
//! author obviously meant: the stack's resolved size minus twice the offset.
//!
//! Measured on real round-2 phone screens (taxi / hotel / calendar): a
//! `layout: "none"` stack (the taxi "搜索区堆叠", 500px tall) holds a floating
//! card authored as `x: 24` + `width: "fill_container"`. jian resolves the
//! keyword to the stack's FULL width, the 24px offset then pushes the card
//! 24px past the right edge and the geometry loop clips it — the card reads
//! as shifted right and cut. The intent is a card inset by the same amount on
//! both sides, so the repair is `width = parent_width − 2·x` (and likewise
//! `height = parent_height − 2·y` for a `height: "fill_container"` child).
//!
//! Runs in the overflow pre-pass BEFORE the absolute-child clamp: after this
//! rewrite the child already fits, so the clamp and the clip fallback never
//! see it. Geometry edits go through `EditorCommand::UpdateNode` — the project
//! rule is that numeric geometry (`x` / numeric `width`) is set via
//! `UpdateNode`, never `SetNodeLayoutProp`.

use super::*;

/// Rewrite the inset offenders under `root_id`. Returns the number of edits
/// the sink accepted.
pub(crate) fn repair_none_stack_insets(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let rects = resolved_rects(sink.state());
    let cmds = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return 0;
        };
        let Ok(v) = serde_json::to_value(root) else {
            return 0;
        };
        let mut cmds = Vec::new();
        collect_none_stack_inset_fixes(&v, &rects, &mut cmds);
        cmds
    };
    let mut applied = 0;
    for cmd in cmds {
        if sink.apply(cmd) {
            applied += 1;
        }
    }
    applied
}

/// Collect an `UpdateNode` size rewrite for every child of a
/// `layout: "none"` parent that pairs a `fill_container` size keyword with a
/// positive authored offset on the same axis. The status bar stacks its
/// chrome the same way but is chrome-protected everywhere else, so its whole
/// subtree is skipped.
pub(super) fn collect_none_stack_inset_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let mut ancestors = Vec::new();
    collect_in_tree(v, rects, &mut ancestors, cmds);
}

fn collect_in_tree<'a>(
    v: &'a Value,
    rects: &HashMap<String, Rect>,
    ancestors: &mut Vec<&'a Value>,
    cmds: &mut Vec<EditorCommand>,
) {
    // Only an explicit `layout: "none"` stack pins children by x/y while still
    // honouring their `fill_container` keywords; a layout-less container
    // defaults to a row flow where an authored x/y carries no inset intent.
    if layout_str(v) == Some("none") && !in_status_bar(v, ancestors) {
        // A stack without a resolved box offers no width to inset against —
        // leave its children untouched.
        if let Some(pr) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        {
            for c in children(v) {
                push_inset_command(c, pr, cmds);
            }
        }
    }
    ancestors.push(v);
    for c in children(v) {
        collect_in_tree(c, rects, ancestors, cmds);
    }
    ancestors.pop();
}

fn in_status_bar(v: &Value, ancestors: &[&Value]) -> bool {
    crate::cleanup::is_status_bar_from_json(v)
        || ancestors
            .iter()
            .any(|ancestor| crate::cleanup::is_status_bar_from_json(ancestor))
}

fn push_inset_command(c: &Value, pr: &Rect, cmds: &mut Vec<EditorCommand>) {
    let width = inset_size(c, "width", "x", pr.w);
    let height = inset_size(c, "height", "y", pr.h);
    if width.is_none() && height.is_none() {
        return;
    }
    let Some(cid) = c.get("id").and_then(Value::as_str) else {
        return;
    };
    cmds.push(EditorCommand::UpdateNode {
        node_id: NodeId::new(cid.to_string()),
        x: None,
        y: None,
        width,
        height,
        name: None,
        fill_hex: None,
        page_id: None,
    });
}

/// `parent_size − 2·offset`, rounded, when the child pairs a `fill_container`
/// `size_key` with a positive numeric `offset_key`. `None` for a numeric or
/// absent size (an authored fixed box is not this pass's contract), a zero /
/// absent offset, an unresolved parent axis, or an offset so large that
/// `2·offset ≥ parent_size` (nothing left to inset into).
fn inset_size(c: &Value, size_key: &str, offset_key: &str, parent_size: f64) -> Option<i32> {
    if !parent_size.is_finite() {
        return None;
    }
    if c.get(size_key).and_then(Value::as_str) != Some("fill_container") {
        return None;
    }
    let offset = positive_offset(c, offset_key)?;
    let size = parent_size - 2.0 * offset;
    (size > 0.0).then(|| size.round() as i32)
}

/// A positive numeric prop — `x` / `y`. `None` for absent, null, keyword, or
/// non-positive values.
fn positive_offset(v: &Value, key: &str) -> Option<f64> {
    let offset = match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }?;
    (offset > 0.0).then_some(offset)
}

#[cfg(test)]
#[path = "none_stack_inset_tests.rs"]
mod tests;
