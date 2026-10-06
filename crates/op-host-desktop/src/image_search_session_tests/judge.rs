//! `resolve_judge` three-state selection tests: the env judge wins, the
//! provider-backed `ChatVisionJudge` is next, and `NoJudge` is the
//! fallback. Env mutation follows the same mutex + restore-guard isolation
//! the op-host-services env tests use.

use super::super::*;

use op_ai::chat_provider::{ChatDelta, ChatRequest, StopReason};
use op_image_enrich::net::RelevanceVerdict;
use std::sync::{Mutex, MutexGuard, OnceLock};

const BASE_URL_ENV: &str = "OPENPENCIL_IMAGE_JUDGE_BASE_URL";
const API_KEY_ENV: &str = "OPENPENCIL_IMAGE_JUDGE_API_KEY";
const MODEL_ENV: &str = "OPENPENCIL_IMAGE_JUDGE_MODEL";

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock")
}

struct JudgeEnvGuard {
    values: [Option<String>; 3],
}

impl JudgeEnvGuard {
    fn clear() -> Self {
        let keys = [BASE_URL_ENV, API_KEY_ENV, MODEL_ENV];
        let values = keys.map(|key| std::env::var(key).ok());
        for key in keys {
            std::env::remove_var(key);
        }
        Self { values }
    }
}

impl Drop for JudgeEnvGuard {
    fn drop(&mut self) {
        for (key, value) in [BASE_URL_ENV, API_KEY_ENV, MODEL_ENV]
            .into_iter()
            .zip(self.values.iter())
        {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

/// Scripted provider with a fixed verdict reply.
struct ScriptedJudgeProvider {
    reply: String,
}

impl ChatProvider for ScriptedJudgeProvider {
    fn provider_label(&self) -> &str {
        "scripted-judge"
    }
    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(
            vec![
                ChatDelta::TextDelta(self.reply.clone()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter(),
        )
    }
}

fn scripted_provider(reply: &str) -> Arc<dyn ChatProvider> {
    Arc::new(ScriptedJudgeProvider {
        reply: reply.to_string(),
    })
}

#[test]
fn env_judge_wins_over_provider() {
    let _lock = env_lock();
    let _env = JudgeEnvGuard::clear();
    std::env::set_var(BASE_URL_ENV, "https://judge.example/v1");
    std::env::set_var(API_KEY_ENV, "test-key");
    std::env::set_var(MODEL_ENV, "vision-model");

    let (judge, choice) = resolve_judge(Some(scripted_provider(r#"{"verdict":"off"}"#)), None);
    assert_eq!(choice, JudgeChoice::Env);
    // Zero thumbnails means zero network calls; every judge maps its input
    // one-to-one, so an empty batch stays empty.
    assert!(judge.judge("q", "i", &[]).is_empty());
}

#[test]
fn provider_judge_is_used_when_no_env_is_configured() {
    let _lock = env_lock();
    let _env = JudgeEnvGuard::clear();

    let (judge, choice) = resolve_judge(
        Some(scripted_provider(
            r#"{"verdict":"off","reason":"different subject"}"#,
        )),
        None,
    );
    assert_eq!(choice, JudgeChoice::Provider("scripted-judge".to_string()));
    let judged = judge.judge("kyoto temple", "kyoto temple", &[vec![0xFF, 0xD8, 0xFF, 1]]);
    assert_eq!(judged.len(), 1);
    assert_eq!(judged[0].index, 0);
    assert_eq!(judged[0].verdict, RelevanceVerdict::Off);
    assert_eq!(judged[0].reason, "different subject");
}

#[test]
fn no_judge_is_the_fallback_without_env_or_provider() {
    let _lock = env_lock();
    let _env = JudgeEnvGuard::clear();

    let (judge, choice) = resolve_judge(None, None);
    assert_eq!(choice, JudgeChoice::None);
    let judged = judge.judge("q", "i", &[vec![1], vec![2]]);
    assert_eq!(judged.len(), 2);
    assert!(
        judged
            .iter()
            .all(|candidate| candidate.verdict == RelevanceVerdict::On),
        "NoJudge keeps every candidate eligible"
    );
}
