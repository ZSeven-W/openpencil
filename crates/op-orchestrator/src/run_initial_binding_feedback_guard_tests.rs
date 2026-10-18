use super::*;
use futures::StreamExt;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

struct SnapshotSink {
    before: EditorState,
    changed: EditorState,
    show_changed: Arc<AtomicBool>,
    applied: Vec<EditorCommand>,
}

impl DocSink for SnapshotSink {
    fn state(&self) -> &EditorState {
        if self.show_changed.load(Ordering::SeqCst) {
            &self.changed
        } else {
            &self.before
        }
    }

    fn apply(&mut self, command: EditorCommand) -> bool {
        self.applied.push(command);
        false
    }

    fn begin_undo_batch(&mut self) {}
    fn end_undo_batch(&mut self) {}
}

struct ChangeDuringResponse {
    inner: ScriptedLlm,
    calls: AtomicUsize,
    change_on_call: usize,
    show_changed: Arc<AtomicBool>,
}

impl LlmClient for ChangeDuringResponse {
    fn call(&self, request: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let stream = self.inner.call(request);
        if call != self.change_on_call {
            return stream;
        }
        let show_changed = self.show_changed.clone();
        Box::pin(stream.inspect(move |_| show_changed.store(true, Ordering::SeqCst)))
    }
}

#[derive(Clone, Copy, Debug)]
enum SnapshotChange {
    Revision,
    Generation,
    ActivePage,
    Document,
}

fn changed_snapshot(before: &EditorState, change: SnapshotChange) -> EditorState {
    let mut changed = before.clone();
    match change {
        SnapshotChange::Revision => changed.mark_document_changed(),
        SnapshotChange::Generation => changed.replace_document(before.doc.clone()),
        SnapshotChange::ActivePage => changed.ui.active_page_index = 1,
        SnapshotChange::Document => {
            let mut document = serde_json::to_value(&changed.doc).unwrap();
            document["pages"][0]["children"][0]["name"] = json!("User renamed this root");
            changed.doc = serde_json::from_value(document).unwrap();
        }
    }
    // Each case isolates one guard, so another identity check cannot mask it.
    assert_eq!(
        changed.document_revision() != before.document_revision(),
        matches!(change, SnapshotChange::Revision)
    );
    assert_eq!(
        changed.document_generation() != before.document_generation(),
        matches!(change, SnapshotChange::Generation)
    );
    assert_eq!(
        changed.ui.active_page_index != before.ui.active_page_index,
        matches!(change, SnapshotChange::ActivePage)
    );
    assert_eq!(
        changed.doc != before.doc,
        matches!(change, SnapshotChange::Document)
    );
    changed
}

#[test]
fn changed_document_identity_rejects_initial_and_format_corrected_proposals() {
    for final_call in [1, 2] {
        for change in [
            SnapshotChange::Revision,
            SnapshotChange::Generation,
            SnapshotChange::ActivePage,
            SnapshotChange::Document,
        ] {
            let (original, plan, request, mut outcomes) = fixture();
            // Keep owner ids unique across the document so the binding check
            // reaches repair. The explicit error assertion proves the identity
            // guard rejects a page switch before original-node validation.
            let document = json!({"version":"1.0.0","children":[],"pages":[
                {"id":"first-page","name":"First","children":original.state.doc.children},
                {"id":"second-page","name":"Second","children":[]}
            ]});
            let before = EditorState::from_document(serde_json::from_value(document).unwrap());
            let changed = changed_snapshot(&before, change);
            let expected_doc = changed.doc.clone();
            let expected_revision = changed.document_revision();
            let expected_generation = changed.document_generation();
            let expected_page = changed.ui.active_page_index;
            let show_changed = Arc::new(AtomicBool::new(false));
            let mut sink = SnapshotSink {
                before,
                changed,
                show_changed: show_changed.clone(),
                applied: Vec::new(),
            };
            let mut responses = Vec::new();
            if final_call == 2 {
                responses.push(ScriptResponse::Text("{}".into()));
            }
            responses.push(ScriptResponse::Text(proposal(|_| {})));
            let llm = ChangeDuringResponse {
                inner: ScriptedLlm::new(responses),
                calls: AtomicUsize::new(0),
                change_on_call: final_call,
                show_changed: show_changed.clone(),
            };
            let (failed, _) = repair(
                &mut sink,
                &plan,
                &request,
                &mut outcomes,
                &llm,
                &AbortFlag::new(),
            );
            assert!(failed, "{change:?}, call {final_call}");
            assert_eq!(llm.inner.user_prompts().len(), final_call);
            assert!(show_changed.load(Ordering::SeqCst));
            assert!(sink.applied.is_empty(), "{change:?}, call {final_call}");
            assert_eq!(sink.state().doc, expected_doc);
            assert_eq!(sink.state().document_revision(), expected_revision);
            assert_eq!(sink.state().document_generation(), expected_generation);
            assert_eq!(sink.state().ui.active_page_index, expected_page);
            assert_eq!(outcomes[0].inserted_root_ids, vec!["original"]);
            assert_eq!(outcomes[0].node_count, 1);
            assert!(outcomes[0].subtask.is_none());
            let error = outcomes[0].error.as_deref().unwrap();
            assert!(error.contains(UNRESOLVED));
            assert!(
                error.contains("live document changed during correction"),
                "{error}"
            );
        }
    }
}

struct AbortDuringSecondResponse {
    inner: ScriptedLlm,
    calls: AtomicUsize,
    abort: AbortFlag,
    during_stream: bool,
}

impl LlmClient for AbortDuringSecondResponse {
    fn call(&self, request: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let stream = self.inner.call(request);
        if self.calls.fetch_add(1, Ordering::SeqCst) != 1 {
            return stream;
        }
        if self.during_stream {
            let abort = self.abort.clone();
            Box::pin(stream.inspect(move |_| abort.set()))
        } else {
            self.abort.set();
            stream
        }
    }
}

#[test]
fn abort_during_second_call_or_stream_never_commits_the_corrected_proposal() {
    for during_stream in [false, true] {
        let (mut sink, plan, request, mut outcomes) = fixture();
        let before = sink.state.doc.clone();
        let abort = AbortFlag::new();
        let llm = AbortDuringSecondResponse {
            inner: ScriptedLlm::new(vec![
                ScriptResponse::Text("{}".into()),
                ScriptResponse::Text(proposal(|_| {})),
            ]),
            calls: AtomicUsize::new(0),
            abort: abort.clone(),
            during_stream,
        };
        let (failed, _) = repair(&mut sink, &plan, &request, &mut outcomes, &llm, &abort);
        assert!(failed);
        assert!(abort.is_set());
        assert_eq!(llm.inner.user_prompts().len(), 2);
        assert_eq!(sink.state.doc, before);
        assert!(sink.applied.is_empty());
        assert_eq!(outcomes[0].inserted_root_ids, vec!["original"]);
        assert_eq!(outcomes[0].node_count, 1);
        assert!(outcomes[0].error.as_deref().unwrap().contains(UNRESOLVED));
    }
}
