use super::*;

use std::sync::{Mutex, MutexGuard, OnceLock};

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

#[test]
fn from_env_requires_all_three_settings() {
    let _lock = env_lock();
    let _env = JudgeEnvGuard::clear();
    let keys = [BASE_URL_ENV, API_KEY_ENV, MODEL_ENV];
    for missing in keys {
        std::env::set_var(BASE_URL_ENV, "https://judge.example/v1");
        std::env::set_var(API_KEY_ENV, "test-key");
        std::env::set_var(MODEL_ENV, "vision-model");
        std::env::remove_var(missing);
        assert!(
            OpenAiCompatVisionJudge::from_env().is_none(),
            "missing {missing} must disable the judge"
        );
    }
}

#[test]
fn response_json_is_read_from_content_first() {
    let result = parse_verdict_response(
        Some(r#"prefix {"verdict":"on","reason":"clear subject match"} suffix"#),
        Some(r#"{"verdict":"off","reason":"ignored"}"#),
    );
    assert_eq!(
        result,
        Some((RelevanceVerdict::On, "clear subject match".to_string()))
    );
}

#[test]
fn response_json_falls_back_to_reasoning_content() {
    let result = parse_verdict_response(
        Some("thinking only"),
        Some(r#"{"verdict":"weak","reason":"partial visual match"}"#),
    );
    assert_eq!(
        result,
        Some((RelevanceVerdict::Weak, "partial visual match".to_string()))
    );
}

#[test]
fn invalid_response_is_downgraded_to_weak() {
    let fallback = parse_candidate_response(4, Some("not JSON"), Some("also not JSON"));
    assert_eq!(fallback.verdict, RelevanceVerdict::Weak);
    assert_eq!(fallback.reason, "judge unavailable");
}
