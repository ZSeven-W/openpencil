//! CSS list markers (`list-style-type` / `list-style-position`).
//!
//! Jian has no marker box, so the marker is materialised as an inline text run
//! prepended to the list item's own inline content (see
//! `text_children::map_children`). That deliberately reuses the existing
//! inline-formatting machinery: the marker inherits the item's typography,
//! participates in white-space collapsing, and never costs an extra node.
//!
//! The visible cost is `list-style-position: outside`, whose hanging marker
//! lives in the item's left padding in a browser but sits inside the content
//! box here. That degradation is reported once per import.

use crate::css::cascade::ComputedStyle;
use crate::dom::{DomElement, DomNode};
use crate::import_warning::ImportWarning;
use crate::mapper::MapCtx;

/// The counter styles this importer can render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkerType {
    Disc,
    Circle,
    Square,
    Decimal,
    DecimalLeadingZero,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

impl MarkerType {
    fn is_ordered(self) -> bool {
        !matches!(self, Self::Disc | Self::Circle | Self::Square)
    }

    /// Bullet glyph or ordinal text, including the space that separates the
    /// marker from the item text.
    pub(crate) fn render(self, ordinal: i64) -> String {
        match self {
            Self::Disc => "\u{2022} ".to_string(),
            Self::Circle => "\u{25e6} ".to_string(),
            Self::Square => "\u{25aa} ".to_string(),
            Self::Decimal => format!("{ordinal}. "),
            Self::DecimalLeadingZero => {
                // The counter style pads the ABSOLUTE value to two digits and
                // prefixes the sign, so -1 renders `-01.` exactly as a browser
                // does. `unsigned_abs` keeps `i64::MIN` from overflowing.
                let magnitude = ordinal.unsigned_abs();
                let sign = if ordinal < 0 { "-" } else { "" };
                if magnitude < 10 {
                    format!("{sign}0{magnitude}. ")
                } else {
                    format!("{sign}{magnitude}. ")
                }
            }
            Self::LowerAlpha => format!("{}. ", alphabetic(ordinal).to_lowercase()),
            Self::UpperAlpha => format!("{}. ", alphabetic(ordinal)),
            Self::LowerRoman => format!("{}. ", roman(ordinal).to_lowercase()),
            Self::UpperRoman => format!("{}. ", roman(ordinal)),
        }
    }
}

/// Marker text for the `<li>` at the end of `path`, or `None` when the element
/// is not a list item or its counter style is `none`.
pub(crate) fn marker_for(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
) -> Option<String> {
    let element = *path.last()?;
    if element.tag != "li" {
        return None;
    }
    if style
        .get("list-style-image")
        .is_some_and(|value| !value.trim().eq_ignore_ascii_case("none"))
    {
        context.warn_once(ImportWarning::ListStyleImageIgnored);
    }
    let marker = resolve_type(context, path, style)?;
    let inside = style
        .get("list-style-position")
        .or_else(|| style.get("list-style").and_then(shorthand_position))
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("inside"));
    if !inside {
        context.warn_once(ImportWarning::ListMarkerPositionOutsideApproximated);
    }
    let ordinal = if marker.is_ordered() {
        ordinal_of(path)
    } else {
        1
    };
    Some(marker.render(ordinal))
}

/// The counter style in effect: an authored `list-style-type` (or the type
/// component of the `list-style` shorthand) wins, otherwise `<ol>` defaults to
/// `decimal` and `<ul>` walks the conventional disc → circle → square cycle.
fn resolve_type(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
) -> Option<MarkerType> {
    let authored = style
        .get("list-style-type")
        .map(str::trim)
        .or_else(|| style.get("list-style").and_then(shorthand_type));
    if let Some(authored) = authored {
        if authored.eq_ignore_ascii_case("none") {
            return None;
        }
        if let Some(marker) = parse_type(authored) {
            return Some(marker);
        }
        context.warn_once(ImportWarning::ListStyleTypeUnsupported {
            value: authored.to_string(),
        });
        return Some(MarkerType::Disc);
    }
    let ordered = path
        .iter()
        .rev()
        .skip(1)
        .find(|element| matches!(element.tag.as_str(), "ul" | "ol" | "menu"))
        .is_some_and(|element| element.tag == "ol");
    if ordered {
        return Some(MarkerType::Decimal);
    }
    let depth = path
        .iter()
        .rev()
        .skip(1)
        .filter(|element| matches!(element.tag.as_str(), "ul" | "menu"))
        .count();
    Some(match depth {
        0 | 1 => MarkerType::Disc,
        2 => MarkerType::Circle,
        _ => MarkerType::Square,
    })
}

pub(crate) fn marker_is_suppressed(style: &ComputedStyle) -> bool {
    style
        .get("list-style-type")
        .map(str::trim)
        .or_else(|| style.get("list-style").and_then(shorthand_type))
        .is_some_and(|value| value.eq_ignore_ascii_case("none"))
}

pub(crate) fn parse_type(value: &str) -> Option<MarkerType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "disc" => Some(MarkerType::Disc),
        "circle" => Some(MarkerType::Circle),
        "square" => Some(MarkerType::Square),
        "decimal" => Some(MarkerType::Decimal),
        "decimal-leading-zero" => Some(MarkerType::DecimalLeadingZero),
        "lower-alpha" | "lower-latin" => Some(MarkerType::LowerAlpha),
        "upper-alpha" | "upper-latin" => Some(MarkerType::UpperAlpha),
        "lower-roman" => Some(MarkerType::LowerRoman),
        "upper-roman" => Some(MarkerType::UpperRoman),
        _ => None,
    }
}

/// The counter-style component of a `list-style` shorthand. The shorthand is
/// never expanded into longhands by the declaration parser, so both the type
/// and the position are read back out of the raw value here.
fn shorthand_type(value: &str) -> Option<&str> {
    let mut fallback = None;
    for token in value.split_whitespace() {
        if token.contains('(') {
            continue;
        }
        if token.eq_ignore_ascii_case("inside") || token.eq_ignore_ascii_case("outside") {
            continue;
        }
        if token.eq_ignore_ascii_case("none") {
            fallback = Some(token);
            continue;
        }
        if parse_type(token).is_some() {
            return Some(token);
        }
    }
    fallback
}

fn shorthand_position(value: &str) -> Option<&str> {
    value
        .split_whitespace()
        .find(|token| token.eq_ignore_ascii_case("inside") || token.eq_ignore_ascii_case("outside"))
}

/// The item's counter value, honouring `<ol start>`, `<ol reversed>` and the
/// per-item `value` attribute.
///
/// Siblings hidden with `display: none` are still counted: resolving every
/// sibling's cascade here would make list mapping quadratic, and an authored
/// hidden list item is rare next to that cost.
///
/// `start` and `value` are HTML reflected `long` attributes, so a number that
/// does not fit a 32-bit integer is not a valid value and the default applies —
/// which is also what keeps the counter from overflowing. The step is
/// saturating anyway, as defence in depth against a future parse widening.
fn ordinal_of(path: &[&DomElement]) -> i64 {
    let Some(item) = path.last() else {
        return 1;
    };
    let Some(parent) = path.get(path.len().wrapping_sub(2)) else {
        return 1;
    };
    let items: Vec<&DomElement> = parent
        .children
        .iter()
        .filter_map(|child| match child {
            DomNode::Element(element) if element.tag == "li" => Some(element),
            _ => None,
        })
        .collect();
    let list = path
        .iter()
        .rev()
        .skip(1)
        .find(|element| matches!(element.tag.as_str(), "ul" | "ol" | "menu"));
    let reversed = list.is_some_and(|list| list.attr("reversed").is_some());
    let step = if reversed { -1 } else { 1 };
    let mut counter = list
        .and_then(|list| list.attr("start"))
        .and_then(parse_reflected_long)
        .unwrap_or(if reversed { items.len() as i64 } else { 1 });
    for candidate in items {
        if let Some(value) = candidate.attr("value").and_then(parse_reflected_long) {
            counter = value;
        }
        if std::ptr::eq(candidate, *item) {
            return counter;
        }
        counter = counter.saturating_add(step);
    }
    counter
}

/// An HTML reflected `long` attribute: a value outside the 32-bit range is not
/// a valid integer for the attribute, so the element keeps its default.
fn parse_reflected_long(value: &str) -> Option<i64> {
    value.trim().parse::<i32>().ok().map(i64::from)
}

/// Bijective base-26: 1 → `A`, 26 → `Z`, 27 → `AA`. Non-positive ordinals have
/// no alphabetic form and fall back to their decimal spelling, as browsers do.
fn alphabetic(ordinal: i64) -> String {
    if ordinal <= 0 {
        return ordinal.to_string();
    }
    let mut remaining = ordinal;
    let mut output = Vec::new();
    while remaining > 0 {
        let index = (remaining - 1) % 26;
        output.push(b'A' + index as u8);
        remaining = (remaining - 1) / 26;
    }
    output.reverse();
    String::from_utf8(output).unwrap_or_else(|_| ordinal.to_string())
}

/// Roman numerals over the classic 1..=3999 range; anything outside it falls
/// back to decimal, matching browser behaviour.
fn roman(ordinal: i64) -> String {
    const TABLE: &[(i64, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    if !(1..=3999).contains(&ordinal) {
        return ordinal.to_string();
    }
    let mut remaining = ordinal;
    let mut output = String::new();
    for (value, glyph) in TABLE {
        while remaining >= *value {
            output.push_str(glyph);
            remaining -= value;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_every_supported_counter_style() {
        assert_eq!(MarkerType::Disc.render(1), "\u{2022} ");
        assert_eq!(MarkerType::Circle.render(1), "\u{25e6} ");
        assert_eq!(MarkerType::Square.render(1), "\u{25aa} ");
        assert_eq!(MarkerType::Decimal.render(12), "12. ");
        assert_eq!(MarkerType::DecimalLeadingZero.render(7), "07. ");
        assert_eq!(MarkerType::DecimalLeadingZero.render(70), "70. ");
        assert_eq!(MarkerType::LowerAlpha.render(1), "a. ");
        assert_eq!(MarkerType::UpperAlpha.render(27), "AA. ");
        assert_eq!(MarkerType::LowerRoman.render(4), "iv. ");
        assert_eq!(MarkerType::UpperRoman.render(1994), "MCMXCIV. ");
    }

    #[test]
    fn alphabetic_and_roman_degrade_outside_their_domain() {
        assert_eq!(alphabetic(0), "0");
        assert_eq!(alphabetic(-3), "-3");
        assert_eq!(alphabetic(26), "Z");
        assert_eq!(alphabetic(52), "AZ");
        assert_eq!(roman(0), "0");
        assert_eq!(roman(4000), "4000");
        assert_eq!(roman(3999), "MMMCMXCIX");
    }

    #[test]
    fn shorthand_yields_the_counter_style_and_position() {
        assert_eq!(shorthand_type("square inside"), Some("square"));
        assert_eq!(shorthand_type("inside upper-roman"), Some("upper-roman"));
        assert_eq!(shorthand_type("none"), Some("none"));
        assert_eq!(shorthand_type("url(dot.png) outside"), None);
        assert_eq!(shorthand_position("square inside"), Some("inside"));
        assert_eq!(shorthand_position("square"), None);
    }

    #[test]
    fn parses_the_aliases_browsers_accept() {
        assert_eq!(parse_type("LOWER-LATIN"), Some(MarkerType::LowerAlpha));
        assert_eq!(parse_type("upper-latin"), Some(MarkerType::UpperAlpha));
        assert_eq!(parse_type("hebrew"), None);
    }

    /// Every text run in the imported tree, in document order.
    fn runs(html: &str) -> (Vec<String>, Vec<String>) {
        use jian_ops_schema::node::text::TextContent;
        use jian_ops_schema::node::PenNode;

        fn walk(nodes: &[PenNode], out: &mut Vec<String>) {
            for node in nodes {
                match node {
                    PenNode::Text(text) => out.push(match &text.content {
                        TextContent::Plain(value) => value.clone(),
                        TextContent::Styled(segments) => segments
                            .iter()
                            .map(|segment| segment.text.as_str())
                            .collect(),
                    }),
                    PenNode::Frame(frame) => {
                        walk(frame.children.as_deref().unwrap_or_default(), out)
                    }
                    _ => {}
                }
            }
        }
        let result = crate::import_html(html, &crate::HtmlImportOptions::default());
        let mut out = Vec::new();
        walk(&result.nodes, &mut out);
        (out, result.warnings)
    }

    #[test]
    fn list_style_none_suppresses_the_marker_without_a_warning() {
        let (texts, warnings) = runs("<ul style='list-style:none'><li>plain</li></ul>");
        assert_eq!(texts, ["plain"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn an_inside_position_needs_no_approximation_warning() {
        let (texts, warnings) =
            runs("<ul style='list-style-position:inside;list-style-type:square'><li>x</li></ul>");
        assert_eq!(texts, ["\u{25aa} x"]);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn reversed_ordered_lists_count_down_from_the_item_count() {
        let (texts, _) = runs("<ol reversed><li>a</li><li>b</li><li>c</li></ol>");
        assert_eq!(texts, ["3. a", "2. b", "1. c"]);

        let (started, _) = runs("<ol reversed start='10'><li>a</li><li>b</li></ol>");
        assert_eq!(started, ["10. a", "9. b"]);
    }

    /// `start` / `value` outside the 32-bit reflected-`long` range are invalid
    /// attribute values, so the list keeps its default counter instead of
    /// seeding an ordinal that overflows on the next sibling.
    #[test]
    fn out_of_range_start_and_value_attributes_cannot_overflow_the_counter() {
        let (giant_start, _) = runs(&format!(
            "<ol start='{}'><li>a</li><li>b</li></ol>",
            i64::MAX
        ));
        assert_eq!(giant_start, ["1. a", "2. b"]);

        let (giant_value, _) = runs(&format!(
            "<ol><li>a</li><li value='{}'>b</li><li>c</li></ol>",
            i64::MAX
        ));
        assert_eq!(giant_value, ["1. a", "2. b", "3. c"]);

        let (negative, _) = runs(&format!(
            "<ol start='{}'><li>a</li><li>b</li></ol>",
            i64::MIN
        ));
        assert_eq!(negative, ["1. a", "2. b"]);

        // The largest value the attribute really can carry still counts up
        // without wrapping, because the counter itself is wider than `i32`.
        let (edge, _) = runs("<ol start='2147483647'><li>a</li><li>b</li></ol>");
        assert_eq!(edge, ["2147483647. a", "2147483648. b"]);
    }

    #[test]
    fn decimal_leading_zero_pads_the_magnitude_of_a_negative_ordinal() {
        assert_eq!(MarkerType::DecimalLeadingZero.render(-1), "-01. ");
        assert_eq!(MarkerType::DecimalLeadingZero.render(-42), "-42. ");
        assert_eq!(MarkerType::DecimalLeadingZero.render(0), "00. ");
        assert_eq!(
            MarkerType::DecimalLeadingZero.render(i64::MIN),
            "-9223372036854775808. "
        );
    }

    #[test]
    fn an_unsupported_counter_style_falls_back_to_a_disc() {
        let (texts, warnings) = runs("<ul style='list-style-type:hebrew'><li>x</li></ul>");
        assert_eq!(texts, ["\u{2022} x"]);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("list-style-type `hebrew`")),
            "{warnings:?}"
        );
    }

    #[test]
    fn list_style_image_and_marker_rules_are_reported() {
        let (_, warnings) = runs(
            "<style>li::marker{color:red}</style>\
             <ul style='list-style-image:url(dot.png)'><li>x</li></ul>",
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("list-style-image")),
            "{warnings:?}"
        );
        assert!(
            warnings.iter().any(|warning| warning.contains("::marker")),
            "{warnings:?}"
        );
    }

    /// The `::marker` report is a selector match, not a substring scan: a
    /// prelude that merely mentions the word in an attribute value is not a
    /// dropped marker rule.
    #[test]
    fn the_marker_report_does_not_fire_on_an_attribute_value() {
        let (_, warnings) = runs(
            "<style>a[href*=\"::marker\"]{color:red}[data-role='li::marker']{color:blue}\
             .marker{color:green}</style><p>x</p>",
        );
        assert!(
            !warnings.iter().any(|warning| warning.contains("::marker")),
            "{warnings:?}"
        );

        let (_, real) = runs("<style>li::MARKER{color:red}</style><ul><li>x</li></ul>");
        assert!(
            real.iter().any(|warning| warning.contains("::marker")),
            "{real:?}"
        );
    }

    /// The marker is generated by the list item, so an enclosing `<a>` never
    /// swallows the bullet into the link.
    #[test]
    fn a_marker_inside_a_link_is_not_part_of_the_link() {
        use jian_ops_schema::node::text::TextContent;
        use jian_ops_schema::node::PenNode;
        let result = crate::import_html(
            "<ul><li><a href='/x'>linked</a></li></ul>",
            &crate::HtmlImportOptions::default(),
        );
        let mut hrefs = Vec::new();
        collect_segment_hrefs(&result.nodes, &mut hrefs);
        assert_eq!(
            hrefs,
            [
                ("\u{2022} ".to_string(), None),
                ("linked".into(), Some("/x".into()))
            ]
        );

        fn collect_segment_hrefs(nodes: &[PenNode], out: &mut Vec<(String, Option<String>)>) {
            for node in nodes {
                match node {
                    PenNode::Text(text) => {
                        if let TextContent::Styled(segments) = &text.content {
                            for segment in segments {
                                out.push((segment.text.clone(), segment.href.clone()));
                            }
                        }
                    }
                    PenNode::Frame(frame) => {
                        collect_segment_hrefs(frame.children.as_deref().unwrap_or_default(), out)
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn markers_inherit_the_items_own_typography() {
        use jian_ops_schema::node::text::TextContent;
        use jian_ops_schema::node::PenNode;
        let result = crate::import_html(
            "<ul style='font-size:24px;text-transform:uppercase'><li>item</li></ul>",
            &crate::HtmlImportOptions::default(),
        );
        let PenNode::Frame(root) = &result.nodes[0] else {
            panic!()
        };
        let PenNode::Frame(list) =
            crate::mapper::unwrap_margin_node(&root.children.as_ref().unwrap()[0])
        else {
            panic!()
        };
        let PenNode::Frame(item) = &list.children.as_ref().unwrap()[0] else {
            panic!()
        };
        let PenNode::Text(text) = &item.children.as_ref().unwrap()[0] else {
            panic!("the marker shares the item's text node")
        };
        assert_eq!(text.font_size, Some(24.0));
        assert!(matches!(&text.content, TextContent::Plain(value) if value == "\u{2022} ITEM"));
    }
}
