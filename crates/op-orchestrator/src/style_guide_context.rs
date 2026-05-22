//! 规划 prompt 的 style-guide 上下文构造 —— port of
//! `orchestrator-prompt-optimizer.ts` 的 catalog 路径。

use crate::design_md_policy::{
    build_design_md_style_policy, guess_neutral_background_from_theme, infer_design_md_background,
};
use crate::design_type::{contains_word, detect_design_type, DesignType};
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::types::PlanningMode;
use jian_ops_schema::DesignMdSpec;
use op_ai_skills::style_guide::{
    extract_style_guide_values, style_guide_registry, ParsedStyleGuide, Platform,
};

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

/// industry tag —— 命中得 +30(其余 tag +10)。
const INDUSTRY_TAGS: &[&str] = &[
    "warm-tones",
    "wellness",
    "fintech",
    "developer",
    "monospace",
];

/// 单个 guide 对 `(tags, platform)` 的加权分 —— port of
/// `styleGuidePromptScore`。industry tag +30 / 其余命中 +10 /
/// platform 不符 -30。
pub(crate) fn style_guide_prompt_score(
    guide: &ParsedStyleGuide,
    tags: &[String],
    platform: Platform,
) -> i32 {
    let mut score = 0;
    for t in tags {
        if guide.tags.iter().any(|gt| gt == t) {
            score += if INDUSTRY_TAGS.contains(&t.as_str()) {
                30
            } else {
                10
            };
        }
    }
    if guide.platform != platform {
        score -= 30;
    }
    score
}

/// 对全 catalog 按加权分降序排名(不过滤)—— port of
/// `rankStyleGuidesForPrompt`。平局:platform-match 优先,再 name 升序。
pub(crate) fn rank_style_guides_for_prompt(
    tags: &[String],
    platform: Platform,
) -> Vec<&'static ParsedStyleGuide> {
    let mut scored: Vec<(&ParsedStyleGuide, i32)> = style_guide_registry()
        .iter()
        .map(|g| (g, style_guide_prompt_score(g, tags, platform)))
        .collect();
    scored.sort_by(|(a, sa), (b, sb)| {
        sb.cmp(sa)
            .then_with(|| (b.platform == platform).cmp(&(a.platform == platform)))
            .then_with(|| a.name.cmp(&b.name))
    });
    scored.into_iter().map(|(g, _)| g).collect()
}

const STYLE_GUIDE_METADATA_TAG_LIMIT: usize = 4;
const STYLE_GUIDE_SNIPPET_TAG_LIMIT: usize = 6;

/// 一行 guide 元数据 —— port of `formatGuideMetadataLine`。
/// `- {name} [{platform}]{bg} :: {tags}`。Rich 取 4 tag,Minimal 取 3。
pub(crate) fn format_guide_metadata_line(guide: &ParsedStyleGuide, mode: PlanningMode) -> String {
    let values = extract_style_guide_values(&guide.content);
    let bg = values
        .colors
        .background
        .as_ref()
        .map(|b| format!(" bg:{b}"))
        .unwrap_or_default();
    let tag_limit = match mode {
        PlanningMode::Rich => STYLE_GUIDE_METADATA_TAG_LIMIT,
        // Compact mode never reaches this formatter (compact planning prompts
        // don't use the style-guide catalog); handled for exhaustiveness.
        PlanningMode::Minimal | PlanningMode::Compact => 3,
    };
    let tags = guide
        .tags
        .iter()
        .take(tag_limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "- {} [{}]{} :: {}",
        guide.name,
        guide.platform.as_str(),
        bg,
        tags
    )
}

/// 一份 guide 的详细 snippet —— port of `formatGuideSnippet`。
/// 多行块,无数据的行丢弃。
pub(crate) fn format_guide_snippet(guide: &ParsedStyleGuide) -> String {
    let v = extract_style_guide_values(&guide.content);
    let mut lines: Vec<String> = vec![format!("### {} [{}]", guide.name, guide.platform.as_str())];

    let tags = guide
        .tags
        .iter()
        .take(STYLE_GUIDE_SNIPPET_TAG_LIMIT)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if !tags.is_empty() {
        lines.push(format!("tags: {tags}"));
    }

    let mut color_parts: Vec<String> = Vec::new();
    if let Some(b) = &v.colors.background {
        color_parts.push(format!("bg={b}"));
    }
    if let Some(s) = &v.colors.surface {
        color_parts.push(format!("surface={s}"));
    }
    if let Some(a) = &v.colors.accent {
        color_parts.push(format!("accent={a}"));
    }
    if !color_parts.is_empty() {
        lines.push(format!("colors: {}", color_parts.join(", ")));
    }

    // TS formatGuideSnippet's fonts line is display+body only — data_font intentionally not emitted.
    let mut font_parts: Vec<String> = Vec::new();
    if let Some(d) = &v.typography.display_font {
        font_parts.push(format!("display={d}"));
    }
    if let Some(b) = &v.typography.body_font {
        font_parts.push(format!("body={b}"));
    }
    if !font_parts.is_empty() {
        lines.push(format!("fonts: {}", font_parts.join(", ")));
    }

    let mut radius_parts: Vec<String> = Vec::new();
    if let Some(c) = v.radius.card {
        radius_parts.push(format!("card={c}"));
    }
    if let Some(b) = v.radius.button {
        radius_parts.push(format!("button={b}"));
    }
    if !radius_parts.is_empty() {
        lines.push(format!("radius: {}", radius_parts.join(", ")));
    }

    lines.join("\n")
}

/// `build_planning_style_guide_context` 的产物。`available_style_guides`
/// 是 1a 唯一消费项;计数/名字给诊断(1b 可用)。crate 内部结构。
#[derive(Debug, Clone)]
pub(crate) struct PlanningStyleGuideContext {
    pub available_style_guides: String,
    /// 诊断字段 —— tests + S3b-1b 消费;production 暂未读取。
    #[allow(dead_code)]
    pub metadata_count: usize,
    /// 诊断字段 —— tests + S3b-1b 消费;production 暂未读取。
    #[allow(dead_code)]
    pub snippet_count: usize,
    /// 诊断字段 —— tests + S3b-1b 消费;production 暂未读取。
    #[allow(dead_code)]
    pub top_guide_names: Vec<String>,
    /// 诊断字段 —— tests + S3b-1b 消费;production 暂未读取。
    #[allow(dead_code)]
    pub snippet_guide_names: Vec<String>,
}

/// Rich 模式各 tier 的 snippet 上限。
fn snippet_limit(tier: ModelTier) -> usize {
    match tier {
        ModelTier::Full => 8,
        ModelTier::Standard => 6,
        ModelTier::Basic => 4,
    }
}

/// 构造规划 prompt 的 `{{availableStyleGuides}}` 上下文 —— port of
/// `buildPlanningStyleGuideContext`。design.md 在场时走早返回分支。
pub(crate) fn build_planning_style_guide_context(
    prompt: &str,
    model: Option<&str>,
    mode: PlanningMode,
    design_md: Option<&DesignMdSpec>,
) -> PlanningStyleGuideContext {
    // —— design.md 分支:不碰 catalog ——
    if let Some(spec) = design_md {
        let policy = build_design_md_style_policy(spec);
        let bg_hint = infer_design_md_background(spec);
        // When design.md has no palette entry explicitly marked as background/
        // surface/canvas, do NOT ask the model to "pick" from the palette — it
        // will happily pick a brand/CTA color and paint the whole page that
        // color. Give it a neutral default instead, biased by visualTheme
        // keywords.
        let neutral_default = guess_neutral_background_from_theme(spec.visual_theme.as_deref());
        let root_fill_directive = match &bg_hint {
            Some(hint) => format!(
                "- Set rootFrame.fill color to \"{hint}\" (the primary background \
                 color from the design.md palette)."
            ),
            None => format!(
                "- Set rootFrame.fill color to \"{neutral_default}\" (neutral page \
                 background — design.md has no palette entry tagged as background, \
                 so DO NOT pick a brand/CTA/accent/text color from the palette for \
                 the page background)."
            ),
        };
        let lines: Vec<String> = vec![
            "The user has a custom design system (design.md). DO NOT pick a style \
             guide from a catalog."
                .to_string(),
            "Use the rules below for all style decisions:".to_string(),
            String::new(),
            if policy.is_empty() {
                "(design.md is present but has no extractable policy; use project defaults)"
                    .to_string()
            } else {
                policy
            },
            String::new(),
            "Output directives:".to_string(),
            "- Set \"styleGuideName\": \"design-md-custom\" (exact string).".to_string(),
            root_fill_directive,
        ];
        return PlanningStyleGuideContext {
            available_style_guides: lines.join("\n"),
            metadata_count: 0,
            snippet_count: 0,
            top_guide_names: vec!["design-md-custom".to_string()],
            snippet_guide_names: Vec::new(),
        };
    }

    // —— catalog 分支 ——
    let preset = detect_design_type(prompt);
    let platform = if preset.type_ == DesignType::MobileScreen {
        Platform::Mobile
    } else {
        Platform::Webapp
    };
    let tags = infer_tags_from_prompt(prompt);
    let tier = resolve_model_profile(model.unwrap_or("")).tier;
    let ranked = rank_style_guides_for_prompt(&tags, platform);

    let metadata_lines: Vec<String> = ranked
        .iter()
        .map(|g| format_guide_metadata_line(g, mode))
        .collect();
    let limit = if mode == PlanningMode::Rich {
        snippet_limit(tier)
    } else {
        0
    };
    let snippet_guides: Vec<&ParsedStyleGuide> = ranked.iter().take(limit).copied().collect();

    let mut parts: Vec<String> = vec![
        "Available style guides (compact catalog; all candidates are listed below):".to_string(),
    ];
    parts.extend(metadata_lines.iter().cloned());
    if !snippet_guides.is_empty() {
        parts.push(String::new());
        parts.push(
            "Detailed references for the best-matching candidates (prefer these before \
             inventing a styleGuideName):"
                .to_string(),
        );
        for g in &snippet_guides {
            parts.push(format_guide_snippet(g));
        }
    }

    PlanningStyleGuideContext {
        available_style_guides: parts.join("\n"),
        metadata_count: metadata_lines.len(),
        snippet_count: snippet_guides.len(),
        top_guide_names: ranked.iter().take(12).map(|g| g.name.clone()).collect(),
        snippet_guide_names: snippet_guides.iter().map(|g| g.name.clone()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_context_lists_all_guides() {
        let ctx = build_planning_style_guide_context(
            "a fintech dashboard",
            Some("claude-opus"),
            PlanningMode::Rich,
            None,
        );
        assert_eq!(ctx.metadata_count, style_guide_registry().len());
        assert!(ctx.snippet_count > 0); // Rich + full tier → 有 snippet
        assert!(ctx
            .available_style_guides
            .contains("Available style guides"));
    }

    #[test]
    fn minimal_mode_has_no_snippets() {
        let ctx = build_planning_style_guide_context(
            "a fintech dashboard",
            Some("claude-opus"),
            PlanningMode::Minimal,
            None,
        );
        assert_eq!(ctx.snippet_count, 0);
    }

    #[test]
    fn design_md_branch_skips_catalog() {
        let spec = jian_ops_schema::DesignMdSpec {
            raw: String::new(),
            project_name: None,
            visual_theme: Some("calm".into()),
            color_palette: None,
            typography: None,
            component_styles: None,
            layout_principles: None,
            generation_notes: None,
        };
        let ctx = build_planning_style_guide_context(
            "a page",
            Some("claude-opus"),
            PlanningMode::Rich,
            Some(&spec),
        );
        assert_eq!(ctx.metadata_count, 0);
        assert_eq!(ctx.top_guide_names, vec!["design-md-custom".to_string()]);
        assert!(ctx.available_style_guides.contains("custom design system"));
    }

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

    #[test]
    fn rank_ranks_full_registry_no_filter() {
        let ranked = rank_style_guides_for_prompt(&["fintech".to_string()], Platform::Webapp);
        // 排名不过滤 —— 全 catalog 都在
        assert_eq!(ranked.len(), style_guide_registry().len());
    }

    #[test]
    fn rank_industry_tag_outweighs_plain_tag() {
        // fintech(industry,+30)的 guide 应排在只命中普通 tag(+10)的前面
        let ranked = rank_style_guides_for_prompt(
            &["fintech".to_string(), "minimal".to_string()],
            Platform::Webapp,
        );
        assert!(!ranked.is_empty());
        // 首个的分数 >= 其后任意(降序)
        let s0 = style_guide_prompt_score(
            ranked[0],
            &["fintech".into(), "minimal".into()],
            Platform::Webapp,
        );
        let s1 = style_guide_prompt_score(
            ranked[ranked.len() - 1],
            &["fintech".into(), "minimal".into()],
            Platform::Webapp,
        );
        assert!(s0 >= s1);
    }

    #[test]
    fn metadata_line_shape() {
        let g = &style_guide_registry()[0];
        let line = format_guide_metadata_line(g, PlanningMode::Rich);
        // `- {name} [{platform}]...  :: ...`
        assert!(line.starts_with(&format!("- {} [", g.name)));
        assert!(line.contains(" :: "));
    }

    #[test]
    fn metadata_line_emits_bg_segment() {
        // Find a registry guide whose content yields an extractable background;
        // its metadata line must carry the ` bg:` segment.
        let with_bg = style_guide_registry()
            .iter()
            .find(|g| {
                extract_style_guide_values(&g.content)
                    .colors
                    .background
                    .is_some()
            })
            .expect("at least one style guide should expose a background color");
        let line = format_guide_metadata_line(with_bg, PlanningMode::Rich);
        assert!(
            line.contains(" bg:"),
            "metadata line missing bg segment: {line}"
        );
    }

    #[test]
    fn snippet_has_heading_and_tags() {
        let g = &style_guide_registry()[0];
        let snip = format_guide_snippet(g);
        assert!(snip.starts_with(&format!("### {} [", g.name)));
        assert!(snip.contains("tags:"));
    }
}
