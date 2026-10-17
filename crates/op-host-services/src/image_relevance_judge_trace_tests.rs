use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{sync_channel, TryRecvError};
use std::time::Instant;

struct TraceFile {
    root: PathBuf,
    path: PathBuf,
}

impl TraceFile {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "op-judge-trace-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed),
        ));
        Self {
            path: root.join("records.jsonl"),
            root,
        }
    }

    fn batch(&self, route: &'static str) -> JudgeTraceBatch {
        JudgeTraceBatch::with_path(
            Some(self.path.clone()),
            route,
            "bottle photo",
            "glass bottle",
        )
    }

    fn records(&self, expected: usize) -> Vec<Value> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(text) = std::fs::read_to_string(&self.path) {
                // The writer may be partway through its last write_all. Only
                // newline-terminated records are eligible for JSON parsing.
                let complete: Vec<_> = text
                    .split_inclusive('\n')
                    .filter(|line| line.ends_with('\n'))
                    .collect();
                if complete.len() >= expected {
                    return complete
                        .into_iter()
                        .map(|line| {
                            serde_json::from_str(line).expect("one complete JSON object per line")
                        })
                        .collect();
                }
            }
            assert!(
                Instant::now() < deadline,
                "trace writer did not produce {expected} records"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for TraceFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn candidate(index: usize) -> JudgedCandidate {
    JudgedCandidate {
        index,
        verdict: RelevanceVerdict::Weak,
        reason: "slightly softer lighting".into(),
    }
}

fn response<'a>(candidate: &'a JudgedCandidate, content: Option<&'a str>) -> TraceResponse<'a> {
    TraceResponse {
        attempt: 2,
        content,
        reasoning: None,
        candidate,
        elapsed: Duration::from_millis(123),
        timing: TraceTiming::default(),
        error: None,
    }
}

fn evidence() -> Value {
    let checks: Vec<_> = [
        "subject",
        "count",
        "required_object",
        "color_material",
        "appearance",
        "action",
        "background",
        "composition",
        "medium",
        "unrequested_brand",
        "watermark_ad",
        "forbidden_text",
    ]
    .iter()
    .map(|code| json!([code, "requirement", "visible evidence", "match"]))
    .collect();
    json!({
        "schema_version": 2,
        "checks": checks,
        "appearance_contract": { "kind": "surface_texture", "source_quote": "smooth glass" },
        "aesthetic_preferences": [["warm daylight", "lighting", "slightly softer light", "deviation"]],
        "soft_deviations": ["lighting"],
        "hard_conflicts": [],
        "verdict": "weak",
        "reason": "slightly softer lighting",
    })
}

#[test]
fn disabled_trace_and_unused_enabled_trace_do_not_create_files() {
    let file = TraceFile::new();
    for trace in [
        JudgeTraceBatch::default(),
        JudgeTraceBatch::with_path(None, "env", "q", "i"),
        JudgeTraceBatch::with_path(Some(PathBuf::new()), "env", "q", "i"),
    ] {
        assert!(trace.state.is_none());
        trace.record(
            1,
            b"private image bytes",
            response(&candidate(1), Some("unparsed text")),
        );
    }
    let _unused = file.batch("env");
    assert!(!file.root.exists(), "trace is lazy and disabled by default");
}

#[test]
fn trace_keeps_hash_candidate_identity_and_complete_small_evidence() {
    let file = TraceFile::new();
    let trace = file.batch("chat");
    let evidence = evidence();
    let text = format!("```json\n{evidence}\n```");
    let candidate = candidate(7);
    let mut result = response(&candidate, Some(&text));
    result.reasoning = Some(&text);
    trace.record(7, b"abc", result);
    let records = file.records(1);
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record["pid"], std::process::id());
    assert_eq!(record["route"], "chat");
    assert_eq!(record["query"], "bottle photo");
    assert_eq!(record["intent"], "glass bottle");
    assert_eq!(record["index"], 7);
    assert_eq!(record["candidate_index"], 7);
    assert_eq!(record["attempt"], 2);
    assert_eq!(record["elapsed_ms"], 123);
    assert_eq!(record["phase_timing"], "unavailable");
    for field in [
        "queue_ms",
        "provider_ms",
        "budget_before_queue_ms",
        "budget_after_queue_ms",
        "failure_stage",
    ] {
        assert!(record[field].is_null(), "unmeasured field {field}");
    }
    assert_eq!(record["host_verdict"], "weak");
    assert_eq!(record["host_reason"], candidate.reason);
    assert_eq!(
        record["thumb_sha256"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(record["content_evidence"], evidence);
    assert_eq!(record["reasoning_evidence"], evidence);
    assert!(record["error_detail"].is_null());
    assert!(record.get("thumb").is_none() && record.get("raw_response").is_none());
}

#[test]
fn trace_serializes_measured_phases_without_reinterpreting_elapsed() {
    let (trace, record) = JudgeTraceBatch::capture_for_test();
    let candidate = candidate(0);
    let mut result = response(&candidate, None);
    result.timing = TraceTiming {
        queue: Some(Duration::from_millis(12)),
        provider: Some(Duration::from_millis(100)),
        budget_before_queue: Some(Duration::from_millis(1000)),
        budget_after_queue: Some(Duration::from_millis(988)),
        failure_stage: Some(TraceFailureStage::Provider),
    };
    trace.record(0, b"image", result);
    let record = record();
    assert_eq!(record["elapsed_ms"], 123);
    assert_eq!(record["phase_timing"], "available");
    assert_eq!(record["queue_ms"], 12);
    assert_eq!(record["provider_ms"], 100);
    assert_eq!(record["budget_before_queue_ms"], 1000);
    assert_eq!(record["budget_after_queue_ms"], 988);
    assert_eq!(record["failure_stage"], "provider");
}

#[test]
fn concurrent_batches_append_without_interleaved_or_lost_lines() {
    let file = TraceFile::new();
    let batches = [file.batch("env"), file.batch("chat")];
    assert_ne!(
        batches[0].state.as_ref().unwrap().id,
        batches[1].state.as_ref().unwrap().id
    );
    std::thread::scope(|scope| {
        for worker in 0..8 {
            let batch = batches[worker % 2].clone();
            scope.spawn(move || {
                for item in 0..2 {
                    let index = worker * 2 + item;
                    batch.record(
                        index,
                        index.to_string().as_bytes(),
                        response(&candidate(index), None),
                    );
                }
            });
        }
    });
    let records = file.records(16);
    assert_eq!(records.len(), 16);
    let mut indices = HashSet::new();
    let mut batch_counts = HashMap::<String, usize>::new();
    for record in records {
        let index = record["index"].as_u64().unwrap();
        assert!(indices.insert(index));
        assert_eq!(record["candidate_index"], index);
        assert_eq!(
            record["thumb_sha256"],
            format!("{:x}", Sha256::digest(index.to_string().as_bytes()))
        );
        *batch_counts
            .entry(record["batch_id"].as_str().unwrap().into())
            .or_default() += 1;
    }
    assert_eq!(batch_counts.len(), 2);
    assert!(batch_counts.values().all(|count| *count == 8));
}

#[test]
fn trace_redacts_payloads_and_credentials_inside_allowed_fields_and_briefs() {
    let file = TraceFile::new();
    let trace = JudgeTraceBatch::with_path(
        Some(file.path.clone()),
        "env",
        "apiKey=private-abc",
        "reference data:image/png;base64,private-image-payload",
    );
    let mut evidence = evidence();
    evidence["api_key"] = json!("private-top-secret");
    evidence["raw_response"] = json!("private-raw-response");
    evidence["appearance_contract"]["accessToken"] = json!("private-nested-token");
    evidence["appearance_contract"]["source_quote"] =
        json!("inline {\"Authorization\":\"Bearer private-bearer\"}");
    evidence["checks"][0][2] = json!(format!("visible photo {}", "Z".repeat(200)));
    evidence["checks"][1][2] = json!("image is data:image/jpeg;base64,private-jpeg");
    evidence["checks"][2][2] = json!("embedded {\"token\" : \"private-spaced-token\"}");
    evidence["checks"][3][2] = json!(r#"embedded {"api\u005fkey":"private-escaped-key"}"#);
    evidence["reason"] = json!("use sk-private-value");
    let text = evidence.to_string();
    let mut candidate = candidate(1);
    candidate.reason = "failed token=private-host-token".into();
    let mut result = response(&candidate, Some(&text));
    result.reasoning = Some("raw non-JSON private-reasoning-secret");
    result.error = Some("HTTP 429 Authorization: Bearer private-error-token");
    trace.record(1, b"private-raw-image", result);
    let records = file.records(1);
    let raw = std::fs::read_to_string(&file.path).unwrap();
    for forbidden in [
        "private-",
        "data:image",
        &"Z".repeat(200),
        "api_key",
        "accessToken",
        "raw_response",
    ] {
        assert!(!raw.contains(forbidden), "trace leaked {forbidden}");
    }
    assert_eq!(records[0]["query"], REDACTED);
    assert_eq!(records[0]["intent"], REDACTED);
    assert_eq!(records[0]["host_reason"], REDACTED);
    assert_eq!(records[0]["error_detail"], REDACTED);
    assert!(records[0]["reasoning_evidence"].is_null());
    assert_eq!(
        records[0]["content_evidence"]["checks"]
            .as_array()
            .unwrap()
            .len(),
        12
    );
    assert_eq!(
        records[0]["content_evidence"]["appearance_contract"]["kind"],
        "surface_texture"
    );
}

#[test]
fn trace_omits_invalid_or_oversized_responses_and_retains_safe_error_detail() {
    let file = TraceFile::new();
    let trace = file.batch("env");
    let oversized = format!("{{\"reason\":\"{}\"}}", "x".repeat(MAX_RESPONSE_BYTES));
    let candidate = candidate(0);
    let mut result = response(&candidate, Some(&oversized));
    result.reasoning = Some("not JSON and never recorded");
    result.error = Some("HTTP 429");
    trace.record(0, b"a", result);
    let records = file.records(1);
    assert!(records[0]["content_evidence"].is_null());
    assert!(records[0]["reasoning_evidence"].is_null());
    assert_eq!(records[0]["error_detail"], "HTTP 429");
}

#[test]
fn unavailable_trace_destination_never_changes_the_candidate() {
    let file = TraceFile::new();
    std::fs::create_dir_all(&file.path).unwrap();
    let (tx, rx) = sync_channel(1);
    let trace = JudgeTraceBatch::with_sender(Some(file.path.clone()), "chat", "q", "i", tx);
    let candidate = candidate(3);
    trace.record(3, b"a", response(&candidate, Some("{}")));
    let write = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(append_record(&write).is_err());
    assert_eq!(candidate.verdict, RelevanceVerdict::Weak);
    assert_eq!(candidate.reason, "slightly softer lighting");
}

#[test]
fn full_or_disconnected_trace_queue_never_blocks_record_or_drop() {
    let file = TraceFile::new();
    let (tx, rx) = sync_channel(1);
    let trace = JudgeTraceBatch::with_sender(Some(file.path.clone()), "chat", "q", "i", tx);
    let started = Instant::now();
    for index in 0..16 {
        trace.record(index, b"a", response(&candidate(index), None));
    }
    drop(trace);
    assert!(started.elapsed() < Duration::from_millis(200));
    let first = rx.try_recv().expect("one queued record survives");
    let record: Value = serde_json::from_slice(&first.line).unwrap();
    assert_eq!(record["index"], 0);
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Disconnected)));
    assert!(!file.path.exists());

    let (tx, rx) = sync_channel(1);
    drop(rx);
    let trace = JudgeTraceBatch::with_sender(Some(file.path.clone()), "env", "q", "i", tx);
    let started = Instant::now();
    trace.record(4, b"a", response(&candidate(4), None));
    drop(trace);
    assert!(started.elapsed() < Duration::from_millis(200));
    assert!(!file.path.exists());
}

#[test]
fn blocked_writer_keeps_one_queued_record_and_discards_overflow_without_waiting() {
    let file = TraceFile::new();
    let (tx, rx) = sync_channel::<TraceWrite>(1);
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(move || {
        let first = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        entered_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        append_record(&first).unwrap();
        for record in rx {
            append_record(&record).unwrap();
        }
    });
    let trace = JudgeTraceBatch::with_sender(Some(file.path.clone()), "env", "q", "i", tx);
    trace.record(0, b"a", response(&candidate(0), None));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let started = Instant::now();
    trace.record(1, b"b", response(&candidate(1), None));
    trace.record(2, b"c", response(&candidate(2), None));
    drop(trace);
    assert!(started.elapsed() < Duration::from_millis(200));
    assert!(
        !file.path.exists(),
        "writer is still held behind its release gate"
    );
    release_tx.send(()).unwrap();
    writer.join().unwrap();
    let records = file.records(2);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["index"], 0);
    assert_eq!(records[1]["index"], 1);
}

#[test]
fn full_trace_queue_does_not_retain_a_real_image_judge_permit() {
    use crate::image_relevance_judge::{
        ChatVisionJudge, JUDGE_REQUEST_PERMITS, MAX_CONCURRENT_CALLS,
    };
    use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};

    struct ImmediateProvider;
    impl ChatProvider for ImmediateProvider {
        fn provider_label(&self) -> &str {
            "trace-permit-test"
        }
        fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            Box::new(std::iter::once(ChatDelta::TextDelta(
                crate::image_relevance_judge::tests::valid_reply("on", "match"),
            )))
        }
    }
    let _transport = crate::image_relevance_judge::tests::transport_lock();
    let file = TraceFile::new();
    let (tx, rx) = sync_channel(1);
    let trace = JudgeTraceBatch::with_sender(Some(file.path.clone()), "chat", "q", "i", tx);
    trace.record(99, b"a", response(&candidate(99), None));
    let judge = ChatVisionJudge::new(Arc::new(ImmediateProvider));
    let started = Instant::now();
    let result = judge.judge_one(
        0,
        ("bottle", "glass bottle"),
        &[0xff, 0xd8, 0xff, 1],
        started + Duration::from_millis(150),
        &trace,
    );
    assert_eq!(result.verdict, RelevanceVerdict::On);
    assert!(started.elapsed() < Duration::from_millis(200));
    let _all_permits = JUDGE_REQUEST_PERMITS
        .try_acquire_many(MAX_CONCURRENT_CALLS as u32)
        .expect("completed judge releases capacity while its trace writer is blocked");
    assert_eq!(
        serde_json::from_slice::<Value>(&rx.try_recv().unwrap().line).unwrap()["index"],
        99
    );
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

#[cfg(unix)]
#[test]
fn regular_trace_files_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let file = TraceFile::new();
    append_record(&TraceWrite {
        path: file.path.clone(),
        line: b"{}\n".to_vec(),
    })
    .unwrap();
    let metadata = std::fs::metadata(&file.path).unwrap();
    assert!(metadata.is_file());
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
}

#[cfg(unix)]
#[test]
fn fifo_device_directory_and_symlink_are_rejected_without_waiting() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let file = TraceFile::new();
    std::fs::create_dir_all(&file.root).unwrap();
    let fifo = file.root.join("blocked.fifo");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: this is a valid NUL-terminated local pathname owned by this test.
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    let target = file.root.join("target.jsonl");
    std::fs::write(&target, b"unchanged\n").unwrap();
    let link = file.root.join("linked.jsonl");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    for path in [fifo, PathBuf::from("/dev/null"), file.root.clone(), link] {
        let (tx, rx) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            tx.send(append_record(&TraceWrite {
                path,
                line: b"{}\n".to_vec(),
            }))
            .unwrap();
        });
        assert!(rx
            .recv_timeout(Duration::from_millis(200))
            .expect("special-file rejection must not block")
            .is_err());
        writer.join().unwrap();
    }
    assert_eq!(std::fs::read(target).unwrap(), b"unchanged\n");
}
