//! Deterministic, bounded art direction from the slot's own screen and card.

use jian_ops_schema::node::{PenNode, TextContent};
use jian_ops_schema::style::PenFill;
use op_editor_core::PenNodeExt as _;

use super::query::page_context_name;

const CONTEXT_LIMIT: usize = 240;
const PART_LIMIT: usize = 60;
const NAME_LIMIT: usize = 40;

#[derive(Default)]
pub(super) struct PageContext {
    pub context: Option<String>,
    pub subject: Option<String>,
}

pub(super) fn derive(ancestors: &[&PenNode], document_name: Option<&str>) -> PageContext {
    if ancestors.iter().any(|node| is_status_bar(node)) {
        return PageContext::default();
    }
    let mut parts = Vec::new();
    // The first ancestor belongs to this tree's screen, never another root
    // in the active page's forest. A naked image has no screen ancestor.
    if let Some(root) = ancestors.first() {
        add_name(&mut parts, root.base().name.as_deref());
    }
    // Bound semantic ancestry independently of anonymous layout wrappers.
    let nearest: Vec<_> = ancestors
        .iter()
        .skip(1)
        .rev()
        .filter_map(|node| page_context_name(node.base().name.as_deref().unwrap_or_default()))
        .take(2)
        .collect();
    for name in nearest.iter().rev() {
        add_name(&mut parts, Some(name));
    }
    // Prefer the nearest card's heading, then its section. Exclude the branch
    // containing the slot and media-bearing cousin cards at every level.
    let mut subject = None;
    for (index, ancestor) in ancestors.iter().enumerate().rev() {
        let excluded = ancestors.get(index + 1).map(|node| node.base().id.as_str());
        if let Some(description) = heading_description(ancestor, excluded) {
            add_part(&mut parts, &description);
            subject = Some(description);
            break;
        }
    }
    add_name(&mut parts, document_name);
    PageContext {
        context: (!parts.is_empty()).then(|| {
            parts
                .join(" > ")
                .chars()
                .take(CONTEXT_LIMIT)
                .collect::<String>()
                .trim()
                .to_string()
        }),
        subject,
    }
}

fn add_name(parts: &mut Vec<String>, name: Option<&str>) {
    if let Some(name) = name.and_then(page_context_name) {
        add_part(parts, &name.chars().take(NAME_LIMIT).collect::<String>());
    }
}

fn add_part(parts: &mut Vec<String>, value: &str) {
    let part = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let part = part.trim();
    if !part.is_empty()
        && !parts
            .iter()
            .any(|existing| existing.to_lowercase() == part.to_lowercase())
    {
        parts.push(part.to_string());
    }
}

fn is_status_bar(node: &PenNode) -> bool {
    [node.base().role.as_deref(), node.base().name.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| {
            let compact: String = value
                .chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect();
            compact.contains("statusbar") || compact.contains("状态栏")
        })
}

struct TextCandidate<'a> {
    value: String,
    size: f64,
    parent: &'a str,
    auxiliary_allowed: bool,
}

fn heading_description(container: &PenNode, excluded: Option<&str>) -> Option<String> {
    fn walk<'a>(
        container: &'a PenNode,
        excluded: Option<&str>,
        auxiliary_blocked: bool,
        texts: &mut Vec<TextCandidate<'a>>,
    ) {
        let Some(nodes) = container.children() else {
            return;
        };
        for node in nodes {
            if Some(node.base().id.as_str()) == excluded
                || node.base().visible == Some(false)
                || is_status_bar(node)
            {
                continue;
            }
            let auxiliary_blocked = auxiliary_blocked || is_control_or_tag(node);
            match node {
                PenNode::Text(text) => {
                    let value = match &text.content {
                        TextContent::Plain(value) => value.clone(),
                        TextContent::Styled(segments) => segments
                            .iter()
                            .map(|segment| segment.text.as_str())
                            .collect(),
                    };
                    let mut size = text
                        .font_size
                        .filter(|size| size.is_finite())
                        .unwrap_or(0.0);
                    if let TextContent::Styled(segments) = &text.content {
                        for font_size in segments.iter().filter_map(|segment| segment.font_size) {
                            if font_size.is_finite() {
                                size = size.max(f64::from(font_size));
                            }
                        }
                    }
                    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !value.is_empty() {
                        texts.push(TextCandidate {
                            value,
                            size,
                            parent: &container.base().id,
                            auxiliary_allowed: !auxiliary_blocked,
                        });
                    }
                }
                _ if !contains_media(node) => {
                    walk(node, excluded, auxiliary_blocked, texts);
                }
                _ => {}
            }
        }
    }
    let mut texts = Vec::new();
    walk(
        container,
        excluded,
        is_control_or_tag(container),
        &mut texts,
    );
    // max_by would choose the last tie, but the first heading is canonical.
    let (index, heading) = texts.iter().enumerate().reduce(|best, next| {
        if next.1.size > best.1.size {
            next
        } else {
            best
        }
    })?;
    let auxiliary = texts
        .iter()
        .enumerate()
        .filter(|(_, text)| {
            text.auxiliary_allowed
                && text.size < heading.size
                && !text.value.eq_ignore_ascii_case(&heading.value)
                && is_auxiliary_text(&text.value)
        })
        .max_by(|(left_index, left), (right_index, right)| {
            (left.parent == heading.parent)
                .cmp(&(right.parent == heading.parent))
                .then_with(|| left.size.total_cmp(&right.size))
                .then_with(|| right_index.abs_diff(index).cmp(&left_index.abs_diff(index)))
                .then_with(|| right_index.cmp(left_index))
        })
        .map(|(_, text)| text.value.as_str());
    let title: String = heading.value.chars().take(PART_LIMIT).collect();
    Some(match auxiliary {
        Some(auxiliary) => format!("{title} — {auxiliary}"),
        None => title,
    })
}

fn is_control_or_tag(node: &PenNode) -> bool {
    [node.base().role.as_deref(), node.base().name.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| {
            value.split(|c: char| !c.is_alphanumeric()).any(|word| {
                matches!(
                    word.to_ascii_lowercase().as_str(),
                    "button" | "btn" | "cta" | "tag" | "badge" | "chip" | "按钮" | "标签"
                )
            })
        })
}

fn is_auxiliary_text(value: &str) -> bool {
    if value.chars().count() > PART_LIMIT || !value.chars().any(char::is_alphabetic) {
        return false;
    }
    // Single uppercase words are usually category badges (e.g. DIGITAL).
    if value.split_whitespace().count() == 1
        && value.chars().any(char::is_uppercase)
        && !value.chars().any(char::is_lowercase)
    {
        return false;
    }
    let lower = value.to_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let has_digits = value.chars().any(|c| c.is_ascii_digit());
    if has_digits
        && (value.contains(['$', '€', '£', '¥', '￥', '₩', '₹'])
            || words
                .iter()
                .any(|word| matches!(*word, "usd" | "eur" | "gbp" | "cny" | "rmb"))
            || value.ends_with(['元', '円'])
            || value
                .chars()
                .all(|c| !c.is_alphabetic() || "年月日号".contains(c))
            || words.iter().all(|word| {
                word.chars().all(|c| c.is_ascii_digit())
                    || matches!(
                        *word,
                        "jan"
                            | "january"
                            | "feb"
                            | "february"
                            | "mar"
                            | "march"
                            | "apr"
                            | "april"
                            | "may"
                            | "jun"
                            | "june"
                            | "jul"
                            | "july"
                            | "aug"
                            | "august"
                            | "sep"
                            | "sept"
                            | "september"
                            | "oct"
                            | "october"
                            | "nov"
                            | "november"
                            | "dec"
                            | "december"
                    )
            }))
    {
        return false;
    }
    let label = lower.trim_matches(|c: char| !c.is_alphanumeric());
    !matches!(
        label,
        "read more"
            | "learn more"
            | "view project"
            | "view case study"
            | "view work"
            | "see project"
            | "explore"
            | "details"
            | "buy now"
            | "add to cart"
            | "shop now"
            | "contact"
            | "contact us"
            | "next"
            | "previous"
            | "submit"
            | "查看项目"
            | "查看详情"
            | "了解更多"
            | "立即购买"
            | "加入购物车"
            | "联系我们"
    )
}

fn contains_media(node: &PenNode) -> bool {
    let fills = match node {
        PenNode::Frame(frame) => frame.container.fill.as_deref(),
        PenNode::Rectangle(rect) => rect.container.fill.as_deref(),
        _ => None,
    };
    matches!(node, PenNode::Image(_))
        || fills.is_some_and(|fills| fills.iter().any(|fill| matches!(fill, PenFill::Image(_))))
        || super::has_empty_image_fill(node)
        || super::is_frame_placeholder_still_unfilled(node)
        || super::is_image_area_rectangle_by_heuristic(node)
        || node
            .children()
            .is_some_and(|children| children.iter().any(contains_media))
}
