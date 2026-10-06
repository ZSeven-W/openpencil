//! 规划 prompt 的 style-guide 上下文构造 —— port of
//! `orchestrator-prompt-optimizer.ts` 的 catalog 路径。

use crate::design_md_policy::{
    build_design_md_style_policy, guess_neutral_background_from_theme, infer_design_md_background,
};
use crate::design_type::{contains_word, detect_design_type, DesignType};
use crate::model_profile::{resolve_model_profile, ModelTier};
use crate::plan::OrchestratorPlan;
use crate::types::{DesignRequest, PlanningMode};
use jian_ops_schema::DesignMdSpec;
use op_ai_skills::style_guide::{
    extract_style_guide_values, find_style_guide, style_guide_registry, ParsedStyleGuide, Platform,
    StyleGuideRef,
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
        push("food");
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
    // —— social card series ——
    // Purely ADDITIVE (`card-system-0808.md` §8.2 P0-2): a card prompt has to
    // reach the card shelf's own tags, and nothing else in this function is
    // removed or reweighted, so existing routing is untouched. The
    // design-type rung is the authority on WHETHER this is a card request;
    // this only supplies the vocabulary once it is.
    if crate::design_type::is_card_series_prompt(&lower) {
        push("social-card");
        push("card-series");
        push("vertical-portrait");
        // Every shipped card guide is CJK-first typography — the tag is what
        // separates them from a latin social template.
        push("cjk-type");
    }
    // Content words the four shipped guides split on. Kept out of the block
    // above so a non-card prompt that happens to say "手帐" still gets no
    // card tags — the type rung stays the only gate.
    if any(
        &lower,
        &["手帐", "手賬", "笔记", "筆記", "notebook", "journal"],
    ) {
        push("education");
        push("friendly");
    }
    if any(&lower, &["观点", "觀點", "金句", "editorial", "opinion"]) {
        push("editorial");
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
    "food",
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

/// Resolve a user-pinned style-guide id against both halves of the catalogue.
///
/// This is the single short-circuit every planning path shares: a hit means
/// the pin wins outright and no ranking runs, a miss means the pin is stale
/// (a guide renamed or retired between releases, or a `user:` import deleted
/// since it was pinned) and the caller falls back to its own inference. A
/// stale pin is logged rather than surfaced as an error — the user asked for
/// an aesthetic, not for the request to fail — but it is logged, because a pin
/// that silently stops applying is otherwise invisible.
pub(crate) fn resolve_pinned_style_guide(pinned: Option<&str>) -> Option<StyleGuideRef> {
    let name = pinned.map(str::trim).filter(|name| !name.is_empty())?;
    match find_style_guide(name) {
        Some(guide) => Some(guide),
        None => {
            tracing::warn!(
                pinned = %name,
                "pinned style guide is in neither the corpus nor the imported set — \
                 falling back to prompt ranking"
            );
            None
        }
    }
}

/// Force a user's pinned style guide onto a finished plan.
///
/// Shrinking the planning menu to one entry only *asks* the model to echo the
/// pinned name back; it is free not to, and measured on a Full-tier run it
/// did not — the plan came back with no `styleGuideName` at all and the whole
/// generation ran in an unrelated palette. The `forced_style_guide_name`
/// backfill that was supposed to catch this is only populated on the Compact
/// planning path (`prompt.rs`), so on Rich and Minimal it was a no-op, and the
/// fallback plan built after two planning failures carried no guide either.
///
/// A pin is a setting, not a suggestion, so it is applied here rather than
/// negotiated with the model — the same treatment `plan_repair::finalize_plan`
/// already gives design.md.
///
/// Precedence is `design.md > pin > whatever the model chose`. design.md wins
/// because it is a design system the user wrote down and the rest of the
/// pipeline keys off its `design-md-custom` contract; a pin is a choice from a
/// catalog, and there is no catalog in play once design.md is present.
///
/// A pin naming a guide in neither half of the catalogue (an import the user
/// has since deleted) resolves to nothing and is left alone, so a stale pin
/// still degrades to the model's own choice instead of forcing a dead name.
///
/// Returns whether the plan changed.
pub(crate) fn enforce_pinned_style_guide(
    plan: &mut OrchestratorPlan,
    request: &DesignRequest,
) -> bool {
    if request.design_md.is_some() {
        return false;
    }
    let Some(guide) = resolve_pinned_style_guide(request.pinned_style_guide.as_deref()) else {
        return false;
    };
    let id = guide.id();
    if plan.style_guide_name.as_deref() == Some(id) {
        return false;
    }
    tracing::debug!(
        pinned = %id,
        replaced = ?plan.style_guide_name,
        "forcing the pinned style guide onto the plan"
    );
    plan.style_guide_name = Some(id.to_string());
    true
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

/// 一行 guide 元数据 —— softened (user direction 2026-06-23): names only.
/// The catalog is a menu of `styleGuideName` choices; the explicit type-tags
/// and background color are intentionally dropped so the style guide no longer
/// dictates "what type / what color" — the model picks palette from the prompt.
pub(crate) fn format_guide_metadata_line(guide: &ParsedStyleGuide, _mode: PlanningMode) -> String {
    // The planner picks a guide by mood, so it has to see the mood: the
    // first "Key aesthetics" bullet rides along with the name and platform.
    // Name and platform alone made the pick a blind draw (review 2026-09-06).
    // Only the bullet's label ("Electric lime on black"), not its sentence:
    // sixty-odd guides ride in one planning prompt, so each line must stay
    // a handful of tokens.
    let lead = op_ai_skills::style_guide::key_aesthetics(&guide.content, 1)
        .into_iter()
        .next()
        .map(|bullet| bullet.split(':').next().unwrap_or("").trim().to_string())
        .filter(|label| !label.is_empty());
    let mut line = match lead {
        Some(label) => format!("- {} [{}] — {}", guide.name, guide.platform.as_str(), label),
        None => format!("- {} [{}]", guide.name, guide.platform.as_str()),
    };
    let recipes = op_ai_skills::style_guide::signature_recipes(&guide.content, 2);
    if !recipes.is_empty() {
        line.push_str(" · recipes: ");
        line.push_str(&recipes.join(" / "));
    }
    line
}

/// 一份 guide 的详细 snippet —— softened (user direction 2026-06-23): FONT
/// direction only. Colors, type-tags, and corner-radius are intentionally
/// dropped so the catalog suggests typography but leaves palette + shape to the
/// model (the brand-accent-consistency RULE in the design-system / jsonl skills
/// still keeps whatever accent it picks coherent). No-data → heading only.
pub(crate) fn format_guide_snippet(guide: &ParsedStyleGuide) -> String {
    let v = extract_style_guide_values(&guide.content);
    let mut lines: Vec<String> = vec![format!("### {} [{}]", guide.name, guide.platform.as_str())];

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

    lines.join("\n")
}

/// Planning budget for a pinned import's prose, in characters.
///
/// An imported `DESIGN.md` is written for humans, not against the corpus's
/// section grammar, so [`format_guide_snippet`]'s value extractor usually
/// finds nothing in one and the planner would see a bare heading. Handing it
/// the document instead is what makes a pinned import steer planning at all;
/// the cap is what stops a long one from crowding out the rest of the prompt.
/// Sub-agents get the full text later through
/// `prompt_style_skills::build_style_guide_instruction`, so a truncation here
/// costs planning nuance, not the design's actual style.
const USER_GUIDE_PLANNING_CHARS: usize = 2000;

/// A pinned import's snippet: its id, then as much of the document as the
/// planning budget allows.
fn format_user_guide_snippet(label: &str, guide: &ParsedStyleGuide) -> String {
    let body = guide.content.trim();
    let mut out = format!("### {label} [{}]\n", guide.platform.as_str());
    if body.chars().count() > USER_GUIDE_PLANNING_CHARS {
        out.extend(body.chars().take(USER_GUIDE_PLANNING_CHARS));
        out.push_str("\n… (truncated; the full guide is given to the sub-agents)");
    } else {
        out.push_str(body);
    }
    out
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
    pinned: Option<&str>,
) -> PlanningStyleGuideContext {
    // —— design.md 分支:不碰 catalog ——
    //
    // design.md outranks a pinned guide on purpose: it is a design system the
    // user wrote down, and this branch's contract (`styleGuideName:
    // design-md-custom`) is what the rest of the pipeline reads. A pin is a
    // catalog choice, and there is no catalog here to choose from.
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

    // —— pinned 分支:短路排序,恒选钉住的那一份 ——
    //
    // The catalog is offered as a menu the model picks from, so a pin cannot
    // be expressed by re-ordering it — the model would still be free to pick
    // something else. It is expressed by shrinking the menu to one entry.
    if let Some(guide) = resolve_pinned_style_guide(pinned) {
        // The id, not the display name, is the exact string: an imported guide
        // may legitimately name itself after a corpus one, and the directive
        // has to name the guide the user actually pinned.
        let label = guide.id().to_string();
        let snippet = if guide.is_user() {
            format_user_guide_snippet(&label, &guide)
        } else {
            format_guide_snippet(&guide)
        };
        let metadata_line = {
            let metadata = format_guide_metadata_line(&guide, mode);
            if guide.is_user() {
                let display_prefix = format!("- {}", guide.name);
                metadata
                    .strip_prefix(&display_prefix)
                    .map(|suffix| format!("- {label}{suffix}"))
                    .unwrap_or(metadata)
            } else {
                metadata
            }
        };
        let lines: Vec<String> = vec![
            "The user pinned a style guide in the Asset Center. Use it — do NOT \
             pick a different one."
                .to_string(),
            metadata_line,
            String::new(),
            snippet,
            String::new(),
            "Output directives:".to_string(),
            format!("- Set \"styleGuideName\": \"{label}\" (exact string)."),
        ];
        return PlanningStyleGuideContext {
            available_style_guides: lines.join("\n"),
            metadata_count: 1,
            snippet_count: 1,
            top_guide_names: vec![label.clone()],
            snippet_guide_names: vec![label],
        };
    }

    // —— catalog 分支 ——
    let preset = detect_design_type(prompt);
    // Same shelf routing as the compact path — the card guides live behind a
    // hard platform filter, so BOTH planning modes have to ask for them by
    // name or the rich path silently keeps offering webapp guides.
    let platform = match preset.type_ {
        DesignType::MobileScreen => Platform::Mobile,
        DesignType::Card => Platform::Card,
        _ => Platform::Webapp,
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
#[path = "style_guide_context_tests.rs"]
mod tests;
