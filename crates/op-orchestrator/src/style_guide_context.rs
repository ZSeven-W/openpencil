//! 规划 prompt 的 style-guide 上下文构造 —— port of
//! `orchestrator-prompt-optimizer.ts` 的 catalog 路径。

// Functions are pub(crate); callers added in B3/B4/B5.
#![allow(dead_code)]

use crate::design_type::contains_word;

/// `lower` 含 `words` 任一(按 `contains_word`:ASCII 词边界 / CJK 子串)。
fn any(lower: &str, words: &[&str]) -> bool {
    words.iter().any(|w| contains_word(lower, w))
}

/// prompt → 风格 tag 列表 —— port of `inferTagsFromPrompt`
/// (`orchestrator-prompt-optimizer.ts:439-550`)。**无去重、无上限**,
/// 顺序 = 组顺序;空结果 → `["minimal","light-mode"]`。
pub(crate) fn infer_tags_from_prompt(prompt: &str) -> Vec<String> {
    let lower = prompt.to_lowercase();
    let mut tags: Vec<String> = Vec::new();
    let mut push = |t: &str| tags.push(t.to_string());

    // —— tone(互斥)——
    if any(&lower, &["dark", "cyber", "terminal", "neon", "暗"]) {
        push("dark-mode");
    } else {
        push("light-mode");
    }
    // —— visual ——
    if any(&lower, &["minimal", "minimalist", "clean", "极简", "简洁"]) {
        push("minimal");
    }
    if any(&lower, &["brutal", "brutalist", "brutalism", "粗犷"]) {
        push("brutalist");
    }
    if any(&lower, &["elegant", "luxury", "luxurious", "优雅", "奢华"]) {
        push("elegant");
    }
    if any(&lower, &["playful", "fun", "whimsical", "活泼", "趣味"]) {
        push("playful");
    }
    if any(&lower, &["modern", "modernist", "contemporary", "现代"]) {
        push("modern");
    }
    // —— industry ——
    if any(
        &lower,
        &[
            "food",
            "delivery",
            "restaurant",
            "takeout",
            "cuisine",
            "recipe",
            "meal",
            "diner",
            "dining",
            "eatery",
            "cafe",
            "café",
            "餐",
            "美食",
            "外卖",
        ],
    ) {
        push("warm-tones");
        push("friendly");
    }
    if any(
        &lower,
        &[
            "finance",
            "fintech",
            "banking",
            "investing",
            "trading",
            "crypto",
            "stocks",
            "金融",
        ],
    ) {
        push("fintech");
    }
    let wallet_kinds = [
        "crypto", "digital", "payment", "hot", "cold", "hardware", "web3",
    ];
    let wallet_phrase = wallet_kinds
        .iter()
        .any(|k| lower.contains(&format!("{k} wallet")))
        || lower.contains("wallet payment")
        || lower.contains("wallet connect");
    if wallet_phrase {
        push("fintech");
    }
    // TS `APPLE_WALLET_CONTEXT` matches `gift cards?` / `punch cards?` /
    // `stamp cards?` / `vaccination cards?` (singular + plural). Plain
    // `str::contains` bypasses `contains_word`'s ASCII word boundary so the
    // trailing plural `s` still matches — faithful to TS's `cards?`.
    let apple_wallet = any(
        &lower,
        &[
            "pass",
            "passes",
            "boarding",
            "ticket",
            "tickets",
            "ticketing",
            "coupon",
            "coupons",
            "loyalty",
            "membership",
        ],
    ) || lower.contains("gift card")
        || lower.contains("punch card")
        || lower.contains("stamp card")
        || lower.contains("vaccination card");
    if lower.contains("wallet app") && !apple_wallet {
        push("fintech");
    }
    let budget = ["budget", "expense"].iter().any(|b| {
        [
            "tracker",
            "app",
            "report",
            "management",
            "manager",
            "tracking",
        ]
        .iter()
        .any(|s| lower.contains(&format!("{b} {s}")))
    });
    if budget {
        push("fintech");
    }
    if any(
        &lower,
        &[
            "developer",
            "coding",
            "programming",
            "terminal",
            "engineering",
            "开发",
        ],
    ) {
        push("developer");
        push("monospace");
    }
    let code_phrase = [
        "editor",
        "review",
        "repo",
        "repository",
        "completion",
        "snippet",
        "base",
    ]
    .iter()
    .any(|s| lower.contains(&format!("code {s}")));
    let api_phrase = [
        "console",
        "platform",
        "portal",
        "docs",
        "documentation",
        "reference",
        "sdk",
        "gateway",
        "playground",
        "key",
        "keys",
    ]
    .iter()
    .any(|s| lower.contains(&format!("api {s}")));
    let dev_phrase = [
        "tool",
        "tools",
        "portal",
        "experience",
        "environment",
        "console",
        "platform",
    ]
    .iter()
    .any(|s| lower.contains(&format!("dev {s}")));
    if code_phrase || api_phrase || dev_phrase {
        push("developer");
        push("monospace");
    }
    if any(
        &lower,
        &[
            "wellness",
            "fitness",
            "yoga",
            "meditation",
            "mindful",
            "health",
            "healthy",
            "wellbeing",
            "spa",
            "gym",
            "exercise",
            "workout",
            "健康",
        ],
    ) {
        push("wellness");
    }
    // —— accent ——
    if any(
        &lower,
        &[
            "coral",
            "orange",
            "peach",
            "amber",
            "tangerine",
            "珊瑚",
            "橙",
        ],
    ) {
        push("orange-accent");
    }
    if any(&lower, &["blue", "navy", "sapphire", "cobalt", "蓝"]) {
        push("blue-accent");
    }
    if any(&lower, &["green", "emerald", "绿"]) {
        push("sage-green");
    }
    if any(&lower, &["gold", "golden", "金"]) {
        push("gold-accent");
    }
    if any(&lower, &["red", "crimson", "scarlet", "ruby", "红"]) {
        push("red-accent");
    }
    // —— technique ——
    if any(&lower, &["rounded", "圆角"]) {
        push("rounded");
    }
    if any(&lower, &["gradient", "渐变"]) {
        push("gradient");
    }

    if tags.is_empty() {
        vec!["minimal".to_string(), "light-mode".to_string()]
    } else {
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unmatched_prompt_yields_only_tone_tag() {
        // tone 组(dark/light 互斥 if/else)永远 push 一个 tag,故 `tags`
        // 永不为空 —— TS 末尾的 `['minimal','light-mode']` 兜底是死分支。
        // 无关键词命中的 prompt → 只剩 tone tag(light-mode)。
        assert_eq!(infer_tags_from_prompt("xyz123"), vec!["light-mode"]);
    }

    #[test]
    fn tone_and_visual_tags() {
        let t = infer_tags_from_prompt("a dark minimalist dashboard");
        assert!(t.contains(&"dark-mode".to_string()));
        assert!(t.contains(&"minimal".to_string()));
    }

    #[test]
    fn industry_food_pushes_two_tags() {
        let t = infer_tags_from_prompt("a food delivery app");
        assert!(t.contains(&"warm-tones".to_string()));
        assert!(t.contains(&"friendly".to_string()));
    }

    #[test]
    fn developer_pushes_developer_and_monospace() {
        let t = infer_tags_from_prompt("a coding tool");
        assert!(t.contains(&"developer".to_string()));
        assert!(t.contains(&"monospace".to_string()));
    }

    #[test]
    fn no_dedup_source_order() {
        // fintech 可被多组 push;不去重(TS 行为)
        let t = infer_tags_from_prompt("a fintech banking finance app");
        assert!(t.iter().filter(|x| *x == "fintech").count() >= 1);
        // light-mode 总在最前(tone 组最先)
        assert_eq!(t[0], "light-mode");
    }

    #[test]
    fn wallet_app_for_gift_cards_is_apple_wallet_not_fintech() {
        // "gift cards" (plural) is apple-wallet context → must NOT push fintech.
        let t = infer_tags_from_prompt("a wallet app for gift cards");
        assert!(!t.contains(&"fintech".to_string()));
    }
}
