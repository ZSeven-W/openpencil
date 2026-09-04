//! Post-batch quality scans over the emitted subtree — duplicate roots,
//! header icon rows, empty shells, hairline rings, icon glyphs and text
//! contrast — plus the shared colour-resolution helpers they need. Split out
//! of `design_agent_tools.rs` to keep the spine under the 800-line cap.

use super::*;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ContrastIssue {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "nodeName")]
    pub node_name: Option<String>,
    pub fg: String,
    pub bg: String,
    pub ratio: f64,
    pub target: f64,
}

pub(super) const CONTRAST_AA_TARGET: f64 = 4.5;
pub(super) const CONTRAST_ICON_TARGET: f64 = 3.0;

/// Structure echo for abandoned rebuilds: TWO top-level frames with the
/// same name means the model started a fresh copy instead of filling the
/// existing root (measured: MiniMax-M3 left the original `Explore` with an
/// empty AppContent and built everything in a second `Explore` — the user
/// sees a blank artboard mid-run). Finalize's duplicate-root pass repairs
/// the END state, but the in-loop model should merge NOW.
pub fn scan_duplicate_root_issues(nodes: &[PenNode]) -> Vec<String> {
    use std::collections::HashMap;
    let mut by_name: HashMap<&str, Vec<&PenNode>> = HashMap::new();
    for node in nodes {
        if let PenNode::Frame(_) = node {
            if let Some(name) = node.base().name.as_deref() {
                if !name.trim().is_empty() {
                    by_name.entry(name).or_default().push(node);
                }
            }
        }
    }
    let mut out = Vec::new();
    for (name, dupes) in by_name {
        if dupes.len() < 2 {
            continue;
        }
        let ids: Vec<&str> = dupes.iter().map(|n| n.id_str()).collect();
        out.push(format!(
            "duplicate top-level roots named \"{name}\" ({}) — you rebuilt a copy instead of \
             filling the existing frame. Move your content into ONE root with M() and D() the \
             abandoned empty copy; never leave both.",
            ids.join(", ")
        ));
    }
    out.sort();
    out
}

/// Contract echo for broken icons: an `icon_font` whose `iconFontName` is
/// missing, empty, or a FONT FAMILY name ("lucide" /
/// "material symbols …") renders as the fallback dot — the model wrote the
/// family into the glyph field (measured: test0711-1.op shipped every icon
/// as `iconFontName:"lucide"` with no glyph anywhere). The intended glyph
/// cannot be recovered deterministically, so this echoes the offending ids
/// for the in-loop model to repair with `U()`.
/// Hairline "activity ring" echo: a cluster of large concentric ellipses
/// stroked ~1px reads as faint wireframe circles, not progress rings
/// (measured: GLM-5.2 test0711-2.op stacked six 1px ellipses for the
/// Today's Activity ring). Ring thickness is the model's design intent, so
/// this echoes instead of auto-fixing.
/// Inventory of still-empty named shells — the skeleton-first protocol's
/// countdown. Informational (not an "issue"): intermediate batches SHOULD
/// have empty shells; the model uses the list to know what remains and to
/// never end the turn with one unfilled (measured: an aborted run shipped
/// an empty TabBar + MiniPlayer, test0711-22).
/// A header-named row holding ONLY icons while its title text sits outside
/// as a SIBLING — the bell floats alone in a full-width strip above the
/// greeting (measured: "Header Row" = [bell], "Good evening" outside it,
/// test0711-22). Which text belongs in the row is intent, so this echoes.
pub fn scan_header_icon_row_issues(nodes: &[PenNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn is_standard_brand_actions_header(parent: &PenNode, children: &[PenNode]) -> bool {
        let PenNode::Frame(frame) = parent else {
            return false;
        };
        let parent_name = frame.base.name.as_deref().unwrap_or("");
        if !parent_name.to_ascii_lowercase().contains("header")
            || !matches!(frame.container.layout, Some(LayoutMode::Horizontal))
            || !matches!(
                frame.container.justify_content,
                Some(jian_ops_schema::node::JustifyContent::SpaceBetween)
            )
            || children.len() != 2
        {
            return false;
        }
        let title_count = children
            .iter()
            .filter(|child| matches!(child, PenNode::Text(_)))
            .count();
        let icon_row_count = children
            .iter()
            .filter(|child| {
                child.children().is_some_and(|row_children| {
                    !row_children.is_empty()
                        && row_children
                            .iter()
                            .all(|row_child| matches!(row_child, PenNode::IconFont(_)))
                })
            })
            .count();
        title_count == 1 && icon_row_count == 1
    }
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if out.len() >= 4 {
                return;
            }
            let Some(children) = node.children() else {
                continue;
            };
            let has_text_sibling_ctx = children.iter().any(|c| matches!(c, PenNode::Text(_)));
            let is_standard_header = is_standard_brand_actions_header(node, children);
            for child in children {
                let name = child.base().name.as_deref().unwrap_or("");
                if !name.to_ascii_lowercase().contains("header") {
                    continue;
                }
                let Some(row_children) = child.children() else {
                    continue;
                };
                let icons_only = !row_children.is_empty()
                    && row_children
                        .iter()
                        .all(|c| matches!(c, PenNode::IconFont(_)));
                if icons_only && has_text_sibling_ctx && !is_standard_header {
                    out.push(format!(
                        "{} ({}): contains ONLY icons while the title text sits outside as a                          sibling - M() the title INTO this row (layout horizontal,                          justifyContent space_between) so the greeting and the icons share                          one line",
                        name,
                        child.id_str()
                    ));
                }
            }
            walk(children, out);
        }
    }
    walk(nodes, &mut out);
    out
}

pub fn scan_empty_shells(nodes: &[PenNode]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(nodes: &[PenNode], parent_layout_is_none: bool, out: &mut Vec<String>) {
        for (index, node) in nodes.iter().enumerate() {
            if out.len() >= 12 {
                return;
            }
            // OS chrome is ours, not the model's unfinished work. Skipping the
            // whole subtree — not just the bar node — is the point: the
            // scaffold's battery is a named frame of named childless
            // rectangles (Border / Cap / Capacity), so descending reported
            // three empty shells per screen that no author could act on.
            if node.base().role.as_deref() == Some("status-bar") {
                continue;
            }
            if let Some(children) = node.children() {
                let named = node.base().name.as_deref().unwrap_or("");
                let is_candidate = children.is_empty() && !named.is_empty();
                if is_candidate {
                    // A childless named frame under `layout:none` that
                    // substantially overlaps a non-empty sibling of near-
                    // identical size is a decorative deck/stack "peek" layer
                    // (e.g. a flashcard's shadow layers behind the front
                    // card), not an unfinished skeleton slot — skip it.
                    if parent_layout_is_none && is_decorative_stack_layer(node, nodes, index) {
                        continue;
                    }
                    // Carry the node id alongside the name (matches the other
                    // structural scans' shape) so a loop-end corrective nudge
                    // can name a specific, D()-able / M()-able target instead
                    // of a possibly-ambiguous name alone.
                    out.push(format!("{named} ({})", node.id_str()));
                } else {
                    walk(children, node_layout_is_none(node), out);
                }
            }
        }
    }
    walk(nodes, false, &mut out);
    out
}

/// `layout: none` (or the field omitted — same default) positions children
/// by explicit x/y instead of flowing them, which is the only regime where
/// two siblings can legitimately occupy overlapping rects (a flowed
/// vertical/horizontal container never stacks children on top of each
/// other).
pub(super) fn node_layout_is_none(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(n) => matches!(n.container.layout, None | Some(LayoutMode::None)),
        PenNode::Group(n) => matches!(n.container.layout, None | Some(LayoutMode::None)),
        PenNode::Rectangle(n) => matches!(n.container.layout, None | Some(LayoutMode::None)),
        _ => false,
    }
}

pub(super) fn node_rect(node: &PenNode) -> Option<(f64, f64, f64, f64)> {
    let x = node.base().x?;
    let y = node.base().y?;
    let w = node.width_px()?;
    let h = node.height_px()?;
    Some((x, y, w, h))
}

/// A sibling counts as "non-empty" for stack-layer detection when it isn't
/// itself an empty shell: a container with at least one child, or any leaf
/// node (text/image/icon/etc, which have no `children()` at all and always
/// carry their own content).
pub(super) fn is_nonempty_sibling(node: &PenNode) -> bool {
    match node.children() {
        Some(children) => !children.is_empty(),
        None => true,
    }
}

/// Pure geometry/structure check — no name matching — so it can't be gamed
/// by renaming and can't misfire on an ordinary empty section scaffold
/// (which has no overlapping non-empty sibling to key off).
pub(super) fn is_decorative_stack_layer(
    node: &PenNode,
    siblings: &[PenNode],
    index: usize,
) -> bool {
    let Some(rect) = node_rect(node) else {
        return false;
    };
    siblings.iter().enumerate().any(|(j, sibling)| {
        if j == index || !is_nonempty_sibling(sibling) {
            return false;
        }
        let Some(other) = node_rect(sibling) else {
            return false;
        };
        rects_substantially_overlap(rect, other) && rects_near_same_size(rect, other)
    })
}

/// Intersection area is at least half of EACH rect's own area — a weak
/// corner-touch doesn't count, only a genuine stacked-on-top overlap.
pub(super) fn rects_substantially_overlap(
    a: (f64, f64, f64, f64),
    b: (f64, f64, f64, f64),
) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    if aw <= 0.0 || ah <= 0.0 || bw <= 0.0 || bh <= 0.0 {
        return false;
    }
    let iw = (ax + aw).min(bx + bw) - ax.max(bx);
    let ih = (ay + ah).min(by + bh) - ay.max(by);
    if iw <= 0.0 || ih <= 0.0 {
        return false;
    }
    let overlap_area = iw * ih;
    overlap_area >= 0.5 * (aw * ah) && overlap_area >= 0.5 * (bw * bh)
}

/// Width AND height each within 20% of one another.
pub(super) fn rects_near_same_size(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    let (_, _, aw, ah) = a;
    let (_, _, bw, bh) = b;
    let w_diff = (aw - bw).abs() / aw.max(bw).max(1.0);
    let h_diff = (ah - bh).abs() / ah.max(bh).max(1.0);
    w_diff <= 0.2 && h_diff <= 0.2
}

pub fn scan_ring_issues(nodes: &[PenNode]) -> Vec<String> {
    const MIN_RING_SIZE: f64 = 48.0;
    const HAIRLINE: f32 = 2.5;
    let mut out = op_design_lint::detect_missing_progress_rings(nodes)
        .into_iter()
        .take(4)
        .map(|missing| {
            format!(
                "missing-progress-ring at {} ({}): numeric metric has no visible circle or arc - author ellipse/arc geometry or a painted circular frame",
                missing.node_id, missing.node_name
            )
        })
        .collect::<Vec<_>>();
    fn hairline_ring(node: &PenNode) -> bool {
        let PenNode::Ellipse(ellipse) = node else {
            return false;
        };
        if node.width_px().unwrap_or(0.0) < MIN_RING_SIZE {
            return false;
        }
        matches!(
            ellipse.stroke.as_ref().map(|s| &s.thickness),
            Some(jian_ops_schema::style::StrokeThickness::Uniform(t)) if *t <= HAIRLINE
        )
    }
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if out.len() >= 4 {
                return;
            }
            if let Some(children) = node.children() {
                let hairlines = children.iter().filter(|c| hairline_ring(c)).count();
                if hairlines >= 2 {
                    out.push(format!(
                        "{}: {hairlines} large ellipses stroked <=2px look like faint wireframe                          circles, not progress rings - give each ring a thick stroke                          (thickness 8-12), muted track + accent progress",
                        node.id_str()
                    ));
                }
                walk(children, out);
            }
        }
    }
    walk(nodes, &mut out);
    out
}

pub(super) fn scan_icon_issues(nodes: &[PenNode]) -> Vec<String> {
    // `feather` is itself a real Lucide glyph. Treating it as a family name
    // made a valid icon look like a fallback dot, so only unambiguous family
    // placeholders stay in this denylist.
    const FAMILY_NAMES: [&str; 2] = ["lucide", "material symbols"];
    let mut out = Vec::new();
    fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
        for node in nodes {
            if out.len() >= 12 {
                return;
            }
            if let PenNode::IconFont(icon) = node {
                let name = icon.icon_font_name.trim();
                let lowered = name.to_ascii_lowercase();
                let family_as_glyph = FAMILY_NAMES
                    .iter()
                    .any(|family| lowered.starts_with(family));
                if name.is_empty() || family_as_glyph {
                    out.push(format!(
                        "icon {}: iconFontName is {} — it must be the GLYPH name \
                         (e.g. \"home\", \"compass\"), not the font family",
                        icon.base.id,
                        if name.is_empty() {
                            "missing".to_string()
                        } else {
                            format!("\"{name}\"")
                        }
                    ));
                }
            }
            if let Some(children) = node.children() {
                walk(children, out);
            }
        }
    }
    walk(nodes, &mut out);
    out
}

pub(super) fn scan_contrast_issues(
    nodes: &[PenNode],
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
) -> Vec<ContrastIssue> {
    let mut candidates = Vec::new();
    let mut bg_stack = Vec::new();
    for node in nodes {
        collect_contrast_candidates(node, variables, theme, &mut bg_stack, &mut candidates);
    }

    let pairs: Vec<(String, String, f64)> = candidates
        .iter()
        .map(|candidate| (candidate.fg.clone(), candidate.bg.clone(), candidate.target))
        .collect();
    let report = op_ai_skills::color::contrast::scan_pairs(&pairs);
    let mut violations = report.violations.into_iter().peekable();
    let mut issues = Vec::new();
    for candidate in candidates {
        let Some(violation) = violations.peek() else {
            break;
        };
        if violation.fg == candidate.fg
            && violation.bg == candidate.bg
            && (violation.target - candidate.target).abs() < f64::EPSILON
        {
            let violation = violations.next().expect("peeked violation");
            issues.push(ContrastIssue {
                node_id: candidate.node_id,
                node_name: candidate.node_name,
                fg: violation.fg,
                bg: violation.bg,
                ratio: violation.ratio,
                target: violation.target,
            });
        }
    }
    issues
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ContrastCandidate {
    pub(super) node_id: String,
    pub(super) node_name: Option<String>,
    pub(super) fg: String,
    pub(super) bg: String,
    pub(super) target: f64,
}

pub(super) fn collect_contrast_candidates(
    node: &PenNode,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    bg_stack: &mut Vec<String>,
    out: &mut Vec<ContrastCandidate>,
) {
    let opacity = resolved_node_opacity(node, variables, theme);
    let pushed_bg = container_background_hex(
        node,
        variables,
        theme,
        bg_stack.last().map(String::as_str),
        opacity,
    );
    if let Some(bg) = pushed_bg.as_ref() {
        bg_stack.push(bg.clone());
    }

    let foreground = match node {
        PenNode::Text(text) => Some((&text.fill, CONTRAST_AA_TARGET)),
        PenNode::IconFont(icon) => Some((&icon.fill, CONTRAST_ICON_TARGET)),
        _ => None,
    };
    if let (Some((fill, target)), Some(bg)) = (foreground, bg_stack.last()) {
        if let Some(fg) = first_solid_hex(fill, variables, theme, Some(bg), opacity) {
            out.push(ContrastCandidate {
                node_id: node.id_str().to_string(),
                node_name: node.base().name.clone(),
                fg,
                bg: bg.clone(),
                target,
            });
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_contrast_candidates(child, variables, theme, bg_stack, out);
        }
    }

    if pushed_bg.is_some() {
        bg_stack.pop();
    }
}

pub(super) fn container_background_hex(
    node: &PenNode,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    parent_bg: Option<&str>,
    node_opacity: f64,
) -> Option<String> {
    let fill = match node {
        PenNode::Frame(n) => &n.container.fill,
        PenNode::Group(n) => &n.container.fill,
        PenNode::Rectangle(n) => &n.container.fill,
        PenNode::Tabs(n) => &n.fill,
        _ => return None,
    };
    first_solid_hex(fill, variables, theme, parent_bg, node_opacity)
}

pub(super) fn first_solid_hex(
    fill: &Option<Vec<PenFill>>,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    background: Option<&str>,
    node_opacity: f64,
) -> Option<String> {
    fill.as_ref()?.iter().find_map(|fill| match fill {
        PenFill::Solid(body) => resolved_hex(
            &body.color,
            variables,
            theme,
            background,
            node_opacity * f64::from(body.opacity.unwrap_or(1.0)),
        ),
        _ => None,
    })
}

pub(super) fn resolved_node_opacity(
    node: &PenNode,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
) -> f64 {
    match node.base().opacity.as_ref() {
        Some(NumberOrExpression::Number(opacity)) => opacity.clamp(0.0, 1.0),
        Some(NumberOrExpression::Expression(reference)) => {
            op_editor_core::variables_resolve::resolve_numeric_ref(reference, variables, theme)
                .unwrap_or(1.0)
                .clamp(0.0, 1.0)
        }
        None => 1.0,
    }
}

pub(super) fn resolved_hex(
    color: &str,
    variables: Option<
        &std::collections::BTreeMap<String, jian_ops_schema::variable::VariableDefinition>,
    >,
    theme: &std::collections::BTreeMap<String, String>,
    background: Option<&str>,
    opacity: f64,
) -> Option<String> {
    let resolved = op_editor_core::variables_resolve::resolve_color_ref(color, variables, theme)?;
    composite_color(resolved.trim(), background.unwrap_or("#FFFFFF"), opacity)
}

pub(super) fn composite_color(foreground: &str, background: &str, opacity: f64) -> Option<String> {
    let (fg_r, fg_g, fg_b) = op_editor_core::parse_hex_rgb(foreground)?;
    let (bg_r, bg_g, bg_b) = op_editor_core::parse_hex_rgb(background)?;
    let alpha = f64::from(op_editor_core::parse_hex_alpha(foreground)) * opacity.clamp(0.0, 1.0);
    let blend = |foreground: f32, background: f32| {
        ((f64::from(foreground) * alpha + f64::from(background) * (1.0 - alpha)) * 255.0).round()
            as u8
    };
    Some(format!(
        "#{:02X}{:02X}{:02X}",
        blend(fg_r, bg_r),
        blend(fg_g, bg_g),
        blend(fg_b, bg_b)
    ))
}

pub(super) fn contrast_hint(issues: &[ContrastIssue]) -> String {
    let text_count = issues
        .iter()
        .filter(|issue| (issue.target - CONTRAST_AA_TARGET).abs() < f64::EPSILON)
        .count();
    let icon_count = issues.len().saturating_sub(text_count);
    format!(
        "{} foreground/background pair(s) below their target (text: {text_count} below {CONTRAST_AA_TARGET}:1; icons: {icon_count} below {CONTRAST_ICON_TARGET}:1). Use a deliberate semantic foreground with sufficient contrast.",
        issues.len()
    )
}

/// `pub(crate)` — reused by the live-MCP indicator hook (`mcp_live.rs`)
/// for the same "reveal_started_ms" clock read.
pub fn reveal_now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
