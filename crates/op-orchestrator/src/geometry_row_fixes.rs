//! Row-level fix collectors: row gaps, card-row heights, over-full rows
//! (including the value-chip stack rewrite) and the collapsed `fill_container`
//! predicate.

use super::*;

/// Default gap injected into a geometry-proven jammed data row.
pub(super) const ROW_GAP_FIX: f64 = 16.0;

/// GEOMETRY-driven column-gap repair — the name-blind big brother of
/// `table_repair::ensure_table_column_gap`. A row qualifies when the REAL
/// layout proves every adjacent pair of its ≥3 frame cells touches (<3px
/// breathing) and the cells carry text — "Oct 24, 2024"+"42" reading as
/// "202442" regardless of how many unnamed wrappers bury the table (measured:
/// rows nested TWO wrapper levels below the table-named frame slipped past
/// the name gate). Flush segmented controls stay safe: those are 2-3 equal
/// small children, gated out by the ≥3-cells + row-cell height + text checks.
pub(super) fn collect_row_gap_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
) {
    if compact_trailing_status_pair(v, rects).is_some() {
        // A title + status badge pair needs a small explicit breathing gap,
        // but the generic 16px data-column gap consumes too much of a narrow
        // two-up card. Normalize both fresh 0px rows and previously damaged
        // 16px rows to the same stable value.
        if (num(v, "gap") - 8.0).abs() > f64::EPSILON {
            if let Some(id) = v.get("id").and_then(Value::as_str) {
                cmds.push(EditorCommand::SetNodeLayoutProp {
                    node_id: NodeId::new(id.to_string()),
                    property: "gap".to_string(),
                    value: LayoutPropValue::Number(8.0),
                });
            }
        }
    // `< 8.0`, not `<= 0.0`: a 1-7px authored gap still resolves as touching
    // (the jam proof below requires <3px breathing anyway), and an
    // exactly-1.0 hairline gap slipped through every gate (measured).
    } else if layout_str(v) == Some("horizontal") && num(v, "gap") < 8.0 {
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
pub(super) const CARD_ROW_HEIGHT_EPS: f64 = 6.0;

/// A horizontal row of painted KPI/stat cards whose parent explicitly requests
/// cross-axis stretch. Jian currently renders `alignItems:"stretch"` as start,
/// so implement that authored intent by making each Hug card fill the row's
/// cross axis. A ragged row without explicit stretch remains content-sized.
pub(super) fn collect_card_row_height_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    in_table: bool,
) {
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    let explicitly_stretched = v.get("alignItems").and_then(Value::as_str) == Some("stretch");
    if layout_str(v) == Some("horizontal") && explicitly_stretched && !clips && !in_table {
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

pub(super) fn is_colored_frame_card(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some("frame")
        && (fill_is_non_empty(v) || stroke_is_non_null(v))
}

pub(super) fn fill_is_non_empty(v: &Value) -> bool {
    match v.get("fill") {
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Null) | None => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(_) => true,
    }
}

pub(super) fn stroke_is_non_null(v: &Value) -> bool {
    v.get("stroke").is_some_and(|stroke| !stroke.is_null())
}

pub(super) fn child_height_is_hug_or_unset(v: &Value) -> bool {
    match v.get("height") {
        Some(Value::String(s)) => s == "fit_content",
        Some(Value::Null) | None => true,
        _ => false,
    }
}

/// Does this node carry an authored `x` or `y` (absolute placement)?
pub(super) fn has_authored_position(v: &Value) -> bool {
    v.get("x").map(|x| !x.is_null()).unwrap_or(false)
        || v.get("y").map(|y| !y.is_null()).unwrap_or(false)
}

/// Slack before a row counts as overfull — sub-8px overhangs are invisible.
pub(super) const ROW_OVERFULL_EPS: f64 = 8.0;
/// Only children this wide are worth flexifying — icons / dots / dividers
/// can't meaningfully absorb a deficit.
pub(super) const MIN_FLEXIFY_W: f64 = 120.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TableCellWidthMode {
    Fixed,
    Fill,
    Hug,
    Other,
}

fn table_row_signature(row: &Value) -> Option<Vec<TableCellWidthMode>> {
    if layout_str(row) != Some("horizontal")
        || children(row).len() < 3
        || children(row).iter().any(is_elastic_row_spacer)
        || children(row).iter().all(is_pure_bar_track)
        || children(row).iter().all(is_intrinsic_chip)
    {
        return None;
    }
    Some(
        children(row)
            .iter()
            .map(|cell| {
                if fixed_width(cell).is_some() {
                    TableCellWidthMode::Fixed
                } else {
                    match cell.get("width").and_then(Value::as_str) {
                        Some("fill_container") => TableCellWidthMode::Fill,
                        Some("fit_content") | None => TableCellWidthMode::Hug,
                        Some(_) => TableCellWidthMode::Other,
                    }
                }
            })
            .collect(),
    )
}

// Independently sized painted chips in adjacent bands have no shared data
// columns. Reserving a table-text floor for each chip falsely rejects compact
// metadata and difficulty selectors. Fixed/fill-width cells remain table-like.
fn is_intrinsic_chip(cell: &Value) -> bool {
    cell.get("type").and_then(Value::as_str) == Some("frame")
        && layout_str(cell) == Some("horizontal")
        && matches!(
            cell.get("width").and_then(Value::as_str),
            None | Some("fit_content")
        )
        && fixed_width(cell).is_none()
        && cell
            .get("height")
            .and_then(Value::as_f64)
            .is_some_and(|h| (1.0..=36.0).contains(&h))
        && num(cell, "cornerRadius") > 0.0
        && horizontal_padding(cell) >= 16.0
        && cell
            .get("fill")
            .and_then(Value::as_array)
            .is_some_and(|fill| !fill.is_empty())
        && children(cell)
            .iter()
            .filter(|n| n.get("type").and_then(Value::as_str) == Some("text"))
            .count()
            == 1
        && children(cell).len() <= 2
        && children(cell).iter().all(|n| {
            matches!(
                n.get("type").and_then(Value::as_str),
                Some("text" | "icon_font")
            )
        })
}

/// A bottom-aligned rectangle inside a fixed-height track encodes a bar, not
/// a data cell. Otherwise a plot and its label band share the same fill-width
/// signature and reserve a full table-text floor for every short axis label.
/// Keep this narrow: text, icons, images and nested content retain row coverage.
fn is_pure_bar_track(cell: &Value) -> bool {
    if cell.get("type").and_then(Value::as_str) != Some("frame")
        || layout_str(cell) != Some("vertical")
        || cell.get("justifyContent").and_then(Value::as_str) != Some("end")
    {
        return false;
    }
    let Some(track_height) = cell.get("height").and_then(Value::as_f64) else {
        return false;
    };
    let [bar] = children(cell) else {
        return false;
    };
    track_height > 0.0
        && bar.get("type").and_then(Value::as_str) == Some("rectangle")
        && children(bar).is_empty()
        && bar
            .get("height")
            .and_then(Value::as_f64)
            .is_some_and(|height| height > 0.0 && height <= track_height)
        && fill_is_non_empty(bar)
        && [cell, bar].iter().all(|node| {
            !node
                .get("fill")
                .and_then(Value::as_array)
                .is_some_and(|fills| {
                    fills
                        .iter()
                        .any(|fill| fill.get("type").and_then(Value::as_str) == Some("image"))
                })
        })
}

/// An empty flex spacer aligns the trailing price/actions by absorbing slack;
/// it is not a data column that needs a readable width reservation.
fn is_elastic_row_spacer(cell: &Value) -> bool {
    cell.get("type").and_then(Value::as_str) == Some("frame")
        && cell.get("width").and_then(Value::as_str) == Some("fill_container")
        && cell
            .get("height")
            .and_then(Value::as_f64)
            .is_some_and(|h| (0.0..=4.0).contains(&h))
        && children(cell).is_empty()
        && ["fill", "effects"].iter().all(|key| {
            cell.get(*key)
                .is_none_or(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
        })
        && ["stroke", "role", "events", "state", "constraints", "x", "y"]
            .iter()
            .all(|key| cell.get(*key).is_none_or(Value::is_null))
}

/// Return the repeated, contiguous row run that gives `v` a table shape.
///
/// A data table has a header and data rows next to each other under one
/// container, with the same per-column sizing contract. Merely finding two
/// unrelated horizontal bands is not enough: a mobile content column often
/// contains a three-item top bar and a three-item bottom tab bar separated by
/// business sections. Treating those bands as table rows suppresses the normal
/// row fixers and can emit an impossible "too many columns" diagnostic.
///
/// The predicate uses layout, cell structure and width-mode facts. Names and
/// roles are deliberately irrelevant, so unnamed generated tables remain covered.
pub(super) fn table_rows(v: &Value) -> Vec<&Value> {
    if layout_str(v) == Some("horizontal") {
        return Vec::new();
    }

    let mut best = Vec::new();
    let mut run = Vec::new();
    let mut run_signature: Option<Vec<TableCellWidthMode>> = None;
    for child in children(v) {
        let Some(signature) = table_row_signature(child) else {
            if run.len() > best.len() {
                best = std::mem::take(&mut run);
            } else {
                run.clear();
            }
            run_signature = None;
            continue;
        };
        if run_signature.as_ref() == Some(&signature) {
            run.push(child);
        } else {
            if run.len() > best.len() {
                best = std::mem::take(&mut run);
            } else {
                run.clear();
            }
            run.push(child);
            run_signature = Some(signature);
        }
    }
    if run.len() > best.len() {
        best = run;
    }
    if best.len() >= 2 {
        best
    } else {
        Vec::new()
    }
}

/// Is `v` table-shaped? Overfull TABLE rows belong to the column scaler, which
/// keeps columns aligned across rows — flexifying one row's widest column would
/// break the vertical alignment.
pub(super) fn is_table_shape(v: &Value) -> bool {
    !table_rows(v).is_empty()
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
pub(super) fn collect_row_overfull_fixes(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    in_table: bool,
) {
    collect_row_overfull_fixes_with_context(v, rects, cmds, in_table, false);
}

pub(super) fn collect_row_overfull_fixes_with_context(
    v: &Value,
    rects: &HashMap<String, Rect>,
    cmds: &mut Vec<EditorCommand>,
    in_table: bool,
    protected_scroller_lane: bool,
) {
    let starts_scroller = is_intentional_horizontal_scroller(v, rects);
    let protect_current = protected_scroller_lane || starts_scroller;
    let clips = v.get("clipContent").and_then(Value::as_bool) == Some(true);
    if layout_str(v) == Some("horizontal") && !clips && !in_table && !protect_current {
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
                let compact_pair = compact_trailing_status_pair(v, rects);
                if let Some((_, tail)) = compact_pair {
                    if tail.get("width").and_then(Value::as_str) == Some("fill_container") {
                        if let Some(tail_id) = tail.get("id").and_then(Value::as_str) {
                            cmds.push(EditorCommand::SetNodeLayoutProp {
                                node_id: NodeId::new(tail_id.to_string()),
                                property: "width".to_string(),
                                value: LayoutPropValue::Keyword("fit_content".to_string()),
                            });
                        }
                    }
                }
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
                let is_overfull = sum > inner + ROW_OVERFULL_EPS;
                let compact_pair_reflowed = compact_pair.is_some_and(|(leading, tail)| {
                    let needs_reflow = is_overfull
                        || (inner <= 180.0 && compact_status_header_is_damaged(leading, tail));
                    if needs_reflow {
                        stack_compact_status_header(v, leading, tail, cmds);
                    }
                    needs_reflow
                });
                // NARROW-CARD anatomy guard, independent of the text measure:
                // a display value + a painted chip can't share a ~200px line
                // even when a lossy measure claims they fit (the estimate
                // backend under-reads 40px display digits; the PAINT
                // overlapped — measured). Reference metric cards stack them.
                let stacked = if !compact_pair_reflowed && is_overfull && inner <= 260.0 {
                    let before = cmds.len();
                    stack_overfull_value_chip_row(v, &kids, rects, cmds);
                    cmds.len() > before
                } else {
                    compact_pair_reflowed
                };
                if !stacked && is_overfull {
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
        collect_row_overfull_fixes_with_context(c, rects, cmds, table, starts_scroller);
    }
}

/// Dead-end branch of the overfull repair: the row has NO flexify candidate
/// (a display-size TEXT value can't shrink, a painted CHIP must hug). A KPI
/// card's bottom row — a 40px "$48,920" beside a "+8.2%" trend chip in a
/// ~180px card — overflows with nothing to give (measured: the chip's tinted
/// box painted OVER the value's tail). The design-correct repair is the
/// reference metric-card anatomy: value on its own line, change chip BELOW.
/// Applies only to the exact [display text, small painted chip] pair.
pub(super) fn stack_overfull_value_chip_row(
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

pub(super) fn is_collapsed_fill_container(v: &Value, rects: &HashMap<String, Rect>) -> bool {
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
