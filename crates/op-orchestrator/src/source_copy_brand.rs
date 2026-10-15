//! Only explicit quoted brand declarations become required visible copy.

use regex::Regex;
use std::sync::LazyLock;

pub(super) fn names(brief: &str) -> Vec<String> {
    static LABEL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)(?:品牌(?:名称|名)?|\bbrand(?:\s+name)?)\s*(?:为|是|叫|[:：]|is\b|named\b)?\s*",
        )
        .expect("brand declaration pattern")
    });
    let mut names = Vec::new();
    for label in LABEL.find_iter(brief) {
        let prefix = brief[..label.start()]
            .rsplit(['。', '；', ';', '\n', '.', '，', ','])
            .next()
            .unwrap_or_default()
            .to_lowercase();
        if [
            "不要",
            "无需",
            "不显示",
            "不展示",
            "隐藏",
            "omit",
            "hide",
            "without",
            "do not",
            "don't",
        ]
        .iter()
        .any(|word| prefix.contains(word))
        {
            continue;
        }
        let tail = &brief[label.end()..];
        let Some(open) = tail.chars().next() else {
            continue;
        };
        let close = match open {
            '“' => '”',
            '「' => '」',
            '『' => '』',
            '"' => '"',
            '\'' => '\'',
            _ => continue,
        };
        let tail = &tail[open.len_utf8()..];
        let Some(end) = tail.find(close) else {
            continue;
        };
        let name = tail[..end].trim();
        let suffix = tail[end + close.len_utf8()..]
            .split(['。', '；', ';', '\n', '.', '，', ','])
            .next()
            .unwrap_or_default()
            .to_lowercase();
        if name.is_empty()
            || name.chars().count() > 80
            || name.contains('\n')
            || [
                "仅用图标",
                "只用图标",
                "不显示",
                "不展示",
                "不要文字",
                "icon only",
                "logo only",
            ]
            .iter()
            .any(|word| suffix.contains(word))
        {
            continue;
        }
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
        if names.len() == 8 {
            break;
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_brand_names_preserve_spelling_and_quote_pairs() {
        for brief in [
            "品牌“晚点食堂”，深色海军蓝配暖橙色",
            "品牌名为「晚点食堂」",
            "品牌名称：\"晚点食堂\"",
            "Brand name is '晚点食堂'",
            "brand: “晚点食堂”",
        ] {
            assert_eq!(names(brief), ["晚点食堂"], "{brief}");
        }
        assert_eq!(names("品牌“Acme”，品牌“Acme”"), ["Acme"]);
    }
    #[test]
    fn style_examples_negation_and_icon_only_requests_are_not_copy_contracts() {
        for brief in [
            "品牌风格‘温暖’",
            "不要显示品牌“Acme”",
            "Hide brand 'Acme'",
            "品牌“Acme”仅用图标",
            "brand 'Acme' logo only",
            "品牌“Acme'",
            "unbrand 'Acme'",
            "品牌“”",
        ] {
            assert!(names(brief).is_empty(), "{brief}");
        }
    }
}
