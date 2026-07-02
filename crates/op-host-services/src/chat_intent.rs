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
//! - Node parsing reuses `op_orchestrator::parse::parse_nodes`
//!   (a superset of TS `extractJsonFromResponse` for messy LLM
//!   output, but typed: a response of partial patch objects without
//!   a `type` field fails to parse where untyped TS would accept it).

use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, ChatToolExecutor, StopReason};
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;
use op_orchestrator::{AppendContext, DesignRequest};

use crate::chat_canvas_tools::UiChatToolExecutor;
use crate::chat_provider_llm::ChatProviderLlmClient;
use crate::design_session::{run_design_worker, DesignCmdReq, DesignDelta};

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

/// TS `classifyByKeywords` — verbatim rule order.
pub fn classify_by_keywords(text: &str) -> DesignIntent {
    let lower = text.to_lowercase();
    let chat = matches_any_word_phrase(&lower, CHAT_KEYWORDS);
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
    text: &str,
    model: Option<String>,
) -> DesignIntent {
    // A whole-screen *draw* (creation verb + page noun, e.g. "重新画一个
    // search 页面") is unambiguously a new screen — it must win over the
    // modify classifier so it routes to the new-frame path, not edit-in-place.
    // It already excludes existing-screen context ("把发现页改成深色" has no
    // creation verb), so genuine edits still fall through to Modify below.
    if requests_new_whole_screen(text) {
        return DesignIntent::New;
    }
    if looks_like_modify_request(text) {
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
    DesignIntent::New
}

/// TS `classifyIntent` — one lightweight LLM call through the (chat-
/// session-untracked) provider, with the TS 8s abort and the TS
/// fallback to `new` on any failure / timeout.
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
    std::thread::Builder::new()
        .name("op-chat-classify".into())
        .spawn(move || {
            for delta in iter {
                if tx.send(delta).is_err() {
                    return;
                }
            }
        })
        .expect("spawn op-chat-classify thread");

    let deadline = Instant::now() + timeout;
    let mut out = String::new();
    loop {
        let now = Instant::now();
        if now >= deadline {
            // TS: AbortController fires → catch → { intent: 'new' }.
            return DesignIntent::New;
        }
        match rx.recv_timeout(deadline - now) {
            Ok(ChatDelta::TextDelta(s)) => out.push_str(&s),
            // TS consumeSSEAsText only accumulates text chunks.
            Ok(ChatDelta::Thinking(_)) | Ok(ChatDelta::ToolUse { .. }) => {}
            // TS: `if (!response.ok) throw` → catch → 'new'.
            Ok(ChatDelta::Error(_)) => return DesignIntent::New,
            Ok(ChatDelta::Done { .. }) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => return DesignIntent::New,
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
    if is_section_add_request(prompt) {
        return false;
    }
    // An edit pointed at the CURRENT CJK screen ("重新画一下这个页面") — the
    // creation verb is present but 这个/当前 marks it as the existing one.
    if EXISTING_SCREEN_CTX_CJK.iter().any(|k| prompt.contains(k)) {
        return false;
    }
    let cjk_page = ["页面", "页", "屏幕", "屏"].iter().any(|k| prompt.contains(k));
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

type PenNode = jian_ops_schema::node::PenNode;

fn is_frame(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_))
}

fn is_status_bar_like(node: &PenNode) -> bool {
    let name = node.base().name.as_deref().unwrap_or("");
    is_status_bar_like_text(&format!("{name} {}", node.id_str()))
}

fn node_label(node: &PenNode) -> String {
    node.base()
        .name
        .clone()
        .unwrap_or_else(|| node.id_str().to_string())
}

/// TS `pickContentRoot` — prefer a child frame named like a content
/// root, else the page frame itself.
fn pick_content_root(page: &PenNode) -> (&PenNode, Vec<String>) {
    let children: &[PenNode] = page.children().map(Vec::as_slice).unwrap_or(&[]);
    let content_frames: Vec<&PenNode> = children
        .iter()
        .filter(|n| is_frame(n) && !is_status_bar_like(n))
        .collect();

    const CONTENT_NAME: &[&str] = &["content", "main", "body", "root"];
    let candidate = content_frames.iter().find(|f| {
        let name = f.base().name.as_deref().unwrap_or("").to_lowercase();
        matches_any_word_phrase(&name, CONTENT_NAME)
    });
    if let Some(candidate) = candidate {
        let grand = candidate.children();
        let labels = grand
            .map(|kids| {
                kids.iter()
                    .filter(|n| is_frame(n) && !is_status_bar_like(n))
                    .map(node_label)
                    .collect()
            })
            .unwrap_or_default();
        return (candidate, labels);
    }

    (page, content_frames.iter().map(|n| node_label(n)).collect())
}

/// TS `detectAppendIntent` against the live editor state. The active
/// page frame is the first `Frame` among the active page's children
/// (TS `pickActivePageFrame` fallback branch — the Rust shell's
/// active page is always a real page entry, never a frame-as-page
/// alias).
pub fn detect_append_intent(state: &EditorState, prompt: &str) -> Option<AppendContext> {
    if prompt.trim().is_empty() {
        return None;
    }
    let lower = prompt.to_lowercase();
    let has_append = matches_any_word_phrase(&lower, APPEND_EN)
        || APPEND_CJK.iter().any(|k| prompt.contains(k))
        || matches_cjk_add_section(prompt);
    if !has_append {
        return None;
    }
    if is_new_screen_veto(prompt) {
        return None;
    }

    let page_frame = state.active_children().iter().find(|n| is_frame(n))?;
    let page_has_content = page_frame
        .children()
        .is_some_and(|kids| kids.iter().any(|c| is_frame(c) && !is_status_bar_like(c)));
    if !page_has_content {
        return None;
    }

    let (target, section_labels) = pick_content_root(page_frame);
    let width = page_frame.width_px().unwrap_or(375.0);

    Some(AppendContext {
        target_parent_id: target.id_str().to_string(),
        target_width: target.width_px().unwrap_or(width),
        existing_section_labels: section_labels,
        is_mobile: width <= 480.0,
    })
}

/// Design generation should ask the selected LLM to extract a design.md from
/// the current canvas for named follow-on pages (Discover / Orders / Profile,
/// etc.). A document-bound design.md wins, and append mode keeps its
/// append-specific context instead of creating a new sibling screen.
pub fn should_auto_generate_design_md(
    state: &EditorState,
    prompt: &str,
    append_context: Option<&AppendContext>,
) -> bool {
    state.doc.design_md.is_none()
        && append_context.is_none()
        && !state.active_children().is_empty()
        && (is_named_follow_on_screen(prompt) || requests_new_whole_screen(prompt))
}

// ---------------------------------------------------------------------------
// Modification plan — port of generateDesignModification's inputs
// ---------------------------------------------------------------------------

/// TS `buildVariableContext` (design-generator.ts:43-72). `None` when
/// the document has no variables. BTreeMap iteration is sorted where
/// TS uses insertion order — content is identical, ordering may not be.
pub fn build_variable_context(state: &EditorState) -> Option<String> {
    let vars = state.doc.variables.as_ref().filter(|v| !v.is_empty())?;
    let mut lines: Vec<String> = vec![
        "DOCUMENT VARIABLES (use \"$name\" to reference, e.g. fill color \"$color-1\"):".into(),
    ];
    for (name, def) in vars {
        let kind = variable_kind_label(&def.kind);
        match &def.value {
            jian_ops_schema::variable::VariableValue::Themed(values) => {
                let default_val = values
                    .first()
                    .map(|v| scalar_display(&v.value))
                    .unwrap_or_else(|| "?".into());
                lines.push(format!("  - {name} ({kind}): {default_val} [themed]"));
            }
            jian_ops_schema::variable::VariableValue::Scalar(value) => {
                lines.push(format!("  - {name} ({kind}): {}", scalar_display(value)));
            }
        }
    }
    if let Some(themes) = state.doc.themes.as_ref().filter(|t| !t.is_empty()) {
        let summary = themes
            .iter()
            .map(|(axis, values)| format!("{axis}: [{}]", values.join(", ")))
            .collect::<Vec<_>>()
            .join("; ");
        lines.push(format!("Themes: {summary}"));
    }
    Some(lines.join("\n"))
}

fn variable_kind_label(kind: &jian_ops_schema::variable::VariableKind) -> &'static str {
    use jian_ops_schema::variable::VariableKind;
    match kind {
        VariableKind::Color => "color",
        VariableKind::Number => "number",
        VariableKind::String => "string",
        VariableKind::Boolean => "boolean",
    }
}

/// JS template-literal rendering of a variable scalar.
fn scalar_display(value: &jian_ops_schema::variable::VariableScalar) -> String {
    use jian_ops_schema::variable::VariableScalar;
    match value {
        VariableScalar::Bool(b) => b.to_string(),
        VariableScalar::Str(s) => s.clone(),
        VariableScalar::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
    }
}

/// Pre-built `generateDesignModification` request inputs.
pub struct ModifyPlan {
    /// `CONTEXT NODES + INSTRUCTION (+ variable context)` user message.
    pub user_message: String,
    /// Maintenance skills (+ design-md style policy) system prompt.
    pub system_prompt: String,
}

/// Build the modification plan: target selection per
/// `ai-chat-handlers.ts:709-719` (selected nodes, else last frame of
/// the page, else last page child), then the
/// `generateDesignModification` message/prompt assembly. `None` when
/// the page has no usable target (the caller degrades to `new`).
pub fn build_modify_plan(state: &EditorState, instruction: &str) -> Option<ModifyPlan> {
    let children = state.active_children();
    let mut targets: Vec<&PenNode> = Vec::new();
    if !state.selection.set.is_empty() {
        for id in &state.selection.set {
            if let Some(node) = op_editor_core::walkers::find_node(children, id) {
                targets.push(node);
            }
        }
    } else {
        let frames: Vec<&PenNode> = children.iter().filter(|n| is_frame(n)).collect();
        if let Some(last_frame) = frames.last() {
            targets.push(last_frame);
        } else if let Some(last) = children.last() {
            targets.push(last);
        }
    }
    if targets.is_empty() && children.is_empty() {
        return None;
    }

    let context_json = serde_json::to_string(&targets).ok()?;
    let mut user_message = format!("CONTEXT NODES:\n{context_json}\n\nINSTRUCTION:\n{instruction}");
    if let Some(var_context) = build_variable_context(state) {
        user_message.push_str("\n\n");
        user_message.push_str(&var_context);
    }

    // Maintenance-phase skills (TS resolveSkills('maintenance', …)).
    let has_variables = state
        .doc
        .variables
        .as_ref()
        .is_some_and(|vars| !vars.is_empty());
    let mut options = op_ai_skills::ResolveOptions::default();
    options
        .flags
        .insert("hasVariables".to_string(), has_variables);
    options
        .flags
        .insert("hasDesignMd".to_string(), state.doc.design_md.is_some());
    let ctx = op_ai_skills::resolve_skills(op_ai_skills::Phase::Maintenance, instruction, &options);
    let mut system_prompt = ctx
        .skills
        .iter()
        .map(|s| s.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if let Some(spec) = state.doc.design_md.as_ref() {
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&op_orchestrator::build_design_md_style_policy(spec));
    }

    Some(ModifyPlan {
        user_message,
        system_prompt,
    })
}

// ---------------------------------------------------------------------------
// Router worker
// ---------------------------------------------------------------------------

/// Everything the router worker needs, pre-computed on the UI thread.
pub struct CliTurnPlan {
    pub user_text: String,
    /// TS `pageChildren.length === 0` (modify degrades to new).
    pub page_children_empty: bool,
    /// Session-untracked transport for the classification call.
    pub classify_provider: Box<dyn ChatProvider>,
    /// Chat-session-tracked transport for the plain chat route.
    pub chat_provider: Box<dyn ChatProvider>,
    /// Session-untracked transport for the design / modify routes.
    pub design_provider: Box<dyn ChatProvider>,
    /// Fully-assembled plain-chat request (system prompt + history +
    /// per-turn knobs + attachments).
    pub chat_request: ChatRequest,
    /// `generateDesignModification` request; `None` when the page had
    /// no modification target (route degrades to new).
    pub modify_request: Option<ChatRequest>,
    /// Orchestrator request for the new-design route (append context
    /// already detected + attached).
    pub design_request: DesignRequest,
    /// State snapshot for the design route's `RemoteDocSink` mirror.
    pub initial_state: EditorState,
    /// Host-owned indicator epoch shared with the parked `DesignSession`.
    /// The worker registers frame/node indicators under this value and
    /// `DesignSession::drop` clears that same run when Done / Stop /
    /// New Chat retires the session.
    pub indicator_epoch: u64,
    pub model: Option<String>,
}

/// TS `ai-chat-handlers.ts:721-722` — the modification progress step.
const MODIFY_STEP: &str =
    r#"<step title="Checking guidelines">Analyzing modification request...</step>"#;

/// The TS degrade rules (`ai-chat-handlers.ts:700-705`): a modify
/// intent on an empty page becomes a new design, and `isModification`
/// additionally requires a usable target (here: a built modify plan —
/// after the empty-page degrade a surviving Modify always has one in
/// practice, so the plan check is a belt-and-braces guard).
fn resolve_route(
    classified: DesignIntent,
    page_children_empty: bool,
    has_modify_plan: bool,
) -> DesignIntent {
    match classified {
        DesignIntent::Modify if page_children_empty => DesignIntent::New,
        DesignIntent::Modify if !has_modify_plan => DesignIntent::New,
        other => other,
    }
}

/// Run one CLI standard-mode turn end-to-end on the worker thread:
/// classify, then route to chat / modification / design. The caller
/// parked a `ChatSession` on `chat_tx` + the tool channel behind
/// `executor`, and a `DesignSession` on `delta_tx` / `cmd_tx`; the
/// routes not taken drop their senders so the matching pump retires
/// its session.
pub fn run_cli_turn(
    plan: CliTurnPlan,
    chat_tx: Sender<ChatDelta>,
    executor: UiChatToolExecutor,
    delta_tx: Sender<DesignDelta>,
    cmd_tx: Sender<DesignCmdReq>,
) {
    let classified = classify_intent_for_standard_route(
        plan.classify_provider.as_ref(),
        &plan.user_text,
        plan.model.clone(),
    );
    let modify_request = plan.modify_request;
    let intent = resolve_route(
        classified,
        plan.page_children_empty,
        modify_request.is_some(),
    );

    match intent {
        DesignIntent::Chat => {
            drop(delta_tx);
            drop(cmd_tx);
            drop(executor);
            for delta in plan.chat_provider.send(plan.chat_request) {
                if chat_tx.send(delta).is_err() {
                    return; // turn aborted (Stop / New Chat)
                }
            }
        }
        DesignIntent::Modify => {
            drop(delta_tx);
            drop(cmd_tx);
            let request = modify_request.expect("checked above");
            run_modify_turn(plan.design_provider.as_ref(), request, &chat_tx, &executor);
        }
        DesignIntent::New => {
            // Hold the chat channel open for the duration so the
            // trailing bubble keeps its streaming state while the
            // design pumps fill it.
            let _chat_hold = chat_tx;
            drop(executor);
            let llm =
                ChatProviderLlmClient::new(Arc::from(plan.design_provider)).with_model(plan.model);
            run_design_worker(
                llm,
                plan.design_request,
                plan.initial_state,
                delta_tx,
                cmd_tx,
                plan.indicator_epoch,
            );
        }
    }
}

/// The DESIGN_MODIFY route — port of `generateDesignModification` +
/// `extractAndApplyDesignModification` + the handler glue
/// (`ai-chat-handlers.ts:708-741`, `design-generator.ts:95-173`).
pub fn run_modify_turn(
    provider: &dyn ChatProvider,
    request: ChatRequest,
    chat_tx: &Sender<ChatDelta>,
    executor: &UiChatToolExecutor,
) {
    if chat_tx
        .send(ChatDelta::TextDelta(MODIFY_STEP.to_string()))
        .is_err()
    {
        return;
    }
    let mut full_response = String::new();
    let mut stream_error: Option<String> = None;
    for delta in provider.send(request) {
        match delta {
            ChatDelta::TextDelta(s) => full_response.push_str(&s),
            // TS: thinking chunks are ignored for modification — the
            // caller already shows progress.
            ChatDelta::Thinking(_) | ChatDelta::ToolUse { .. } => {}
            ChatDelta::Error(msg) => {
                stream_error = Some(msg);
                break;
            }
            ChatDelta::Done { .. } => break,
        }
    }

    // TS order: parse first; a stream error only surfaces when no
    // nodes could be extracted (design-generator.ts:158-165).
    let nodes = op_orchestrator::parse::parse_nodes(&full_response).unwrap_or_default();
    if !nodes.is_empty() {
        // TS: accumulated = rawResponse (the transcript's design-block
        // renderer shows the JSON as an applyable card).
        if chat_tx
            .send(ChatDelta::TextDelta(format!("\n{full_response}")))
            .is_err()
        {
            return;
        }
        let args = serde_json::json!({
            "nodes": nodes
                .iter()
                .map(|n| serde_json::to_value(n).unwrap_or(serde_json::Value::Null))
                .collect::<Vec<_>>(),
        });
        let result = executor.execute(APPLY_MODIFICATION_OP, &args.to_string());
        let applied = serde_json::from_str::<serde_json::Value>(&result.content)
            .ok()
            .and_then(|v| v.get("count").and_then(|c| c.as_u64()))
            .unwrap_or(0);
        if applied > 0 {
            // TS `ai-chat-handlers.ts:830-831`.
            let _ = chat_tx.send(ChatDelta::TextDelta("\n\n<!-- APPLIED -->".to_string()));
        }
        let _ = chat_tx.send(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        });
        return;
    }

    let message = if let Some(err) = stream_error {
        err
    } else {
        // TS parse-failure error text, verbatim.
        let trimmed = full_response.trim();
        let hint = if trimmed.is_empty() {
            "The model returned an empty response.".to_string()
        } else {
            let preview: String = trimmed.chars().take(150).collect();
            let ellipsis = if full_response.chars().count() > 150 {
                "…"
            } else {
                ""
            };
            format!("Model output: \"{preview}{ellipsis}\"")
        };
        format!("Could not parse design nodes from model response. {hint}")
    };
    let _ = chat_tx.send(ChatDelta::Error(message));
    let _ = chat_tx.send(ChatDelta::Done {
        stop_reason: StopReason::Aborted,
    });
}

#[cfg(test)]
#[path = "chat_intent_tests.rs"]
mod tests;
