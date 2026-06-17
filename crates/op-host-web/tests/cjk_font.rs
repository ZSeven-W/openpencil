const ROBOTO_FONT: &[u8] = include_bytes!("../assets/Roboto-Regular.ttf");
const INTL_FONT: &[u8] = include_bytes!("../assets/NotoSans-OpenPencilSubset.ttf");
const CJK_FONT: &[u8] = include_bytes!("../assets/NotoSansSC-OpenPencilSubset.otf");
const DEVANAGARI_FONT: &[u8] = include_bytes!("../assets/NotoSansDevanagari-OpenPencilSubset.ttf");
const THAI_FONT: &[u8] = include_bytes!("../assets/NotoSansThai-OpenPencilSubset.ttf");
const ZH_CN_LOCALE: &str = include_str!("../../op-i18n/src/i18n/zh_cn.rs");
const ZH_CN_GIT_LOCALE: &str = include_str!("../../op-i18n/src/i18n/zh_cn_git.rs");

#[test]
fn bundled_cjk_font_covers_chinese_editor_chrome() {
    let faces = parse_faces(CJK_FONT);
    let text = format!("{ZH_CN_LOCALE}\n{ZH_CN_GIT_LOCALE}");
    let missing = missing_glyphs(&faces, &text);

    assert!(
        missing.is_empty(),
        "bundled CJK font is missing glyphs for: {missing:?}"
    );
}

#[test]
fn bundled_web_fallback_fonts_cover_locale_picker_display_names() {
    let font_stack = [ROBOTO_FONT, INTL_FONT, CJK_FONT, DEVANAGARI_FONT, THAI_FONT];
    let faces = font_stack
        .iter()
        .flat_map(|bytes| parse_faces(bytes))
        .collect::<Vec<_>>();
    let text = op_i18n::Locale::ALL
        .iter()
        .map(|locale| locale.display_name())
        .collect::<Vec<_>>()
        .join("\n");
    let missing = missing_glyphs(&faces, &text);

    assert!(
        missing.is_empty(),
        "bundled CJK font is missing locale picker glyphs for: {missing:?}"
    );
}

fn parse_faces(bytes: &[u8]) -> Vec<ttf_parser::Face<'_>> {
    let face_count = ttf_parser::fonts_in_collection(bytes).unwrap_or(1);
    let faces = (0..face_count)
        .filter_map(|index| ttf_parser::Face::parse(bytes, index).ok())
        .collect::<Vec<_>>();

    assert!(!faces.is_empty(), "bundled CJK font must be parseable");
    faces
}

fn missing_glyphs(faces: &[ttf_parser::Face<'_>], text: &str) -> Vec<char> {
    let mut missing = text
        .chars()
        .filter(|ch| !ch.is_ascii() && !ch.is_control() && !ch.is_whitespace())
        .filter(|ch| !is_browser_fallback_symbol(*ch))
        .filter(|ch| faces.iter().all(|face| face.glyph_index(*ch).is_none()))
        .collect::<Vec<_>>();
    missing.sort_unstable();
    missing.dedup();
    missing
}

fn is_browser_fallback_symbol(c: char) -> bool {
    matches!(
        c as u32,
        0x1f000..=0x1faff
            | 0x2600..=0x27bf
            | 0x2b00..=0x2bff
            | 0x2300..=0x23ff
    )
}
