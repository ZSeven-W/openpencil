//! Explicit statistics fields display aggregate values, not repeated items.
//! Mask only their numeric spans; a neighboring list keeps its own promise.

use regex::Regex;
use std::sync::LazyLock;

pub(super) fn mask_counts(text: &str) -> String {
    static FIELD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)
            (?:(?:销量(?:[与和/、]?评分)?|评分(?:[与和/、]?销量)?|评价统计|评论统计|统计信息|元数据|元信息)
              (?:行|文字|标签|摘要)
              |\b(?:meta(?:data)?|statistics?|rating(?:[\x20\t]+and[\x20\t]+sales)?)[\x20\t]+(?:row|text|label|summary))
            [\x20\t]*[:：][\x20\t]*")
        .expect("valid explicit statistics field")
    });
    let mut masked = text.as_bytes().to_vec();
    for field in FIELD.find_iter(text) {
        let end = text[field.end()..]
            .find(['\n', '\r', ';', '；', '。', '!', '！', '?', '？', ':', '：'])
            .map_or(text.len(), |offset| field.end() + offset);
        let mut start = field.end();
        let mut values = Vec::new();
        for (offset, ch) in text[start..end].char_indices() {
            let index = field.end() + offset;
            if matches!(ch, ',' | '，' | '·' | '|' | '｜') && !grouped_comma(text, index, ch) {
                values.push(start..index);
                start = index + ch.len_utf8();
            }
        }
        values.push(start..end);
        for (index, span) in values.iter().enumerate() {
            // "3 reviews, each with ..." owns real rows even in a metadata
            // field. A separate "show 3 reviews" keeps its own count instead.
            let continuation = values
                .get(index + 1)
                .is_some_and(|next| item_continuation(&text[next.clone()]));
            if !continuation {
                mask_value(&text[span.clone()], span.start, &mut masked);
            }
        }
    }
    String::from_utf8(masked).unwrap_or_else(|_| text.to_owned())
}

// The mobile normalizer may insert whitespace after a thousands separator.
// Keep both spellings atomic here without changing the authored display copy.
fn grouped_comma(text: &str, index: usize, ch: char) -> bool {
    if ch != ',' || !text[..index].ends_with(|ch: char| ch.is_ascii_digit()) {
        return false;
    }
    let tail = text[index + 1..].trim_start_matches([' ', '\t']);
    tail.chars().take_while(char::is_ascii_digit).count() == 3
}

fn item_continuation(value: &str) -> bool {
    static EACH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)^[\x20\t]*(?:每条|每项|每个|逐条|逐项|(?:each|per)\b)")
            .expect("valid per-item continuation after statistics value")
    });
    static LIST: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?ix)^[\x20\t]*(?:列表(?:含|包含|带)|条目(?:含|包含|带)
            |(?:list|rows?|cards?)[\x20\t]+(?:with|contain|contains|including)\b)",
        )
        .expect("valid list continuation after statistics value")
    });
    static OWN_COUNT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r"(?ix)
            (?:[0-9][0-9,]*|[零〇一二两三四五六七八九十百千万亿]+)
            (?:[\x20\t]*[{}]|[\x20\t]+(?:{})\b)",
            super::MEASURE_WORDS,
            super::ENGLISH_ITEM_WORDS
        ))
        .expect("valid independent list count after statistics value")
    });
    // A later list that declares its own item count does not turn the prior
    // aggregate into a promise. Mere sizes such as 12px are not item counts.
    EACH.is_match(value) || (LIST.is_match(value) && !OWN_COUNT.is_match(value))
}

fn mask_value(value: &str, offset: usize, masked: &mut [u8]) {
    static INSTRUCTION: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?ix)
            列表|网格|卡片|条目|评价卡|评论卡|评价项|评论项|示例|样例|逐项|逐条|每条|每项|每个|包含|列出|生成|显示|展示|绘制|渲染
            |\b(?:list|grid|rows?|cards?|tiles?|render|generate|include|show|display|repeat|each|with|per)\b")
        .expect("valid repeated-item instruction in statistics value")
    });
    // A field's title is not enough to erase a real instruction in its value.
    if INSTRUCTION.is_match(value) {
        return;
    }
    static VALUE: LazyLock<Regex> = LazyLock::new(|| {
        let number = r"(?:[0-9]+(?:,[\x20\t]*[0-9]{3})*|[零〇一二两三四五六七八九十百千万亿]+)";
        Regex::new(&format!(r"(?ix)(?P<count>{number})[\x20\t]*
            (?:(?:条|个|项|款)[\x20\t]*(?:评价|评论|反馈|订单)
              |(?:reviews?|comments?|ratings?|orders?|(?:products?|items?|entries)[\x20\t]+(?:sold|total))\b)"))
            .expect("valid statistics display value")
    });
    for captures in VALUE.captures_iter(value) {
        let count = captures.name("count").unwrap();
        masked[offset + count.start()..offset + count.end()].fill(b' ');
    }
}
