//! Text-mapper tests.

use super::*;

fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
    FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

#[test]
fn plain_text_when_no_style_runs() {
    let node = obj(vec![(
        "textData",
        obj(vec![("characters", FigValue::Str("Hello".into()))]),
    )]);
    let props = map_figma_text_props(&node);
    assert_eq!(props.content, TextContent::Plain("Hello".into()));
}

#[test]
fn font_weight_parsed_from_style_name() {
    assert_eq!(parse_font_weight("semibold italic"), Some(600));
    // "extra light" contains "light" → 300 (no "extralight" token).
    assert_eq!(parse_font_weight("extra light"), Some(300));
    assert_eq!(parse_font_weight("extralight"), Some(200));
    assert_eq!(parse_font_weight("extrabold"), Some(800));
    assert_eq!(parse_font_weight("bold"), Some(700));
    assert_eq!(parse_font_weight("regular"), Some(400));
}

#[test]
fn font_props_and_italic_detected() {
    let node = obj(vec![
        (
            "fontName",
            obj(vec![
                ("family", FigValue::Str("Inter".into())),
                ("style", FigValue::Str("Bold Italic".into())),
            ]),
        ),
        ("fontSize", FigValue::Float(18.0)),
        (
            "textData",
            obj(vec![("characters", FigValue::Str("x".into()))]),
        ),
    ]);
    let props = map_figma_text_props(&node);
    assert_eq!(props.font_family.as_deref(), Some("Inter"));
    assert_eq!(props.font_size, Some(18.0));
    assert_eq!(props.font_weight, Some(700));
    assert_eq!(props.font_style, Some(FontStyleKind::Italic));
}

#[test]
fn line_height_pixels_becomes_multiplier() {
    let node = obj(vec![
        ("fontSize", FigValue::Float(20.0)),
        (
            "lineHeight",
            obj(vec![
                ("units", FigValue::Str("PIXELS".into())),
                ("value", FigValue::Float(30.0)),
            ]),
        ),
    ]);
    // 30 / 20 = 1.5.
    assert_eq!(map_line_height(&node), Some(1.5));
}

#[test]
fn text_case_upper_applies() {
    let node = obj(vec![
        ("textCase", FigValue::Str("UPPER".into())),
        (
            "textData",
            obj(vec![("characters", FigValue::Str("hello".into()))]),
        ),
    ]);
    assert_eq!(
        map_figma_text_props(&node).content,
        TextContent::Plain("HELLO".into())
    );
}

#[test]
fn title_case_capitalizes_word_starts() {
    assert_eq!(title_case("hello world-foo"), "Hello World-Foo");
}

#[test]
fn align_and_growth_mapped() {
    let node = obj(vec![
        ("textAlignHorizontal", FigValue::Str("CENTER".into())),
        ("textAlignVertical", FigValue::Str("CENTER".into())),
        ("textAutoResize", FigValue::Str("HEIGHT".into())),
        ("textDecoration", FigValue::Str("UNDERLINE".into())),
    ]);
    let props = map_figma_text_props(&node);
    assert_eq!(props.text_align, Some(TextAlign::Center));
    assert_eq!(props.text_align_vertical, Some(TextAlignVertical::Middle));
    assert_eq!(props.text_growth, Some(TextGrowth::FixedWidth));
    assert_eq!(props.underline, Some(true));
}

#[test]
fn styled_segments_built_from_style_runs() {
    // 4 chars: first 2 style 0, last 2 style 1 (bold override).
    let node = obj(vec![(
        "textData",
        obj(vec![
            ("characters", FigValue::Str("abcd".into())),
            (
                "characterStyleIDs",
                FigValue::Array(vec![
                    FigValue::Int(0),
                    FigValue::Int(0),
                    FigValue::Int(1),
                    FigValue::Int(1),
                ]),
            ),
            (
                "styleOverrideTable",
                FigValue::Array(vec![
                    obj(vec![]),
                    obj(vec![(
                        "fontName",
                        obj(vec![("style", FigValue::Str("Bold".into()))]),
                    )]),
                ]),
            ),
        ]),
    )]);
    match map_figma_text_props(&node).content {
        TextContent::Styled(segs) => {
            assert_eq!(segs.len(), 2);
            assert_eq!(segs[0].text, "ab");
            assert_eq!(segs[1].text, "cd");
            assert_eq!(segs[1].font_weight, Some(700));
        }
        TextContent::Plain(p) => panic!("expected styled, got plain {p:?}"),
    }
}
