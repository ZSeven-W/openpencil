//! Geometry-driven structural validation — the deterministic analogue of
//! Pencil's per-batch `snapshot_layout` feedback.
//!
//! The tree-shape post-passes (`role_layout_post_pass`, `table_repair`, …) reason
//! about the AUTHORED tree; they can't see where a node actually LANDS once jian
//! resolves flex sizing. A weak model that ignores the prompt's
//! "total_width ≤ parent_inner_width" rule (glm routinely does) emits fixed table
//! columns that sum WIDER than their row — jian then overflows them past the
//! right edge (columns overlap, the last column is clipped). No amount of
//! tree-shape heuristics catch it, because the widths are individually valid.
//!
//! This module runs the REAL jian layout (`editor_state_to_layout_scene`, the
//! same pass `snapshot_layout` uses), reads each node's RESOLVED absolute rect,
//! and fixes what the resolved geometry proves is wrong. First detector: a table
//! row whose fixed columns overflow its resolved width → scale the fixed columns
//! (and the column gap) down to fit, keeping proportions and column alignment.

use std::collections::HashMap;

use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, LayoutPropValue, NodeId};
use serde_json::Value;

use crate::types::DocSink;

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    /// Not read yet — vertical stacking-overlap detection will need it; kept
    /// so the resolved-rect map carries the full geometry.
    #[allow(dead_code)]
    y: f64,
    w: f64,
    h: f64,
}

/// EditorState → resolved absolute rect (w, h) per node id, via the SAME jian
/// flex pass `snapshot_layout` uses.
fn resolved_rects(state: &EditorState) -> HashMap<String, Rect> {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let mut map = HashMap::new();
    for page in &scene.pages {
        collect_rects(&page.children, &mut map);
    }
    map
}

fn collect_rects(nodes: &[SceneNode], map: &mut HashMap<String, Rect>) {
    for n in nodes {
        let b = n.aggregate_bounds();
        map.insert(
            n.id.clone(),
            Rect {
                x: f64::from(b.origin.x),
                y: f64::from(b.origin.y),
                w: f64::from(b.size.x),
                h: f64::from(b.size.y),
            },
        );
        collect_rects(&n.children, map);
    }
}

// ── tolerant Value readers ──

fn children(v: &Value) -> &[Value] {
    v.get("children")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn layout_str(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

/// A cell's fixed (numeric) width, or `None` when it is a keyword sizing
/// (`fill_container` / `fit_content`).
fn fixed_width(v: &Value) -> Option<f64> {
    match v.get("width") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn num(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Sum of a frame's LEFT + RIGHT padding — the schema authors `padding` as a
/// number (all sides), `[vertical, horizontal]`, or `[top, right, bottom,
/// left]`. The overflow math must compare column widths against the row's
/// INNER width; a `[12, 16]`-padded 860px row only offers 828px to its cells,
/// and ignoring that put a real table 2px on the "fits" side of the gate
/// while its flex column starved to 6px (measured: test0703.op).
fn horizontal_padding(v: &Value) -> f64 {
    match v.get("padding") {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) * 2.0,
        Some(Value::Array(a)) => match a.len() {
            1 => a[0].as_f64().unwrap_or(0.0) * 2.0,
            2 => a[1].as_f64().unwrap_or(0.0) * 2.0,
            4 => a[1].as_f64().unwrap_or(0.0) + a[3].as_f64().unwrap_or(0.0),
            _ => 0.0,
        },
        _ => 0.0,
    }
}

// ── Table column-overflow fix ──

/// Reserved width for each `fill_container` column so scaling leaves it room.
const MIN_FILL_COL: f64 = 40.0;
/// A flex column that CARRIES TEXT needs real room, not a sliver — 40px still
/// shreds "a.sterling@email.com" into a one-glyph-per-line tower that blows
/// the row to 400px tall. Reserve enough for a readable two-line wrap.
const MIN_FILL_TEXT_COL: f64 = 120.0;
/// Slack so a table that fits by a hair isn't needlessly rescaled.
const OVERFLOW_EPS: f64 = 4.0;
/// Leave a little breathing room after scaling.
const FIT_MARGIN: f64 = 0.97;
/// Never shrink columns below this fraction of their authored width — beyond it
/// the table is unsalvageable by scaling and needs a structural rethink (left to
/// a later detector); clamp so we don't produce 5px columns.
const MIN_SCALE: f64 = 0.35;

/// Detect + fix table rows whose FIXED columns overflow their resolved width.
/// Returns `true` iff any column was rescaled. A numeric width is set via
/// `UpdateNode` (`SetNodeLayoutProp` only accepts KEYWORD widths); the column gap
/// via `SetNodeLayoutProp`. Both mutate props in place — node ids never change,
/// so no `ReplaceSubtree` id churn.
pub fn fix_table_column_overflow(sink: &mut dyn DocSink, root_id: &str) -> bool {
    let rects = resolved_rects(sink.state());
    let ops: Vec<EditorCommand> = {
        let Some(root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return false;
        };
        let Ok(v) = serde_json::to_value(root) else {
            return false;
        };
        let mut ops = Vec::new();
        collect_scale_ops(&v, &rects, &mut ops);
        ops
    };
    if ops.is_empty() {
        return false;
    }
    for cmd in ops {
        sink.apply(cmd);
    }
    true
}

/// Max detect→fix→relayout rounds (Pencil's multi-round `snapshot_layout`
/// feedback, bounded so a hard-to-fix design can't spin).
const MAX_ROUNDS: usize = 3;

/// Geometry-driven validation LOOP: recompute the REAL layout, detect + fix the
/// structural violations the resolved rects prove (table column overflow,
/// collapsed fill containers), and repeat until a round finds nothing to do or
/// `MAX_ROUNDS` is hit. The deterministic analogue of Pencil's per-batch
/// `snapshot_layout` feedback. Returns the number of rounds that applied a fix.
pub fn geometry_validate_and_fix(sink: &mut dyn DocSink, root_id: &str) -> usize {
    let mut rounds = 0;
    for _ in 0..MAX_ROUNDS {
        let rects = resolved_rects(sink.state());
        let cmds = {
            let Some(root) = op_editor_core::walkers::find_node(
                sink.state().active_children(),
                &NodeId::new(root_id.to_string()),
            ) else {
                break;
            };
            let Ok(v) = serde_json::to_value(root) else {
                break;
            };
            let mut cmds = Vec::new();
            collect_scale_ops(&v, &rects, &mut cmds);
            collect_collapse_fixes(&v, &rects, &mut cmds);
            collect_text_overflow_fixes(&v, &rects, &mut cmds);
            collect_frame_overflow_fixes(&v, &rects, &mut cmds);
            collect_oversized_image_fixes(&v, &mut cmds);
            collect_absolute_fill_image_fixes(&v, &rects, &mut cmds);
            collect_image_slot_fixes(&v, &rects, &mut cmds);
            collect_grow_to_fit_fixes(&v, &rects, &mut cmds);
            collect_starved_rail_card_fixes(&v, &rects, &mut cmds);
            collect_row_gap_fixes(&v, &rects, &mut cmds);
            collect_card_row_height_fixes(&v, &rects, &mut cmds, false);
            collect_row_overfull_fixes(&v, &rects, &mut cmds, false);
            collect_card_overflow_clips(&v, &rects, &mut cmds);
            cmds
        };
        if cmds.is_empty() {
            break;
        }
        for cmd in cmds {
            sink.apply(cmd);
        }
        rounds += 1;
    }
    rounds
}

/// A container resolves to ~0 height while declaring `fill_container`. With the
/// engine resolving fill-of-hug to content size on both axes, this is the ONLY
/// collapse repairer left (the tree-shape `fix_circular_fill_height` demoter is
/// retired) — and it acts on proof from the real layout, never on tree shape.
const COLLAPSE_H: f64 = 6.0;
/// A child must carry real height for the parent's 0-height to read as a collapse
/// (not an intentionally-empty spacer).
const CHILD_MIN_H: f64 = 12.0;

fn collect_collapse_fixes(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    if is_collapsed_fill_container(v, rects) {
        if std::env::var("OPENPENCIL_DEBUG_CLEANUP").is_ok() {
            let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
            let r = rects.get(id);
            eprintln!(
                "[COLLAPSE-PROBE] demoting {} ({id}): resolved={:?}",
                v.get("name").and_then(Value::as_str).unwrap_or("?"),
                r.map(|r| (r.x, r.y, r.w, r.h))
            );
        }
        if let Some(id) = v.get("id").and_then(Value::as_str) {
            // Make it hug its content instead of filling a hugging ancestor.
            cmds.push(EditorCommand::SetNodeLayoutProp {
                node_id: NodeId::new(id.to_string()),
                property: "height".to_string(),
                value: LayoutPropValue::Keyword("fit_content".to_string()),
            });
            // A now-hug vertical column can't distribute on the main axis — jian
            // collapses `space_between` there to an overlap. Top-pack + gap.
            let distributes = matches!(
                v.get("justifyContent").and_then(Value::as_str),
                Some("space_between" | "space_around" | "space_evenly")
            );
            if distributes && layout_str(v) == Some("vertical") {
                cmds.push(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(id.to_string()),
                    property: "justifyContent".to_string(),
                    value: LayoutPropValue::Keyword("start".to_string()),
                });
                if num(v, "gap") <= 0.0 {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "gap".to_string(),
                        value: LayoutPropValue::Number(8.0),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_collapse_fixes(c, rects, cmds);
    }
}

/// Slack so a text a hair wider than its block isn't needlessly wrapped.
const TEXT_OVERFLOW_EPS: f64 = 2.0;

/// A text node whose RESOLVED width exceeds its parent block's — the parent is a
/// constrained (`fill_container` / fixed) block whose `min_size: 0` lets it shrink
/// BELOW its `fit_content` text, so the text overflows into the next column
/// (measured: a 260px sidebar's schedule rows painted the client name over the
/// appointment time). Constrain the text to its block — `width: fill_container` +
/// `textGrowth: fixed-width` — so it wraps inside instead of overflowing. The
/// next round re-resolves with the text now bounded, so it converges immediately.
fn collect_text_overflow_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    // Only meaningful in a FLEX parent: under `layout: none` children are
    // absolutely positioned, so "wider than the parent" is not an overflow to
    // repair (and `width: fill_container` means nothing there).
    let flex_parent = matches!(layout_str(v), Some("vertical" | "horizontal"));
    if let Some((parent_x, parent_w)) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|r| (r.x, r.w))
        .filter(|_| flex_parent)
    {
        let pill_parent = crate::chip_repair::is_pill_chip(v);
        for c in children(v) {
            if c.get("type").and_then(Value::as_str) != Some("text") {
                continue;
            }
            if pill_parent {
                if let Some(cid) = c.get("id").and_then(Value::as_str) {
                    let fill = c.get("width").and_then(Value::as_str) == Some("fill_container");
                    let wrap = c
                        .get("textGrowth")
                        .and_then(Value::as_str)
                        .is_some_and(|g| g.starts_with("fixed-width"));
                    if fill {
                        cmds.push(EditorCommand::SetNodeLayoutProp {
                            node_id: NodeId::new(cid.to_string()),
                            property: "width".to_string(),
                            value: LayoutPropValue::Keyword("fit_content".to_string()),
                        });
                    }
                    if wrap {
                        cmds.push(EditorCommand::SetNodeLayoutProp {
                            node_id: NodeId::new(cid.to_string()),
                            property: "textGrowth".to_string(),
                            value: LayoutPropValue::Keyword("auto".to_string()),
                        });
                    }
                }
                continue;
            }
            let Some(cid) = c.get("id").and_then(Value::as_str) else {
                continue;
            };
            let Some(cr) = rects.get(cid) else {
                continue;
            };
            // Already bounded to its block (fill + wrap) → nothing to correct.
            let fill = c.get("width").and_then(Value::as_str) == Some("fill_container");
            let wrap = c
                .get("textGrowth")
                .and_then(Value::as_str)
                .is_some_and(|g| g.starts_with("fixed-width"));
            if fill && wrap {
                continue;
            }
            // Wider than the block, OR its right edge past the block's right
            // edge (a sibling pushed it out — combined overflow the width-only
            // check misses: a 36px avatar + a fit name inside a 116px row).
            let past_right = cr.x + cr.w > parent_x + parent_w + TEXT_OVERFLOW_EPS;
            if cr.w > parent_w + TEXT_OVERFLOW_EPS || past_right {
                cmds.push(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(cid.to_string()),
                    property: "width".to_string(),
                    value: LayoutPropValue::Keyword("fill_container".to_string()),
                });
                cmds.push(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(cid.to_string()),
                    property: "textGrowth".to_string(),
                    value: LayoutPropValue::Keyword("fixed-width".to_string()),
                });
            }
        }
    }
    for c in children(v) {
        collect_text_overflow_fixes(c, rects, cmds);
    }
}

/// A NON-TEXT child that resolved wider than its flex parent → retarget it to
/// `fill_container` so it shares the row instead of spilling across the design.
/// Two authored shapes trip this:
/// - a NUMERIC width bigger than the parent (an 800px avatar bar in a ~550px
///   row — measured on a loop run);
/// - a `fit_content` container whose max-content is rigid — fit never shrinks,
///   so an icon+text pair inside an 80px card paints over its siblings
///   (measured: a hero's "Brewing now" chip stack). Retargeting to
///   `fill_container` gives it `min:0` shrink; the text inside then overflows
///   ITS block and the text-overflow fixer wraps it on the NEXT loop round —
///   the detectors converge as a chain.
///
/// Text children are handled by [`collect_text_overflow_fixes`]; `clipContent`
/// parents crop on purpose — skipped.
/// An IMAGE child whose numeric size exceeds its parent's DECLARED numeric
/// size. jian grows the parent to contain it instead of overflowing, so the
/// resolved-rect fixers above see nothing wrong — but the design's declared
/// intent (a 42px avatar strip, a 170px card cover) is destroyed by the
/// inflation (measured: a 400x300 enrichment image blew a music card open,
/// test0711-22; a 300px headshot blew a 42px avatar strip, test0711-1).
/// Fitting the image to its slot is a CONTRACT repair: the slot's size is
/// the design decision, the image serves it.
const IMAGE_INFLATION_SLACK: f64 = 8.0;

fn collect_oversized_image_fixes(v: &Value, cmds: &mut Vec<EditorCommand>) {
    let declared_w = fixed_width(v);
    let declared_h = match v.get("height") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    };
    if declared_w.is_some() || declared_h.is_some() {
        for c in children(v) {
            if c.get("type").and_then(Value::as_str) != Some("image") {
                continue;
            }
            let Some(cid) = c.get("id").and_then(Value::as_str) else {
                continue;
            };
            let child_w = fixed_width(c);
            let child_h = match c.get("height") {
                Some(Value::Number(n)) => n.as_f64(),
                Some(Value::String(s)) => s.parse::<f64>().ok(),
                _ => None,
            };
            let too_wide = matches!((child_w, declared_w), (Some(cw), Some(pw)) if cw > pw + IMAGE_INFLATION_SLACK);
            let too_tall = matches!((child_h, declared_h), (Some(ch), Some(ph)) if ch > ph + IMAGE_INFLATION_SLACK);
            if too_wide || too_tall {
                for property in ["width", "height"] {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(cid.to_string()),
                        property: property.to_string(),
                        value: LayoutPropValue::Keyword("fill_container".to_string()),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_oversized_image_fixes(c, cmds);
    }
}

fn collect_frame_overflow_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if matches!(layout_str(v), Some("vertical" | "horizontal")) && !clips {
        if let Some((parent_x, pw)) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| (r.x, r.w))
        {
            for c in children(v) {
                if c.get("type").and_then(Value::as_str) == Some("text") {
                    continue;
                }
                // An authored x/y is ABSOLUTE placement — a full-bleed bottom
                // nav (x:0, width:root) deliberately overrides its wrapper's
                // padding, and an overlay badge sits where it was pinned.
                // Absolute children aren't flex participants; retargeting
                // them to fill_container un-bleeds/unpins them (measured: a
                // normalized mobile nav lost its edge-to-edge width).
                if has_authored_position(c) {
                    continue;
                }
                // Numeric, fit_content, or ABSENT width (auto hugs like fit —
                // the ATELIER name stacks carried no width key at all and
                // slipped past a fixed/fit-only gate while their tails sat
                // 21px into the next column).
                let resizable = fixed_width(c).is_some()
                    || c.get("width").and_then(Value::as_str) == Some("fit_content")
                    || c.get("width").is_none();
                if !resizable {
                    continue;
                }
                let Some(cid) = c.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let Some(cr) = rects.get(cid) else {
                    continue;
                };
                // Wider than the parent, OR its right edge past the parent's
                // right edge — a leading sibling pushed it out (a 36px avatar
                // + a fit name stack inside a 120px cell put the stack's tail
                // 21px into the NEXT column; the width-only check saw
                // 93 < 120 and acquitted it — measured, ATELIER). The
                // right-edge branch is CONTAINERS-only: a 16px icon nudged
                // 2px out by its label must not be stretched to fill.
                let is_container = matches!(
                    c.get("type").and_then(Value::as_str),
                    Some("frame" | "group")
                );
                let past_right = is_container && cr.x + cr.w > parent_x + pw + TEXT_OVERFLOW_EPS;
                if (cr.w > pw + TEXT_OVERFLOW_EPS || past_right) && pw > 1.0 {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(cid.to_string()),
                        property: "width".to_string(),
                        value: LayoutPropValue::Keyword("fill_container".to_string()),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_frame_overflow_fixes(c, rects, cmds);
    }
}

/// Default gap injected into a geometry-proven jammed data row.
const ROW_GAP_FIX: f64 = 16.0;

/// GEOMETRY-driven column-gap repair — the name-blind big brother of
/// `table_repair::ensure_table_column_gap`. A row qualifies when the REAL
/// layout proves every adjacent pair of its ≥3 frame cells touches (<3px
/// breathing) and the cells carry text — "Oct 24, 2024"+"42" reading as
/// "202442" regardless of how many unnamed wrappers bury the table (measured:
/// rows nested TWO wrapper levels below the table-named frame slipped past
/// the name gate). Flush segmented controls stay safe: those are 2-3 equal
/// small children, gated out by the ≥3-cells + row-cell height + text checks.
fn collect_row_gap_fixes(v: &Value, rects: &HashMap<String, Rect>, cmds: &mut Vec<EditorCommand>) {
    // `< 8.0`, not `<= 0.0`: a 1-7px authored gap still resolves as touching
    // (the jam proof below requires <3px breathing anyway), and an
    // exactly-1.0 hairline gap slipped through every gate (measured).
    if layout_str(v) == Some("horizontal") && num(v, "gap") < 8.0 {
        let kids = children(v);
        let frame_kids: Vec<&Value> = kids
            .iter()
            .filter(|c| {
                matches!(
                    c.get("type").and_then(Value::as_str),
                    Some("frame" | "group")
                )
            })
            .collect();
        // TWO text-bearing frame columns jammed at 0px (a date column against
        // a details stack) are the two-column form of the same defect. A
        // space_between pair USED to be excluded ("it separates itself") —
        // but a fill_container descendant eats every px of slack, so a
        // distributed top bar resolves its title flush against its search
        // box with nothing left to distribute (measured). Geometry proof
        // overrules the keyword: if they TOUCH, they need a gap; the
        // row-overfull fixer absorbs the added width on the next round.
        // A row whose cells are ALL `fill_container` distributes its space
        // by construction — the fills touching is the layout working as
        // built (a normalized bottom nav's segmented items), not a jam. A
        // row with at least one RIGID cell that still resolves flush is a
        // real jam (a fixed date column against a fill details stack, or
        // two fit blocks whose fill grandchild ate the slack).
        let all_fill = frame_kids
            .iter()
            .all(|c| c.get("width").and_then(Value::as_str) == Some("fill_container"));
        // TWO text-bearing frame columns jammed at 0px are the two-column
        // form of the defect. A space_between pair used to be excluded ("it
        // separates itself") — but a fill descendant eats every px of slack,
        // so a distributed top bar resolves its title flush against its
        // search box with nothing left to distribute (measured). Geometry
        // proof overrules the keyword; the row-overfull fixer absorbs the
        // added width on the next round.
        let enough_cells = !all_fill && frame_kids.len() >= 2;
        if enough_cells {
            let rects_of: Vec<Option<&Rect>> = frame_kids
                .iter()
                .map(|c| {
                    c.get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| rects.get(id))
                })
                .collect();
            let all_resolved = rects_of.iter().all(|r| r.is_some_and(|r| r.w > 0.0));
            let row_cell_like = rects_of
                .iter()
                .flatten()
                .all(|r| r.h <= ROW_CELL_MAX_H && r.h > 0.0);
            let all_jammed = all_resolved
                && rects_of.windows(2).all(|p| {
                    let (a, b) = (p[0].unwrap(), p[1].unwrap());
                    (b.x - (a.x + a.w)) < SIBLING_JAM_GAP
                });
            let texty = frame_kids.iter().filter(|c| bears_text(c)).count() >= 2;
            if all_resolved && row_cell_like && all_jammed && texty {
                if let Some(id) = v.get("id").and_then(Value::as_str) {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "gap".to_string(),
                        value: LayoutPropValue::Number(ROW_GAP_FIX),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_row_gap_fixes(c, rects, cmds);
    }
}

/// Max tolerated resolved height delta inside a KPI/stat card row.
const CARD_ROW_HEIGHT_EPS: f64 = 6.0;

/// A horizontal row of painted KPI/stat cards whose authored card heights all
/// hug content but whose real layout is visibly ragged. Equalize by making each
/// card fill the row cross-axis; jian resolves fill-of-hug to the row content
/// height, guarded by `real_layout_fill_of_hug_parent_resolves_to_content_not_collapse`.
fn collect_card_row_height_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    in_table: bool,
) {
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if layout_str(v) == Some("horizontal") && !clips && !in_table {
        let kids = children(v);
        if kids.len() >= 3
            && kids.iter().all(is_colored_frame_card)
            && kids.iter().all(child_height_is_hug_or_unset)
        {
            let kid_rects: Vec<&Rect> = kids
                .iter()
                .filter_map(|c| {
                    c.get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| rects.get(id))
                })
                .collect();
            if kid_rects.len() == kids.len() {
                let min_h = kid_rects.iter().map(|r| r.h).fold(f64::INFINITY, f64::min);
                let max_h = kid_rects
                    .iter()
                    .map(|r| r.h)
                    .fold(f64::NEG_INFINITY, f64::max);
                if max_h - min_h >= CARD_ROW_HEIGHT_EPS {
                    for c in kids {
                        if let Some(id) = c.get("id").and_then(Value::as_str) {
                            cmds.push(EditorCommand::SetNodeLayoutProp {
                                node_id: NodeId::new(id.to_string()),
                                property: "height".to_string(),
                                value: LayoutPropValue::Keyword("fill_container".to_string()),
                            });
                        }
                    }
                }
            }
        }
    }

    let child_in_table = in_table || is_table_shape(v);
    for c in children(v) {
        collect_card_row_height_fixes(c, rects, cmds, child_in_table);
    }
}

fn is_colored_frame_card(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("frame")
        && (fill_is_non_empty(v) || stroke_is_non_null(v))
}

fn fill_is_non_empty(v: &Value) -> bool {
    match v.get("fill") {
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Null) | None => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(_) => true,
    }
}

fn stroke_is_non_null(v: &Value) -> bool {
    v.get("stroke").is_some_and(|stroke| !stroke.is_null())
}

fn child_height_is_hug_or_unset(v: &Value) -> bool {
    match v.get("height") {
        Some(Value::String(s)) => s == "fit_content",
        Some(Value::Null) | None => true,
        _ => false,
    }
}

/// Does this node carry an authored `x` or `y` (absolute placement)?
fn has_authored_position(v: &Value) -> bool {
    v.get("x").map(|x| !x.is_null()).unwrap_or(false)
        || v.get("y").map(|y| !y.is_null()).unwrap_or(false)
}

/// Slack before a row counts as overfull — sub-8px overhangs are invisible.
const ROW_OVERFULL_EPS: f64 = 8.0;
/// Only children this wide are worth flexifying — icons / dots / dividers
/// can't meaningfully absorb a deficit.
const MIN_FLEXIFY_W: f64 = 120.0;

/// Is `v` table-shaped (≥2 horizontal rows of ≥3 cells)? Overfull TABLE rows
/// belong to the column scaler, which keeps columns aligned across rows —
/// flexifying one row's widest column would break the vertical alignment.
fn is_table_shape(v: &Value) -> bool {
    layout_str(v) != Some("horizontal")
        && children(v)
            .iter()
            .filter(|r| layout_str(r) == Some("horizontal") && children(r).len() >= 3)
            .count()
            >= 2
}

/// A horizontal row whose children's RESOLVED widths + gaps sum wider than
/// its resolved inner width. No single child is wider than the row — the
/// per-child fixers are blind to this — but the row is overfull: children
/// overlap mid-row and the tail child clips at the edge (measured: a top bar
/// whose serif title block + 280px search + date + actions summed ~1110px in
/// an ~876px row — the title ran INTO the search box and the CTA button
/// clipped at the page edge). Repair: retarget the widest rigid child
/// (numeric ≥120 or `fit_content` resolving ≥120) to `fill_container` —
/// flex min-size 0 lets it absorb the deficit, and the loop's next round
/// re-resolves, chaining into nested rows until the row fits.
fn collect_row_overfull_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    in_table: bool,
) {
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if layout_str(v) == Some("horizontal") && !clips && !in_table {
        if let Some(row) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        {
            let inner = row.w - horizontal_padding(v);
            // Absolute children don't consume flex space — exclude them
            // from both the sum and the flexify candidates.
            let kids: Vec<&Value> = children(v)
                .iter()
                .filter(|c| !has_authored_position(c))
                .collect();
            let kid_rects: Vec<Option<&Rect>> = kids
                .iter()
                .map(|c| {
                    c.get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| rects.get(id))
                })
                .collect();
            if inner > 1.0 && !kids.is_empty() && kid_rects.iter().all(Option::is_some) {
                if crate::chip_repair::all_children_are_pill_chips(&kids) {
                    if let Some(row_id) = v.get("id").and_then(Value::as_str) {
                        cmds.push(EditorCommand::SetNodeLayoutProp {
                            node_id: NodeId::new(row_id.to_string()),
                            property: "clipContent".to_string(),
                            value: LayoutPropValue::Bool(true),
                        });
                    }
                    return;
                }
                let gap = num(v, "gap");
                let sum: f64 = kid_rects.iter().flatten().map(|r| r.w).sum::<f64>()
                    + gap * (kids.len().saturating_sub(1)) as f64;
                // NARROW-CARD anatomy guard, independent of the text measure:
                // a display value + a painted chip can't share a ~200px line
                // even when a lossy measure claims they fit (the estimate
                // backend under-reads 40px display digits; the PAINT
                // overlapped — measured). Reference metric cards stack them.
                let stacked = if inner <= 260.0 {
                    let before = cmds.len();
                    stack_overfull_value_chip_row(v, &kids, rects, cmds);
                    cmds.len() > before
                } else {
                    false
                };
                if !stacked && sum > inner + ROW_OVERFULL_EPS {
                    // Widest rigid child ≥120px, containers before text.
                    let candidate = kids
                        .iter()
                        .zip(kid_rects.iter().flatten())
                        .filter(|(c, r)| {
                            let rigid = fixed_width(c).is_some()
                                || c.get("width").and_then(Value::as_str) == Some("fit_content")
                                || c.get("width").is_none();
                            let texty = c.get("type").and_then(Value::as_str) == Some("text");
                            rigid && !texty && r.w >= MIN_FLEXIFY_W
                        })
                        .max_by(|a, b| a.1.w.total_cmp(&b.1.w));
                    if let Some((c, _)) = candidate {
                        if let Some(cid) = c.get("id").and_then(Value::as_str) {
                            cmds.push(EditorCommand::SetNodeLayoutProp {
                                node_id: NodeId::new(cid.to_string()),
                                property: "width".to_string(),
                                value: LayoutPropValue::Keyword("fill_container".to_string()),
                            });
                        }
                    } else {
                        stack_overfull_value_chip_row(v, &kids, rects, cmds);
                    }
                }
            }
        }
    }
    let table = is_table_shape(v);
    for c in children(v) {
        collect_row_overfull_fixes(c, rects, cmds, table);
    }
}

/// Dead-end branch of the overfull repair: the row has NO flexify candidate
/// (a display-size TEXT value can't shrink, a painted CHIP must hug). A KPI
/// card's bottom row — a 40px "$48,920" beside a "+8.2%" trend chip in a
/// ~180px card — overflows with nothing to give (measured: the chip's tinted
/// box painted OVER the value's tail). The design-correct repair is the
/// reference metric-card anatomy: value on its own line, change chip BELOW.
/// Applies only to the exact [display text, small painted chip] pair.
fn stack_overfull_value_chip_row(
    v: &Value,
    kids: &[&Value],
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    if kids.len() != 2 {
        return;
    }
    let is_display_text = |c: &Value| {
        c.get("type").and_then(Value::as_str) == Some("text") && num(c, "fontSize") >= 24.0
    };
    let is_chip = |c: &Value| {
        matches!(
            c.get("type").and_then(Value::as_str),
            Some("frame" | "group")
        ) && c
            .get("fill")
            .map(|f| match f {
                Value::Array(a) => !a.is_empty(),
                Value::Null => false,
                _ => true,
            })
            .unwrap_or(false)
            && c.get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id))
                .is_some_and(|r| r.h <= 44.0)
    };
    let chip = if is_display_text(kids[0]) && is_chip(kids[1]) {
        kids[1]
    } else if is_display_text(kids[1]) && is_chip(kids[0]) {
        kids[0]
    } else {
        return;
    };
    let Some(row_id) = v.get("id").and_then(Value::as_str) else {
        return;
    };
    cmds.push(EditorCommand::SetNodeLayoutProp {
        node_id: NodeId::new(row_id.to_string()),
        property: "layout".to_string(),
        value: LayoutPropValue::Keyword("vertical".to_string()),
    });
    cmds.push(EditorCommand::SetNodeLayoutProp {
        node_id: NodeId::new(row_id.to_string()),
        property: "justifyContent".to_string(),
        value: LayoutPropValue::Keyword("start".to_string()),
    });
    cmds.push(EditorCommand::SetNodeLayoutProp {
        node_id: NodeId::new(row_id.to_string()),
        property: "alignItems".to_string(),
        value: LayoutPropValue::Keyword("start".to_string()),
    });
    cmds.push(EditorCommand::SetNodeLayoutProp {
        node_id: NodeId::new(row_id.to_string()),
        property: "gap".to_string(),
        value: LayoutPropValue::Number(8.0),
    });
    if let Some(chip_id) = chip.get("id").and_then(Value::as_str) {
        // The chip hugs again — an earlier flexify (or the model) may have
        // left it fill_container, which as a stacked line would paint a
        // full-width tinted bar.
        cmds.push(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(chip_id.to_string()),
            property: "width".to_string(),
            value: LayoutPropValue::Keyword("fit_content".to_string()),
        });
    }
}

fn is_collapsed_fill_container(v: &Value, rects: &HashMap<String, Rect>) -> bool {
    if v.get("height").and_then(Value::as_str) != Some("fill_container") {
        return false;
    }
    let Some(id) = v.get("id").and_then(Value::as_str) else {
        return false;
    };
    let Some(rect) = rects.get(id) else {
        return false;
    };
    if rect.h >= COLLAPSE_H {
        return false;
    }
    // A real child with height proves the 0-height parent is a collapse.
    children(v).iter().any(|c| {
        c.get("id")
            .and_then(Value::as_str)
            .and_then(|cid| rects.get(cid))
            .map(|r| r.h >= CHILD_MIN_H)
            .unwrap_or(false)
    })
}

/// Cap on reported issues per call — enough to act on, small enough to not
/// drown the model's context.
const MAX_DIAGNOSTICS: usize = 8;

/// REPORT-mode counterpart of the fix loop: run the real jian layout over the
/// CURRENT document and describe — without fixing — what the resolved geometry
/// proves wrong (collapsed fill containers, table columns overflowing their
/// row, text overflowing its block). Attached to every `batch_design` tool
/// result (including its script-mode path, the default subagent generation
/// protocol) so the design agent SEES each batch's layout consequences
/// immediately and repairs them in-process — the deterministic analogue of
/// Pencil's per-batch `snapshot_layout` feedback, with the detection cost
/// paid in Rust instead of model turns.
pub fn geometry_diagnostics(state: &EditorState) -> Vec<String> {
    let rects = resolved_rects(state);
    let mut out = Vec::new();
    for root in state.active_children() {
        if out.len() >= MAX_DIAGNOSTICS {
            break;
        }
        if let Ok(v) = serde_json::to_value(root) {
            bottom_nav_order_diagnostic(&v, &mut out);
            collect_diagnostics(&v, &rects, &mut out);
        }
    }
    out.truncate(MAX_DIAGNOSTICS);
    out
}

/// Structure echo: on a mobile-width root, any sibling that lands AFTER the
/// bottom tab bar is a misplaced "catch-up" section (measured: MiniMax-M3
/// appended the greeting+search header after the nav). The nav-last CONTRACT
/// is deterministically repaired at finalize (`anchor_bottom_nav_last`);
/// where the late section belongs is INTENT, so this echo tells the in-loop
/// model to relocate it with `M()` instead of us guessing.
fn bottom_nav_order_diagnostic(root: &Value, out: &mut Vec<String>) {
    let width = root
        .get("width")
        .and_then(Value::as_f64)
        .unwrap_or(f64::MAX);
    if width > 480.0 {
        return;
    }
    let Some(children) = root.get("children").and_then(Value::as_array) else {
        return;
    };
    let is_nav = |c: &Value| {
        c.get("role").and_then(Value::as_str) == Some("bottom-tab-bar")
            || c.get("name").and_then(Value::as_str).is_some_and(|n| {
                let n = n.to_ascii_lowercase();
                n.contains("tab bar") || n.contains("bottom nav")
            })
    };
    let Some(nav_index) = children.iter().position(is_nav) else {
        return;
    };
    for late in children.iter().skip(nav_index + 1).filter(|c| !is_nav(c)) {
        if out.len() >= MAX_DIAGNOSTICS {
            return;
        }
        out.push(format!(
            "{}: sits AFTER the bottom tab bar — bottom navigation must be the LAST child. \
             If this is top context (greeting / search / page header), move it to the top of \
             the content with M(); do not leave it below the nav.",
            diag_label(late)
        ));
    }
}

/// `Name (id)` label for a diagnostic line.
fn diag_label(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("frame");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("?");
    format!("{name} ({id})")
}

fn collect_diagnostics(v: &Value, rects: &HashMap<String, Rect>, out: &mut Vec<String>) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    if is_collapsed_fill_container(v, rects) {
        out.push(format!(
            "{}: declared height fill_container but resolved to ~0px (its ancestor hugs) — give the ancestor chain a definite height or use fit_content here",
            diag_label(v)
        ));
    }
    if out.len() < MAX_DIAGNOSTICS {
        let resolved_size = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| (r.w, r.h));
        if let Some(line) = crate::stub_repair::empty_decorated_stub_diagnostic(v, resolved_size) {
            out.push(line);
        }
    }
    if out.len() < MAX_DIAGNOSTICS {
        for line in crate::sidebar_archetype::horizontal_navbar_archetype_diagnostics(v, |id| {
            rects.get(id).map(|r| r.w)
        }) {
            out.push(line);
            if out.len() >= MAX_DIAGNOSTICS {
                return;
            }
        }
    }
    if table_overflow_scale(v, rects).is_some() {
        out.push(format!(
            "{}: fixed column widths sum wider than the resolved row — shrink the column widths (or make columns fill_container) so they fit",
            diag_label(v)
        ));
    }
    if let Some((cols, inner)) = table_columns_exceed_width(v, rects) {
        out.push(format!(
            "{}: {cols} columns cannot fit a {inner}px row — no rescale can save this; drop columns (stack name+email in one cell) or widen the table",
            diag_label(v)
        ));
    }
    // Children overflowing a FLEX parent's resolved width. `clipContent`
    // parents are intentional croppers (scrollers, image masks) — skip them.
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if matches!(layout_str(v), Some("vertical" | "horizontal")) && !clips {
        if let Some((parent_x, pw)) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| (r.x, r.w))
        {
            for c in children(v) {
                if out.len() >= MAX_DIAGNOSTICS {
                    return;
                }
                let Some(cr) = c
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| rects.get(id))
                else {
                    continue;
                };
                // Same trigger the fixers use: wider than the parent OR (for
                // containers) a right edge pushed past the parent's.
                let is_container = matches!(
                    c.get("type").and_then(Value::as_str),
                    Some("frame" | "group")
                );
                let past_right = is_container && cr.x + cr.w > parent_x + pw + TEXT_OVERFLOW_EPS;
                if cr.w <= pw + TEXT_OVERFLOW_EPS && !past_right {
                    continue;
                }
                if crate::chip_repair::is_pill_chip(v)
                    && c.get("type").and_then(Value::as_str) == Some("text")
                {
                    continue;
                }
                if c.get("type").and_then(Value::as_str) == Some("text") {
                    out.push(format!(
                        "{}: text resolved {}px wide inside a {}px block — it overflows into siblings; set width fill_container + textGrowth fixed-width to wrap it",
                        diag_label(c),
                        cr.w.round(),
                        pw.round()
                    ));
                } else {
                    out.push(format!(
                        "{}: resolved {}px wide inside its {}px parent — it spills out; shrink its width (or use fill_container) so it fits",
                        diag_label(c),
                        cr.w.round(),
                        pw.round()
                    ));
                }
            }
        }
    }
    collect_vertical_spill_diagnostics(v, rects, out);
    collect_sibling_jam_diagnostics(v, rects, out);
    collect_starved_fill_diagnostics(v, rects, out);
    for c in children(v) {
        collect_diagnostics(c, rects, out);
    }
}

/// A fixed-height frame whose resolved CHILDREN run a LITTLE past its
/// declared height (a card estimated 156 tall whose art + two text lines
/// resolve to 165 — the artist line's bottom half vanished under the next
/// section, measured test0711-2-ds). Small overshoots grow the frame to
/// fit; big overshoots are the inflation class (content must shrink) and
/// stay with the echo above.
const GROW_TO_FIT_MIN: f64 = 4.0;
const GROW_TO_FIT_MAX_FRACTION: f64 = 0.25;

fn collect_grow_to_fit_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    if let Some(declared) = match v.get("height") {
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    } {
        if v.get("clipContent").and_then(Value::as_bool) != Some(true) && declared > 0.0 {
            if let Some(pr) = v
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id))
            {
                let children_bottom = children(v)
                    .iter()
                    .filter_map(|c| {
                        c.get("id")
                            .and_then(Value::as_str)
                            .and_then(|id| rects.get(id))
                    })
                    .map(|cr| cr.y + cr.h)
                    .fold(f64::MIN, f64::max);
                let overshoot = children_bottom - (pr.y + declared);
                if overshoot > GROW_TO_FIT_MIN && overshoot <= declared * GROW_TO_FIT_MAX_FRACTION {
                    if let Some(id) = v.get("id").and_then(Value::as_str) {
                        cmds.push(EditorCommand::UpdateNode {
                            node_id: NodeId::new(id.to_string()),
                            x: None,
                            y: None,
                            width: None,
                            height: Some((declared + overshoot).ceil() as i32),
                            name: None,
                            fill_hex: None,
                            page_id: None,
                        });
                    }
                }
            }
        }
    }
    for c in children(v) {
        collect_grow_to_fit_fixes(c, rects, cmds);
    }
}

/// A horizontal rail whose fill-width cards resolved SKINNIER than their own
/// fixed-width content (a 5-card destination rail where every fill card got a
/// ~58px share while carrying a 160px image + 188px label row — the images
/// painted as clipped slivers and every city name truncated, measured
/// test0711-1-glm). The authored fixed content is proof of the intended card
/// width, so the repair follows it: cards hug their content, the rail becomes
/// an overflowing scroller, and `space_between` (meaningless once overfull)
/// falls back to start+gap. Marking the rail `clipContent` is load-bearing —
/// it is what keeps the next round's overfull-row flexifier from flipping the
/// cards straight back to fill_container.
const RAIL_STARVE_EPS: f64 = 12.0;
const RAIL_MIN_CARDS: usize = 3;
/// A card is only STARVED when it is genuinely unusable — the measured case
/// was 58px cards around 160px photos. A card that merely CROPS an oversized
/// photo (a 400x300 plate clipped into a 170px card) looks right and is not
/// starved; widening it to the plate would blow one card across the whole
/// screen. That happened (user report 2026-07-12: "自检又违背设计意图了").
const RAIL_CARD_STARVED_W: f64 = 120.0;
/// …and the demand is only credible as an INTENT if the resulting cards still
/// read as a scroll rail (the next card peeks). A "demand" wider than this
/// share of the rail is an oversized image, not a width intent — that class
/// belongs to `collect_oversized_image_fixes`, which shrinks the image.
const RAIL_CARD_MAX_FRACTION: f64 = 0.72;

/// The widest fixed-width DESCENDANT plus the card's own side padding — the
/// width this card provably needs to show its authored content. Descendants,
/// not just direct children: a destination card's photo is often an
/// absolutely-positioned 200px plate two levels down, and that number is
/// still the card's authored width intent (measured test0711-1-glm, where a
/// direct-children-only scan saw nothing and left the cards at 79px).
fn fixed_content_demand(card: &Value) -> f64 {
    fn widest_fixed(v: &Value, depth: usize) -> f64 {
        if depth == 0 {
            return 0.0;
        }
        children(v)
            .iter()
            .map(|c| {
                fixed_width(c)
                    .unwrap_or(0.0)
                    .max(widest_fixed(c, depth - 1))
            })
            .fold(0.0_f64, f64::max)
    }
    let widest = widest_fixed(card, 3);
    if widest == 0.0 {
        0.0
    } else {
        widest + horizontal_padding(card)
    }
}

fn collect_starved_rail_card_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if layout_str(v) == Some("horizontal") && !clips {
        let cards: Vec<&Value> = children(v)
            .iter()
            .filter(|c| {
                !has_authored_position(c)
                    && c.get("width").and_then(Value::as_str) == Some("fill_container")
            })
            .collect();
        let rail_w = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| r.w)
            .unwrap_or(0.0);
        if cards.len() >= RAIL_MIN_CARDS && rail_w > 0.0 {
            let all_starved = cards.iter().all(|c| {
                let demand = fixed_content_demand(c);
                demand > 0.0
                    && demand <= rail_w * RAIL_CARD_MAX_FRACTION
                    && c.get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| rects.get(id))
                        .is_some_and(|r| {
                            r.w < RAIL_CARD_STARVED_W && demand > r.w + RAIL_STARVE_EPS
                        })
            });
            if all_starved {
                for c in &cards {
                    if let Some(id) = c.get("id").and_then(Value::as_str) {
                        // The card takes its DEMAND as a definite width, not
                        // fit_content: an absolutely-positioned photo plate is
                        // out of flow, so hugging would size the card to its
                        // text alone and starve the photo all over again
                        // (measured: hug gave 126px against a 200px plate).
                        cmds.push(EditorCommand::UpdateNode {
                            node_id: NodeId::new(id.to_string()),
                            x: None,
                            y: None,
                            width: Some(fixed_content_demand(c).round() as i32),
                            height: None,
                            name: None,
                            fill_hex: None,
                            page_id: None,
                        });
                    }
                }
                if let Some(id) = v.get("id").and_then(Value::as_str) {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "clipContent".to_string(),
                        value: LayoutPropValue::Bool(true),
                    });
                    if v.get("justifyContent").and_then(Value::as_str) == Some("space_between") {
                        cmds.push(EditorCommand::SetNodeLayoutProp {
                            node_id: NodeId::new(id.to_string()),
                            property: "justifyContent".to_string(),
                            value: LayoutPropValue::Keyword("start".to_string()),
                        });
                    }
                }
            }
        }
    }
    for c in children(v) {
        collect_starved_rail_card_fixes(c, rects, cmds);
    }
}

/// A `fill_container`-sized IMAGE inside a `layout: "none"` (absolute)
/// container — `fill_container` has no meaning without a flex parent, so
/// the engine falls back to the bitmap's own aspect and the "cover" paints
/// as a skewed strip (measured: every New Releases cover rendered as a
/// thin right-edge sliver, test0711-22 00:44). The image is pinned to its
/// parent's RESOLVED rect: x/y 0, numeric width/height.
fn collect_absolute_fill_image_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let absolute_parent = matches!(layout_str(v), Some("none"));
    if absolute_parent {
        if let Some(pr) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        {
            if pr.w > 1.0 && pr.h > 1.0 {
                for c in children(v) {
                    if c.get("type").and_then(Value::as_str) != Some("image") {
                        continue;
                    }
                    let fill_w = c.get("width").and_then(Value::as_str) == Some("fill_container");
                    let fill_h = c.get("height").and_then(Value::as_str) == Some("fill_container");
                    if !fill_w && !fill_h {
                        continue;
                    }
                    let Some(cid) = c.get("id").and_then(Value::as_str) else {
                        continue;
                    };
                    cmds.push(EditorCommand::UpdateNode {
                        node_id: NodeId::new(cid.to_string()),
                        x: Some(0),
                        y: Some(0),
                        width: Some(pr.w.round() as i32),
                        height: Some(pr.h.round() as i32),
                        name: None,
                        fill_hex: None,
                        page_id: None,
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_absolute_fill_image_fixes(c, rects, cmds);
    }
}

/// TWO images in one slot, and a photo band cropped to a sliver.
///
/// Measured (test0711-1-glm): a destination card's image wrapper was authored
/// `height: 56` and held BOTH an absolutely-positioned 200x130 frame whose
/// FILL is the photo AND a sibling image NODE carrying the same slot's photo -
/// two images stacked in one box, of which only the top 56px survived the
/// wrapper's clip. Every card's surviving 56px was that photo's sky, so five
/// different photos rendered as five identical blue bands (user report: "最近
/// 怎么经常有这种情况").
///
/// Two repairs, both provable from the authored tree:
/// 1. **One image per slot** - when a wrapper holds an image-filled frame AND
///    an image node, the image NODE wins (it is what the image pipeline fills
///    and re-searches); the duplicate fill-frame is deleted.
/// 2. **The band keeps the photo's height** - the photo the model authored for
///    the slot declares a definite height; a wrapper shorter than half of it is
///    an authoring slip, not art direction, so the wrapper grows to the photo's
///    height. (A wrapper that already fits, or crops only mildly, is left
///    alone - intentional letterboxing stays intentional.)
const IMAGE_BAND_MIN_VISIBLE_FRACTION: f64 = 0.5;
/// Tallest a grown photo band may be relative to its own width — a card photo
/// is at most mildly portrait; beyond this the "photo" is an oversized plate.
const IMAGE_BAND_MAX_ASPECT: f64 = 1.2;

fn is_image_filled_frame(v: &Value) -> bool {
    matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group" | "rectangle")
    ) && v
        .get("fill")
        .and_then(Value::as_array)
        .is_some_and(|fills| {
            fills.iter().any(|f| {
                f.get("type").and_then(Value::as_str) == Some("image")
                    || f.get("url").is_some()
                    || f.get("src").is_some()
            })
        })
}

fn fixed_height(v: &Value) -> Option<f64> {
    match v.get("height") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn collect_image_slot_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let kids = children(v);
    let image_nodes: Vec<&Value> = kids
        .iter()
        .filter(|c| c.get("type").and_then(Value::as_str) == Some("image"))
        .collect();
    let filled_frames: Vec<&Value> = kids.iter().filter(|c| is_image_filled_frame(c)).collect();

    // (1) One image per slot: the node wins, the duplicate fill-frame goes.
    // Only a CHILDLESS plate qualifies — a filled frame that carries content
    // (a hero with a headline on it) is a real container, not a stray twin.
    if !image_nodes.is_empty() {
        for frame in filled_frames.iter().filter(|f| children(f).is_empty()) {
            if let Some(id) = frame.get("id").and_then(Value::as_str) {
                cmds.push(EditorCommand::DeleteNode {
                    node_id: NodeId::new(id.to_string()),
                    page_id: None,
                });
            }
        }
    }

    // (2) The band keeps the photo's height. The photo's own declared height
    // is the model's intent for the slot; take it from whichever carrier is
    // surviving repair (1).
    if let Some(band) = fixed_height(v) {
        let photo_h = if image_nodes.is_empty() {
            filled_frames
                .iter()
                .filter_map(|f| fixed_height(f))
                .fold(0.0, f64::max)
        } else {
            image_nodes
                .iter()
                .chain(filled_frames.iter())
                .filter_map(|f| fixed_height(f))
                .fold(0.0, f64::max)
        };
        // The photo's declared height is only a credible BAND height if the
        // band would still read as a card photo. A plate taller than its own
        // band is wide is an oversized image (a 400x300 plate in a phone
        // card), not art direction — growing the band to it left Deals cards
        // with a wall of empty space (user report 2026-07-12).
        let band_w = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| r.w)
            .unwrap_or(0.0);
        let plausible = band_w > 0.0 && photo_h <= band_w * IMAGE_BAND_MAX_ASPECT;
        if photo_h > 0.0 && plausible && band < photo_h * IMAGE_BAND_MIN_VISIBLE_FRACTION {
            if let Some(id) = v.get("id").and_then(Value::as_str) {
                cmds.push(EditorCommand::UpdateNode {
                    node_id: NodeId::new(id.to_string()),
                    x: None,
                    y: None,
                    width: None,
                    height: Some(photo_h.round() as i32),
                    name: None,
                    fill_hex: None,
                    page_id: None,
                });
            }
        }
    }
    for c in kids {
        collect_image_slot_fixes(c, rects, cmds);
    }
}

/// A frame declaring a NUMERIC height but resolving MUCH taller — an
/// oversized child inflated it (jian grows the parent instead of letting the
/// child spill, so no edge ever crosses another and the width-overflow echo
/// stays blind). Measured (GLM-5.2 test0711-1.op): a 300px image inside a
/// declared-42px "Avatar" strip blew the strip — and the whole header —
/// to 300px. A generous slack keeps line-height rounding out of the report;
/// a real defect overshoots by multiples.
const VERTICAL_SPILL_SLACK: f64 = 24.0;

fn collect_vertical_spill_diagnostics(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    if v.get("clipContent").and_then(Value::as_bool) == Some(true) {
        return;
    }
    let Some(declared) = v.get("height").and_then(Value::as_f64) else {
        return;
    };
    let Some(resolved) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|r| r.h)
    else {
        return;
    };
    if resolved <= declared + VERTICAL_SPILL_SLACK {
        return;
    }
    let culprit = children(v)
        .iter()
        .filter_map(|c| {
            let cr = c
                .get("id")
                .and_then(Value::as_str)
                .and_then(|id| rects.get(id))?;
            (cr.h > declared + VERTICAL_SPILL_SLACK).then(|| (diag_label(c), cr.h))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let blame = culprit
        .map(|(label, h)| {
            format!(
                " — its child {label} is {}px tall and inflates it",
                h.round()
            )
        })
        .unwrap_or_default();
    out.push(format!(
        "{}: declared {}px tall but resolved {}px{blame}; shrink the oversized content to fit the declared height (or grow the parent on purpose)",
        diag_label(v),
        declared.round(),
        resolved.round()
    ));
}

/// A text-bearing `fill_container` child squeezed to a sliver in a horizontal
/// row — rigid siblings ate the width, the flex column resolved to a few px,
/// and its text wraps one glyph per line into a tower that blows the row's
/// height (measured: a 6px email column inflated its table rows to 432px).
/// Nothing OVERFLOWS in this failure — width-exceeds-parent checks are blind
/// to it — so the starvation itself is the report.
const STARVED_FILL_W: f64 = 24.0;

fn collect_starved_fill_diagnostics(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    if out.len() >= MAX_DIAGNOSTICS || layout_str(v) != Some("horizontal") {
        return;
    }
    let Some(pw) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|r| r.w)
    else {
        return;
    };
    if pw < 200.0 {
        return; // a narrow chip/control row can't meaningfully starve anyone
    }
    for c in children(v) {
        if out.len() >= MAX_DIAGNOSTICS {
            return;
        }
        if c.get("width").and_then(Value::as_str) != Some("fill_container") || !bears_text(c) {
            continue;
        }
        let Some(cr) = c
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        else {
            continue;
        };
        if cr.w > 0.0 && cr.w < STARVED_FILL_W {
            out.push(format!(
                "{}: fill_container column starved to {}px in a {}px row — its fixed-width siblings eat the space and its text shreds vertically; shrink the fixed column widths so this one gets room",
                diag_label(c),
                cr.w.round(),
                pw.round()
            ));
        }
    }
}

/// Two adjacent text-bearing siblings count as JAMMED below this many px of
/// resolved horizontal breathing room ("Oct 24, 2024" + "42" reading as
/// "202442" — measured on a gap-less table row).
const SIBLING_JAM_GAP: f64 = 3.0;
/// Siblings whose resolved rects overlap deeper than this are reported as
/// OVERLAPPING (painted on top of each other), not merely jammed.
const SIBLING_OVERLAP_EPS: f64 = 2.0;
/// The JAM rule targets ROW cells ("Oct 24" + "42" reading as one word) —
/// two PAGE-level columns (an app-shell's sidebar and content) legitimately
/// touch. Only pairs where at least one side is row-cell sized qualify.
const ROW_CELL_MAX_H: f64 = 120.0;

/// Does this subtree carry visible text? A spacer / icon / image sibling
/// touching its neighbor is normal; two TEXT columns touching is a jam.
fn bears_text(v: &Value) -> bool {
    if v.get("type").and_then(Value::as_str) == Some("text") {
        return true;
    }
    children(v).iter().any(bears_text)
}

/// A JAM participant must be a text-bearing CONTAINER cell. Two bare `text`
/// siblings set tight on purpose ("$29"+"/mo", value+unit pairs) are
/// typography, not a data-column jam — measured false positive on a pricing
/// card's price row.
fn is_cell_like(v: &Value) -> bool {
    matches!(
        v.get("type").and_then(Value::as_str),
        Some("frame" | "group")
    ) && bears_text(v)
}

/// Report adjacent siblings of a horizontal FLEX row that resolved jammed
/// (text columns touching) or overlapping. Report-only: flush layouts are
/// sometimes intentional (joined button groups), so the model — not a fixer —
/// arbitrates, and the name-gated table pass stays the deterministic repairer.
fn collect_sibling_jam_diagnostics(
    v: &Value,
    rects: &HashMap<String, Rect>,
    out: &mut Vec<String>,
) {
    let Some(layout @ ("horizontal" | "vertical")) = layout_str(v) else {
        return;
    };
    if out.len() >= MAX_DIAGNOSTICS {
        return;
    }
    let kids = children(v);
    for pair in kids.windows(2) {
        if out.len() >= MAX_DIAGNOSTICS {
            return;
        }
        let (a, b) = (&pair[0], &pair[1]);
        let (Some(ra), Some(rb)) = (
            a.get("id")
                .and_then(Value::as_str)
                .and_then(|i| rects.get(i)),
            b.get("id")
                .and_then(Value::as_str)
                .and_then(|i| rects.get(i)),
        ) else {
            continue;
        };
        if ra.w <= 0.0 || rb.w <= 0.0 {
            continue;
        }
        let breathing = if layout == "vertical" {
            rb.y - (ra.y + ra.h)
        } else {
            rb.x - (ra.x + ra.w)
        };
        // Intentional OVERLAY, not an accident: one sibling's center inside
        // the other's box — a number set on a ring (ellipse + short text), a
        // corner badge on an avatar. Layout can't express children for an
        // ellipse, so models stack a sibling on purpose; don't report it.
        let center_inside = |inner: &Rect, outer: &Rect| {
            let (cx, cy) = (inner.x + inner.w / 2.0, inner.y + inner.h / 2.0);
            cx > outer.x && cx < outer.x + outer.w && cy > outer.y && cy < outer.y + outer.h
        };
        // An ellipse + a SHORT text sibling is the ring/badge stack by
        // construction — an ellipse can't carry children, so models sibling
        // the number on purpose. The center-inside test alone misses the
        // boundary case (a "2" whose center lands EXACTLY on the ring's top
        // edge — measured, p44), so the pair's shape is the intent signal.
        let is_ellipse = |v: &Value| v.get("type").and_then(Value::as_str) == Some("ellipse");
        let is_short_text = |v: &Value| {
            v.get("type").and_then(Value::as_str) == Some("text")
                && v.get("content")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.trim().chars().count() <= 4)
        };
        let ring_badge = (is_ellipse(a) && is_short_text(b)) || (is_ellipse(b) && is_short_text(a));
        let overlay = ring_badge || center_inside(ra, rb) || center_inside(rb, ra);
        let all_fill_cells = crate::chip_repair::is_fill_container_frame(a)
            && crate::chip_repair::is_fill_container_frame(b);
        if breathing < -SIBLING_OVERLAP_EPS && !overlay {
            let axis = if layout == "vertical" { "stack" } else { "row" };
            out.push(format!(
                "{} and {}: siblings OVERLAP by {}px — their resolved boxes collide in the {axis}; resize or add spacing so they do not paint on top of each other",
                diag_label(a),
                diag_label(b),
                (-breathing).round()
            ));
        } else if layout == "horizontal"
            && breathing < SIBLING_JAM_GAP
            && !all_fill_cells
            && ra.h.min(rb.h) <= ROW_CELL_MAX_H
            && is_cell_like(a)
            && is_cell_like(b)
        {
            out.push(format!(
                "{} and {}: text columns touch (only {}px apart) — their contents read as one word; add a gap on the row (e.g. gap: 16)",
                diag_label(a),
                diag_label(b),
                breathing.max(0.0).round()
            ));
        }
    }
}

fn collect_scale_ops(v: &Value, rects: &HashMap<String, Rect>, ops: &mut Vec<EditorCommand>) {
    if let Some(scale) = table_overflow_scale(v, rects) {
        // Apply the same scale to EVERY row's fixed cells (columns stay aligned)
        // and to each row's gap.
        for row in children(v) {
            if layout_str(row) != Some("horizontal") {
                continue;
            }
            let cells = children(row);
            if cells.len() < 3 {
                continue;
            }
            for cell in cells {
                if let (Some(w), Some(id)) =
                    (fixed_width(cell), cell.get("id").and_then(Value::as_str))
                {
                    ops.push(EditorCommand::UpdateNode {
                        node_id: NodeId::new(id.to_string()),
                        x: None,
                        y: None,
                        width: Some((w * scale).round() as i32),
                        height: None,
                        name: None,
                        fill_hex: None,
                        page_id: None,
                    });
                }
            }
            let gap = num(row, "gap");
            if gap > 0.0 {
                if let Some(id) = row.get("id").and_then(Value::as_str) {
                    ops.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "gap".to_string(),
                        value: LayoutPropValue::Number((gap * scale).round()),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_scale_ops(c, rects, ops);
    }
}

/// If `v` is a table-shaped container (≥2 horizontal rows of ≥3 cells — the
/// STRUCTURE is the gate, not the name; "VIP Client List" shipped a starved
/// 6px email column because a name gate only trusted `table`-named frames)
/// whose fixed columns crowd out the rows' RESOLVED inner width, return the
/// scale factor (< 1.0) to apply to its fixed columns + gap. Each row is
/// measured against its own inner width (rect minus padding) and each
/// text-bearing flex column reserves a readable floor; the WORST row decides,
/// so uneven header/data column sets can't hide the deficit. `None` when the
/// shape isn't a table or everything fits.
fn table_overflow_scale(v: &Value, rects: &HashMap<String, Rect>) -> Option<f64> {
    if layout_str(v) == Some("horizontal") {
        return None;
    }
    let rows: Vec<&Value> = children(v)
        .iter()
        .filter(|r| layout_str(r) == Some("horizontal") && children(r).len() >= 3)
        .collect();
    // Need at least a header + one data row to be a real table.
    if rows.len() < 2 {
        return None;
    }
    let mut worst: Option<f64> = None;
    for row in rows {
        let cells = children(row);
        let n_gaps = (cells.len() - 1) as f64;
        let gap = num(row, "gap");
        let mut fixed_sum = 0.0;
        let mut flex_floor = 0.0;
        for cell in cells {
            match fixed_width(cell) {
                Some(w) => fixed_sum += w,
                // fill_container / fit_content — reserve room for it.
                None => {
                    flex_floor += if bears_text(cell) {
                        MIN_FILL_TEXT_COL
                    } else {
                        MIN_FILL_COL
                    }
                }
            }
        }
        if fixed_sum <= 0.0 {
            continue; // all-flex row can't overflow via fixed widths
        }
        let Some(row_id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(row_w) = rects.get(row_id).map(|r| r.w - horizontal_padding(row)) else {
            continue;
        };
        if row_w <= 1.0 {
            continue;
        }
        // Minimum width the row NEEDS: fixed columns + gaps + the flex floors.
        // If that already fits the resolved inner width, this row is fine.
        let needed = fixed_sum + gap * n_gaps + flex_floor;
        if needed <= row_w + OVERFLOW_EPS {
            continue;
        }
        // Scale the fixed budget (columns + gaps) to fit alongside the floors.
        let fixed_budget = (row_w - flex_floor) * FIT_MARGIN;
        let scalable = fixed_sum + gap * n_gaps;
        if scalable <= 0.0 {
            continue;
        }
        // UNSALVAGEABLE by scaling: even at MIN_SCALE the fixed budget can't
        // fit beside the flex floors (a 6-column table crammed into a
        // half-width pane — its five text-bearing fill columns alone need
        // more than the row offers). Scaling anyway is worse than useless:
        // the geometry loop re-applies the scale EVERY round, compounding
        // 0.35ⁿ and crushing the column gap to a sliver (24→3, measured).
        // Leave the row alone and let the too-many-columns diagnostic speak.
        if fixed_budget / scalable < MIN_SCALE {
            continue;
        }
        let scale = (fixed_budget / scalable).clamp(MIN_SCALE, 1.0);
        if scale < 1.0 - 0.001 {
            worst = Some(worst.map_or(scale, |w: f64| w.min(scale)));
        }
    }
    worst
}

#[cfg(test)]
#[path = "geometry_chip_private_tests.rs"]
mod chip_private_tests;
/// A ROUNDED, PAINTED card whose child's resolved rect pokes past the card's
/// own bounds (a heart-rate sparkline path hanging out of the card's right
/// rounded edge — measured). Rounded cards crop by convention (the CSS
/// border-radius + overflow expectation); set `clipContent` so the overshoot
/// crops at the radius instead of painting outside the card. Geometry-proven
/// and one-way (never un-clips); plain unrounded wrappers are left alone —
/// their overflows belong to the resize fixers.
const CARD_CLIP_RADIUS_MIN: f64 = 8.0;
const CARD_CLIP_OVERSHOOT_EPS: f64 = 2.0;

fn collect_card_overflow_clips(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let painted = v
        .get("fill")
        .map(|f| match f {
            Value::Array(a) => !a.is_empty(),
            Value::Null => false,
            _ => true,
        })
        .unwrap_or(false);
    let rounded = num(v, "cornerRadius") >= CARD_CLIP_RADIUS_MIN;
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if painted && rounded && !clips {
        if let Some(pr) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
        {
            // ANY descendant counts — the overshooting sparkline sits two
            // wrappers below the card, not as a direct child.
            fn any_descendant_overshoots(
                v: &Value,
                rects: &HashMap<String, Rect>,
                pr: &Rect,
            ) -> bool {
                children(v).iter().any(|c| {
                    let out = c
                        .get("id")
                        .and_then(Value::as_str)
                        .and_then(|id| rects.get(id))
                        .is_some_and(|cr| {
                            cr.x + cr.w > pr.x + pr.w + CARD_CLIP_OVERSHOOT_EPS
                                || cr.y + cr.h > pr.y + pr.h + CARD_CLIP_OVERSHOOT_EPS
                                || cr.x < pr.x - CARD_CLIP_OVERSHOOT_EPS
                                || cr.y < pr.y - CARD_CLIP_OVERSHOOT_EPS
                        });
                    out || any_descendant_overshoots(c, rects, pr)
                })
            }
            let overshoots = any_descendant_overshoots(v, rects, pr);
            if overshoots {
                if let Some(id) = v.get("id").and_then(Value::as_str) {
                    cmds.push(EditorCommand::SetNodeLayoutProp {
                        node_id: NodeId::new(id.to_string()),
                        property: "clipContent".to_string(),
                        value: LayoutPropValue::Bool(true),
                    });
                }
            }
        }
    }
    for c in children(v) {
        collect_card_overflow_clips(c, rects, cmds);
    }
}

/// Table-shaped container whose TEXT-BEARING flex columns alone (at their
/// readable floors) exceed the row's inner width — unsalvageable by any
/// rescale; the design needs fewer columns. Returns `(columns, inner_px)`
/// of the worst row for the diagnostic.
fn table_columns_exceed_width(v: &Value, rects: &HashMap<String, Rect>) -> Option<(usize, i64)> {
    if layout_str(v) == Some("horizontal") {
        return None;
    }
    let rows: Vec<&Value> = children(v)
        .iter()
        .filter(|r| layout_str(r) == Some("horizontal") && children(r).len() >= 3)
        .collect();
    if rows.len() < 2 {
        return None;
    }
    for row in rows {
        let cells = children(row);
        let mut floor_sum = 0.0;
        for cell in cells {
            floor_sum += match fixed_width(cell) {
                Some(w) => w * MIN_SCALE,
                None => {
                    if bears_text(cell) {
                        MIN_FILL_TEXT_COL
                    } else {
                        MIN_FILL_COL
                    }
                }
            };
        }
        let Some(row_id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(inner) = rects.get(row_id).map(|r| r.w - horizontal_padding(row)) else {
            continue;
        };
        if inner > 1.0 && floor_sum > inner {
            return Some((cells.len(), inner.round() as i64));
        }
    }
    None
}

#[cfg(test)]
#[path = "geometry_validation_tests.rs"]
mod tests;
