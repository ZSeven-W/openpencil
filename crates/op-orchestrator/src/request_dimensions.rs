//! Fact-based root dimensions explicitly requested in the user prompt.

use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RequestedRootDimensions {
    pub width: f64,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct DimensionCandidate {
    start: usize,
    end: usize,
    dimensions: RequestedRootDimensions,
}

const CONTEXT_RADIUS: usize = 56;
const ROOT_CONTEXT_TERMS: &[&str] = &[
    "root",
    "artboard",
    "page",
    "screen",
    "canvas",
    "interface",
    "poster",
    "slide",
    // "desktop dashboard" is the catalog's natural-language shorthand for
    // one desktop screen, including the accepted "1440x900 desktop dashboard"
    // form where no literal "screen" token is present.
    "desktop",
    "dashboard",
    // CJK counterparts of "desktop" / "dashboard" above: "运营数据看板
    // （1440×900，桌面）" names its root with exactly these words.
    "桌面",
    "看板",
    "仪表盘",
    "根画板",
    "画板",
    "页面",
    "屏幕",
    "画布",
    "界面",
    "首页",
    "海报",
    "每张",
    "每页",
];
const NESTED_CONTEXT_TERMS: &[&str] = &[
    "hero",
    "image",
    "card",
    "banner",
    "thumbnail",
    "photo",
    "插图",
    "图片",
    "卡片",
];

fn valid_dimension(value: u32) -> bool {
    (240..=10_000).contains(&value)
}

fn pair_regex() -> &'static Regex {
    static PAIR: OnceLock<Regex> = OnceLock::new();
    PAIR.get_or_init(|| {
        Regex::new(r"(?i)([0-9]{3,5})\s*(?:px\s*)?x\s*([0-9]{3,5})(?:\s*px)?")
            .expect("root dimension pair regex")
    })
}

fn width_regex() -> &'static Regex {
    static WIDTH: OnceLock<Regex> = OnceLock::new();
    WIDTH.get_or_init(|| {
        Regex::new(r"(?i)([0-9]{3,5})\s*(?:px|pixels?)\s+wide").expect("root width regex")
    })
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn term_has_boundaries(text: &str, start: usize, term: &str) -> bool {
    if !term.is_ascii() {
        return true;
    }
    let before = text[..start].chars().next_back();
    let after = text[start + term.len()..].chars().next();
    before.is_none_or(|ch| !is_word_char(ch)) && after.is_none_or(|ch| !is_word_char(ch))
}

fn range_distance(start: usize, end: usize, other_start: usize, other_end: usize) -> usize {
    if other_end <= start {
        start.saturating_sub(other_end)
    } else if end <= other_start {
        other_start.saturating_sub(end)
    } else {
        0
    }
}

fn nearest_term_distance(
    text: &str,
    candidate: DimensionCandidate,
    terms: &[&str],
) -> Option<usize> {
    terms
        .iter()
        .flat_map(|term| {
            text.match_indices(term)
                .filter(move |(start, _)| term_has_boundaries(text, *start, term))
                .map(move |(start, _)| {
                    range_distance(candidate.start, candidate.end, start, start + term.len())
                })
        })
        .filter(|distance| *distance <= CONTEXT_RADIUS)
        .min()
}

/// Longest design name (in chars) a heading parenthetical may follow.
const HEADING_NAME_MAX_CHARS: usize = 32;

/// Sentence / list punctuation that ends a heading: a pair behind any of it is
/// in the body, not in the design's title.
fn is_heading_break(ch: char) -> bool {
    matches!(
        ch,
        '，' | '。' | '：' | '；' | '、' | '！' | '？' | ',' | '.' | ':' | ';' | '!' | '?' | '\n'
    )
}

/// `<design name>（W×H…）` / `<design name> (W×H…)` opening the prompt: the
/// pair is the first thing inside a parenthetical that directly follows the
/// design's own name, so it sizes the design itself. The name must open the
/// prompt with no sentence or list punctuation (an item later in the body
/// never qualifies), and a name carrying a nested-item word ("页面包含卡片")
/// is left to the distance rule.
fn is_heading_parenthetical(text: &str, candidate: DimensionCandidate) -> bool {
    let before = text[..candidate.start].trim_end();
    let Some(open) = before.chars().next_back() else {
        return false;
    };
    if open != '（' && open != '(' {
        return false;
    }
    let name = before[..before.len() - open.len_utf8()].trim();
    if name.is_empty()
        || name.chars().count() > HEADING_NAME_MAX_CHARS
        || name.chars().any(is_heading_break)
    {
        return false;
    }
    !NESTED_CONTEXT_TERMS.iter().any(|term| {
        name.match_indices(term)
            .any(|(start, _)| term_has_boundaries(name, start, term))
    })
}

fn is_root_scoped(text: &str, candidate: DimensionCandidate) -> bool {
    if is_heading_parenthetical(text, candidate) {
        return true;
    }
    let Some(root_distance) = nearest_term_distance(text, candidate, ROOT_CONTEXT_TERMS) else {
        return false;
    };
    nearest_term_distance(text, candidate, NESTED_CONTEXT_TERMS)
        .is_none_or(|nested_distance| root_distance < nested_distance)
}

fn pair_candidates(text: &str) -> Vec<DimensionCandidate> {
    pair_regex()
        .captures_iter(text)
        .filter_map(|captures| {
            let whole = captures.get(0)?;
            let width = captures.get(1)?.as_str().parse::<u32>().ok()?;
            let height = captures.get(2)?.as_str().parse::<u32>().ok()?;
            (valid_dimension(width) && valid_dimension(height)).then_some(DimensionCandidate {
                start: whole.start(),
                end: whole.end(),
                dimensions: RequestedRootDimensions {
                    width: f64::from(width),
                    height: Some(f64::from(height)),
                },
            })
        })
        .collect()
}

fn width_candidates(text: &str, pair_candidates: &[DimensionCandidate]) -> Vec<DimensionCandidate> {
    width_regex()
        .captures_iter(text)
        .filter_map(|captures| {
            let whole = captures.get(0)?;
            if pair_candidates
                .iter()
                .any(|pair| whole.start() < pair.end && pair.start < whole.end())
            {
                return None;
            }
            let width = captures.get(1)?.as_str().parse::<u32>().ok()?;
            valid_dimension(width).then_some(DimensionCandidate {
                start: whole.start(),
                end: whole.end(),
                dimensions: RequestedRootDimensions {
                    width: f64::from(width),
                    height: None,
                },
            })
        })
        .collect()
}

pub(crate) fn requested_root_dimensions(prompt: &str) -> Option<RequestedRootDimensions> {
    let normalized = prompt.to_lowercase().replace('×', "x");
    let pairs = pair_candidates(&normalized);
    let mut candidates = pairs.clone();
    candidates.extend(width_candidates(&normalized, &pairs));

    // A later, explicitly root-scoped statement wins. This matters for prompts
    // that first size a card/image and then state the page/root width.
    candidates
        .into_iter()
        .filter(|candidate| is_root_scoped(&normalized, *candidate))
        .max_by_key(|candidate| candidate.start)
        .map(|candidate| candidate.dimensions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dimensions(width: f64, height: Option<f64>) -> Option<RequestedRootDimensions> {
        Some(RequestedRootDimensions { width, height })
    }

    #[test]
    fn parses_dimension_pair_with_multiplication_sign() {
        assert_eq!(
            requested_root_dimensions("Design a 1440×900 desktop analytics dashboard"),
            dimensions(1440.0, Some(900.0))
        );
    }

    #[test]
    fn ordinary_chinese_delivery_terms_keep_explicit_dimensions() {
        for prompt in [
            "手机App点单首页，只要1页，390×844，中文界面。",
            "活动海报，1080×1440，信息清楚。",
            "做4张知识卡，每张1080×1440。",
        ] {
            let expected = if prompt.contains("390") {
                dimensions(390.0, Some(844.0))
            } else {
                dimensions(1080.0, Some(1440.0))
            };
            assert_eq!(requested_root_dimensions(prompt), expected, "{prompt}");
        }
        assert_eq!(
            requested_root_dimensions("后台里每张卡片500×600，图片640×480。"),
            None
        );
    }

    #[test]
    fn parses_explicit_root_and_artboard_pairs() {
        assert_eq!(
            requested_root_dimensions("Use a root frame sized 1366 x 768px."),
            dimensions(1366.0, Some(768.0))
        );
        assert_eq!(
            requested_root_dimensions("Canvas artboard: 1600px x 1000px"),
            dimensions(1600.0, Some(1000.0))
        );
    }

    #[test]
    fn parses_explicit_pixel_width_without_guessing_height() {
        assert_eq!(
            requested_root_dimensions(
                "Make the root exactly 1440px wide and between 2400 and 5200px tall"
            ),
            dimensions(1440.0, None)
        );
    }

    #[test]
    fn rejects_nested_hero_image_and_card_dimensions() {
        for prompt in [
            "Design a page with a hero image sized 1440×900.",
            "On the dashboard, make the card 420px wide.",
            "Create a screen whose thumbnail image is 640 x 360.",
        ] {
            assert_eq!(
                requested_root_dimensions(prompt),
                None,
                "nested dimensions must not become the root contract: {prompt}"
            );
        }
    }

    #[test]
    fn later_root_width_wins_over_earlier_card_width() {
        assert_eq!(
            requested_root_dimensions(
                "Use a 420px wide card in the hero, then make the page root 1440px wide."
            ),
            dimensions(1440.0, None)
        );
    }

    #[test]
    fn later_page_pair_wins_over_earlier_image_pair() {
        assert_eq!(
            requested_root_dimensions(
                "The hero image is 1200x600. Render the desktop page at 1440x900."
            ),
            dimensions(1440.0, Some(900.0))
        );
    }

    #[test]
    fn heading_parenthetical_after_design_name_is_root_scoped() {
        // arena d01 (0927a/run-3): this prompt's root stayed at the planner's
        // 1200 wide because neither "看板" nor "桌面" counted as root context.
        assert_eq!(
            requested_root_dimensions(
                "运营数据看板（1440×900，桌面）：左侧栏（品牌+六个导航项+底部用户）、顶部工具栏、四个 KPI 卡、折线图+柱图并排、下方数据表 8 行 6 列带分页。"
            ),
            dimensions(1440.0, Some(900.0))
        );
        // The heading rule alone, without any root-context word.
        assert_eq!(
            requested_root_dimensions("咖啡品牌官网（1280×2400）：首屏大图、三栏特色、页脚"),
            dimensions(1280.0, Some(2400.0))
        );
        assert_eq!(
            requested_root_dimensions("Crm workspace (1366x768): pipeline board, activity feed"),
            dimensions(1366.0, Some(768.0))
        );
    }

    #[test]
    fn in_body_item_pairs_are_not_the_root() {
        for prompt in [
            "运营数据看板：四个 KPI 卡片 320×200、下方数据表",
            "健身记录：顶部问候、卡片（320×200）展示今日目标",
            "页面包含卡片（320×200）和一个表格",
            "Team hub: a hero image (1200x600), then a list",
        ] {
            assert_eq!(
                requested_root_dimensions(prompt),
                None,
                "an item size in the body must not become the root: {prompt}"
            );
        }
    }

    #[test]
    fn ignores_aspect_ratios_and_unqualified_numbers() {
        assert_eq!(
            requested_root_dimensions("Crop the top 900px to a 16:10 preview"),
            None
        );
        assert_eq!(
            requested_root_dimensions("Use a 1440x900 image without creating an artboard."),
            None
        );
    }
}
