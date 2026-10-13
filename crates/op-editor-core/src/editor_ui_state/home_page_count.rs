//! Explicit page counts in a Home brief, including Chinese screen counts.
fn is_number(c: char) -> bool {
    c.is_ascii_digit() || "零〇一二两三四五六七八九十百千万".contains(c)
}

fn digit(c: char) -> Option<u32> {
    match c {
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

fn count(value: &str) -> Option<u32> {
    if let Ok(value) = value.parse() {
        return Some(value);
    }
    let chars: Vec<_> = value.chars().collect();
    match chars.as_slice() {
        ['十'] => Some(10),
        [one] => digit(*one),
        ['十', one] => Some(10 + digit(*one)?),
        [ten, '十'] => Some(digit(*ten)? * 10),
        [ten, '十', one] => Some(digit(*ten)? * 10 + digit(*one)?),
        _ => None,
    }
}

pub(super) fn explicit_app_page_count(text: &str) -> Option<u32> {
    for (start, c) in text.char_indices() {
        if !is_number(c) || text[..start].chars().next_back().is_some_and(is_number) {
            continue;
        }
        let tail = &text[start..];
        let end = tail.find(|c| !is_number(c)).unwrap_or(tail.len());
        let Some(count) = count(&tail[..end]).filter(|n| (1..=30).contains(n)) else {
            continue;
        };
        let unit = tail[end..].trim_start().to_lowercase();
        if ["页", "屏", "个页面", "个界面", "个屏幕", "page", "screen"]
            .iter()
            .any(|u| unit.starts_with(u))
        {
            return Some(count);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_complete_counts_without_misreading_dimensions_or_large_numbers() {
        for (brief, expected) in [
            ("两屏可交互（375×812）", Some(2)),
            ("2屏跑步 App", Some(2)),
            ("十页", Some(10)),
            ("二十一页", Some(21)),
            ("30 screens", Some(30)),
            ("三十一页", None),
            ("一百二十页", None),
            ("375×812", None),
        ] {
            assert_eq!(explicit_app_page_count(brief), expected, "{brief}");
        }
    }
}
