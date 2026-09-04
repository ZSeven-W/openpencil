use crate::import_warning::ImportWarning;
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::{FrameNode, PenNode};
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

use crate::css::cascade::{compute_pseudo_style_for_viewport, ComputedStyle};
use crate::css::selectors::PseudoElement;
use crate::dom::DomElement;
use crate::mapper::{apply_base_style, MapCtx};

pub(crate) struct InlinePseudo {
    pub content: String,
    pub style: ComputedStyle,
}

pub(crate) enum MappedPseudo {
    Inline(InlinePseudo),
    Frame {
        node: Box<PenNode>,
        inline_level: bool,
    },
}

/// Map generated content to inline text unless it needs its own CSS box.
///
/// An empty quoted `content` value is still materialized because it is commonly
/// used for decorative overlays. Missing, `none`, `normal`, and malformed
/// content values do not generate a node.
pub(crate) fn map_pseudo(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    parent_style: &ComputedStyle,
    layout_parent_style: &ComputedStyle,
    pseudo: PseudoElement,
    text_fill_override: Option<&str>,
) -> Option<MappedPseudo> {
    let (style, generated) = resolved_pseudo(context, path, parent_style, pseudo)?;
    let needs_frame = crate::text::inline::pseudo_needs_frame(&style, parent_style, context);
    if needs_frame
        && crate::text::inline::pseudo_frame_is_inline(&style)
        && crate::text::inline::has_nonzero_inline_margin(&style, context)
        && context.containing_width_is_definite
        && crate::text::inline::inline_text_may_wrap(&style, &generated)
    {
        context.warn_once(ImportWarning::InlineMarginWrappingApproximated);
    }
    if !needs_frame {
        return Some(MappedPseudo::Inline(InlinePseudo {
            content: generated,
            style,
        }));
    }
    let required_nodes = 1 + usize::from(!generated.is_empty());
    // The originating element reserves its slot before generated content.
    if context.node_count.saturating_add(required_nodes) > crate::MAX_OUTPUT_NODES {
        context.warn_once(ImportWarning::NodeLimitPseudo);
        return None;
    }
    let can_transfer_text_clip = crate::mapper::text_scope::style_allows_text_clip_transfer(&style);
    let (mut container, local_text_fill) =
        crate::mapper::text_scope::container_props_with_text_scope(
            &style,
            context,
            can_transfer_text_clip,
        );
    let text_fill_override = local_text_fill.as_deref().or(text_fill_override);
    apply_inset_sizing(&style, &mut container.width, &mut container.height);

    let name = pseudo_name(pseudo);
    let mut base = PenNodeBase {
        id: context.generate_id(),
        name: Some(name.to_string()),
        ..Default::default()
    };
    // Generated content is never re-parented into an offset wrapper, so an
    // in-flow shift on it has nowhere to go.
    let outcome = apply_base_style(&mut base, &style, context);
    crate::mapper::warn_dropped_flow_offset(context, outcome);

    let mut children = if generated.is_empty() {
        Vec::new()
    } else {
        crate::text::build_generated_text_node(context, &generated, &style, text_fill_override)
            .into_iter()
            .collect()
    };
    let collapsed =
        super::margin::finish_children(context, &style, Some(layout_parent_style), &mut children);
    context.node_count += 1;
    let inline_level = crate::text::inline::pseudo_frame_is_inline(&style);
    let node = PenNode::Frame(FrameNode {
        base,
        container,
        children: Some(children),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        screen: None,
        breakpoint: None,
    });
    let node = super::margin::wrap_margins(
        context,
        node,
        name,
        &style,
        Some(layout_parent_style),
        collapsed,
    );
    Some(MappedPseudo::Frame {
        node: Box::new(node),
        inline_level,
    })
}

pub(crate) fn inline_pseudo(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    parent_style: &ComputedStyle,
    pseudo: PseudoElement,
) -> Option<InlinePseudo> {
    let (style, content) = resolved_pseudo(context, path, parent_style, pseudo)?;
    (!crate::text::inline::pseudo_needs_frame(&style, parent_style, context))
        .then_some(InlinePseudo { content, style })
}

pub(crate) fn has_boxed_content(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    parent_style: &ComputedStyle,
    pseudo: PseudoElement,
) -> bool {
    resolved_pseudo(context, path, parent_style, pseudo).is_some_and(|(style, _)| {
        crate::text::inline::pseudo_needs_frame(&style, parent_style, context)
    })
}

fn resolved_pseudo(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    parent_style: &ComputedStyle,
    pseudo: PseudoElement,
) -> Option<(ComputedStyle, String)> {
    let element = *path.last()?;
    let style = compute_pseudo_style_for_viewport(
        path,
        context.rules,
        Some(parent_style),
        context.opts.base_font_size,
        pseudo,
        context.opts.viewport_width,
        context.opts.viewport_height(),
    );
    if style.get("display").is_some_and(is_none_keyword) {
        return None;
    }
    let generated = generated_content(element, &style)?;
    Some((style, generated))
}

fn pseudo_name(pseudo: PseudoElement) -> &'static str {
    match pseudo {
        PseudoElement::Before => "::before",
        PseudoElement::After => "::after",
    }
}

fn is_none_keyword(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("none")
}

fn apply_inset_sizing(
    style: &ComputedStyle,
    width: &mut Option<SizingBehavior>,
    height: &mut Option<SizingBehavior>,
) {
    if !matches!(style.get("position"), Some("absolute" | "fixed")) {
        return;
    }
    if width.is_none() && has_inset_pair(style, "left", "right") {
        *width = Some(SizingBehavior::Keyword(SizingKeyword::FillContainer));
    }
    if height.is_none() && has_inset_pair(style, "top", "bottom") {
        *height = Some(SizingBehavior::Keyword(SizingKeyword::FillContainer));
    }
}

fn has_inset_pair(style: &ComputedStyle, start: &str, end: &str) -> bool {
    [start, end].into_iter().all(|name| {
        style
            .get(name)
            .is_some_and(|value| !value.trim().eq_ignore_ascii_case("auto"))
    })
}

fn generated_content(element: &DomElement, style: &ComputedStyle) -> Option<String> {
    ContentParser::new(style.get("content")?).parse(element)
}

struct ContentParser<'a> {
    input: &'a str,
    at: usize,
}

impl<'a> ContentParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, at: 0 }
    }

    fn parse(mut self, element: &DomElement) -> Option<String> {
        self.skip_space();
        let remaining = self.input[self.at..].trim();
        if remaining.is_empty()
            || remaining.eq_ignore_ascii_case("none")
            || remaining.eq_ignore_ascii_case("normal")
        {
            return None;
        }

        let mut output = String::new();
        let mut saw_token = false;
        while !self.eof() {
            self.skip_space();
            if self.eof() {
                break;
            }
            match self.peek()? {
                '\'' | '"' => output.push_str(&self.quoted_string()?),
                _ => output.push_str(&self.attribute_value(element)?),
            }
            saw_token = true;
        }
        saw_token.then_some(output)
    }

    fn quoted_string(&mut self) -> Option<String> {
        let quote = self.bump()?;
        let mut output = String::new();
        loop {
            let ch = self.bump()?;
            match ch {
                ch if ch == quote => return Some(output),
                '\\' => self.escape_into(&mut output)?,
                '\n' | '\r' | '\u{000c}' => return None,
                _ => output.push(ch),
            }
        }
    }

    fn attribute_value(&mut self, element: &DomElement) -> Option<String> {
        let function = self.identifier()?;
        if !function.eq_ignore_ascii_case("attr") {
            return None;
        }
        self.skip_space();
        if self.bump()? != '(' {
            return None;
        }
        self.skip_space();
        let name = self.identifier()?;
        self.skip_space();
        if self.bump()? != ')' {
            return None;
        }
        let normalized = name.to_ascii_lowercase();
        Some(element.attr(&normalized).unwrap_or_default().to_string())
    }

    fn identifier(&mut self) -> Option<String> {
        let mut output = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') || !ch.is_ascii() {
                output.push(self.bump()?);
            } else if ch == '\\' {
                self.bump();
                self.escape_into(&mut output)?;
            } else {
                break;
            }
        }
        (!output.is_empty()).then_some(output)
    }

    fn escape_into(&mut self, output: &mut String) -> Option<()> {
        let first = self.bump()?;
        match first {
            '\n' | '\u{000c}' => Some(()),
            '\r' => {
                if self.peek() == Some('\n') {
                    self.bump();
                }
                Some(())
            }
            ch if ch.is_ascii_hexdigit() => {
                let mut digits = String::from(ch);
                while digits.len() < 6 && self.peek().is_some_and(|ch| ch.is_ascii_hexdigit()) {
                    digits.push(self.bump()?);
                }
                if self.peek().is_some_and(char::is_whitespace) {
                    let whitespace = self.bump()?;
                    if whitespace == '\r' && self.peek() == Some('\n') {
                        self.bump();
                    }
                }
                let scalar = u32::from_str_radix(&digits, 16).ok()?;
                output.push(valid_scalar(scalar));
                Some(())
            }
            ch => {
                output.push(ch);
                Some(())
            }
        }
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.at..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.at += ch.len_utf8();
        Some(ch)
    }

    fn eof(&self) -> bool {
        self.at == self.input.len()
    }
}

fn valid_scalar(value: u32) -> char {
    if value == 0 {
        '\u{fffd}'
    } else {
        char::from_u32(value).unwrap_or('\u{fffd}')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::cascade::{compute_style_for_viewport, parse_stylesheet_for_viewport};
    use jian_ops_schema::node::container::CornerRadius;
    use jian_ops_schema::style::{PenEffect, PenFill};

    fn element(attrs: &[(&str, &str)]) -> DomElement {
        DomElement {
            tag: "div".into(),
            attrs: attrs
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into()))
                .collect(),
            children: Vec::new(),
        }
    }

    fn map(css: &str, element: &DomElement, pseudo: PseudoElement) -> Option<MappedPseudo> {
        let options = crate::HtmlImportOptions::default();
        let viewport_h = options.viewport_height();
        let (rules, _warnings) =
            parse_stylesheet_for_viewport(css, 0, options.viewport_width, viewport_h);
        let parent = compute_style_for_viewport(
            &[element],
            &rules,
            None,
            options.base_font_size,
            options.viewport_width,
            viewport_h,
        );
        let mut context = MapCtx {
            opts: &options,
            rules: &rules,
            warnings: Vec::new(),
            warned: Default::default(),
            next_id: 0,
            node_count: 0,
            containing_width: options.viewport_width,
            containing_height: viewport_h,
            containing_width_is_definite: true,
            positioned_width: options.viewport_width,
            positioned_height: viewport_h,
            auto_margin_handled_by_parent: false,
            pending_base_outcome: Default::default(),
        };
        map_pseudo(&mut context, &[element], &parent, &parent, pseudo, None)
    }

    #[test]
    fn maps_quoted_escaped_and_attribute_content() {
        let node = element(&[("data-sku", "A17")]);
        let mapped = map(
            r#"div::after{content:"Sale \2192 " attr(data-sku) "!";color:#f00}"#,
            &node,
            PseudoElement::After,
        )
        .unwrap();
        let MappedPseudo::Inline(inline) = mapped else {
            panic!("default generated content should remain inline")
        };
        assert_eq!(inline.content, "Sale →A17!");
    }

    #[test]
    fn none_normal_missing_and_display_none_do_not_generate() {
        let node = element(&[]);
        for css in [
            "div::before{content:none}",
            "div::before{content:normal}",
            "div::before{color:red}",
            "div::before{content:'x';display:none}",
        ] {
            assert!(map(css, &node, PseudoElement::Before).is_none(), "{css}");
        }
    }

    #[test]
    fn empty_content_keeps_decorative_overlay_and_inset_sizing() {
        let node = element(&[]);
        let mapped = map(
            "div::after{content:'';position:absolute;inset:0;\
             background:linear-gradient(90deg,#000,#fff);border-radius:12px;\
             box-shadow:0 2px 8px #0008}",
            &node,
            PseudoElement::After,
        )
        .unwrap();
        let MappedPseudo::Frame { node, inline_level } = mapped else {
            panic!("expected generated frame")
        };
        let PenNode::Frame(frame) = *node else {
            panic!("expected generated frame")
        };
        assert!(!inline_level);
        assert!(frame.children.as_ref().unwrap().is_empty());
        assert_eq!(frame.container.width, Some(SizingBehavior::Number(1440.0)));
        assert_eq!(frame.container.height, Some(SizingBehavior::Number(900.0)));
        assert!(matches!(
            frame.container.fill.as_deref(),
            Some([PenFill::LinearGradient(_)])
        ));
        assert_eq!(
            frame.container.corner_radius,
            Some(CornerRadius::Uniform(12.0))
        );
        assert!(matches!(
            frame.container.effects.as_deref(),
            Some([PenEffect::Shadow(_)])
        ));
    }

    #[test]
    fn missing_attribute_is_valid_empty_generated_content() {
        let node = element(&[]);
        let mapped = map(
            "div::before{content:attr(data-missing);background:red}",
            &node,
            PseudoElement::Before,
        )
        .unwrap();
        let MappedPseudo::Frame { node, inline_level } = mapped else {
            panic!()
        };
        let PenNode::Frame(frame) = *node else {
            panic!()
        };
        assert!(inline_level);
        assert!(frame.children.as_ref().unwrap().is_empty());
    }

    #[test]
    fn rejects_unquoted_or_malformed_content_tokens() {
        let node = element(&[]);
        for css in [
            "div::after{content:hello}",
            "div::after{content:'unterminated}",
            "div::after{content:attr(data-x}",
        ] {
            assert!(map(css, &node, PseudoElement::After).is_none(), "{css}");
        }
    }
}
