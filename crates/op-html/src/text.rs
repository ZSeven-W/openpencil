use crate::import_warning::ImportWarning;
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::text::{
    FontStyleKind as TextFontStyle, FontWeight, TextAlign, TextContent, TextNode,
};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{
    FontStyleKind as SegmentFontStyle, PenFill, SolidFillBody, StyledTextSegment,
};

use crate::css::cascade::{compute_style_for_viewport, ComputedStyle};
use crate::dom::{DomElement, DomNode};
use crate::length::{parse_length, CssLength, LengthCtx};
use crate::mapper::MapCtx;

#[path = "text_layout.rs"]
mod layout;

#[path = "text_inline.rs"]
pub(crate) mod inline;

#[path = "text_children.rs"]
mod children;

pub(crate) use children::map_children;

#[path = "text_style.rs"]
mod style;
use style::{segment_style, sync_single_run_style};

#[rustfmt::skip]
pub fn is_inline_tag(tag: &str) -> bool {
    matches!(tag, "a" | "b" | "strong" | "i" | "em" | "u" | "s" | "del" | "strike"
        | "span" | "code" | "small" | "sub" | "sup" | "label" | "br" | "mark" | "picture")
}

#[derive(Clone, Debug, PartialEq)]
struct SegStyle {
    weight: Option<u32>,
    style: Option<SegmentFontStyle>,
    underline: Option<bool>,
    strike: Option<bool>,
    fill: Option<String>,
    href: Option<String>,
    font_size: Option<f32>,
    font_family: Option<String>,
}

#[derive(Clone, Debug)]
struct Segment {
    text: String,
    style: SegStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WhiteSpaceMode {
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
    BreakSpaces,
}

impl WhiteSpaceMode {
    fn from_style(style: &ComputedStyle, inherited: Self) -> Self {
        match style.get("white-space") {
            Some("normal") => Self::Normal,
            Some("nowrap") => Self::NoWrap,
            Some("pre") => Self::Pre,
            Some("pre-wrap") => Self::PreWrap,
            Some("pre-line") => Self::PreLine,
            Some("break-spaces") => Self::BreakSpaces,
            _ => inherited,
        }
    }

    fn preserves_spaces(self) -> bool {
        matches!(self, Self::Pre | Self::PreWrap | Self::BreakSpaces)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextTransformMode {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

impl TextTransformMode {
    fn from_style(style: &ComputedStyle, inherited: Self) -> Self {
        match style.get("text-transform") {
            Some("none") => Self::None,
            Some("uppercase") => Self::Uppercase,
            Some("lowercase") => Self::Lowercase,
            Some("capitalize") => Self::Capitalize,
            _ => inherited,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct InlineTextContext {
    white_space: WhiteSpaceMode,
    transform: TextTransformMode,
}

impl InlineTextContext {
    fn root(style: &ComputedStyle) -> Self {
        Self {
            white_space: WhiteSpaceMode::from_style(style, WhiteSpaceMode::Normal),
            transform: TextTransformMode::from_style(style, TextTransformMode::None),
        }
    }

    fn child(self, style: &ComputedStyle) -> Self {
        Self {
            white_space: WhiteSpaceMode::from_style(style, self.white_space),
            transform: TextTransformMode::from_style(style, self.transform),
        }
    }
}

#[derive(Clone, Debug)]
struct RawSegment {
    text: String,
    style: SegStyle,
    context: InlineTextContext,
    forced_break: bool,
}

#[derive(Clone, Copy)]
struct InlineTextScope<'a> {
    context: InlineTextContext,
    fill_override: Option<&'a str>,
}

impl<'a> InlineTextScope<'a> {
    fn child(self, style: &ComputedStyle) -> Self {
        Self {
            context: self.context.child(style),
            ..self
        }
    }
}

pub(crate) fn build_generated_text_node(
    context: &mut MapCtx<'_>,
    text: &str,
    style: &ComputedStyle,
    text_fill_override: Option<&str>,
) -> Option<PenNode> {
    let node = DomNode::Text(text.to_string());
    build_text_node_in_path(
        context,
        &[children::InlineRunItem::Dom(&node)],
        style,
        &[],
        text_fill_override,
    )
}

fn build_text_node_in_path(
    context: &mut MapCtx<'_>,
    run: &[children::InlineRunItem<'_>],
    block_style: &ComputedStyle,
    path: &[&DomElement],
    text_fill_override: Option<&str>,
) -> Option<PenNode> {
    if context.node_count >= crate::MAX_OUTPUT_NODES {
        context.warn_once(ImportWarning::NodeLimitMapping);
        return None;
    }
    let base_style = segment_style(block_style, nearest_href(path), text_fill_override);
    let base_context = InlineTextContext::root(block_style);
    let raw_segments = children::collect_run_segments(
        run,
        context,
        path,
        block_style,
        &base_style,
        base_context,
        text_fill_override,
    );
    let mut segments = normalize_segments(raw_segments);
    if segments.is_empty() {
        return None;
    }
    merge_adjacent_segments(&mut segments);
    let plain =
        base_style.href.is_none() && segments.iter().all(|segment| segment.style == base_style);
    let single_run_style = (!plain && segments.len() == 1).then(|| segments[0].style.clone());
    let content = if plain {
        TextContent::Plain(
            segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect(),
        )
    } else {
        TextContent::Styled(
            segments
                .into_iter()
                .map(|segment| StyledTextSegment {
                    text: segment.text,
                    font_family: segment.style.font_family,
                    font_size: segment.style.font_size,
                    font_weight: segment.style.weight,
                    font_style: segment.style.style,
                    fill: segment.style.fill,
                    underline: segment.style.underline,
                    strikethrough: segment.style.strike,
                    href: segment.style.href,
                })
                .collect(),
        )
    };
    let text_box = layout::map_text_box(block_style, path, context);
    let effects = crate::mapper::map_text_shadow(block_style, context);
    let mut text = TextNode {
        base: PenNodeBase {
            id: context.generate_id(),
            name: Some("Text".to_string()),
            ..Default::default()
        },
        width: text_box.width,
        height: text_box.height,
        content,
        font_family: block_style.get("font-family").map(str::to_string),
        font_size: Some(block_style.font_size),
        font_weight: parse_weight(block_style.get("font-weight")).map(FontWeight::Number),
        font_style: parse_text_font_style(block_style.get("font-style")),
        letter_spacing: block_style
            .get("letter-spacing")
            .and_then(|value| text_length_px(value, block_style, context)),
        line_height: block_style
            .get("line-height")
            .and_then(|value| parse_line_height(value, block_style, context)),
        text_align: block_style.get("text-align").and_then(parse_text_align),
        text_align_vertical: None,
        text_growth: text_box.growth,
        underline: decoration(block_style, "underline").then_some(true),
        strikethrough: decoration(block_style, "line-through").then_some(true),
        fill: crate::mapper::text_paint_color(block_style, text_fill_override)
            .map(|color| vec![solid_fill(color)]),
        effects,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        limits: text_box.limits,
    };
    if let Some(style) = single_run_style.as_ref() {
        sync_single_run_style(&mut text, style);
    }
    let node = PenNode::Text(text);
    context.node_count += 1;
    Some(node)
}

fn collect_segments(
    node: &DomNode,
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    parent_style: &ComputedStyle,
    inherited: &SegStyle,
    scope: InlineTextScope<'_>,
    segments: &mut Vec<RawSegment>,
) {
    match node {
        DomNode::Text(text) => segments.push(RawSegment {
            text: text.clone(),
            style: inherited.clone(),
            context: scope.context,
            forced_break: false,
        }),
        DomNode::Element(element) => {
            let mut child_path = path.to_vec();
            child_path.push(element);
            let computed = compute_style_for_viewport(
                &child_path,
                context.rules,
                Some(parent_style),
                context.opts.base_font_size,
                context.opts.viewport_width,
                context.opts.viewport_height(),
            );
            if computed.get("display") == Some("none") {
                return;
            }
            crate::mapper::warn_segment_text_shadow(context, parent_style, &computed);
            let href = if element.tag == "a" {
                element.attr("href").map(str::to_string)
            } else {
                inherited.href.clone()
            };
            let style = segment_style(&computed, href.clone(), scope.fill_override);
            let child_scope = scope.child(&computed);
            if element.tag == "br" {
                segments.push(RawSegment {
                    text: String::new(),
                    style,
                    context: child_scope.context,
                    forced_break: true,
                });
                return;
            }
            inline::append_inline_pseudo(
                context,
                &child_path,
                &computed,
                crate::css::selectors::PseudoElement::Before,
                href.clone(),
                child_scope,
                segments,
            );
            for child in &element.children {
                collect_segments(
                    child,
                    context,
                    &child_path,
                    &computed,
                    &style,
                    child_scope,
                    segments,
                );
            }
            inline::append_inline_pseudo(
                context,
                &child_path,
                &computed,
                crate::css::selectors::PseudoElement::After,
                href,
                child_scope,
                segments,
            );
        }
    }
}

fn nearest_href(path: &[&DomElement]) -> Option<String> {
    path.iter().rev().find_map(|element| {
        (element.tag == "a")
            .then(|| element.attr("href").map(str::to_string))
            .flatten()
    })
}

fn push_segment(segments: &mut Vec<Segment>, text: String, style: SegStyle) {
    if !text.is_empty() {
        segments.push(Segment { text, style });
    }
}

fn normalize_segments(raw: Vec<RawSegment>) -> Vec<Segment> {
    let mut output = Vec::new();
    let mut pending_space: Option<SegStyle> = None;
    let mut at_line_start = true;
    let mut capitalize_next = true;
    for raw_segment in raw {
        if raw_segment.forced_break {
            pending_space = None;
            push_segment(&mut output, "\n".to_string(), raw_segment.style);
            at_line_start = true;
            capitalize_next = true;
            continue;
        }
        let normalized = canonical_newlines(&raw_segment.text);
        for ch in normalized.chars() {
            let segment_break = ch == '\n';
            let collapsible = is_css_whitespace(ch);
            if raw_segment.context.white_space.preserves_spaces() {
                if pending_space.take().is_some() && !at_line_start && !collapsible {
                    push_segment(&mut output, " ".to_string(), raw_segment.style.clone());
                }
                if collapsible {
                    push_segment(&mut output, ch.to_string(), raw_segment.style.clone());
                    at_line_start = segment_break || at_line_start;
                    capitalize_next = true;
                } else {
                    let transformed =
                        transform_char(ch, raw_segment.context.transform, &mut capitalize_next);
                    push_segment(&mut output, transformed, raw_segment.style.clone());
                    at_line_start = false;
                }
                continue;
            }
            if raw_segment.context.white_space == WhiteSpaceMode::PreLine && segment_break {
                pending_space = None;
                push_segment(&mut output, "\n".to_string(), raw_segment.style.clone());
                at_line_start = true;
                capitalize_next = true;
            } else if collapsible {
                if pending_space.is_none() {
                    pending_space = Some(raw_segment.style.clone());
                }
                capitalize_next = true;
            } else {
                if let Some(space_style) = pending_space.take() {
                    if !at_line_start {
                        push_segment(&mut output, " ".to_string(), space_style);
                    }
                }
                let transformed =
                    transform_char(ch, raw_segment.context.transform, &mut capitalize_next);
                push_segment(&mut output, transformed, raw_segment.style.clone());
                at_line_start = false;
            }
        }
    }
    output
}

fn canonical_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace(['\r', '\u{c}'], "\n")
}

fn is_css_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n')
}

fn transform_char(ch: char, mode: TextTransformMode, capitalize_next: &mut bool) -> String {
    let transformed = match mode {
        TextTransformMode::Uppercase => ch.to_uppercase().collect(),
        TextTransformMode::Lowercase => ch.to_lowercase().collect(),
        TextTransformMode::Capitalize if *capitalize_next && ch.is_alphabetic() => {
            ch.to_uppercase().collect()
        }
        _ => ch.to_string(),
    };
    *capitalize_next = !ch.is_alphanumeric();
    transformed
}

fn merge_adjacent_segments(segments: &mut Vec<Segment>) {
    let mut merged: Vec<Segment> = Vec::new();
    for segment in segments.drain(..) {
        if let Some(previous) = merged.last_mut() {
            if previous.style == segment.style {
                previous.text.push_str(&segment.text);
                continue;
            }
        }
        merged.push(segment);
    }
    *segments = merged;
}

fn parse_weight(value: Option<&str>) -> Option<u32> {
    match value? {
        "normal" | "regular" => Some(400),
        "bold" => Some(700),
        value => value.parse().ok(),
    }
}

fn decoration(style: &ComputedStyle, needle: &str) -> bool {
    style
        .get("text-decoration-line")
        .or_else(|| style.get("text-decoration"))
        .is_some_and(|value| value.split_whitespace().any(|part| part == needle))
}

fn parse_text_font_style(value: Option<&str>) -> Option<TextFontStyle> {
    match value? {
        "italic" | "oblique" => Some(TextFontStyle::Italic),
        "normal" => Some(TextFontStyle::Normal),
        _ => None,
    }
}

fn parse_text_align(value: &str) -> Option<TextAlign> {
    match value {
        "left" | "start" => Some(TextAlign::Left),
        "center" => Some(TextAlign::Center),
        "right" | "end" => Some(TextAlign::Right),
        "justify" => Some(TextAlign::Justify),
        _ => None,
    }
}

fn text_length_px(value: &str, style: &ComputedStyle, context: &MapCtx<'_>) -> Option<f64> {
    match parse_length(value, &text_length_context(style, context))? {
        CssLength::Px(value) => Some(value),
        CssLength::Percent(_) => None,
        length => Some(length.resolve(context.opts.viewport_width)),
    }
}

fn parse_line_height(value: &str, style: &ComputedStyle, context: &MapCtx<'_>) -> Option<f64> {
    if let Ok(multiplier) = value.parse::<f64>() {
        return Some(multiplier);
    }
    Some(
        parse_length(value, &text_length_context(style, context))?.resolve(style.font_size)
            / style.font_size,
    )
}

fn text_length_context(style: &ComputedStyle, context: &MapCtx<'_>) -> LengthCtx {
    LengthCtx {
        font_size: style.font_size,
        root_font_size: context.opts.base_font_size,
        viewport_w: context.opts.viewport_width,
        viewport_h: context.opts.viewport_height(),
    }
}

fn solid_fill(color: String) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color,
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

#[cfg(test)]
mod tests {
    use jian_ops_schema::node::text::TextContent;
    use jian_ops_schema::node::PenNode;

    use super::{FontWeight, PenFill, SegmentFontStyle, TextFontStyle};

    fn content_string(content: &TextContent) -> String {
        match content {
            TextContent::Plain(text) => text.clone(),
            TextContent::Styled(segments) => segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect(),
        }
    }

    fn text_of(html: &str, css: &str) -> Vec<PenNode> {
        let dom = crate::dom::parse_dom(html);
        let (mut rules, _) = crate::css::cascade::parse_stylesheet_with_origin(
            crate::css::cascade::UA_STYLESHEET,
            0,
            crate::css::cascade::StyleOrigin::UserAgent,
        );
        // These tests exercise text-run shaping, not browser default block
        // spacing. Keep the authored nodes direct so margin-wrapper structure
        // is covered only by the dedicated mapper margin tests.
        let (reset_margins, _) =
            crate::css::cascade::parse_stylesheet("p,h1,h2,h3,h4,h5,h6,ul,ol,hr{margin:0}", 999);
        rules.extend(reset_margins);
        let (author, _) = crate::css::cascade::parse_stylesheet(css, 1000);
        rules.extend(author);
        let options = crate::HtmlImportOptions::default();
        let mut context = crate::mapper::MapCtx {
            opts: &options,
            rules: &rules,
            warnings: Vec::new(),
            warned: Default::default(),
            next_id: 0,
            node_count: 0,
            containing_width: options.viewport_width,
            containing_height: options.viewport_height(),
            containing_width_is_definite: true,
            positioned_width: options.viewport_width,
            positioned_height: options.viewport_height(),
            auto_margin_handled_by_parent: false,
            pending_base_outcome: Default::default(),
        };
        let crate::dom::DomNode::Element(root) = &dom.body[0] else {
            panic!()
        };
        let Some(PenNode::Frame(frame)) = crate::mapper::map_element(&mut context, &[root], None)
        else {
            panic!()
        };
        frame.children.unwrap_or_default()
    }

    #[test]
    fn plain_paragraph_merges_to_single_plain_text() {
        let children = text_of("<div><p>hello   world</p></div>", "");
        let PenNode::Frame(paragraph) = &children[0] else {
            panic!("p should be a frame")
        };
        let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
            panic!("expected text")
        };
        assert!(matches!(&text.content, TextContent::Plain(value) if value == "hello world"));
    }

    #[test]
    fn bold_link_run_becomes_styled_segments() {
        let children = text_of(
            "<div><p>see <b>bold</b> and <a href=\"https://x.dev\">link</a></p></div>",
            "p{font-size:18px;font-weight:500;color:#123456}",
        );
        let PenNode::Frame(paragraph) = &children[0] else {
            panic!()
        };
        let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let TextContent::Styled(segments) = &text.content else {
            panic!("expected styled")
        };
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[1].font_weight, Some(700));
        assert_eq!(segments[3].href.as_deref(), Some("https://x.dev"));
        assert_eq!(segments[3].underline, Some(true));
        assert_eq!(segments[3].fill.as_deref(), Some("#0066cc"));
        assert_eq!(text.font_size, Some(18.0));
        assert!(matches!(text.font_weight, Some(FontWeight::Number(500))));
    }

    #[test]
    fn block_font_props_come_from_computed_style() {
        let children = text_of("<div><h1>Title</h1></div>", "");
        let PenNode::Frame(heading) = &children[0] else {
            panic!()
        };
        let PenNode::Text(text) = &heading.children.as_ref().unwrap()[0] else {
            panic!()
        };
        assert_eq!(text.font_size, Some(32.0));
        use jian_ops_schema::node::text::FontWeight;
        assert!(matches!(text.font_weight, Some(FontWeight::Number(700))));
    }

    #[test]
    fn mixed_inline_and_block_children_split_into_runs() {
        let children = text_of("<div>intro <b>x</b><section></section>tail</div>", "");
        assert_eq!(children.len(), 3);
        assert!(matches!(children[0], PenNode::Text(_)));
        assert!(matches!(children[1], PenNode::Frame(_)));
        assert!(matches!(children[2], PenNode::Text(_)));
    }

    #[test]
    fn whitespace_between_inline_elements_is_not_lost() {
        let children = text_of("<div><p><span>a</span> <span>b</span></p></div>", "");
        let PenNode::Frame(paragraph) = &children[0] else {
            panic!()
        };
        let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
            panic!()
        };
        assert_eq!(content_string(&text.content), "a b");
    }

    #[test]
    fn flex_and_grid_keep_each_direct_inline_element_as_an_item() {
        for display in ["flex", "grid"] {
            let css = format!("nav{{display:{display}}}");
            let children = text_of(
                "<div><nav><a href='#one'>one</a><a href='#two'>two</a></nav></div>",
                &css,
            );
            let PenNode::Frame(nav) = &children[0] else {
                panic!()
            };
            let items = nav.children.as_ref().unwrap();
            assert_eq!(items.len(), 2, "display: {display}");
            let PenNode::Frame(first_link) = &items[0] else {
                panic!("link should be an independent flex/grid item")
            };
            let PenNode::Text(text) = &first_link.children.as_ref().unwrap()[0] else {
                panic!()
            };
            let TextContent::Styled(segments) = &text.content else {
                panic!("standalone links must retain href metadata")
            };
            assert_eq!(segments[0].href.as_deref(), Some("#one"));
        }
    }

    #[test]
    fn computed_display_controls_inline_grouping_and_visibility() {
        let children = text_of(
            "<div>a<span class='item'>b</span><span class='hidden'>x</span><span hidden>y</span>c</div>",
            ".item{display:inline-flex}.hidden{display:none}",
        );
        let [PenNode::Frame(row)] = children.as_slice() else {
            panic!("inline boxes should be grouped into a horizontal row")
        };
        let row_children = row.children.as_ref().unwrap();
        assert_eq!(row_children.len(), 3);
        assert!(
            matches!(&row_children[0], PenNode::Text(text) if content_string(&text.content) == "a")
        );
        assert!(matches!(&row_children[1], PenNode::Frame(_)));
        assert!(
            matches!(&row_children[2], PenNode::Text(text) if content_string(&text.content) == "c")
        );
    }

    #[test]
    fn descendant_selectors_style_nested_inline_segments() {
        let children = text_of(
            "<div class='category'><p><span>detail</span></p></div>",
            ".category span{color:#123456}",
        );
        let PenNode::Frame(paragraph) = &children[0] else {
            panic!()
        };
        let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let TextContent::Styled(segments) = &text.content else {
            panic!()
        };
        assert_eq!(segments[0].fill.as_deref(), Some("#123456"));
    }

    #[test]
    fn single_styled_run_promotes_its_effective_style_to_the_text_node() {
        let children = text_of(
            "<div class='host'><span class='detail'>detail</span></div>",
            ".host{font:400 15px sans-serif;color:#17223b}.detail{font-family:monospace;font-size:12px;font-weight:700;font-style:italic;color:#6e7890;text-decoration:underline}",
        );
        let PenNode::Text(text) = &children[0] else {
            panic!("expected anonymous text")
        };
        let TextContent::Styled(segments) = &text.content else {
            panic!("nested style should remain a rich-text run")
        };
        assert_eq!(segments.len(), 1);
        assert_eq!(text.font_family.as_deref(), Some("monospace"));
        assert_eq!(text.font_size, Some(12.0));
        assert!(matches!(text.font_weight, Some(FontWeight::Number(700))));
        assert_eq!(text.font_style, Some(TextFontStyle::Italic));
        assert_eq!(text.underline, Some(true));
        assert!(
            matches!(text.fill.as_deref(), Some([PenFill::Solid(fill)]) if fill.color == "#6e7890")
        );
    }

    #[test]
    fn supports_css_white_space_modes() {
        let cases = [
            ("normal", "a b c"),
            ("nowrap", "a b c"),
            ("pre", "  a \n b\t c  "),
            ("pre-wrap", "  a \n b\t c  "),
            ("pre-line", "a\nb c"),
            ("break-spaces", "  a \n b\t c  "),
        ];
        for (mode, expected) in cases {
            let css = format!("p{{white-space:{mode}}}");
            let children = text_of("<div><p>  a \n b\t c  </p></div>", &css);
            let PenNode::Frame(paragraph) = &children[0] else {
                panic!()
            };
            let PenNode::Text(text) = &paragraph.children.as_ref().unwrap()[0] else {
                panic!()
            };
            assert_eq!(content_string(&text.content), expected, "mode: {mode}");
        }
    }

    #[test]
    fn applies_text_transform_and_author_semantic_overrides() {
        let children = text_of(
            "<div><p class='upper'>hi <span class='lower'>THERE</span></p><p><b>B</b><i>I</i><u>U</u></p></div>",
            ".upper{text-transform:uppercase}.lower{text-transform:lowercase}b{font-weight:400}i{font-style:normal}u{text-decoration:none}",
        );
        let PenNode::Frame(transformed) = &children[0] else {
            panic!()
        };
        let PenNode::Text(text) = &transformed.children.as_ref().unwrap()[0] else {
            panic!()
        };
        assert_eq!(content_string(&text.content), "HI there");

        let PenNode::Frame(overridden) = &children[1] else {
            panic!()
        };
        let PenNode::Text(text) = &overridden.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let TextContent::Styled(segments) = &text.content else {
            panic!()
        };
        assert_eq!(segments[0].font_weight, Some(400));
        assert_eq!(segments[1].font_style, Some(SegmentFontStyle::Normal));
        assert_eq!(segments[2].underline, None);
    }
}
