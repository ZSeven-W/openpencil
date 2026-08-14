//! CLI standard-mode chat routing — three-way intent classification
//! plus the DESIGN_MODIFY / append-to-document flows (GAP #33).
//!
//! Ports the TS standard-mode pipeline that runs for external CLI
//! providers (builtin / ACP turns return early in both stacks):
//!
//! - `apps/web/src/components/panels/ai-chat-intent-classifier.ts`
//!   — `classifyIntent` (LLM call, 8s abort, fallback `new`) and
//!   `classifyByKeywords`, both verbatim.
//! - `apps/web/src/components/panels/ai-chat-handlers.ts:693-776`
//!   — modify-vs-new degrade rules, `generateDesignModification`
//!   target selection, the design/chat dispatch.
//! - `apps/web/src/services/ai/design-generator.ts:95-173`
//!   — `generateDesignModification` (maintenance skills + design-md
//!   policy system prompt, `CONTEXT NODES / INSTRUCTION` user
//!   message, variable context, parse-or-error semantics).
//! - `apps/web/src/services/ai/design-generator.ts:43-72`
//!   — `buildVariableContext`.
//! - `apps/web/src/services/ai/append-intent-detector.ts`
//!   — `detectAppendIntent` (append keywords, new-screen veto,
//!   content-root pick, status-bar filter).
//!
//! Threading: classification needs an LLM round-trip, so the whole
//! route decision runs on a worker thread ([`run_cli_turn`]).
//! `chat_session::launch_if_pending` pre-builds every route's inputs
//! and channels on the UI thread, then parks BOTH a `ChatSession`
//! and a `DesignSession` on the app; the worker uses whichever route
//! the classifier picks and drops the other route's senders so its
//! session pump cleans up.
//!
//! Documented divergences from TS (file:line in the report):
//! - TS replaces the "Checking guidelines" step text with the raw
//!   modification response; Rust deltas append, so the step line
//!   stays above the response.
//! - TS wraps the modification apply in one history batch; Rust
//!   applies per-node through the MCP tool path (same granularity
//!   as the Rust design pipeline until host batch mode lands).
//! - Node parsing replays modify-only `I(parent, {...})` scripts with
//!   `op_mcp::script_runner::run_script_to_program`, preserving authored ids
//!   for recursive diff application, then falls back to
//!   `op_orchestrator::parse::parse_nodes` for legacy flat JSON output.

use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, ChatToolExecutor, StopReason,
};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use op_orchestrator::{AbortFlag, AppendContext, DesignRequest};

use crate::chat_canvas_tools::UiChatToolExecutor;
use crate::chat_provider_llm::ChatProviderLlmClient;
use crate::design_session::{run_design_worker, DesignCmdReq, DesignDelta};

#[path = "chat_intent_screen_sets.rs"]
mod screen_sets;
pub use screen_sets::listed_whole_screen_names;
use screen_sets::requests_listed_whole_screens;

/// Internal host-op name the modify worker sends over the chat tool
/// channel; intercepted by `chat_session::drain_tool_requests` (never
/// advertised to any model).
pub const APPLY_MODIFICATION_OP: &str = "__apply_design_modification";

/// TS `classifyIntent` abort budget (`ai-chat-intent-classifier.ts:26`).
const CLASSIFY_TIMEOUT: Duration = Duration::from_secs(8);

/// TS `DesignIntent = 'new' | 'modify' | 'chat'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignIntent {
    New,
    Modify,
    Chat,
}

const RETRY_PHRASES: &[&str] = &[
    "retry",
    "try again",
    "run it again",
    "please try again",
    "vuelve a intentar",
    "intenta de nuevo",
    "reintenta",
    "otra vez",
];

fn is_retry_phrase(text: &str) -> bool {
    let normalized = text
        .trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    RETRY_PHRASES.contains(&normalized.as_str())
}

fn last_turn_failed(history: &[(ChatHistoryRole, String)]) -> bool {
    history
        .iter()
        .rev()
        .find(|(_, text)| !text.trim().is_empty())
        .is_some_and(|(role, text)| {
            *role == ChatHistoryRole::Assistant
                && text.trim_start().to_ascii_lowercase().starts_with("error:")
        })
}

/// Resolve a short retry utterance to the most recent actionable user request.
/// The transcript remains unchanged; callers use this only as the effective
/// instruction sent to stateless design/modify providers.
pub fn resolve_retry_instruction(user_text: &str, history: &[(ChatHistoryRole, String)]) -> String {
    if !is_retry_phrase(user_text) || !last_turn_failed(history) {
        return user_text.to_string();
    }
    history
        .iter()
        .rev()
        .find_map(|(role, text)| {
            (*role == ChatHistoryRole::User && !text.trim().is_empty() && !is_retry_phrase(text))
                .then(|| text.clone())
        })
        .unwrap_or_else(|| user_text.to_string())
}

/// TS `CLASSIFY_PROMPT` — verbatim.
const CLASSIFY_PROMPT: &str = "You are a UI design tool assistant. Classify the user's message intent.
Reply with EXACTLY one of these tags, nothing else:
- DESIGN_NEW — user wants to create or generate a NEW design, screen, page, or component from scratch
- DESIGN_MODIFY — user wants to modify, adjust, refine, or iterate on an EXISTING design (e.g. change colors, resize, restyle, add/remove elements)
- CHAT — user is asking a question, seeking help, or having a conversation";

/// TS `MODIFY_KEYWORDS` alternatives (the `\b(...)\b/i` regex), plus
/// Rust-shell CJK shorthands used before an LLM route decision.
const MODIFY_KEYWORDS: &[&str] = &[
    "change", "modify", "update", "adjust", "resize", "move", "restyle", "refine", "fix", "tweak",
    "edit", "replace", "remove", "delete", "add to", "smaller", "larger", "bigger", "wider",
    "taller",
];
const MODIFY_CJK: &[&str] = &[
    "修改",
    "改成",
    "改为",
    "改一下",
    "调整",
    "修复",
    "替换",
    "换成",
    "变成",
    "删除",
    "移除",
    "小一点",
    "大一点",
];

/// TS `CHAT_KEYWORDS` alternatives.
const CHAT_KEYWORDS: &[&str] = &[
    "what is", "how do", "explain", "tell me", "help", "why", "can you", "question", "describe",
];
const CHAT_CJK: &[&str] = &["是什么", "什么", "怎么", "为什么", "解释", "说明", "帮助"];

fn is_word_char(c: char) -> bool {
    // JS `\w` — ASCII alphanumeric plus underscore.
    c.is_ascii_alphanumeric() || c == '_'
}

/// `\b<phrase>\b` case-insensitive matcher over a (possibly
/// multi-word) literal phrase. Whitespace runs in the haystack are
/// treated as single separators so `\s+`-joined phrases still match.
fn matches_word_phrase(text_lower: &str, phrase: &str) -> bool {
    let haystack: String = text_lower.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(phrase) {
        let abs = start + pos;
        let before_ok = haystack[..abs]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word_char(c));
        let after_ok = haystack[abs + phrase.len()..]
            .chars()
            .next()
            .is_none_or(|c| !is_word_char(c));
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

fn matches_any_word_phrase(text_lower: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| matches_word_phrase(text_lower, p))
}

/// True when the message has no word or number content to route.
/// Punctuation-only reactions must stay in chat instead of opening a design
/// turn that can mutate the canvas.
pub fn is_non_request_text(text: &str) -> bool {
    !text.chars().any(char::is_alphanumeric)
}

/// TS `classifyByKeywords` — verbatim rule order.
pub fn classify_by_keywords(text: &str) -> DesignIntent {
    let lower = text.to_lowercase();
    let chat =
        matches_any_word_phrase(&lower, CHAT_KEYWORDS) || CHAT_CJK.iter().any(|k| text.contains(k));
    let modify = matches_any_word_phrase(&lower, MODIFY_KEYWORDS)
        || MODIFY_CJK.iter().any(|k| text.contains(k));
    if chat && !modify {
        return DesignIntent::Chat;
    }
    if modify {
        return DesignIntent::Modify;
    }
    DesignIntent::New
}

pub fn looks_like_modify_request(text: &str) -> bool {
    classify_by_keywords(text) == DesignIntent::Modify
}

/// Standard-mode routing should not let a lightweight classifier
/// overrule explicit edit wording. Providers like Codex / Claude Code
/// can be conservative and answer DESIGN_NEW for terse CJK edit
/// prompts; keep those on the modify path before asking the model.
pub fn classify_intent_for_standard_route(
    provider: &dyn ChatProvider,
    state: &EditorState,
    text: &str,
    model: Option<String>,
) -> DesignIntent {
    if is_non_request_text(text) {
        return DesignIntent::Chat;
    }
    // A whole-screen *draw* (creation verb + page noun, e.g. "重新画一个
    // search 页面") is unambiguously a new screen — it must win over the
    // modify classifier so it routes to the new-frame path, not edit-in-place.
    // It already excludes existing-screen context ("把发现页改成深色" has no
    // creation verb), so genuine edits still fall through to Modify below.
    if requests_new_whole_screen(text) {
        return DesignIntent::New;
    }
    let keyword_intent = classify_by_keywords(text);
    if !state.selection.set.is_empty() && keyword_intent == DesignIntent::Chat {
        return DesignIntent::Chat;
    }
    // A selection biases toward modify — UNLESS the prompt is itself a
    // whole-screen design request (a stray selection must not hijack "Design
    // a … page" into an edit; `requests_new_whole_screen` misses these when
    // they mention "section", so check the veto-free creation signal too).
    let selected_target_instruction = !state.selection.set.is_empty()
        && keyword_intent != DesignIntent::Chat
        && !has_new_screen_creation_signal(text);
    if keyword_intent == DesignIntent::Modify || selected_target_instruction {
        return DesignIntent::Modify;
    }
    if is_named_follow_on_screen(text) {
        return DesignIntent::New;
    }
    classify_intent_llm(provider, text, model)
}

/// TS classification-tag parsing (`ai-chat-intent-classifier.ts:46-51`).
pub fn parse_classified(text: &str) -> DesignIntent {
    let upper = text.trim().to_uppercase();
    if upper.contains("DESIGN_MODIFY") {
        return DesignIntent::Modify;
    }
    if upper.contains("DESIGN_NEW") || upper.contains("DESIGN") {
        return DesignIntent::New;
    }
    if upper.contains("CHAT") {
        return DesignIntent::Chat;
    }
    DesignIntent::Chat
}

/// TS `classifyIntent` — one lightweight LLM call through the (chat-
/// session-untracked) provider, with the TS 8s abort and the TS
/// conservative fallback to chat on any failure / timeout.
pub fn classify_intent_llm(
    provider: &dyn ChatProvider,
    text: &str,
    model: Option<String>,
) -> DesignIntent {
    classify_intent_llm_with_timeout(provider, text, model, CLASSIFY_TIMEOUT)
}

fn classify_intent_llm_with_timeout(
    provider: &dyn ChatProvider,
    text: &str,
    model: Option<String>,
    timeout: Duration,
) -> DesignIntent {
    let req = ChatRequest {
        system_prompt: CLASSIFY_PROMPT.to_string(),
        user_message: text.to_string(),
        max_output_tokens: 4096,
        model,
        ..Default::default()
    };
    // `provider.send` returns a blocking iterator. We are already on
    // the router worker thread, but the 8s budget needs a timed recv,
    // so the drain rides one more (detached) thread; dropping `rx`
    // after a timeout makes its sends fail and the drain unwind.
    let iter = provider.send(req);
    let (tx, rx) = mpsc::channel::<ChatDelta>();
    let spawned = std::thread::Builder::new()
        .name("op-chat-classify".into())
        .spawn(move || {
            for delta in iter {
                if tx.send(delta).is_err() {
                    return;
                }
            }
        });
    if let Err(err) = spawned {
        eprintln!("[chat-intent] failed to spawn classify drain thread: {err}");
        return DesignIntent::Chat;
    }

    let deadline = Instant::now() + timeout;
    let mut out = String::new();
    loop {
        let now = Instant::now();
        if now >= deadline {
            return parse_classified(&out);
        }
        match rx.recv_timeout(deadline - now) {
            Ok(ChatDelta::TextDelta(s)) => out.push_str(&s),
            // TS consumeSSEAsText only accumulates text chunks.
            Ok(ChatDelta::Thinking(_)) | Ok(ChatDelta::ToolUse { .. }) => {}
            Ok(ChatDelta::Error(_)) => return parse_classified(&out),
            Ok(ChatDelta::Done { .. }) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => return parse_classified(&out),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    parse_classified(&out)
}

// ---------------------------------------------------------------------------
// Append-intent detection — port of append-intent-detector.ts
// ---------------------------------------------------------------------------

const APPEND_EN: &[&str] = &[
    "continue",
    "continuing",
    "append",
    "also add",
    // `add (?:another|more|a new)\s+section` expanded.
    "add another section",
    "add more section",
    "add a new section",
    "one more",
    "next section",
    "add to the",
];
const APPEND_CJK: &[&str] = &[
    "继续",
    "接着",
    "再加",
    "再加一个",
    "再添加",
    "再生成",
    "再来一个",
    "补充",
    "追加",
];
const NEW_SCREEN_EN: &[&str] = &[
    "new page",
    "new screen",
    "new design",
    "new mockup",
    "another page",
    "another screen",
    "another design",
    "another mockup",
    "from scratch",
    "brand new",
];
const NEW_SCREEN_CJK: &[&str] = &[
    "新页面",
    "新屏",
    "新设计",
    "从零",
    "全新",
    "另起",
    "另外一页",
];
const NAMED_SCREEN_EN: &[&str] = &[
    "discover page",
    "discover screen",
    "search page",
    "search screen",
    "orders page",
    "orders screen",
    "order page",
    "order screen",
    "profile page",
    "profile screen",
    "account page",
    "account screen",
    "favorites page",
    "favorites screen",
    "saved page",
    "saved screen",
    "detail page",
    "detail screen",
    "details page",
    "details screen",
    "cart page",
    "cart screen",
    "checkout page",
    "checkout screen",
    "category page",
    "category screen",
    "menu page",
    "menu screen",
];
const NAMED_SCREEN_CJK: &[&str] = &[
    "发现页",
    "发现页面",
    "搜索页",
    "搜索页面",
    "订单页",
    "订单页面",
    "我的页",
    "我的页面",
    "个人页",
    "个人页面",
    "账户页",
    "账户页面",
    "收藏页",
    "收藏页面",
    "详情页",
    "详情页面",
    "购物车页",
    "购物车页面",
    "结算页",
    "结算页面",
    "分类页",
    "分类页面",
    "菜单页",
    "菜单页面",
];

/// TS `加一个.{0,6}(区块|栏|模块|section|段)` — "加一个" followed by one
/// of the suffixes within 0-6 characters.
fn matches_cjk_add_section(text: &str) -> bool {
    const SUFFIXES: &[&str] = &["区块", "栏", "模块", "section", "段"];
    let mut search = text;
    while let Some(pos) = search.find("加一个") {
        let after = &search[pos + "加一个".len()..];
        // A suffix may start at char offsets 0..=6 after the head.
        for (byte_idx, _) in after.char_indices().take(7) {
            if SUFFIXES.iter().any(|s| after[byte_idx..].starts_with(s)) {
                return true;
            }
        }
        search = after;
    }
    false
}

pub fn is_named_follow_on_screen(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    matches_any_word_phrase(&lower, NAMED_SCREEN_EN)
        || NAMED_SCREEN_CJK.iter().any(|k| prompt.contains(k))
}

/// True when the appended unit is a *section* of the current screen rather
/// than a whole new screen — "继续加一个区块", "add another section". These
/// keep appending into the existing frame.
fn is_section_add_request(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    matches_cjk_add_section(prompt)
        || ["区块", "模块", "栏目"].iter().any(|k| prompt.contains(k))
        || lower.contains("section")
}

/// CJK verbs that mean "create a whole thing" (vs. editing an existing one).
/// Used to confirm a page/screen noun is the unit being *made*, not just
/// referenced — "画一个登录页" makes a page; "给页面加个按钮" edits one.
/// Deliberately excludes 加 / 整 — they appear inside 主页 / 整个 / 增加 and
/// would mis-fire on "给主页加个卡片" (add a card to the home page).
const DRAW_VERB_CJK: &[&str] = &["画", "绘", "做", "搞", "生成", "设计", "来"];
/// English creation verbs — the page/screen noun must be the thing being
/// *made*. Deliberately excludes the ambiguous "make" / "add" (which read as
/// edits in "make the page blue" / "add a button to the page"), so an English
/// EDIT that merely mentions a page/screen ("change the home page layout",
/// "resize the screen header") is NOT mistaken for a new-screen request.
const DRAW_VERB_EN: &[&str] = &[
    "draw",
    "create",
    "design",
    "generate",
    "build",
    "wireframe",
    "mock up",
    "mockup",
    "sketch",
    "prototype",
    "lay out",
];
/// Markers that the request points at the CURRENT screen, so it is an edit of
/// the existing frame, not a new one.
const EXISTING_SCREEN_CTX_CJK: &[&str] = &["这个", "这一", "当前", "现有", "此页", "这页", "这屏"];

/// True when the unit being drawn is a whole *page / screen* — "继续画一下
/// search 页面", "再来一个登录页", "continue, add a settings screen". The named
/// list (`is_named_follow_on_screen`) only covers a fixed vocabulary in a
/// single language, so it misses generic or mixed-language phrasings like
/// "search 页面" (English noun + Chinese 页面). The user's rule is broader:
/// drawing a whole page/screen always becomes a NEW top-level frame placed to
/// the right, never rows appended into the current screen.
///
/// Guards against false positives: a section-add ("再加一个区块") keeps
/// appending, and an edit pointed at the current screen ("给这个页面加个按钮",
/// "重新画一下这个页面", "change the home page layout") is not a new screen.
/// CJK requires a creation verb on a page noun; English keys on the determiner
/// nearest the page noun (see [`english_requests_new_screen`]).
pub fn requests_new_whole_screen(prompt: &str) -> bool {
    // A continuation can name several sibling screens with one shared
    // whole-screen noun ("完成 explore/profile界面"). This is stronger than the generic
    // section/current-screen heuristics below, but deliberately requires an
    // explicit target list so "继续完成这个界面" remains an in-place edit.
    if requests_listed_whole_screens(prompt) {
        return true;
    }
    if is_section_add_request(prompt) {
        return false;
    }
    // An edit pointed at the CURRENT CJK screen ("重新画一下这个页面") — the
    // creation verb is present but 这个/当前 marks it as the existing one.
    if EXISTING_SCREEN_CTX_CJK.iter().any(|k| prompt.contains(k)) {
        return false;
    }
    let cjk_page = ["页面", "页", "屏幕", "屏"]
        .iter()
        .any(|k| prompt.contains(k));
    if cjk_page && DRAW_VERB_CJK.iter().any(|v| prompt.contains(v)) {
        return true;
    }
    english_requests_new_screen(&prompt.to_lowercase())
}

/// Core "this is a NEW screen/design" signal: a creation verb aimed at a
/// page/screen noun. Unlike [`requests_new_whole_screen`] it does NOT veto on
/// section-add phrasing — a full new-page spec that merely mentions the word
/// "section" ("Include a search section…", "Deals of the Week section") still
/// reads as new. Used to stop a stray canvas selection from hijacking a
/// whole-screen design prompt into the modify path (measured: a travel-app
/// design with a node selected routed to modify → M3 flat-JSONL → empty).
pub fn has_new_screen_creation_signal(prompt: &str) -> bool {
    if requests_listed_whole_screens(prompt) {
        return true;
    }
    let cjk_page = ["页面", "页", "屏幕", "屏"]
        .iter()
        .any(|k| prompt.contains(k));
    if cjk_page && DRAW_VERB_CJK.iter().any(|v| prompt.contains(v)) {
        return true;
    }
    english_requests_new_screen(&prompt.to_lowercase())
}

/// English new-screen detection by the DETERMINER nearest a page/screen noun:
/// an indefinite article ("a / an / another / new") means a NEW page is being
/// made ("make a login page", "draw a checkout page"); a definite/demonstrative
/// ("the / this / that / current / existing") means an EXISTING page is being
/// referenced — an edit or a location ("change the home page", "create a button
/// on the login page" — here the page is where the button goes). Keyword
/// presence alone is too blunt: it both misses "make a login page" (a creation
/// verb the simple list excludes) and misroutes "create a button on the login
/// page" (verb + page noun, but the page is a location, not the object).
fn english_requests_new_screen(lower: &str) -> bool {
    const NOUNS: &[&str] = &["page", "screen"];
    const DEFINITE: &[&str] = &[
        "the", "this", "that", "these", "those", "current", "existing", "same",
    ];
    const INDEFINITE: &[&str] = &["a", "an", "another", "new", "fresh"];
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let mut saw_noun = false;
    for (i, tok) in tokens.iter().enumerate() {
        if !NOUNS.contains(tok) {
            continue;
        }
        saw_noun = true;
        // The NEAREST determiner within 4 tokens before the noun decides
        // (adjectives like "login" / "checkout" sit between it and the noun).
        for j in (i.saturating_sub(4)..i).rev() {
            if DEFINITE.contains(&tokens[j]) {
                return false;
            }
            if INDEFINITE.contains(&tokens[j]) {
                return true;
            }
        }
    }
    // No determiner cue near a page noun: a strong creation verb still signals
    // a new screen ("draw search page"); otherwise it's not a whole-screen req.
    saw_noun && matches_any_word_phrase(lower, DRAW_VERB_EN)
}

fn is_new_screen_veto(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    matches_any_word_phrase(&lower, NEW_SCREEN_EN)
        || NEW_SCREEN_CJK.iter().any(|k| prompt.contains(k))
        || is_named_follow_on_screen(prompt)
        || requests_new_whole_screen(prompt)
}

/// TS `STATUS_BAR_RE = /(status[\s_-]*bar|system[\s_-]*chrome|状态栏|系统栏)/i`.
fn is_status_bar_like_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    if lower.contains("状态栏") || lower.contains("系统栏") {
        return true;
    }
    separated_pair(&lower, "status", "bar") || separated_pair(&lower, "system", "chrome")
}

/// `<head>[\s_-]*<tail>` scanner.
fn separated_pair(lower: &str, head: &str, tail: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = lower[start..].find(head) {
        let abs = start + pos + head.len();
        let rest = lower[abs..].trim_start_matches([' ', '\t', '\n', '\r', '_', '-']);
        if rest.starts_with(tail) {
            return true;
        }
        start = start + pos + 1;
    }
    false
}

mod context;
mod turns;

pub use context::*;
pub use turns::*;

#[cfg(test)]
#[path = "chat_intent_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chat_intent_selection_tests.rs"]
mod selection_tests;
