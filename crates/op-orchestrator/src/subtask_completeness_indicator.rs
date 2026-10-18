//! Pagination markers do not promise repeated content cards or images.

use regex::Regex;
use std::sync::LazyLock;

const CJK_NUMBER: &str = r"(?:[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)";
const CJK_INDICATOR: &str = r"(?:(?:小?圆点|小点|点状)(?:分页|轮播)?指示器|(?:分页|轮播(?:图)?)(?:(?:小?圆点|小点)?指示器|指示(?:小?圆点|小点|点)|小?圆点|小点))";
const ENGLISH_NUMBER: &str =
    r"(?:\d[\d,]*|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)";
const ENGLISH_INDICATOR: &str = r"(?:(?:pagination|carousel|page)[ \t]+(?:dot[ \t]+)?indicators?|(?:pagination|carousel)[ \t]+dots?|dot[ \t]+indicators?|indicator[ \t]+dots?)";

/// Mask only the number/range attached to an explicit indicator noun, keeping
/// offsets and every other promise intact. A nearby carousel or section name
/// alone cannot exempt images, product cards, or an ordinary dotted list.
pub(super) fn mask_counts(text: &str) -> String {
    static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        // Horizontal spacing keeps label/elements and separate lines from
        // borrowing each other's noun or count. Ranges are one masked span.
        let cjk_count = format!(r"{CJK_NUMBER}(?:[ \t]*(?:-|–|—|至|到)[ \t]*{CJK_NUMBER})?");
        let english_count =
            format!(r"{ENGLISH_NUMBER}(?:[ \t]*(?:-|–|—|to)[ \t]*{ENGLISH_NUMBER})?");
        [
            format!(r"(?P<count>{cjk_count})[ \t]*[个枚项][ \t]*{CJK_INDICATOR}"),
            format!(
                r"{CJK_INDICATOR}[ \t]*[:：]?[ \t]*(?:共|为|数量为?|总计)?[ \t]*(?P<count>{cjk_count})[ \t]*[个枚项]"
            ),
            format!(
                r"{CJK_INDICATOR}[ \t]*\([ \t]*(?P<count>{cjk_count})[ \t]*[个枚项][ \t]*\)"
            ),
            format!(
                r"{CJK_INDICATOR}[ \t]*（[ \t]*(?P<count>{cjk_count})[ \t]*[个枚项][ \t]*）"
            ),
            format!(r"(?i)\b(?P<count>{english_count})[ \t]+{ENGLISH_INDICATOR}\b"),
            format!(
                r"(?i)\b{ENGLISH_INDICATOR}\b[ \t]*:?[ \t]*(?:(?:a[ \t]+)?list[ \t]+of[ \t]+)?(?P<count>{english_count})\b"
            ),
        ]
        .into_iter()
        .map(|pattern| Regex::new(&pattern).expect("valid indicator-count pattern"))
        .collect()
    });
    let mut masked = text.as_bytes().to_vec();
    for pattern in PATTERNS.iter() {
        for captures in pattern.captures_iter(text) {
            let full = captures.get(0).unwrap();
            if item_suffix(&text[full.end()..]) {
                continue;
            }
            masked[captures.name("count").unwrap().range()].fill(b' ');
        }
    }
    String::from_utf8(masked).unwrap_or_else(|_| text.to_owned())
}

fn item_suffix(text: &str) -> bool {
    static ITEMS: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)^[\x20\t]*(?:商品|产品|商家|卡片?|列表|条目|项目|清单|行|图片|图集|示例|教程|说明|文档
            |(?:of[\x20\t]+)?(?:cards?|items?|rows?|entries|tiles?|posts?|products?|merchants?|lists?|examples?|tutorials?|images?|photos?)\b)",
        )
        .expect("valid indicator item suffix")
    });
    ITEMS.is_match(text)
}
