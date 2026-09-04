use super::*;

use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

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

// ── ChatVisionJudge ──────────────────────────────────────────────────────

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason};

/// Records the requests it receives and replies with a fixed verdict —
/// same shape as the `RecordingVisionProvider` in `validation_providers`.
struct ScriptedJudgeProvider {
    seen: Arc<Mutex<Vec<ChatRequest>>>,
    reply: String,
}

impl ChatProvider for ScriptedJudgeProvider {
    fn provider_label(&self) -> &str {
        "scripted-judge"
    }
    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.seen.lock().unwrap().push(request);
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

fn scripted_judge(reply: &str) -> (ChatVisionJudge, Arc<Mutex<Vec<ChatRequest>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let provider = Arc::new(ScriptedJudgeProvider {
        seen: seen.clone(),
        reply: reply.to_string(),
    });
    (ChatVisionJudge::new(provider), seen)
}

fn fake_jpeg(seed: u8) -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xE0, seed]
}

#[test]
fn chat_vision_judge_maps_off_verdict_and_sends_shared_prompt() {
    let (judge, seen) = scripted_judge(r#"{"verdict":"off","reason":"different subject"}"#);
    let judged = judge.judge("kyoto temple", "a photo of a kyoto temple", &[fake_jpeg(1)]);

    assert_eq!(judged.len(), 1);
    assert_eq!(judged[0].index, 0);
    assert_eq!(judged[0].verdict, RelevanceVerdict::Off);
    assert_eq!(judged[0].reason, "different subject");

    let requests = seen.lock().unwrap();
    let request = requests.first().expect("provider called once");
    assert!(
        request.user_message.contains("kyoto temple")
            && request.user_message.contains("a photo of a kyoto temple"),
        "the shared judge prompt carries query + intent: {}",
        request.user_message
    );
    assert_eq!(request.attachments.len(), 1, "one thumbnail attachment");
    assert_eq!(request.attachments[0].media_type, "image/jpeg");
    assert_eq!(request.attachments[0].data, fake_jpeg(1));
}

#[test]
fn chat_vision_judge_downgrades_unparseable_reply_to_weak() {
    let (judge, _seen) = scripted_judge("no JSON in this reply");
    let judged = judge.judge("q", "i", &[fake_jpeg(1)]);
    assert_eq!(judged.len(), 1);
    assert_eq!(judged[0].verdict, RelevanceVerdict::Weak);
    assert_eq!(judged[0].reason, "judge unavailable");
}

#[test]
fn chat_vision_judge_returns_one_verdict_per_thumb_index_aligned() {
    let (judge, _seen) = scripted_judge(r#"{"verdict":"on","reason":"clear subject match"}"#);
    let thumbs = vec![fake_jpeg(1), fake_jpeg(2), fake_jpeg(3)];
    let judged = judge.judge("q", "i", &thumbs);
    assert_eq!(judged.len(), 3, "one verdict per input thumbnail");
    for (position, candidate) in judged.iter().enumerate() {
        assert_eq!(candidate.index, position, "verdicts align with input order");
        assert_eq!(candidate.verdict, RelevanceVerdict::On);
    }
}

// ── env judge inside the image runtime ──────────────────────────────────────

/// Minimal HTTP stub: accepts POSTs on 127.0.0.1:0, drains the request, and
/// replies with an OpenAI-shaped chat completion whose message content is a
/// fixed `on` verdict. One std::thread per connection because the judge fires
/// candidates concurrently. The listener thread is intentionally leaked.
fn start_verdict_stub() -> u16 {
    use std::io::{Read, Write};

    fn handle(mut stream: std::net::TcpStream) {
        let mut head = Vec::new();
        let mut byte = [0_u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            if stream.read(&mut byte).unwrap_or(0) == 0 || head.len() > 64 * 1024 {
                return;
            }
            head.push(byte[0]);
        }
        let head = String::from_utf8_lossy(&head);
        let content_length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.trim().eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let mut body = vec![0_u8; content_length];
        let _ = stream.read_exact(&mut body);
        let payload =
            r#"{"choices":[{"message":{"content":"{\"verdict\":\"on\",\"reason\":\"stub\"}"}}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            payload.len(),
            payload
        );
        let _ = stream.write_all(response.as_bytes());
    }

    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind stub server");
    let port = listener.local_addr().expect("stub addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => std::thread::spawn(move || handle(stream)),
                Err(_) => break,
            };
        }
    });
    port
}

/// Production reaches `OpenAiCompatVisionJudge::judge` from sync code already
/// running inside `block_on_image_runtime`; driving the HTTP calls through
/// that same runtime again panics with "Cannot start a runtime from within a
/// runtime" and every candidate comes back unjudged. This pins the fix: the
/// sync judge call must survive running on an image-runtime worker thread.
#[test]
fn env_judge_survives_being_called_inside_the_image_runtime() {
    let _lock = env_lock();
    let _env = JudgeEnvGuard::clear();
    let port = start_verdict_stub();
    std::env::set_var(BASE_URL_ENV, format!("http://127.0.0.1:{port}/v1"));
    std::env::set_var(API_KEY_ENV, "test-key");
    std::env::set_var(MODEL_ENV, "vision-model");
    let judge = OpenAiCompatVisionJudge::from_env().expect("env settings build the judge");

    let judged = op_image_enrich::net::block_on_image_runtime(async move {
        judge.judge(
            "kyoto temple",
            "a photo of a kyoto temple",
            &[fake_jpeg(1), fake_jpeg(2)],
        )
    });

    assert_eq!(judged.len(), 2, "both candidates judged, none lost");
    for (position, candidate) in judged.iter().enumerate() {
        assert_eq!(candidate.index, position);
        assert_eq!(candidate.verdict, RelevanceVerdict::On);
        assert_eq!(candidate.reason, "stub");
    }
}
