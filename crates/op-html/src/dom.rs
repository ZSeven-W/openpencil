use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, ParseOpts};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

const DROP_TAGS: &[&str] = &[
    "script", "noscript", "template", "meta", "link", "base", "head",
];
pub(crate) const MAX_DOM_DEPTH: usize = 32;
// HTML tokenization replaces source NULs, so parsed author CSS cannot collide
// with this length-prefixed internal envelope.
const INLINE_MEDIA_MARKER: &str = "\0op-html-media:";
pub(crate) const INITIAL_ROOT_FONT_SIZE_ATTR: &str = "\0op-html-initial-root-font-size";

#[derive(Clone, Debug, PartialEq)]
pub enum DomNode {
    Element(DomElement),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomElement {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<DomNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StylesheetSource {
    Inline(String),
    Link(String),
}

impl DomElement {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(attr_name, _)| attr_name == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn classes(&self) -> Vec<&str> {
        self.attr("class")
            .map(|value| value.split_whitespace().collect())
            .unwrap_or_default()
    }

    pub fn id(&self) -> Option<&str> {
        self.attr("id")
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedDom {
    pub body: Vec<DomNode>,
    pub html_attrs: Vec<(String, String)>,
    pub head_attrs: Vec<(String, String)>,
    pub body_attrs: Vec<(String, String)>,
    pub style_blocks: Vec<String>,
    pub stylesheet_sources: Vec<StylesheetSource>,
    /// Document-order candidates. URL validity depends on the caller's document URL.
    pub base_hrefs: Vec<String>,
    pub title: Option<String>,
    pub depth_truncated: bool,
}

#[derive(Default)]
struct StylesheetSetState {
    preferred_title: Option<String>,
}

pub fn parse_dom(source: &str) -> ParsedDom {
    let mut bytes = source.as_bytes();
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut bytes)
        .expect("reading from &[u8] cannot fail");
    let mut out = ParsedDom::default();
    let mut stylesheet_sets = StylesheetSetState {
        preferred_title: preferred_stylesheet_title(&dom.document, 0),
    };
    walk_document(&dom.document, &mut out, &mut stylesheet_sets, 0);
    out
}

fn walk_document(
    handle: &Handle,
    out: &mut ParsedDom,
    stylesheet_sets: &mut StylesheetSetState,
    depth: usize,
) {
    if depth >= MAX_DOM_DEPTH {
        out.depth_truncated = true;
        return;
    }
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        match name.local.as_ref() {
            "html" => {
                out.html_attrs = collect_attrs(&attrs.borrow());
            }
            "head" => {
                out.head_attrs = collect_attrs(&attrs.borrow());
                harvest_head(handle, out, stylesheet_sets, depth);
                return;
            }
            "body" => {
                out.body_attrs = collect_attrs(&attrs.borrow());
                for child in handle.children.borrow().iter() {
                    if let Some(node) = convert(child, out, stylesheet_sets, depth + 1) {
                        out.body.push(node);
                    }
                }
                return;
            }
            _ => {}
        }
    }
    for child in handle.children.borrow().iter() {
        walk_document(child, out, stylesheet_sets, depth + 1);
    }
}

fn collect_attrs(attrs: &[html5ever::Attribute]) -> Vec<(String, String)> {
    attrs
        .iter()
        .map(|attr| {
            (
                attr.name.local.to_string().to_lowercase(),
                attr.value.to_string(),
            )
        })
        .collect()
}

fn preferred_stylesheet_title(handle: &Handle, depth: usize) -> Option<String> {
    if depth >= MAX_DOM_DEPTH {
        return None;
    }
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        let attrs = attrs.borrow();
        let eligible = match name.local.as_ref() {
            "style" => style_type_is_css(&attrs),
            "link" => {
                let rel = attr_value(&attrs, "rel").unwrap_or_default();
                let mut tokens = rel.split_whitespace();
                link_type_is_css(&attrs)
                    && !has_attr(&attrs, "disabled")
                    && href_attr(&attrs).is_some()
                    && tokens
                        .clone()
                        .any(|token| token.eq_ignore_ascii_case("stylesheet"))
                    && !tokens.any(|token| token.eq_ignore_ascii_case("alternate"))
            }
            _ => false,
        };
        if eligible {
            if let Some(title) = attr_value(&attrs, "title").filter(|title| !title.is_empty()) {
                return Some(title.to_string());
            }
        }
    }
    handle
        .children
        .borrow()
        .iter()
        .find_map(|child| preferred_stylesheet_title(child, depth + 1))
}

fn harvest_head(
    handle: &Handle,
    out: &mut ParsedDom,
    stylesheet_sets: &mut StylesheetSetState,
    depth: usize,
) {
    if depth >= MAX_DOM_DEPTH {
        out.depth_truncated = true;
        return;
    }
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        match name.local.as_ref() {
            "style" => {
                push_inline_stylesheet(out, stylesheet_sets, text_content(handle), &attrs.borrow());
                return;
            }
            "link" => {
                if let Some(source) = stylesheet_link(&attrs.borrow(), stylesheet_sets) {
                    out.stylesheet_sources.push(source);
                }
                return;
            }
            "base" => {
                if let Some(href) = href_attr(&attrs.borrow()) {
                    out.base_hrefs.push(href);
                }
                return;
            }
            "title" => {
                out.title = Some(text_content(handle));
                return;
            }
            _ => {}
        }
    }
    for child in handle.children.borrow().iter() {
        harvest_head(child, out, stylesheet_sets, depth + 1);
    }
}

fn push_inline_stylesheet(
    out: &mut ParsedDom,
    stylesheet_sets: &mut StylesheetSetState,
    stylesheet: String,
    attrs: &[html5ever::Attribute],
) {
    if !style_type_is_css(attrs) {
        return;
    }
    out.style_blocks.push(stylesheet.clone());
    if !stylesheet_set_is_active(attrs, stylesheet_sets) {
        return;
    }
    let stylesheet = match media_attr(attrs) {
        Some(media) => encode_inline_media(stylesheet, &media),
        None => stylesheet,
    };
    // `disabled` is not a content attribute on `<style>`. Browsers therefore
    // keep a literal `<style disabled>` active; only its CSSStyleSheet can be
    // disabled through script after the document has loaded.
    out.stylesheet_sources
        .push(StylesheetSource::Inline(stylesheet));
}

fn stylesheet_link(
    attrs: &[html5ever::Attribute],
    stylesheet_sets: &mut StylesheetSetState,
) -> Option<StylesheetSource> {
    if !link_type_is_css(attrs) || has_attr(attrs, "disabled") {
        return None;
    }
    let rel = attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == "rel")?
        .value
        .to_string();
    let rel_tokens: Vec<_> = rel.split_whitespace().collect();
    let alternate = rel_tokens
        .iter()
        .any(|token| token.eq_ignore_ascii_case("alternate"));
    if !rel_tokens
        .iter()
        .any(|token| token.eq_ignore_ascii_case("stylesheet"))
        || (alternate && attr_value(attrs, "title").is_none_or(str::is_empty))
    {
        return None;
    }
    let href = attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == "href")?
        .value
        .to_string();
    if !stylesheet_set_is_active(attrs, stylesheet_sets) {
        return None;
    }
    Some(match media_attr(attrs) {
        Some(media) => StylesheetSource::Inline(format!(
            "@import url(\"{}\") {media};",
            escape_css_string(&href)
        )),
        None => StylesheetSource::Link(href),
    })
}

fn stylesheet_set_is_active(attrs: &[html5ever::Attribute], state: &StylesheetSetState) -> bool {
    let Some(title) = attr_value(attrs, "title").filter(|title| !title.is_empty()) else {
        return true;
    };
    state.preferred_title.as_deref() == Some(title)
}

fn attr_value<'a>(attrs: &'a [html5ever::Attribute], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == name)
        .map(|attr| attr.value.as_ref())
}

fn encode_inline_media(stylesheet: String, media: &str) -> String {
    format!("{INLINE_MEDIA_MARKER}{}:{media}{stylesheet}", media.len())
}

pub(crate) fn decode_inline_media(source: &str) -> Option<(&str, &str)> {
    let encoded = source.strip_prefix(INLINE_MEDIA_MARKER)?;
    let (length, payload) = encoded.split_once(':')?;
    let media_length = length.parse::<usize>().ok()?;
    let media = payload.get(..media_length)?;
    let stylesheet = payload.get(media_length..)?;
    Some((media, stylesheet))
}

fn media_attr(attrs: &[html5ever::Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == "media")
        .map(|attr| attr.value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            // The value is embedded into a synthetic `@media` / `@import`.
            // CSS media syntax never needs rule/string delimiters; replacing
            // them with a known-false query prevents an attribute from
            // escaping that wrapper while keeping ordinary queries intact.
            if value
                .chars()
                .any(|character| matches!(character, '{' | '}' | ';' | '\\' | '\'' | '"'))
            {
                "not all".to_string()
            } else {
                sanitize_import_modifier_queries(&value)
            }
        })
}

fn sanitize_import_modifier_queries(value: &str) -> String {
    let mut queries = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                queries.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    queries.push(&value[start..]);
    queries
        .into_iter()
        .map(|query| {
            let query = query.trim();
            if begins_with_import_modifier(query) {
                "not all"
            } else {
                query
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn begins_with_import_modifier(value: &str) -> bool {
    starts_with_css_keyword(value, "layer", true)
        || starts_with_css_keyword(value, "supports", false)
}

fn starts_with_css_keyword(value: &str, keyword: &str, allow_bare: bool) -> bool {
    let Some(prefix) = value.get(..keyword.len()) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case(keyword) {
        return false;
    }
    match value[keyword.len()..].chars().next() {
        Some('(') => true,
        Some(character) => allow_bare && character.is_ascii_whitespace(),
        None => allow_bare,
    }
}

fn has_attr(attrs: &[html5ever::Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr.name.local.as_ref() == name)
}

fn style_type_is_css(attrs: &[html5ever::Attribute]) -> bool {
    attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == "type")
        .map(|attr| attr.value.as_ref())
        .is_none_or(|value| value.is_empty() || value.eq_ignore_ascii_case("text/css"))
}

fn link_type_is_css(attrs: &[html5ever::Attribute]) -> bool {
    attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == "type")
        .map(|attr| attr.value.as_ref())
        .is_none_or(|value| {
            value.is_empty()
                || value
                    .split_once(';')
                    .map_or(value, |(essence, _)| essence)
                    .trim()
                    .eq_ignore_ascii_case("text/css")
        })
}

fn escape_css_string(value: &str) -> String {
    use std::fmt::Write as _;

    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' | '\\' => {
                escaped.push('\\');
                escaped.push(character);
            }
            '\0' => escaped.push('�'),
            character if character.is_ascii_control() => {
                write!(escaped, "\\{:x} ", character as u32)
                    .expect("writing to a String cannot fail");
            }
            character => escaped.push(character),
        }
    }
    escaped
}

fn href_attr(attrs: &[html5ever::Attribute]) -> Option<String> {
    attrs
        .iter()
        .find(|attr| attr.name.local.as_ref() == "href")
        .map(|attr| attr.value.to_string())
}

fn text_content(handle: &Handle) -> String {
    let mut output = String::new();
    let mut pending = vec![handle.clone()];
    while let Some(node) = pending.pop() {
        if let NodeData::Text { contents } = &node.data {
            output.push_str(&contents.borrow());
        } else {
            pending.extend(node.children.borrow().iter().rev().cloned());
        }
    }
    output
}

fn convert(
    handle: &Handle,
    out: &mut ParsedDom,
    stylesheet_sets: &mut StylesheetSetState,
    depth: usize,
) -> Option<DomNode> {
    if depth >= MAX_DOM_DEPTH {
        out.depth_truncated = true;
        return None;
    }
    match &handle.data {
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.to_string().to_lowercase();
            if tag == "style" {
                push_inline_stylesheet(out, stylesheet_sets, text_content(handle), &attrs.borrow());
                return None;
            }
            if tag == "link" {
                if let Some(source) = stylesheet_link(&attrs.borrow(), stylesheet_sets) {
                    out.stylesheet_sources.push(source);
                }
                return None;
            }
            if tag == "title" {
                out.title = Some(text_content(handle));
                return None;
            }
            if DROP_TAGS.contains(&tag.as_str()) {
                return None;
            }
            let attrs = attrs
                .borrow()
                .iter()
                .map(|attr| {
                    (
                        attr.name.local.to_string().to_lowercase(),
                        attr.value.to_string(),
                    )
                })
                .collect();
            let children = handle
                .children
                .borrow()
                .iter()
                .filter_map(|child| convert(child, out, stylesheet_sets, depth + 1))
                .collect();
            Some(DomNode::Element(DomElement {
                tag,
                attrs,
                children,
            }))
        }
        NodeData::Text { contents } => {
            let text = contents.borrow().to_string();
            // Whitespace-only nodes can be significant in an inline formatting
            // context.  In particular, dropping the node between two inline
            // elements turns `<span>a</span> <span>b</span>` into `ab` before
            // the text mapper has a chance to apply CSS whitespace collapsing.
            (!text.is_empty()).then_some(DomNode::Text(text))
        }
        NodeData::Document => None,
        NodeData::Doctype { .. }
        | NodeData::Comment { .. }
        | NodeData::ProcessingInstruction { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fragment_without_wrapper() {
        let d = parse_dom("<div class=\"a b\" id=\"x\"><p>hi</p></div>");
        assert_eq!(d.body.len(), 1);
        let DomNode::Element(div) = &d.body[0] else {
            panic!("expected element")
        };
        assert_eq!(div.tag, "div");
        assert_eq!(div.classes(), vec!["a", "b"]);
        assert_eq!(div.id(), Some("x"));
        let DomNode::Element(p) = &div.children[0] else {
            panic!("expected p")
        };
        assert_eq!(p.tag, "p");
        assert!(matches!(&p.children[0], DomNode::Text(t) if t == "hi"));
    }

    #[test]
    fn collects_style_blocks_and_title_drops_script() {
        let d = parse_dom(
            "<html><head><title>Page</title><style>.a{color:red}</style></head>\
             <body><script>evil()</script><span>ok</span></body></html>",
        );
        assert_eq!(d.title.as_deref(), Some("Page"));
        assert_eq!(d.style_blocks, vec![".a{color:red}".to_string()]);
        assert_eq!(
            d.stylesheet_sources,
            vec![StylesheetSource::Inline(".a{color:red}".to_string())]
        );
        assert_eq!(d.body.len(), 1);
    }

    #[test]
    fn tolerates_dirty_html() {
        let d = parse_dom("<div><b>unclosed<div>next</div>");
        assert!(!d.body.is_empty());
    }

    #[test]
    fn style_inside_body_is_still_collected() {
        let d = parse_dom("<div><style>p{margin:0}</style><p>x</p></div>");
        assert_eq!(d.style_blocks, vec!["p{margin:0}".to_string()]);
    }

    #[test]
    fn preserves_whitespace_between_inline_elements() {
        let d = parse_dom("<p><span>a</span> <span>b</span></p>");
        let DomNode::Element(paragraph) = &d.body[0] else {
            panic!("expected paragraph")
        };
        assert!(matches!(
            &paragraph.children[1],
            DomNode::Text(text) if text == " "
        ));
    }

    #[test]
    fn preserves_body_attributes_separately_from_body_children() {
        let d = parse_dom("<body class=page lang=zh-CN style='color:red'><main>x</main></body>");
        assert_eq!(
            d.body_attrs,
            vec![
                ("class".to_string(), "page".to_string()),
                ("lang".to_string(), "zh-CN".to_string()),
                ("style".to_string(), "color:red".to_string()),
            ]
        );
        assert_eq!(d.body.len(), 1);
    }

    #[test]
    fn preserves_document_element_and_head_attributes() {
        let d = parse_dom(
            "<html class=theme lang=en><head id=metadata></head><body><p>x</p></body></html>",
        );
        assert_eq!(
            d.html_attrs,
            vec![
                ("class".to_string(), "theme".to_string()),
                ("lang".to_string(), "en".to_string()),
            ]
        );
        assert_eq!(
            d.head_attrs,
            vec![("id".to_string(), "metadata".to_string())]
        );
    }

    #[test]
    fn fragment_keeps_the_parser_generated_document_chain() {
        let d = parse_dom("<main>x</main>");
        assert!(d.html_attrs.is_empty());
        assert!(d.head_attrs.is_empty());
        assert!(matches!(d.body.as_slice(), [DomNode::Element(main)] if main.tag == "main"));
    }

    #[test]
    fn preserves_link_and_inline_stylesheet_source_order() {
        let d = parse_dom(
            "<head><link rel='preload stylesheet' href='a.css'><style>.a{color:red}</style>\
             <link rel='stylesheet' href='b.css'></head><body>x</body>",
        );
        assert_eq!(
            d.stylesheet_sources,
            vec![
                StylesheetSource::Link("a.css".into()),
                StylesheetSource::Inline(".a{color:red}".into()),
                StylesheetSource::Link("b.css".into()),
            ]
        );
    }

    #[test]
    fn encodes_stylesheet_activation_without_changing_the_public_source_shape() {
        let d = parse_dom(
            "<head>\
               <style media='screen and (max-width: 700px)' disabled>.a{color:red}</style>\
               <style type='TEXT/CSS'>.upper{color:red}</style>\
               <style type=''>.empty{color:red}</style>\
               <style type='text/css; charset=utf-8'>.parameterized{color:red}</style>\
               <style type=' text/css '>.spaced{color:red}</style>\
               <style type='text/less'>.ignored{color:red}</style>\
               <link rel='stylesheet' href='print.css' media='print'>\
               <link rel='stylesheet' href='disabled.css' disabled>\
               <link rel='alternate stylesheet' href='alternate.css'>\
               <link rel='stylesheet' href='parameterized.css' type='text/css; charset=utf-8'>\
               <link rel='stylesheet' href='ignored.css' type='text/x-scss'>\
             </head><body>x</body>",
        );
        assert_eq!(
            d.stylesheet_sources,
            vec![
                StylesheetSource::Inline(encode_inline_media(
                    ".a{color:red}".into(),
                    "screen and (max-width: 700px)"
                )),
                StylesheetSource::Inline(".upper{color:red}".into()),
                StylesheetSource::Inline(".empty{color:red}".into()),
                StylesheetSource::Inline("@import url(\"print.css\") print;".into()),
                StylesheetSource::Link("parameterized.css".into()),
            ]
        );
        assert_eq!(
            d.style_blocks,
            [".a{color:red}", ".upper{color:red}", ".empty{color:red}",]
        );
    }

    #[test]
    fn escapes_media_link_href_as_a_single_css_string() {
        let d = parse_dom(
            "<link rel='stylesheet' media='screen' \
             href='theme&quot;); @import url(evil.css); /*&#10;next.css'>",
        );
        assert_eq!(
            d.stylesheet_sources,
            [StylesheetSource::Inline(
                "@import url(\"theme\\\"); @import url(evil.css); /*\\a next.css\") screen;".into()
            )]
        );
    }

    #[test]
    fn media_attribute_delimiters_cannot_escape_the_synthetic_css_wrapper() {
        let d = parse_dom(
            "<style media='screen{}'>.a{color:red}</style>\
             <link rel='stylesheet' href='theme.css' media='screen; print'>",
        );
        assert_eq!(
            d.stylesheet_sources,
            [
                StylesheetSource::Inline(encode_inline_media(".a{color:red}".into(), "not all")),
                StylesheetSource::Inline("@import url(\"theme.css\") not all;".into()),
            ]
        );
    }

    #[test]
    fn media_that_looks_like_an_import_modifier_is_forced_false() {
        let d = parse_dom(
            "<style media='layer(theme)'>.a{color:red}</style>\
             <link rel='stylesheet' href='layer.css' media='LAYER(theme) screen'>\
             <link rel='stylesheet' href='supports.css' media='supports(display: grid)'>\
             <link rel='stylesheet' href='fallback.css' media='layer(theme), screen'>",
        );
        assert_eq!(
            d.stylesheet_sources,
            [
                StylesheetSource::Inline(encode_inline_media(".a{color:red}".into(), "not all")),
                StylesheetSource::Inline("@import url(\"layer.css\") not all;".into()),
                StylesheetSource::Inline("@import url(\"supports.css\") not all;".into()),
                StylesheetSource::Inline("@import url(\"fallback.css\") not all, screen;".into()),
            ]
        );
    }

    #[test]
    fn keeps_persistent_and_first_preferred_stylesheet_set_sources() {
        let d = parse_dom(
            "<head>\
               <style>.persistent{color:black}</style>\
               <link rel='alternate stylesheet' title='light' href='alternate.css'>\
               <link rel='stylesheet' title='light' href='light-a.css'>\
               <style title='dark'>.dark{color:black}</style>\
               <link rel='stylesheet' title='dark' href='dark.css'>\
               <style title='light'>.light{color:black}</style>\
               <link rel='stylesheet' href='persistent.css'>\
             </head><body>x</body>",
        );
        assert_eq!(
            d.stylesheet_sources,
            [
                StylesheetSource::Inline(".persistent{color:black}".into()),
                StylesheetSource::Link("alternate.css".into()),
                StylesheetSource::Link("light-a.css".into()),
                StylesheetSource::Inline(".light{color:black}".into()),
                StylesheetSource::Link("persistent.css".into()),
            ]
        );
        assert_eq!(
            d.style_blocks,
            [
                ".persistent{color:black}",
                ".dark{color:black}",
                ".light{color:black}",
            ]
        );
    }

    #[test]
    fn preserves_base_href_candidates_in_document_order() {
        let parsed = parse_dom(
            "<head><base target=_blank><base href='assets/'><base href='/ignored/'></head>\
             <body><base href='body/'><p>x</p></body>",
        );
        assert_eq!(parsed.base_hrefs, ["assets/", "/ignored/"]);
        assert!(
            matches!(parsed.body.as_slice(), [DomNode::Element(element)] if element.tag == "p")
        );
    }

    #[test]
    fn truncates_pathological_dom_depth_before_recursive_mapping() {
        let mut html = String::new();
        for _ in 0..(MAX_DOM_DEPTH + 100) {
            html.push_str("<div>");
        }
        html.push('x');
        for _ in 0..(MAX_DOM_DEPTH + 100) {
            html.push_str("</div>");
        }
        let parsed = parse_dom(&html);
        assert!(parsed.depth_truncated);
    }
}
