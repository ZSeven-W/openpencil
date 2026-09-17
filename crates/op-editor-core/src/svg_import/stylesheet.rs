//! `<style>` blocks: the subset of CSS that design-tool SVG exports
//! lean on — `.class`, `#id` and bare tag selectors carrying
//! presentation properties (`fill`, `stroke`, `stroke-width`,
//! `fill-rule`) — resolved onto each element before the
//! node builder looks at it.
//!
//! Resolution follows CSS precedence for what the importer can see:
//! a presentation attribute loses to a matching stylesheet rule, and
//! both lose to the inline `style="…"` that
//! [`extract_style_or_attr`] already consults first. Within the sheet,
//! specificity is the usual tag < class < id, ties going to the later
//! rule. Anything the parser does not understand — combinators,
//! pseudo-classes, `@media`, attribute selectors — is skipped whole,
//! so an exotic sheet degrades to "no sheet" rather than to a wrong
//! colour.

use super::*;

/// One parsed rule: the simple selector it applies to and its
/// property declarations in source order.
#[derive(Debug, Clone, PartialEq)]
struct CssRule {
    selector: Selector,
    declarations: Vec<Declaration>,
}

/// One `property: value` pair, with whether it carried `!important`.
#[derive(Debug, Clone, PartialEq)]
struct Declaration {
    property: String,
    value: String,
    important: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Selector {
    Tag(String),
    Class(String),
    /// `rect.cls`: the class, restricted to one element type.
    TagClass(String, String),
    Id(String),
}

impl Selector {
    /// CSS specificity collapsed to one rank: (ids, classes, tags)
    /// compares lexicographically, and the four shapes here never tie
    /// across ranks.
    fn specificity(&self) -> u8 {
        match self {
            Selector::Tag(_) => 0,
            Selector::Class(_) => 1,
            Selector::TagClass(..) => 2,
            Selector::Id(_) => 3,
        }
    }
}

/// Every rule from every `<style>` block of a document, in source
/// order.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Stylesheet {
    rules: Vec<CssRule>,
}

impl Stylesheet {
    pub(super) fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Gather the `<style>` blocks of `body` (the text between the
    /// root `<svg>` tags), CDATA-wrapped or not.
    pub(super) fn from_svg_body(body: &str) -> Stylesheet {
        let lower: String = body.chars().map(|c| c.to_ascii_lowercase()).collect();
        let mut rules = Vec::new();
        let mut from = 0usize;
        while let Some(rel) = lower[from..].find("<style") {
            let open = from + rel;
            let Some(open_end_rel) = lower[open..].find('>') else {
                break;
            };
            let text_start = open + open_end_rel + 1;
            let Some(close_rel) = lower[text_start..].find("</style") else {
                break;
            };
            let text = &body[text_start..text_start + close_rel];
            parse_rules(strip_cdata(text), &mut rules);
            from = text_start + close_rel;
        }
        Stylesheet { rules }
    }

    /// The declarations that apply to an element with `tag`, `id` and
    /// `class` attributes, as a property → value map in winning order:
    /// more specific and later rules win, and an `!important`
    /// declaration wins over every ordinary one whatever its rule's
    /// specificity.
    fn declarations_for(&self, tag: &str, attrs: &[(String, String)]) -> Vec<(String, String)> {
        let id = attrs.iter().find(|(k, _)| k == "id").map(|(_, v)| v.trim());
        let classes: Vec<&str> = attrs
            .iter()
            .find(|(k, _)| k == "class")
            .map(|(_, v)| v.split_whitespace().collect())
            .unwrap_or_default();
        let has_class = |c: &str| classes.contains(&c);
        let mut matched: Vec<(u8, usize, &CssRule)> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| match &rule.selector {
                Selector::Tag(t) => t == tag,
                Selector::Class(c) => has_class(c),
                Selector::TagClass(t, c) => t == tag && has_class(c),
                Selector::Id(i) => id == Some(i.as_str()),
            })
            .map(|(order, rule)| (rule.selector.specificity(), order, rule))
            .collect();
        matched.sort_by_key(|(specificity, order, _)| (*specificity, *order));
        let mut out: Vec<(String, String)> = Vec::new();
        // Two passes in the same order: ordinary declarations first, then
        // the important ones on top of them.
        for important in [false, true] {
            for (_, _, rule) in &matched {
                for decl in rule
                    .declarations
                    .iter()
                    .filter(|d| d.important == important)
                {
                    match out.iter_mut().find(|(k, _)| *k == decl.property) {
                        Some(slot) => slot.1 = decl.value.clone(),
                        None => out.push((decl.property.clone(), decl.value.clone())),
                    }
                }
            }
        }
        out
    }

    /// Write the sheet's winning declarations onto every element's
    /// attribute list, replacing a presentation attribute of the same
    /// name (the sheet outranks it) and leaving inline `style` alone
    /// (it outranks the sheet, and `extract_style_or_attr` reads it
    /// first). Done once, up front, so the rest of the importer keeps
    /// seeing plain attributes.
    pub(super) fn apply(&self, tree: &mut [SvgTree]) {
        if self.is_empty() {
            return;
        }
        for el in tree {
            for (k, v) in self.declarations_for(&el.tag, &el.attrs) {
                match el.attrs.iter_mut().find(|(ak, _)| *ak == k) {
                    Some(slot) => slot.1 = v,
                    None => el.attrs.push((k, v)),
                }
            }
            self.apply(&mut el.children);
        }
    }
}

/// The presentation properties the importer understands; other
/// declarations are dropped at parse time so they can never shadow a
/// same-named attribute the builder does read.
const KNOWN_PROPERTIES: &[&str] = &["fill", "stroke", "stroke-width", "fill-rule"];

fn strip_cdata(text: &str) -> &str {
    let t = text.trim();
    t.strip_prefix("<![CDATA[")
        .and_then(|inner| inner.strip_suffix("]]>"))
        .unwrap_or(t)
}

/// Parse `selector { decl; decl } …` blocks. A selector list
/// (`.a, .b`) yields one rule per simple selector; a selector the
/// importer cannot match is skipped along with its block, and an
/// at-rule (`@media`, `@font-face`, …) is skipped as a whole, nested
/// braces included, so neither its conditional rules nor the rule
/// after it are misread.
fn parse_rules(css: &str, out: &mut Vec<CssRule>) {
    let css = strip_comments(css);
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        let selectors = rest[..open].trim();
        if selectors.starts_with('@') {
            rest = match balanced_block_end(rest, open) {
                Some(end) => &rest[end + 1..],
                None => break,
            };
            continue;
        }
        let Some(close_rel) = rest[open..].find('}') else {
            break;
        };
        let block = &rest[open + 1..open + close_rel];
        rest = &rest[open + close_rel + 1..];
        let declarations: Vec<Declaration> = block
            .split(';')
            .filter_map(|decl| decl.split_once(':'))
            .map(|(k, v)| {
                let value = v.trim();
                let (value, important) = match value.strip_suffix("!important") {
                    Some(bare) => (bare.trim(), true),
                    None => (value, false),
                };
                Declaration {
                    property: k.trim().to_ascii_lowercase(),
                    value: value.to_string(),
                    important,
                }
            })
            .filter(|d| KNOWN_PROPERTIES.contains(&d.property.as_str()) && !d.value.is_empty())
            .collect();
        if declarations.is_empty() {
            continue;
        }
        for selector in selectors.split(',') {
            let Some(selector) = parse_simple_selector(selector.trim()) else {
                continue;
            };
            out.push(CssRule {
                selector,
                declarations: declarations.clone(),
            });
        }
    }
}

/// Index of the `}` that closes the `{` at `open`, counting nesting.
fn balanced_block_end(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_simple_selector(s: &str) -> Option<Selector> {
    if s.is_empty()
        || s.chars()
            .any(|c| c.is_whitespace() || matches!(c, '>' | '+' | '~' | ':' | '[' | '*'))
    {
        return None;
    }
    if let Some(class) = s.strip_prefix('.') {
        return (!class.is_empty() && !class.contains('.'))
            .then(|| Selector::Class(class.to_string()));
    }
    if let Some(id) = s.strip_prefix('#') {
        return (!id.is_empty()).then(|| Selector::Id(id.to_string()));
    }
    // `rect.cls` — the class restricted to one element type.
    if let Some((tag, class)) = s.split_once('.') {
        let tag_ok = !tag.is_empty() && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        return (tag_ok && !class.is_empty() && !class.contains('.'))
            .then(|| Selector::TagClass(tag.to_ascii_lowercase(), class.to_string()));
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
        .then(|| Selector::Tag(s.to_ascii_lowercase()))
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(css: &str) -> Stylesheet {
        Stylesheet::from_svg_body(&format!("<style>{css}</style>"))
    }

    fn el(tag: &str, attrs: &[(&str, &str)]) -> SvgTree {
        SvgTree {
            tag: tag.to_string(),
            attrs: attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            children: Vec::new(),
        }
    }

    fn attr<'a>(el: &'a SvgTree, k: &str) -> Option<&'a str> {
        el.attr(k)
    }

    #[test]
    fn class_id_and_tag_rules_resolve_with_css_precedence() {
        let s = sheet("rect { fill: #111 } .a { fill: #222; stroke: red } #x { fill: #333 }");
        let mut tree = vec![
            el("rect", &[]),
            el("rect", &[("class", "a")]),
            el("rect", &[("class", "a"), ("id", "x")]),
        ];
        s.apply(&mut tree);
        assert_eq!(attr(&tree[0], "fill"), Some("#111"));
        assert_eq!(attr(&tree[1], "fill"), Some("#222"));
        assert_eq!(attr(&tree[1], "stroke"), Some("red"));
        assert_eq!(attr(&tree[2], "fill"), Some("#333"));
        assert_eq!(attr(&tree[2], "stroke"), Some("red"));
    }

    #[test]
    fn a_sheet_rule_beats_the_presentation_attribute_but_not_inline_style() {
        let s = sheet(".a { fill: #222 }");
        let mut tree = vec![el(
            "rect",
            &[("fill", "#000"), ("class", "a"), ("style", "fill:#999")],
        )];
        s.apply(&mut tree);
        assert_eq!(attr(&tree[0], "fill"), Some("#222"));
        assert_eq!(
            extract_style_or_attr(&tree[0].attrs, "fill").as_deref(),
            Some("#999")
        );
    }

    #[test]
    fn later_rules_win_ties_and_selector_lists_split() {
        let s = sheet(".a, .b { fill: #111 } .a { fill: #222 }");
        let mut tree = vec![el("path", &[("class", "a")]), el("path", &[("class", "b")])];
        s.apply(&mut tree);
        assert_eq!(attr(&tree[0], "fill"), Some("#222"));
        assert_eq!(attr(&tree[1], "fill"), Some("#111"));
    }

    #[test]
    fn an_important_declaration_beats_a_more_specific_ordinary_one() {
        let s = sheet(".a { fill: #111 !important } #x { fill: #222 } .a { stroke: red !important; stroke: blue }");
        let mut tree = vec![el("rect", &[("class", "a"), ("id", "x")])];
        s.apply(&mut tree);
        assert_eq!(attr(&tree[0], "fill"), Some("#111"));
        assert_eq!(attr(&tree[0], "stroke"), Some("red"));
    }

    #[test]
    fn a_tag_qualified_class_applies_only_to_that_tag() {
        let s = sheet("rect.k { fill: #111 } .k { fill: #222 }");
        let mut tree = vec![
            el("rect", &[("class", "k")]),
            el("circle", &[("class", "k")]),
        ];
        s.apply(&mut tree);
        assert_eq!(
            attr(&tree[0], "fill"),
            Some("#111"),
            "rect.k outranks .k on a rect"
        );
        assert_eq!(
            attr(&tree[1], "fill"),
            Some("#222"),
            "rect.k does not reach a circle"
        );
    }

    #[test]
    fn an_at_rule_is_skipped_whole_and_the_rule_after_it_survives() {
        let s = sheet("@media print { .a { fill: red } .b { fill: red } } .b { fill: #222 }");
        let mut tree = vec![el("rect", &[("class", "a")]), el("rect", &[("class", "b")])];
        s.apply(&mut tree);
        assert!(
            attr(&tree[0], "fill").is_none(),
            "a rule conditional on @media must not apply"
        );
        assert_eq!(
            attr(&tree[1], "fill"),
            Some("#222"),
            "the rule after the block still lands"
        );
    }

    #[test]
    fn cdata_comments_media_and_complex_selectors_are_tolerated() {
        let s = Stylesheet::from_svg_body(
            "<style type=\"text/css\"><![CDATA[ /* c */ @media print { .a { fill: red } } g > .a { fill: blue } .a:hover { fill: green } .a { fill: #abc; unknown: 1 } ]]></style>",
        );
        let mut tree = vec![el("rect", &[("class", "a")])];
        s.apply(&mut tree);
        assert_eq!(attr(&tree[0], "fill"), Some("#abc"));
        assert!(attr(&tree[0], "unknown").is_none());
    }

    #[test]
    fn rules_reach_nested_children_and_an_empty_sheet_is_a_no_op() {
        let s = sheet(".k { stroke-width: 3 }");
        let mut parent = el("g", &[]);
        parent.children.push(el("circle", &[("class", "k")]));
        let mut tree = vec![parent];
        s.apply(&mut tree);
        assert_eq!(attr(&tree[0].children[0], "stroke-width"), Some("3"));
        let mut untouched = vec![el("rect", &[("fill", "#000")])];
        Stylesheet::from_svg_body("<rect/>").apply(&mut untouched);
        assert_eq!(attr(&untouched[0], "fill"), Some("#000"));
    }
}
