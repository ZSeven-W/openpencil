//! Overflow fix collectors: collapsed containers, text overflow, oversized
//! images and frame overflow.

use super::*;

/// A container resolves to ~0 height while declaring `fill_container`. With the
/// engine resolving fill-of-hug to content size on both axes, this is the ONLY
/// collapse repairer left (the tree-shape `fix_circular_fill_height` demoter is
/// retired) — and it acts on proof from the real layout, never on tree shape.
pub(super) const COLLAPSE_H: f64 = 6.0;
/// A child must carry real height for the parent's 0-height to read as a collapse
/// (not an intentionally-empty spacer).
pub(super) const CHILD_MIN_H: f64 = 12.0;

pub(super) fn collect_collapse_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
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
pub(super) const TEXT_OVERFLOW_EPS: f64 = 2.0;

fn collect_pill_text_overflow_clip_fix(
    pill: &Value,
    text_child: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    let has_numeric_width = pill
        .get("width")
        .and_then(|w| match w {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        })
        .is_some_and(|w| w > 0.0);
    if !has_numeric_width {
        return;
    }
    let already_clips = pill.get("clipContent").and_then(Value::as_bool) == Some(true);
    if already_clips {
        return;
    }
    let Some(pill_id) = pill.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(pill_rect) = rects.get(pill_id) else {
        return;
    };
    let Some(text_id) = text_child.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(text_rect) = rects.get(text_id) else {
        return;
    };
    if text_rect.w > pill_rect.w + TEXT_OVERFLOW_EPS {
        cmds.push(EditorCommand::SetNodeLayoutProp {
            node_id: NodeId::new(pill_id.to_string()),
            property: "clipContent".to_string(),
            value: LayoutPropValue::Bool(true),
        });
    }
}

/// A text node whose RESOLVED width exceeds its parent block's — the parent is a
/// constrained (`fill_container` / fixed) block whose `min_size: 0` lets it shrink
/// BELOW its `fit_content` text, so the text overflows into the next column
/// (measured: a 260px sidebar's schedule rows painted the client name over the
/// appointment time). Constrain the text to its block — `width: fill_container` +
/// `textGrowth: fixed-width` — so it wraps inside instead of overflowing. The
/// next round re-resolves with the text now bounded, so it converges immediately.
pub(super) fn collect_text_overflow_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    collect_text_overflow_fixes_with_context(v, rects, cmds, false);
}

pub(super) fn collect_text_overflow_fixes_with_context(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    protected_scroller_lane: bool,
) {
    let starts_scroller = is_intentional_horizontal_scroller(v, rects);
    let protect_current = protected_scroller_lane || starts_scroller;
    // Only meaningful in a FLEX parent: under `layout: none` children are
    // absolutely positioned, so "wider than the parent" is not an overflow to
    // repair (and `width: fill_container` means nothing there).
    let flex_parent = matches!(layout_str(v), Some("vertical" | "horizontal"));
    if let Some((parent_x, parent_w)) = v
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| rects.get(id))
        .map(|r| (r.x, r.w))
        .filter(|_| flex_parent && !protect_current)
    {
        let status_badge = is_compact_status_badge(v, rects);
        let pill_parent = crate::chip_repair::is_pill_chip(v) || status_badge;
        for c in children(v) {
            if c.get("type").and_then(Value::as_str) != Some("text") {
                continue;
            }
            if pill_parent {
                if let Some(cid) = c.get("id").and_then(Value::as_str) {
                    let fill = c.get("width").and_then(Value::as_str) == Some("fill_container");
                    let status_label_has_non_hug_width = status_badge
                        && c.get("width").is_some()
                        && c.get("width").and_then(Value::as_str) != Some("fit_content");
                    let wrap = c
                        .get("textGrowth")
                        .and_then(Value::as_str)
                        .is_some_and(|g| g.starts_with("fixed-width"));
                    if fill || status_label_has_non_hug_width {
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
                collect_pill_text_overflow_clip_fix(v, c, rects, cmds);
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
        collect_text_overflow_fixes_with_context(c, rects, cmds, starts_scroller);
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
pub(super) const IMAGE_INFLATION_SLACK: f64 = 8.0;

pub(super) fn collect_oversized_image_fixes(v: &Value, cmds: &mut Vec<EditorCommand>) {
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

pub(super) fn collect_frame_overflow_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    collect_frame_overflow_fixes_with_context(v, rects, cmds, false);
}

pub(super) fn collect_frame_overflow_fixes_with_context(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    protected_scroller_lane: bool,
) {
    let starts_scroller = is_intentional_horizontal_scroller(v, rects);
    let protect_current = protected_scroller_lane || starts_scroller;
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if matches!(layout_str(v), Some("vertical" | "horizontal")) && !clips && !protect_current {
        if let Some((parent_x, pw)) = v
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| rects.get(id))
            .map(|r| (r.x, r.w))
        {
            let compact_tail_id = compact_trailing_status_pair(v, rects)
                .and_then(|(_, tail)| tail.get("id").and_then(Value::as_str));
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
                // A trailing status badge stays Hug when only its right edge
                // spills because the title pair is collectively overfull; the
                // row repair stacks the pair instead. If the badge is
                // intrinsically wider than the whole parent, the width branch
                // below still applies.
                let past_right = is_container
                    && compact_tail_id != Some(cid)
                    && cr.x + cr.w > parent_x + pw + TEXT_OVERFLOW_EPS;
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
        collect_frame_overflow_fixes_with_context(c, rects, cmds, starts_scroller);
    }
}
