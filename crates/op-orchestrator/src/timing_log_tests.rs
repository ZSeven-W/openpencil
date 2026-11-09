use super::*;
use futures::executor::block_on;
use futures::stream::{self, StreamExt};
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};

use crate::types::{AbortFlag, CallRequest, DocSink, LlmChunk, LlmClient, LlmError, Progress};

fn capture() -> (RunTiming, Arc<Mutex<Vec<String>>>) {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let sink = lines.clone();
    let timing = RunTiming::with_emitter(Box::new(move |line| {
        sink.lock().unwrap().push(line.to_string());
    }));
    (timing, lines)
}

fn joined(lines: &Arc<Mutex<Vec<String>>>) -> String {
    lines.lock().unwrap().join("\n")
}

struct ScriptedLlm(Vec<Result<LlmChunk, LlmError>>);

impl LlmClient for ScriptedLlm {
    fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        Box::pin(stream::iter(self.0.clone()))
    }
}

fn request() -> CallRequest {
    CallRequest {
        system_prompt: "s".repeat(2048),
        user_prompt: "u".repeat(1024),
        model: None,
        provider: None,
        timeout: Duration::from_secs(1),
        abort: AbortFlag::default(),
        no_text_timeout: None,
        first_text_timeout: None,
    }
}

#[derive(Default)]
struct RecordingSink {
    state: EditorState,
    applied: Vec<EditorCommand>,
    inserted: usize,
}

impl DocSink for RecordingSink {
    fn state(&self) -> &EditorState {
        &self.state
    }
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.applied.push(cmd);
        true
    }
    fn insert_subtree_returning_root_ids(
        &mut self,
        nodes: Vec<PenNode>,
        _parent_id: &NodeId,
    ) -> Option<Vec<String>> {
        self.inserted += nodes.len();
        Some(vec!["root".into()])
    }
    fn begin_undo_batch(&mut self) {}
    fn end_undo_batch(&mut self) {}
    fn is_buffered(&self) -> bool {
        true
    }
}

fn frame_with_children(children: usize) -> PenNode {
    let kids: Vec<serde_json::Value> = (0..children)
        .map(|i| serde_json::json!({"id": format!("t{i}"), "type": "text", "content": "x"}))
        .collect();
    serde_json::from_value(serde_json::json!({
        "id": "f", "type": "frame", "children": kids
    }))
    .unwrap()
}

#[test]
fn switch_values() {
    assert!(!switch_on(None));
    assert!(!switch_on(Some("")));
    assert!(!switch_on(Some(" 0 ")));
    assert!(switch_on(Some("1")));
    assert!(switch_on(Some("yes")));
}

#[test]
fn disabled_timing_passes_everything_through_silently() {
    let timing = RunTiming::disabled();
    let llm = ScriptedLlm(vec![Ok(LlmChunk::Text("hello".into()))]);
    let timed = timing.llm(&llm);
    let chunks: Vec<_> = block_on(timed.call(request()).collect());
    assert_eq!(chunks.len(), 1);

    let mut inner = RecordingSink::default();
    {
        let mut sink = timing.sink(&mut inner);
        assert!(sink.is_buffered(), "is_buffered must forward");
        sink.insert_subtree_returning_root_ids(vec![frame_with_children(2)], &NodeId::NONE);
    }
    assert_eq!(inner.inserted, 1);
}

#[test]
fn call_reports_first_text_sizes_and_phase() {
    let (timing, lines) = capture();
    timing.progress(&Progress::Planning);
    let llm = ScriptedLlm(vec![
        Ok(LlmChunk::Thinking("hmm".into())),
        Ok(LlmChunk::Text("a".repeat(2048))),
    ]);
    let timed = timing.llm(&llm);
    let chunks: Vec<_> = block_on(timed.call(request()).collect());
    assert_eq!(chunks.len(), 2, "every chunk still reaches the consumer");
    let log = joined(&lines);
    assert!(log.contains("call#1 start phase=plan in=3.0KB"), "{log}");
    assert!(log.contains("call#1 end phase=plan"), "{log}");
    assert!(log.contains("text=2.0KB thinking=3B"), "{log}");
    assert!(!log.contains("first-text=-"), "{log}");
}

#[test]
fn errored_call_is_reported_once_on_drop() {
    let (timing, lines) = capture();
    let llm = ScriptedLlm(vec![Err(LlmError {
        message: "429 rate limited".into(),
        aborted: false,
    })]);
    let timed = timing.llm(&llm);
    let mut stream = timed.call(request());
    let first = block_on(stream.next());
    assert!(matches!(first, Some(Err(_))));
    drop(stream);
    let log = joined(&lines);
    assert_eq!(log.matches("call#1 error").count(), 1, "{log}");
    assert!(log.contains("error=\"429 rate limited\""), "{log}");
    assert!(log.contains("first-text=-"), "{log}");
}

#[test]
fn first_insert_after_scaffold_is_flagged_once() {
    let (timing, lines) = capture();
    let mut inner = RecordingSink::default();
    {
        let mut sink = timing.sink(&mut inner);
        // Before the scaffold lands, inserts are scaffold work, not content.
        sink.apply(EditorCommand::InsertSubtree {
            nodes: vec![frame_with_children(0)],
            parent_id: NodeId::NONE,
            page_id: None,
        });
        timing.progress(&Progress::ScaffoldDone);
        timing.progress(&Progress::SubtaskStarted {
            id: "hero".into(),
            label: "Hero".into(),
        });
        sink.insert_subtree_returning_root_ids(vec![frame_with_children(3)], &NodeId::NONE);
        sink.apply(EditorCommand::Batch {
            commands: vec![EditorCommand::InsertSubtree {
                nodes: vec![frame_with_children(1)],
                parent_id: NodeId::NONE,
                page_id: None,
            }],
        });
        sink.rollback_inserted_roots(&["root".into()]);
    }
    let log = joined(&lines);
    assert!(log.contains("insert#1 nodes=1 phase=start\n"), "{log}");
    assert!(
        log.contains("insert#2 nodes=4 phase=subtask:hero (first section on canvas)"),
        "{log}"
    );
    assert!(
        log.contains("insert#3 nodes=2 phase=subtask:hero\n"),
        "{log}"
    );
    assert_eq!(log.matches("first section").count(), 1, "{log}");
    assert!(log.contains("rollback roots=1 phase=subtask:hero"), "{log}");
    // InsertSubtree + Batch, plus the inner sink's default rollback, which
    // deletes through `apply` — proof the rollback reached the inner sink.
    assert_eq!(inner.applied.len(), 3, "apply and rollback must forward");
}

#[test]
fn worker_scoped_progress_prefixes_the_group() {
    let event = Progress::worker_scoped(
        2,
        "Home",
        crate::agent_identity::AgentIdentity {
            color: "#fff".into(),
            name: "Kiki".into(),
        },
        Progress::SubtaskRetry {
            id: "list".into(),
            attempt: 2,
            reason: "x".into(),
        },
    );
    assert_eq!(phase_hint(&event).as_deref(), Some("g2/subtask:list#2"));
    assert_eq!(phase_hint(&Progress::ScaffoldDone), None);
}

#[test]
fn run_end_line_summarises_counts() {
    let (timing, lines) = capture();
    let llm = ScriptedLlm(vec![Ok(LlmChunk::Text("x".into()))]);
    let _ = block_on(timing.llm(&llm).call(request()).collect::<Vec<_>>());
    drop(timing);
    let log = joined(&lines);
    assert!(log.starts_with("[TIMING] +0.0s run start"), "{log}");
    assert!(log.contains("run end total="), "{log}");
    assert!(log.contains("llm-calls=1 inserts=0"), "{log}");
}

#[test]
fn preview_flattens_and_truncates() {
    assert_eq!(preview("a\nb"), "a b");
    let long = "字".repeat(PREVIEW_CHARS + 5);
    let out = preview(&long);
    assert_eq!(out.chars().count(), PREVIEW_CHARS + 1);
    assert!(out.ends_with('…'));
}
