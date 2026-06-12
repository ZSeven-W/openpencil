//! Chat-history trimming + digest helpers.
//!
//! Ports the TS sliding-window policy from
//! `apps/web/src/services/ai/context-optimizer.ts::trimChatHistory`:
//! keep at most [`DEFAULT_MAX_MESSAGES`] recent turns within a
//! [`DEFAULT_MAX_CHARS`] character budget, always preserving the
//! first user message for context continuity. The Rust history is
//! text-only (attachments stay current-turn-only on `ChatRequest`),
//! so the TS attachment-stripping pass has no equivalent here.
//!
//! [`history_digest`] renders a trimmed history into a compact text
//! block for prompt-only transports (CLI subprocess / Copilot / ACP)
//! whose wire carries a single prompt string instead of `messages[]`.
//!
//! DELIBERATE DIVERGENCE from TS: budgets count Rust `char`s (Unicode
//! scalar values), not JS UTF-16 code units, and truncation never
//! splits a character — TS `slice()` can cut a surrogate pair in
//! half. Counts differ only for non-BMP text; the budget is a
//! heuristic, safety wins.

use crate::chat_provider::ChatHistoryRole;

/// Sliding-window message cap (TS `DEFAULT_MAX_MESSAGES`).
pub const DEFAULT_MAX_MESSAGES: usize = 10;
/// Sliding-window character budget (TS `DEFAULT_MAX_CHARS`).
pub const DEFAULT_MAX_CHARS: usize = 32_000;
/// Per-message cap inside [`history_digest`] so one giant turn can't
/// eat the whole digest budget.
const DIGEST_PER_MESSAGE_CHARS: usize = 400;
/// Total digest budget — digests ride inside a prompt string, so they
/// stay much smaller than the full `messages[]` window.
pub const DEFAULT_DIGEST_CHARS: usize = 4_000;

/// Sliding window for chat history (TS `trimChatHistory` port).
/// Keeps the most recent `max_messages` while respecting `max_chars`;
/// always preserves the first user message. A message that overflows
/// the budget is truncated (with a marker) when at least 200 chars of
/// budget remain, else dropped — byte-for-byte the TS policy.
pub fn trim_chat_history(
    messages: &[(ChatHistoryRole, String)],
    max_messages: usize,
    max_chars: usize,
) -> Vec<(ChatHistoryRole, String)> {
    if messages.len() <= max_messages {
        let total: usize = messages.iter().map(|(_, c)| c.chars().count()).sum();
        if total <= max_chars {
            return messages.to_vec();
        }
    }

    // Always keep the first user message for context continuity.
    let first_user = messages
        .iter()
        .position(|(role, _)| *role == ChatHistoryRole::User);
    let recent_start = messages.len().saturating_sub(max_messages);

    let mut window: Vec<(ChatHistoryRole, String)> = Vec::new();
    let mut char_count = 0usize;

    if let Some(idx) = first_user {
        if idx < recent_start {
            let (role, content) = &messages[idx];
            window.push((*role, content.clone()));
            char_count += content.chars().count();
        }
    }

    for (role, content) in &messages[recent_start..] {
        let msg_chars = content.chars().count();
        if char_count + msg_chars > max_chars {
            // Truncate this message to fit (TS: only when >200 chars
            // of budget remain, else stop).
            let remaining = max_chars.saturating_sub(char_count);
            if remaining > 200 {
                let clipped: String = content.chars().take(remaining).collect();
                window.push((*role, format!("{clipped}\n[...truncated...]")));
            }
            break;
        }
        window.push((*role, content.clone()));
        char_count += msg_chars;
    }

    window
}

/// Render `history` into a compact transcript digest for transports
/// whose wire is a single prompt string. Empty input → empty string
/// (callers skip the prepend). Each message is clipped to a few
/// hundred chars; the whole digest stays within `max_chars`.
pub fn history_digest(history: &[(ChatHistoryRole, String)], max_chars: usize) -> String {
    if history.is_empty() {
        return String::new();
    }
    let mut out =
        String::from("Previous conversation (context only — reply to the final message):");
    let mut used = out.chars().count();
    for (role, content) in history {
        let label = match role {
            ChatHistoryRole::User => "User",
            ChatHistoryRole::Assistant => "Assistant",
        };
        let body = content.trim();
        if body.is_empty() {
            continue;
        }
        let clipped: String = if body.chars().count() > DIGEST_PER_MESSAGE_CHARS {
            let mut c: String = body.chars().take(DIGEST_PER_MESSAGE_CHARS).collect();
            c.push('…');
            c
        } else {
            body.to_string()
        };
        let line = format!("\n{label}: {clipped}");
        let line_chars = line.chars().count();
        if used + line_chars > max_chars {
            break;
        }
        out.push_str(&line);
        used += line_chars;
    }
    // Header-only digest means every message was empty / over budget —
    // nothing useful to prepend.
    if out.chars().count() <= 70 && !out.contains('\n') {
        return String::new();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_provider::ChatHistoryRole::{Assistant, User};

    fn msg(
        role: crate::chat_provider::ChatHistoryRole,
        text: &str,
    ) -> (crate::chat_provider::ChatHistoryRole, String) {
        (role, text.to_string())
    }

    #[test]
    fn trim_keeps_short_histories_verbatim() {
        let h = vec![msg(User, "a"), msg(Assistant, "b"), msg(User, "c")];
        let out = trim_chat_history(&h, DEFAULT_MAX_MESSAGES, DEFAULT_MAX_CHARS);
        assert_eq!(out, h);
    }

    #[test]
    fn trim_window_keeps_most_recent_messages() {
        let h: Vec<_> = (0..20)
            .map(|i| {
                let role = if i % 2 == 0 { User } else { Assistant };
                msg(role, &format!("m{i}"))
            })
            .collect();
        let out = trim_chat_history(&h, 10, DEFAULT_MAX_CHARS);
        // First user message ("m0") survives ahead of the recent window.
        assert_eq!(out[0].1, "m0");
        // The recent window is the last 10 messages, in order.
        assert_eq!(out.len(), 11);
        assert_eq!(out[1].1, "m10");
        assert_eq!(out.last().unwrap().1, "m19");
    }

    #[test]
    fn trim_first_user_not_duplicated_when_inside_window() {
        let h = vec![
            msg(User, "first"),
            msg(Assistant, "a1"),
            msg(User, "u2"),
            msg(Assistant, "a2"),
        ];
        // max_messages covers everything but the char budget forces the
        // trim path; "first" sits inside the recent window so it must
        // not be prepended twice.
        let out = trim_chat_history(&h, 10, 8);
        let firsts = out.iter().filter(|(_, c)| c == "first").count();
        assert_eq!(firsts, 1, "first user message must appear exactly once");
    }

    #[test]
    fn trim_truncates_overflowing_message_with_marker() {
        let big = "x".repeat(1_000);
        let h = vec![msg(User, "intro"), msg(Assistant, &big)];
        let out = trim_chat_history(&h, 10, 500);
        let last = &out.last().unwrap().1;
        assert!(
            last.ends_with("\n[...truncated...]"),
            "overflow message must carry the TS truncation marker, got tail {:?}",
            &last[last.len().saturating_sub(30)..]
        );
        // 500-char budget minus "intro" (5) leaves 495 chars of body.
        assert!(last.chars().count() < 600);
    }

    #[test]
    fn trim_drops_overflow_message_when_budget_remainder_too_small() {
        let h = vec![
            msg(User, &"a".repeat(31_900)),
            msg(Assistant, &"b".repeat(500)),
        ];
        // Remaining budget after the first message is 100 (< 200), so
        // the overflowing second message is dropped, not truncated.
        let out = trim_chat_history(&h, 1, 32_000);
        assert_eq!(out.len(), 1);
        assert!(out[0].1.starts_with('a'));
    }

    #[test]
    fn digest_renders_role_labelled_lines() {
        let h = vec![
            msg(User, "make a card"),
            msg(Assistant, "done — added a card"),
        ];
        let d = history_digest(&h, DEFAULT_DIGEST_CHARS);
        assert!(d.contains("User: make a card"));
        assert!(d.contains("Assistant: done — added a card"));
        assert!(d.starts_with("Previous conversation"));
    }

    #[test]
    fn digest_empty_for_empty_history() {
        assert_eq!(history_digest(&[], DEFAULT_DIGEST_CHARS), "");
    }

    #[test]
    fn digest_clips_long_messages_and_respects_total_budget() {
        let h = vec![
            msg(User, &"y".repeat(2_000)),
            msg(Assistant, &"z".repeat(2_000)),
        ];
        let d = history_digest(&h, 600);
        assert!(d.chars().count() <= 600);
        assert!(
            d.contains('…'),
            "long messages are clipped with an ellipsis"
        );
    }

    #[test]
    fn digest_skips_blank_messages() {
        let h = vec![msg(User, "   "), msg(Assistant, "")];
        assert_eq!(history_digest(&h, DEFAULT_DIGEST_CHARS), "");
    }
}
