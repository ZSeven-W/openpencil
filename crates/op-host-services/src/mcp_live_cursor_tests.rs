//! Agent-cursor / indicator tests for the live-MCP pump — the
//! `client_write` presence path exercised end-to-end through
//! `McpLiveServer::pump`: `batch_design` sweeps, targeted-write focus,
//! epoch coalescing, page-switch guards and wire-level client identity.
//! Split out of `mcp_live_tests.rs` (the shared `fresh_mcp_server`
//! fixture stays there, reached via `super::tests::`) to keep both
//! files under the 800-line cap; still a child module of `mcp_live`,
//! so `super::` paths resolve unchanged.

use super::tests::fresh_mcp_server;
use super::*;

use jian_ops_schema::node::PenNode;
use op_editor_core::pen_node_ext::PenNodeExt;

/// Minimal in-memory `Read + Write` stream for driving `serve_connection`
/// directly — the same shape `admission_tests` uses for its wire tests.
struct MockStream {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl std::io::Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.input, buf)
    }
}

impl std::io::Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A one-frame-one-child subtree, JSON-built (same shape as
/// `design_agent_tools.rs`'s own `batch_design` reveal test) so the
/// insert produces both a fresh top-level root AND a fresh
/// descendant — the frame tag and the reveal path are two distinct
/// code branches in `register_mcp_generation` and this exercises both.
fn root_frame_with_child(root_id: &str, child_id: &str) -> PenNode {
    serde_json::from_str(&format!(
            r#"{{"type":"frame","id":"{root_id}","name":"Root","width":200,"height":200,
                "children":[{{"type":"rectangle","id":"{child_id}","name":"Box","width":50,"height":50}}]}}"#
        ))
        .expect("valid frame json")
}

fn send_batch_design_insert(req_tx: &Sender<UiRequest>, node: PenNode) -> mpsc::Receiver<ApplyAck> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "batch_design".to_string(),
            // `InsertAuthoredSubtree`, not `InsertSubtree` — the
            // latter remaps every incoming id to a fresh editor id
            // (`command.rs`'s own doc: "the caller's ids are
            // structural placeholders only"), which would make this
            // test's assertions target whatever id landed rather
            // than the one it authored. Doesn't change what's under
            // test: `register_mcp_generation` diffs ids generically
            // and doesn't care which `EditorCommand` variant ran.
            cmd: EditorCommand::InsertAuthoredSubtree {
                nodes: vec![node],
                parent_id: op_editor_core::NodeId::NONE,
                page_id: None,
            },
            ack: ack_tx,
        })
        .expect("queue batch_design apply");
    ack_rx
}

#[test]
fn batch_design_apply_populates_the_scan_gate_frames_and_reveals() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    let ack_rx = send_batch_design_insert(&req_tx, root_frame_with_child("root-a", "box-a"));
    let outcome = server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));
    assert!(outcome.repaint);

    // The canvas radar-scan (`canvas_generation_scan::generating_paint_sets`)
    // gates purely on `AgentIndicators.frames` being non-empty — this is
    // the exact fact the bug report needed and the CLI-provider path
    // already has covered (`chat_intent_host_tests.rs`'s
    // `cli_new_design_populates_frame_indicators_the_canvas_scan_gates_on`);
    // this is the MCP-tool-driven counterpart.
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(
        snapshot.frames.contains_key("root-a"),
        "the new top-level root must be tagged: {:?}",
        snapshot.frames
    );
    assert!(
        snapshot.reveals.contains_key("root-a") && snapshot.reveals.contains_key("box-a"),
        "both the new root and its new child must have entrance reveals: {:?}",
        snapshot.reveals
    );

    op_editor_core::agent_indicators::clear();
}

#[test]
fn batch_design_apply_is_a_noop_when_nothing_new_landed() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    // A `batch_design` call whose command carries an EMPTY subtree —
    // shaped like a Direct-op property tweak that inserts nothing new.
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "batch_design".to_string(),
            cmd: EditorCommand::InsertSubtree {
                nodes: vec![],
                parent_id: op_editor_core::NodeId::NONE,
                page_id: None,
            },
            ack: ack_tx,
        })
        .expect("queue batch_design apply");
    let _ = server.pump(&mut state);
    let _ = ack_rx.try_recv();

    assert_eq!(
        op_editor_core::agent_indicators::snapshot().frames.len(),
        0,
        "an apply that produced no new content must not begin an epoch at all"
    );
    assert_eq!(
        server.write_indicators.lock().unwrap().last_epoch(),
        0,
        "no epoch was ever minted"
    );

    op_editor_core::agent_indicators::clear();
}

/// Locks in Track "batch_design 单批自成一轮 + 漂移窗口合并" — the whole
/// value of that design over the simpler "every call gets its own
/// fresh epoch" alternative: a second `batch_design` apply that lands
/// while the FIRST one's reveal queue is still draining must extend
/// the same epoch, not `begin()` a fresh one — `begin()` unconditionally
/// clears every map (`AgentIndicators::clear_maps`), so if this
/// coalescing didn't happen the first root's frame tag would be wiped
/// the instant the second call landed.
#[test]
fn back_to_back_batch_design_calls_coalesce_into_one_epoch_while_reveals_drain() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    let first_ack = send_batch_design_insert(&req_tx, root_frame_with_child("root-a", "box-a"));
    server.pump(&mut state);
    assert!(first_ack.try_recv().is_ok_and(|ack| ack.applied));
    let epoch_after_first = server.write_indicators.lock().unwrap().last_epoch();
    assert_ne!(epoch_after_first, 0, "first call must have minted an epoch");
    // The first call's own reveal queue must still be mid-flight for
    // this test to prove anything — it always is here (`REVEAL_DURATION_MS`
    // is a full second; this whole test runs in microseconds).
    assert!(
        op_editor_core::agent_indicators::latest_reveal_end_ms(epoch_after_first)
            .is_some_and(|end_ms| reveal_now_millis_for_test() < end_ms),
        "test precondition: the first batch's reveal queue must still be draining"
    );

    // Second call, different root, arriving inside that drain window.
    let second_ack = send_batch_design_insert(&req_tx, root_frame_with_child("root-b", "box-b"));
    server.pump(&mut state);
    assert!(second_ack.try_recv().is_ok_and(|ack| ack.applied));

    assert_eq!(
        server.write_indicators.lock().unwrap().last_epoch(),
        epoch_after_first,
        "the second call must REUSE the first call's still-draining epoch, not begin() a fresh one"
    );
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(
        snapshot.frames.contains_key("root-a") && snapshot.frames.contains_key("root-b"),
        "both roots must coexist in the SAME registry snapshot — if the second \
             call had begun a fresh epoch instead, begin()'s clear_maps() would have \
             wiped root-a's tag the instant root-b's call landed: {:?}",
        snapshot.frames
    );
    assert!(
        snapshot.reveals.contains_key("box-a") && snapshot.reveals.contains_key("box-b"),
        "both batches' reveals must coexist too: {:?}",
        snapshot.reveals
    );

    op_editor_core::agent_indicators::clear();
}

/// A targeted write — the `update_node` / `set_fill` shape every MCP
/// tool that tweaks ONE existing node shares — must land cursor focus,
/// the breathing border and the client badge on that node WITHOUT the
/// scale-pop entrance replay a fresh node gets. The reveal entry IS
/// registered (it is what anchors the cursor waypoint), but its start
/// is back-dated past the entrance windows.
#[test]
fn targeted_write_focuses_the_node_with_a_back_dated_reveal() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();
    // Land an existing node directly on the state — the pump's own
    // before/after diff then sees it as PRE-EXISTING, so nothing about
    // this write can qualify for the new-node reveal path.
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".to_string(),
        name: "T".to_string(),
        x: 0,
        y: 0,
        width: 40,
        height: 40,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));
    let nid = state
        .active_children()
        .last()
        .expect("the setup insert landed a root")
        .id_str()
        .to_string();

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "update_node".to_string(),
            cmd: EditorCommand::SetNodeFillHex {
                node_id: op_editor_core::NodeId::new(&nid),
                hex: "#123456".to_string(),
            },
            ack: ack_tx,
        })
        .expect("queue update_node apply");
    server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));

    let snapshot = op_editor_core::agent_indicators::snapshot();
    let tag = snapshot
        .nodes
        .get(&nid)
        .expect("the touched node must carry the client's badge tag");
    assert_eq!(tag.name, "MCP Client", "no initialize → honest fallback");
    assert_eq!(tag.color, "#8A8F98", "neutral MCP color");
    let started = *snapshot
        .reveals
        .get(&nid)
        .expect("a reveal entry anchors the cursor waypoint");
    let now = reveal_now_millis_for_test();
    assert!(
        now.saturating_sub(started) >= FOCUS_REVEAL_LOOKBACK_MS,
        "the waypoint's start must sit in the past so no entrance pop replays: \
             started={started} now={now}"
    );
    // The other half of the contract: the back-date must stay INSIDE the
    // live reveal window or the first paint-side prune drops the anchor
    // and the cursor never lands. `snapshot_at` is the pruned view the
    // paint path reads — `snapshot()` is the raw clone.
    let painted = op_editor_core::agent_indicators::snapshot_at(now);
    assert!(
        painted.reveals.contains_key(&nid),
        "the parked waypoint must survive the live-window prune"
    );
    assert!(
        snapshot.frames.is_empty(),
        "a targeted edit tags no generation root: {:?}",
        snapshot.frames
    );
    assert_eq!(
        snapshot.cursor_agent.as_ref().map(|tag| tag.name.as_str()),
        Some("MCP Client"),
        "the badge names the (fallback) MCP client"
    );

    op_editor_core::agent_indicators::clear();
}

/// A write that never reached the document — collab-refused or a
/// no-op apply — and a command that touches no node at all must mint
/// NO epoch and leave the registry empty: indicators describe work
/// that visibly LANDED.
#[test]
fn failed_and_nodeless_writes_register_no_indicators() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    // 1. Apply fails outright — the node does not exist.
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "update_node".to_string(),
            cmd: EditorCommand::SetNodeFillHex {
                node_id: op_editor_core::NodeId::new("ghost"),
                hex: "#123456".to_string(),
            },
            ack: ack_tx,
        })
        .expect("queue failing apply");
    server.pump(&mut state);
    assert!(
        ack_rx.try_recv().is_ok_and(|ack| !ack.applied),
        "a write against a missing node must report not-applied"
    );
    assert!(
        op_editor_core::agent_indicators::snapshot()
            .reveals
            .is_empty(),
        "a failed apply registers nothing"
    );

    // 2. Apply succeeds but no node is in the command's blast radius
    //    — a page switch flips WHICH tree is active; it must not read
    //    as an all-new reveal sweep or a focus target.
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "set_active_page".to_string(),
            cmd: EditorCommand::SetActivePage { index: 0 },
            ack: ack_tx,
        })
        .expect("queue set_active_page apply");
    server.pump(&mut state);
    let _ = ack_rx.try_recv();
    assert_eq!(
        server.write_indicators.lock().unwrap().last_epoch(),
        0,
        "no successful-but-nodeless write may mint an epoch"
    );
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(snapshot.nodes.is_empty() && snapshot.reveals.is_empty());

    op_editor_core::agent_indicators::clear();
}

/// The client-declared `initialize` name flows through the shared
/// session into the badge — the connection thread records it with
/// `note_client_name`; the pump stamps it on every tag the write lands.
#[test]
fn declared_client_name_lands_on_the_write_badges() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".to_string(),
        name: "T".to_string(),
        x: 0,
        y: 0,
        width: 40,
        height: 40,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));
    let nid = state
        .active_children()
        .last()
        .expect("setup insert")
        .id_str()
        .to_string();
    server
        .write_indicators
        .lock()
        .unwrap()
        .note_client_name(Some("cursor-agent".to_string()));

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "update_node".to_string(),
            cmd: EditorCommand::SetNodeFillHex {
                node_id: op_editor_core::NodeId::new(&nid),
                hex: "#123456".to_string(),
            },
            ack: ack_tx,
        })
        .expect("queue apply");
    server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));

    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert_eq!(
        snapshot.nodes.get(&nid).map(|tag| tag.name.as_str()),
        Some("cursor-agent"),
        "the badge must name the client that declared itself"
    );

    op_editor_core::agent_indicators::clear();
}

/// A `Batch` that CREATES a node and targets the same authored id —
/// `batch_design`'s "author the shell, then fill it" shape — must keep
/// the staggered entrance reveal. The focus path's back-dated start
/// would silently overwrite it (`add_reveal` is a map insert).
#[test]
fn a_node_created_and_touched_in_one_batch_keeps_its_staggered_reveal() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "batch_design".to_string(),
            cmd: EditorCommand::Batch {
                commands: vec![
                    EditorCommand::InsertAuthoredSubtree {
                        nodes: vec![root_frame_with_child("shell-x", "box-x")],
                        parent_id: op_editor_core::NodeId::NONE,
                        page_id: None,
                    },
                    EditorCommand::SetNodeFillHex {
                        node_id: op_editor_core::NodeId::new("shell-x"),
                        hex: "#123456".to_string(),
                    },
                ],
            },
            ack: ack_tx,
        })
        .expect("queue batch apply");
    server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));

    let snap = op_editor_core::agent_indicators::snapshot();
    let started = *snap
        .reveals
        .get("shell-x")
        .expect("the authored root keeps an entrance reveal");
    let now = reveal_now_millis_for_test();
    assert!(
        now.saturating_sub(started) < FOCUS_REVEAL_LOOKBACK_MS,
        "the staggered start was kept — the focus back-date would sit exactly \
             {FOCUS_REVEAL_LOOKBACK_MS}ms in the past: started={started} now={now}"
    );
    assert!(
        !snap.nodes.contains_key("shell-x"),
        "a just-created node takes no focus badge"
    );
    op_editor_core::agent_indicators::clear();
}

/// The declared `initialize` name must travel the REAL wire —
/// `serve_connection`'s stateless arm — into the session the pump tags
/// badges from. Session-level `note_client_name` coverage cannot catch
/// a miswire here.
#[test]
fn a_wire_initialize_name_reaches_the_write_badges() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let session = Arc::new(Mutex::new(ClientWriteIndicators::default()));
    let mut server = McpLiveServer {
        port: 0,
        token: "t".to_string(),
        quit_flag: Arc::new(AtomicBool::new(false)),
        req_rx,
        stop_tx,
        write_indicators: Arc::clone(&session),
        pending_design_md: VecDeque::new(),
    };
    let mut state = EditorState::new();
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".to_string(),
        name: "T".to_string(),
        x: 0,
        y: 0,
        width: 40,
        height: 40,
        fill_hex: None,
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));
    let nid = state
        .active_children()
        .last()
        .expect("setup insert")
        .id_str()
        .to_string();

    let admission = LiveAdmission::new("tok".to_string(), 51234);
    let stateful_lock = Mutex::new(());
    let quit_flag = AtomicBool::new(false);
    let wake_ui: UiWake = Arc::new(|| {});
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"cursor-agent","version":"1.0"}}}"#;
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:51234\r\nX-OpenPencil-Token: tok\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.into_bytes()),
        output: Vec::new(),
    };
    serve_connection(
        &mut stream,
        &req_tx,
        &admission,
        &stateful_lock,
        &quit_flag,
        &wake_ui,
        &session,
    )
    .expect("initialize answered");

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "update_node".to_string(),
            cmd: EditorCommand::SetNodeFillHex {
                node_id: op_editor_core::NodeId::new(&nid),
                hex: "#123456".to_string(),
            },
            ack: ack_tx,
        })
        .expect("queue apply");
    server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));

    let snap = op_editor_core::agent_indicators::snapshot();
    assert_eq!(
        snap.nodes.get(&nid).map(|tag| tag.name.as_str()),
        Some("cursor-agent"),
        "the wire-declared name must reach the badge"
    );
    op_editor_core::agent_indicators::clear();
}

/// A write that swaps WHICH page is active — a `Batch` carrying
/// `SetActivePage`, a `delete_page` on the live page, an undo that
/// restores a different index — must not diff the newly-active tree as
/// fresh content: every node there is pre-existing, so the sweep +
/// frames would paint over work the write never did.
#[test]
fn a_page_switch_does_not_sweep_the_newly_active_page() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();
    // Page 0 gets a node; a second page is appended (becomes active)
    // with a pre-existing child of its own; then flip back to page 0 so
    // the write under test can switch AWAY from a populated page.
    assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![serde_json::from_str(
            r#"{"type":"rectangle","id":"p0-node","name":"A","x":0,"y":0,"width":10,"height":10}"#
        )
        .unwrap()],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    }));
    assert!(state.apply(EditorCommand::AddPage {
        name: None,
        children: Some(vec![serde_json::from_str(
            r#"{"type":"rectangle","id":"p1-node","name":"B","x":0,"y":0,"width":10,"height":10}"#
        )
        .unwrap()]),
    }));
    assert!(state.apply(EditorCommand::SetActivePage { index: 0 }));
    op_editor_core::agent_indicators::clear();

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "set_active_page".to_string(),
            cmd: EditorCommand::Batch {
                commands: vec![EditorCommand::SetActivePage { index: 1 }],
            },
            ack: ack_tx,
        })
        .expect("queue page-switch apply");
    server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));

    let snap = op_editor_core::agent_indicators::snapshot();
    assert!(
        snap.reveals.is_empty() && snap.frames.is_empty() && snap.nodes.is_empty(),
        "the newly-active page's pre-existing tree must not read as fresh content: {snap:?}"
    );
    assert_eq!(
        server.write_indicators.lock().unwrap().last_epoch(),
        0,
        "a navigation write mints no epoch"
    );
    op_editor_core::agent_indicators::clear();
}

/// A `Batch` touching several existing nodes paints the border on each
/// but anchors the cursor with ONE back-dated reveal — a second
/// due-in-window entry counts as a burst at the next paint and the
/// edited nodes would replay the entrance pop.
#[test]
fn a_multi_target_write_paints_all_borders_with_one_anchor() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();
    for id in ["a", "b"] {
        assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
            nodes: vec![serde_json::from_str(&format!(
                r#"{{"type":"rectangle","id":"{id}","name":"n{id}","x":0,"y":0,"width":10,"height":10}}"#
            ))
            .unwrap()],
            parent_id: op_editor_core::NodeId::NONE,
            page_id: None,
        }));
    }

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "update_nodes".to_string(),
            cmd: EditorCommand::Batch {
                commands: vec![
                    EditorCommand::SetNodeFillHex {
                        node_id: op_editor_core::NodeId::new("a"),
                        hex: "#111111".to_string(),
                    },
                    EditorCommand::SetNodeFlag {
                        node_id: op_editor_core::NodeId::new("b"),
                        flag: op_editor_core::NodeFlag::Hidden,
                        value: true,
                    },
                ],
            },
            ack: ack_tx,
        })
        .expect("queue multi-touch apply");
    server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));

    let snap = op_editor_core::agent_indicators::snapshot();
    assert!(snap.nodes.contains_key("a") && snap.nodes.contains_key("b"));
    assert_eq!(
        snap.reveals.len(),
        1,
        "a second back-dated reveal would reschedule into pops"
    );
    op_editor_core::agent_indicators::clear();
}

/// `DeleteNode` reports its target as touched, but the node is gone
/// after apply — no badge or reveal may anchor on it.
#[test]
fn a_deleted_target_gets_no_presence() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();
    assert!(state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![serde_json::from_str(
            r#"{"type":"rectangle","id":"doomed","name":"D","x":0,"y":0,"width":10,"height":10}"#
        )
        .unwrap()],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    }));

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "delete_node".to_string(),
            cmd: EditorCommand::DeleteNode {
                node_id: op_editor_core::NodeId::new("doomed"),
                page_id: None,
            },
            ack: ack_tx,
        })
        .expect("queue delete apply");
    server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));

    let snap = op_editor_core::agent_indicators::snapshot();
    assert!(
        !snap.nodes.contains_key("doomed") && !snap.reveals.contains_key("doomed"),
        "a node the canvas can no longer paint gets no presence: {snap:?}"
    );
    assert_eq!(server.write_indicators.lock().unwrap().last_epoch(), 0);
    op_editor_core::agent_indicators::clear();
}

fn reveal_now_millis_for_test() -> u64 {
    crate::design_agent_tools::reveal_now_millis()
}
