//! Third-party brand fence for stock-photo hits.
//!
//! A design placeholder photo must not carry someone else's trademark the
//! brief never named. Generic app/tech queries are where this bites: on
//! Openverse "mobile app illustration" ranks "Facebook App Center iPhone"
//! and a Reddit mascot selfie first (arena m04, 2026-09-27), and the
//! subject fence happily keeps them because they do mention "mobile"/"app".
//! A hit whose title or tags name a well-known brand is rejected unless the
//! query names that same brand.

/// Well-known consumer brands whose marks commonly appear in photo-corpus
/// titles/tags. Kept to names that are never ordinary English words, so a
/// match is always the brand ("apple", "amazon", "blackberry", "chrome",
/// "firefox", "messenger", "adobe" are deliberately absent: fruit, chrome
/// fittings, red pandas, messenger bags and adobe houses are legitimate; so is
/// "google", which Wikimedia puts in every "… - Google Art Project" title).
const BRAND_WORDS: &[&str] = &[
    "facebook",
    "instagram",
    "whatsapp",
    "reddit",
    "twitter",
    "tiktok",
    "youtube",
    "snapchat",
    "pinterest",
    "linkedin",
    "tumblr",
    "iphone",
    "ipad",
    "macbook",
    "microsoft",
    "windows10",
    "xbox",
    "playstation",
    "nintendo",
    "samsung",
    "huawei",
    "xiaomi",
    "nokia",
    "motorola",
    "starbucks",
    "mcdonalds",
    "cocacola",
    "coca",
    "pepsi",
    "nike",
    "adidas",
    "ikea",
    "netflix",
    "spotify",
    "uber",
    "airbnb",
    "wechat",
    "alipay",
    "taobao",
    "disney",
    "pokemon",
    "lego",
    "bmw",
    "mercedes",
    "ferrari",
    "autodesk",
];

/// Words that mark a hit as a logo or trademark artwork. A generic
/// "brand illustration" query ranked "Carbon360 logo" first (arena m04,
/// 2026-09-28): a placeholder must never ship someone's mark.
const LOGO_WORDS: &[&str] = &["logo", "logos", "logotype", "wordmark", "trademark"];

/// True when `metadata_words` names a brand, or marks a logo, that
/// `query_words` does not ask for.
/// Both sides are expected to be lower-cased word lists.
pub(super) fn names_unrequested_brand(metadata_words: &[String], query_words: &[String]) -> bool {
    let asks_for_logo = query_words
        .iter()
        .any(|word| LOGO_WORDS.contains(&word.as_str()));
    metadata_words.iter().any(|word| {
        (BRAND_WORDS.contains(&word.as_str()) && !query_words.iter().any(|q| q == word))
            || (!asks_for_logo && LOGO_WORDS.contains(&word.as_str()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_lowercase).collect()
    }

    #[test]
    fn an_unnamed_brand_is_fenced_and_a_named_one_is_kept() {
        let query = words("mobile app illustration");
        assert!(names_unrequested_brand(
            &words("reddit i am in you android mobile"),
            &query
        ));
        assert!(names_unrequested_brand(
            &words("facebook app center iphone"),
            &query
        ));
        assert!(!names_unrequested_brand(
            &words("sketch app wireframe illustration"),
            &query
        ));
        assert!(!names_unrequested_brand(
            &words("starbucks cup"),
            &words("starbucks cup")
        ));
    }

    #[test]
    fn a_logo_is_fenced_unless_the_query_asks_for_one() {
        let hit = words("carbon360 logo");
        assert!(names_unrequested_brand(&hit, &words("brand illustration")));
        assert!(!names_unrequested_brand(&hit, &words("logo design sample")));
    }

    #[test]
    fn ordinary_words_that_double_as_brands_are_not_fenced() {
        let query = words("lamp");
        for metadata in [
            "chrome desk lamp",
            "apple on a table",
            "adobe house",
            "messenger bag",
            "the starry night google art project",
        ] {
            assert!(
                !names_unrequested_brand(&words(metadata), &query),
                "{metadata}"
            );
        }
    }
}
