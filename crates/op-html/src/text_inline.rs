use crate::css::cascade::ComputedStyle;
use crate::css::selectors::PseudoElement;
use crate::dom::DomElement;
use crate::length::{parse_length, LengthCtx};
use crate::mapper::pseudo::InlinePseudo;
use crate::mapper::MapCtx;

use super::{is_inline_tag, segment_style, InlineTextScope, RawSegment};

pub(super) fn append_inline_pseudo(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    parent_style: &ComputedStyle,
    pseudo: PseudoElement,
    href: Option<String>,
    scope: InlineTextScope<'_>,
    segments: &mut Vec<RawSegment>,
) {
    if let Some(pseudo) = crate::mapper::pseudo::inline_pseudo(context, path, parent_style, pseudo)
    {
        push_inline_pseudo(&pseudo, href, scope, segments);
    }
}

pub(super) fn push_inline_pseudo(
    pseudo: &InlinePseudo,
    href: Option<String>,
    scope: InlineTextScope<'_>,
    segments: &mut Vec<RawSegment>,
) {
    segments.push(RawSegment {
        text: pseudo.content.clone(),
        style: segment_style(&pseudo.style, href, scope.fill_override),
        context: scope.context.child(&pseudo.style),
        forced_break: false,
    });
}

pub(super) fn participates_in_inline_run(
    element: &DomElement,
    style: &ComputedStyle,
    path: &[&DomElement],
    context: &mut MapCtx<'_>,
) -> bool {
    if style.get("display") == Some("contents") {
        return true;
    }
    if matches!(style.get("position"), Some("absolute" | "fixed"))
        || is_boxed_inline(element, style, path, context)
    {
        return false;
    }
    match style.get("display").map(str::trim) {
        Some("inline" | "inline flow" | "contents" | "ruby") => true,
        Some(
            "block" | "flow-root" | "inline-block" | "inline flow-root" | "flex" | "inline-flex"
            | "grid" | "inline-grid" | "block flex" | "inline flex" | "block grid" | "inline grid"
            | "list-item" | "table",
        ) => false,
        Some(_) | None => is_inline_tag(&element.tag),
    }
}

pub(super) fn is_boxed_inline(
    element: &DomElement,
    style: &ComputedStyle,
    path: &[&DomElement],
    context: &mut MapCtx<'_>,
) -> bool {
    if matches!(style.get("position"), Some("absolute" | "fixed")) {
        return false;
    }
    let non_atomic_inline = matches!(
        style.get("display").map(str::trim),
        Some("inline" | "inline flow" | "ruby")
    ) || (style.get("display").is_none() && is_inline_tag(&element.tag));
    let inline_margin = has_nonzero_inline_margin(style, context);
    let inline_level = match style.get("display").map(str::trim) {
        Some("inline-block" | "inline flow-root" | "inline-flex" | "inline flex")
        | Some("inline-grid" | "inline grid") => true,
        Some("inline" | "inline flow" | "ruby") => {
            has_visual_box(style, context)
                || inline_margin
                || has_boxed_generated_content(path, style, context)
                || contains_replaced_descendant(element)
                || matches!(style.get("position"), Some("relative" | "sticky"))
        }
        Some(_) => false,
        None => {
            (is_inline_tag(&element.tag)
                || matches!(
                    element.tag.as_str(),
                    "button" | "img" | "input" | "select" | "svg" | "textarea"
                ))
                && (has_visual_box(style, context)
                    || inline_margin
                    || has_boxed_generated_content(path, style, context)
                    || contains_replaced_descendant(element)
                    || matches!(style.get("position"), Some("relative" | "sticky"))
                    || !is_inline_tag(&element.tag))
        }
    };
    if inline_level
        && non_atomic_inline
        && inline_margin
        && context.containing_width_is_definite
        && element_text_may_wrap(style, element)
    {
        context.warn_once(crate::import_warning::ImportWarning::InlineMarginWrappingApproximated);
    }
    inline_level
}

fn contains_replaced_descendant(element: &DomElement) -> bool {
    element.children.iter().any(|child| match child {
        crate::dom::DomNode::Text(_) => false,
        crate::dom::DomNode::Element(child) => {
            crate::special::is_special_leaf_tag(&child.tag) || contains_replaced_descendant(child)
        }
    })
}

pub(crate) fn has_nonzero_inline_margin(style: &ComputedStyle, context: &MapCtx<'_>) -> bool {
    let length_context = LengthCtx {
        font_size: style.font_size,
        root_font_size: context.opts.base_font_size,
        viewport_w: context.opts.viewport_width,
        viewport_h: context.opts.viewport_height(),
    };
    ["margin-left", "margin-right"].into_iter().any(|name| {
        style
            .get(name)
            .filter(|value| !value.trim().eq_ignore_ascii_case("auto"))
            .and_then(|value| parse_length(value, &length_context))
            .is_some_and(|length| length.resolve(context.containing_width).abs() > f64::EPSILON)
    })
}

pub(crate) fn inline_text_may_wrap(style: &ComputedStyle, text: &str) -> bool {
    !text.is_empty()
        && style_allows_inline_wrapping(style)
        && (!is_single_unbreakable_token(text)
            || matches!(style.get("overflow-wrap"), Some("anywhere" | "break-word"))
            || matches!(style.get("word-break"), Some("break-all")))
}

fn style_allows_inline_wrapping(style: &ComputedStyle) -> bool {
    !matches!(style.get("white-space"), Some("nowrap" | "pre"))
}

fn element_text_may_wrap(style: &ComputedStyle, element: &DomElement) -> bool {
    if !style_allows_inline_wrapping(style) {
        return false;
    }
    let mut saw_text = false;
    let single_token = element_is_single_unbreakable_token(element, &mut saw_text);
    saw_text
        && (!single_token
            || matches!(style.get("overflow-wrap"), Some("anywhere" | "break-word"))
            || matches!(style.get("word-break"), Some("break-all")))
}

fn element_is_single_unbreakable_token(element: &DomElement, saw_text: &mut bool) -> bool {
    if element.tag == "wbr" {
        *saw_text = true;
        return false;
    }
    element.children.iter().all(|child| match child {
        crate::dom::DomNode::Text(text) => {
            *saw_text |= !text.is_empty();
            is_single_unbreakable_token(text)
        }
        crate::dom::DomNode::Element(child) => element_is_single_unbreakable_token(child, saw_text),
    })
}

fn is_single_unbreakable_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn has_visual_box(style: &ComputedStyle, context: &MapCtx<'_>) -> bool {
    if style
        .get("background-image")
        .is_some_and(|value| value != "none")
        || style
            .get("background-color")
            .is_some_and(|value| !is_transparent(value))
        || style.get("box-shadow").is_some_and(|value| value != "none")
        || style.get("filter").is_some_and(|value| value != "none")
        || style.get("transform").is_some_and(|value| value != "none")
        || style
            .get("opacity")
            .is_some_and(|value| value.parse::<f64>().ok() != Some(1.0))
        || matches!(style.get("visibility"), Some("hidden" | "collapse"))
    {
        return true;
    }
    let length_context = LengthCtx {
        font_size: style.font_size,
        root_font_size: context.opts.base_font_size,
        viewport_w: context.opts.viewport_width,
        viewport_h: context.opts.viewport_height(),
    };
    let nonzero = |name: &str| {
        style
            .get(name)
            .and_then(|value| parse_length(value, &length_context))
            .is_some_and(|length| length.resolve(context.containing_width).abs() > f64::EPSILON)
    };
    let has_box_dimension = [
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
        "width",
        "height",
        "min-width",
        "min-height",
        "max-width",
        "max-height",
    ]
    .iter()
    .any(|name| nonzero(name));
    has_box_dimension
        || ["top", "right", "bottom", "left"].iter().any(|side| {
            let border_style = style
                .get(&format!("border-{side}-style"))
                .or_else(|| style.get("border-style"));
            let visible = border_style.is_some_and(|value| {
                !matches!(
                    value.trim().to_ascii_lowercase().as_str(),
                    "none" | "hidden"
                )
            });
            if !visible {
                return false;
            }
            style
                .get(&format!("border-{side}-width"))
                .or_else(|| style.get("border-width"))
                .is_none_or(|value| match value.trim().to_ascii_lowercase().as_str() {
                    "thin" | "medium" | "thick" => true,
                    _ => parse_length(value, &length_context).is_some_and(|length| {
                        length.resolve(context.containing_width).abs() > f64::EPSILON
                    }),
                })
        })
}

pub(crate) fn pseudo_needs_frame(
    style: &ComputedStyle,
    parent_style: &ComputedStyle,
    context: &MapCtx<'_>,
) -> bool {
    // Rich-text segments cannot carry these node-level metrics. Keep a pseudo
    // with an authored override in its own text box instead of silently
    // inheriting the originating element's value from the shared TextNode.
    if ["letter-spacing", "line-height"]
        .into_iter()
        .any(|name| style.get(name) != parent_style.get(name))
    {
        return true;
    }
    if style
        .get("display")
        .is_some_and(|value| value.trim() == "contents")
    {
        return false;
    }
    if matches!(
        style.get("position"),
        Some("absolute" | "fixed" | "relative" | "sticky")
    ) {
        return true;
    }
    // Generated children of flex/grid containers are flex/grid items and are
    // blockified by CSS. Keeping them in the adjacent anonymous text run would
    // erase item-level gap, alignment, ordering, and sizing.
    if establishes_flex_or_grid(parent_style.get("display")) {
        return true;
    }
    match style.get("display").map(str::trim) {
        None | Some("inline" | "inline flow" | "ruby") => {
            has_visual_box(style, context) || has_nonzero_inline_margin(style, context)
        }
        _ => true,
    }
}

pub(crate) fn pseudo_frame_is_inline(style: &ComputedStyle) -> bool {
    !matches!(style.get("position"), Some("absolute" | "fixed"))
        && matches!(
            style.get("display").map(str::trim),
            None | Some(
                "inline"
                    | "inline flow"
                    | "inline-block"
                    | "inline flow-root"
                    | "inline-flex"
                    | "inline flex"
                    | "inline-grid"
                    | "inline grid"
                    | "ruby"
            )
        )
}

fn is_transparent(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "transparent" | "#0000" | "#00000000" | "rgba(0,0,0,0)" | "rgb(0 0 0 / 0)"
    )
}

fn establishes_flex_or_grid(display: Option<&str>) -> bool {
    matches!(
        display.map(str::trim),
        Some(
            "flex"
                | "inline-flex"
                | "grid"
                | "inline-grid"
                | "block flex"
                | "inline flex"
                | "block grid"
                | "inline grid"
        )
    )
}

fn has_boxed_generated_content(
    path: &[&DomElement],
    style: &ComputedStyle,
    context: &MapCtx<'_>,
) -> bool {
    [PseudoElement::Before, PseudoElement::After]
        .into_iter()
        .any(|pseudo| crate::mapper::pseudo::has_boxed_content(context, path, style, pseudo))
}

#[cfg(test)]
mod tests {
    use crate::import_html;
    use crate::HtmlImportOptions;
    use jian_ops_schema::node::text::TextContent;
    use jian_ops_schema::node::FrameNode;
    use jian_ops_schema::node::PenNode;
    use jian_ops_schema::style::StrokeThickness;

    fn children(frame: &FrameNode) -> &[PenNode] {
        frame.children.as_deref().unwrap_or_default()
    }

    fn authored_frame(node: &PenNode) -> &FrameNode {
        let PenNode::Frame(frame) = crate::mapper::unwrap_margin_node(node) else {
            panic!("expected authored frame")
        };
        frame
    }

    fn text_content(content: &TextContent) -> String {
        match content {
            TextContent::Plain(value) => value.clone(),
            TextContent::Styled(segments) => {
                segments.iter().map(|segment| &*segment.text).collect()
            }
        }
    }

    fn named_frame<'a>(nodes: &'a [PenNode], name: &str) -> Option<&'a FrameNode> {
        for node in nodes {
            let PenNode::Frame(frame) = node else {
                continue;
            };
            if frame.base.name.as_deref() == Some(name) {
                return Some(frame);
            }
            if let Some(found) = named_frame(children(frame), name) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn visual_inline_and_boxed_generated_content_remain_frames() {
        let result = import_html(
            "<p>a<span style='background:#f00;padding:2px'>b</span>c<i>x</i></p>\
             <style>i::before{content:'!';background:#0f0}</style>",
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let paragraph = authored_frame(&children(root)[0]);
        let [PenNode::Frame(row)] = children(paragraph) else {
            panic!("inline boxes should share one row")
        };
        assert_eq!(row.base.name.as_deref(), Some("inline-row"));
        assert_eq!(
            row.container.layout,
            Some(jian_ops_schema::node::container::LayoutMode::Horizontal)
        );
        assert!(matches!(children(row)[1], PenNode::Frame(_)));
        assert!(matches!(children(row)[3], PenNode::Frame(_)));
    }

    #[test]
    fn display_contents_descendants_stay_in_the_surrounding_inline_run() {
        let result = import_html(
            "<p>a<span style='display:contents'><span style='display:contents'><b>b</b></span></span>c</p>",
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let paragraph = authored_frame(&children(root)[0]);
        let [PenNode::Text(text)] = children(paragraph) else {
            panic!("display:contents must not split one inline run into vertical children")
        };
        assert_eq!(text_content(&text.content), "abc");
    }

    #[test]
    fn default_inline_pseudos_join_the_parent_text_run() {
        let result = import_html(
            "<p>a<i>b</i>c</p><style>\
             p::before{content:'<'}p::after{content:'>'}\
             i::before{content:'['}i::after{content:']'}</style>",
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let paragraph = authored_frame(&children(root)[0]);
        let paragraph_children = children(paragraph);
        let [PenNode::Text(text)] = paragraph_children else {
            panic!("inline generated content should share the parent text node: {paragraph_children:#?}")
        };
        assert_eq!(text_content(&text.content), "<a[b]c>");
    }

    #[test]
    fn flex_and_grid_pseudos_are_separate_from_anonymous_text_items() {
        for display in ["flex", "grid"] {
            let html = format!(
                "<div style='display:{display};gap:20px'>Label</div>\
                 <style>div::before{{content:'★'}}</style>"
            );
            let result = import_html(&html, &HtmlImportOptions::default());
            let PenNode::Frame(root) = &result.nodes[0] else {
                panic!()
            };
            let PenNode::Frame(container) = &children(root)[0] else {
                panic!()
            };
            let [PenNode::Frame(pseudo), PenNode::Text(label)] = children(container) else {
                panic!("pseudo and anonymous text must be separate {display} items")
            };
            assert_eq!(pseudo.base.name.as_deref(), Some("::before"));
            assert_eq!(text_content(&label.content), "Label");
        }
    }

    #[test]
    fn pseudo_node_preserves_non_segment_typography() {
        let result = import_html(
            "<p>x</p><style>p::before{content:'A';letter-spacing:8px;line-height:3}</style>",
            &HtmlImportOptions::default(),
        );
        let pseudo = named_frame(&result.nodes, "::before").expect("pseudo frame");
        let [PenNode::Text(text)] = children(pseudo) else {
            panic!("pseudo frame should contain its text")
        };
        assert_eq!(text.letter_spacing, Some(8.0));
        assert_eq!(text.line_height, Some(3.0));
    }

    #[test]
    fn pseudo_border_keyword_materializes_a_visual_frame() {
        let result = import_html(
            "<p>x</p><style>p::before{content:'B';border:solid}</style>",
            &HtmlImportOptions::default(),
        );
        let pseudo = named_frame(&result.nodes, "::before").expect("pseudo frame");
        assert!(matches!(
            pseudo.container.stroke.as_ref().map(|stroke| &stroke.thickness),
            Some(StrokeThickness::Uniform(width)) if *width == 3.0
        ));
    }

    #[test]
    fn display_contents_pseudo_keeps_the_originating_link() {
        let result = import_html(
            "<p><a href='#x' style='display:contents'>A</a></p>\
             <style>a::before{content:'B'}</style>",
            &HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let paragraph = authored_frame(&children(root)[0]);
        let [PenNode::Text(text)] = children(paragraph) else {
            panic!("display:contents link should remain one inline run")
        };
        let TextContent::Styled(segments) = &text.content else {
            panic!("linked content should use styled segments")
        };
        assert_eq!(
            segments
                .iter()
                .map(|segment| &*segment.text)
                .collect::<String>(),
            "BA"
        );
        assert!(segments
            .iter()
            .all(|segment| segment.href.as_deref() == Some("#x")));
    }
}
