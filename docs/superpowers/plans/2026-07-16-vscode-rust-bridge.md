# VSCode 插件 Rust 桥接工作包 — 实施计划（Plan 1/3，v2）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 VSCode 插件落地 spec（`docs/superpowers/specs/2026-07-16-vscode-extension-design.md` v2.3）中的「Rust 桥接工作包」：daemon 启动契约、loopback 鉴权、同步并发控制、wasm postMessage 桥、generation 作用域的保存确认。

**Architecture:** 在现有 crate 内做加法：`op-editor-core`（generation/revision + **纯逻辑同步状态机 SyncGate + 桥协议编解码**，两者放这里是因为该 crate 原生 target 可测，而 `op-host-web` 的 wasm 模块原生测试不编译）、`op-host-services`（daemon HTTP 层）、`op-host-web-server`（入口 flags + 集成测试）、`op-host-web`（DOM 接线，逻辑薄）。

**Tech Stack:** Rust（workspace 现有依赖，不新增外部 crate）；wasm 侧 web-sys/wasm-bindgen（已在用；需在 `op-host-web/Cargo.toml` 的 web-sys features 中补 `MessageEvent`）。

**Plan 系列:** Plan 1 = 本文（Rust 基座）；Plan 2 = TS 插件本体；Plan 3 = 打包/发布。

## Global Constraints

- 单文件 ≤ 800 行；`.rs` snake_case；源码注释英文。
- Conventional Commits：`<type>(<scope>): <subject>`，scope 用 `web` / `editor`。
- `op-editor-ui` 保持 wasm32-clean；web bundle ≤ 6 MiB gzip（`tools/check-wasm-bundle.sh`）。
- 每任务收尾：`cargo test -p <crate>` + `cargo clippy --workspace --all-targets -- -D warnings`。
- wasm 编译门（`op-host-web` 改动必跑）：`cargo check --target wasm32-unknown-unknown -p op-host-web --no-default-features --features canvaskit`——**注意 `web` feature 只是 wasm32-clean 桩，不编译 canvaskit/live_sync_glue/vscode_bridge/dom_io，检查必须用 `canvaskit`**；改动同时保持 `--features web` 桩编译通过（CI 基线）。
- **向后兼容不变量**：非 managed 模式（`op start --web`、desktop `--serve-web`、浏览器直开、`OPENPENCIL_MCP_TOKEN` shutdown 契约）行为一律不变。
- TDD：先写失败测试再实现；一个任务一个 commit。

---

### Task 1: editor-core — document generation + 参数化 mark_saved_revision

**Files:**
- Modify: `crates/op-editor-core/src/state.rs`（字段块 :70-80、构造 :149-151 与 :171 `new()`、`replace_document*` :198-226、`mark_saved_revision` :257）
- Test: `state.rs` 现有 `#[cfg(test)]`（超 800 行则拆 `state_generation_tests.rs` 兄弟文件）

**Interfaces:**
- Produces: `EditorState` 字段 `pub(crate) document_generation: u64`（初始 0；`replace_document` 与 `replace_document_with_undo` 各自增 1）
- Produces: `pub fn document_generation(&self) -> u64`
- Produces: `pub fn mark_saved_revision_at(&mut self, generation: u64, revision: u64) -> bool` — generation 不匹配返回 `false` 且不动状态；匹配则 `saved_revision = revision`、刷新 `editor_ui.document_dirty`、返回 `true`
- 现有无参 `mark_saved_revision()` 保留，改为委托 `mark_saved_revision_at(self.document_generation, self.revision)`
- Consumes: 现有 `EditorState::new()`（:171）、`mark_document_changed()`（:263，测试里用它触发 revision 递增）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn replace_document_bumps_generation() {
    let mut s = EditorState::new();
    assert_eq!(s.document_generation(), 0);
    s.replace_document(jian_ops_schema::PenDocument::default());
    assert_eq!(s.document_generation(), 1);
    s.replace_document_with_undo(jian_ops_schema::PenDocument::default());
    assert_eq!(s.document_generation(), 2);
}

#[test]
fn stale_generation_save_ack_is_rejected() {
    let mut s = EditorState::new();
    s.mark_document_changed(); // revision -> 1
    let (gen0, rev0) = (s.document_generation(), s.document_revision());
    s.replace_document(jian_ops_schema::PenDocument::default()); // generation 1, revision 0
    assert!(!s.mark_saved_revision_at(gen0, rev0)); // delayed pre-replace ack: dropped
    assert_eq!(s.saved_revision(), 0);
    assert!(s.mark_saved_revision_at(s.document_generation(), s.document_revision()));
}

#[test]
fn ack_for_older_revision_does_not_mark_newer_edits_clean() {
    let mut s = EditorState::new();
    s.mark_document_changed(); // revision 1 — snapshot exported here
    let snap = (s.document_generation(), s.document_revision());
    s.mark_document_changed(); // revision 2 — user kept editing during disk write
    assert!(s.mark_saved_revision_at(snap.0, snap.1));
    assert!(s.is_dirty()); // revision 2 != saved 1 — still dirty, correctly
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test -p op-editor-core generation`
Expected: FAIL（`document_generation` 不存在）

- [ ] **Step 3: 最小实现**

```rust
// field block (near revision_counter):
/// Monotonic document identity. Bumped by every `replace_document*`
/// (open / revert / sync replace), so `(generation, revision)` uniquely
/// names a state; a save-ack from a previous generation must be dropped.
pub(crate) document_generation: u64,
// init to 0 in every constructor (new / from_document).

// in replace_document() AND replace_document_with_undo(), next to the
// revision resets:
self.document_generation = self.document_generation.saturating_add(1);

pub fn document_generation(&self) -> u64 {
    self.document_generation
}

/// Mark `revision` saved iff `generation` still names the live document.
pub fn mark_saved_revision_at(&mut self, generation: u64, revision: u64) -> bool {
    if generation != self.document_generation {
        return false;
    }
    self.saved_revision = revision;
    self.editor_ui.document_dirty = self.is_dirty();
    true
}

pub fn mark_saved_revision(&mut self) {
    let (g, r) = (self.document_generation, self.revision);
    let _ = self.mark_saved_revision_at(g, r);
}
```

- [ ] **Step 4: 跑全量测试** — Run: `cargo test -p op-editor-core`；Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/op-editor-core/src/state.rs
git commit -m "feat(editor): scope saved-revision acks to a document generation"
```

---

### Task 2: editor-core — SyncGate 纯逻辑状态机 + 桥协议编解码

（新任务：把 v1 计划里散在 Task 5/6 的判定逻辑收拢为原生可测的纯模块，解决「wasm 模块原生测试不编译」和「take_doc_sync_dirty 清位竞态」两个 review 缺陷。）

**Files:**
- Create: `crates/op-editor-core/src/sync_gate.rs`
- Create: `crates/op-editor-core/src/bridge_protocol.rs`
- Modify: `crates/op-editor-core/src/lib.rs`（`pub mod sync_gate; pub mod bridge_protocol;`）
- Modify: `crates/op-editor-core/src/web_sync.rs`（`WebSyncClient` 增 `last_version()`、`wrap_push_body_with_base`、`parse_push_conflict`）
- Test: 各文件底部 `#[cfg(test)]`

**Interfaces:**
- Produces（`sync_gate.rs`）——基线不是布尔 latch，而是**已同步的 `(generation, revision)` 对**（复用 Task 1 的计数器；revision 是 content-only 的——`state.rs:70` 注释明确 UI/选区/视口不递增——所以「UI 变化误关门」的死锁天然不存在，且「推送 A 飞行中出现编辑 B」时确认只落在 A 序列化时的 revision 上，B 仍判定未推送）：

```rust
/// Pure sync-concurrency state machine shared by the periodic push tick,
/// the pull tick, and the bridge snapshot path. Baseline semantics: the
/// gate remembers the exact (generation, revision) pair last known to
/// match the daemon (after a pull apply, or after a push confirmation —
/// carrying the pair captured AT SERIALIZATION TIME, never "now"). Any
/// current pair differing from the baseline means unpushed local edits:
/// pulls are gated and a push is due. An oversize skip, failed POST, or
/// version conflict simply never advances the baseline, so the gate
/// cannot reopen and remote state cannot overwrite local edits.
#[derive(Default)]
pub struct SyncGate {
    last_synced: Option<(u64, u64)>, // (generation, revision); None = never synced
    conflict: Option<u64>,           // server version at conflict time
    // `opened` not yet emittable; holds the open's TARGET GENERATION —
    // open completion is generation-scoped so a pre-existing in-flight
    // push (whose confirmation carries a pair serialized BEFORE the open's
    // replace_document, i.e. an older generation) can never complete an
    // unrelated pending open prematurely.
    open_pending: Option<u64>,
    open_pull_block: bool, // pulls blocked while the open's probe/push is in flight
    // AcceptRemote resolution records the pair observed at resolve time AND
    // retains the conflict's server version; the resolving pull is allowed
    // ONLY while the pair is unchanged, so edits landing between backup and
    // pull-apply cannot be overwritten — and a broken window can re-enter
    // the conflict flow with the retained version (no wedged state).
    accept_expected: Option<((u64, u64), u64)>, // (expected pair, server version)
}

impl SyncGate {
    /// After a pull apply (new baseline = post-apply pair) or a push
    /// confirmation (baseline = the pair serialized into that push).
    /// Clears conflict + accept_expected + any UNDRAINED conflict latch
    /// (a resolved conflict must not be announced); clears
    /// open_pending/open_pull_block ONLY when `generation >= open target
    /// generation` (older confirmations leave the pending open untouched).
    pub fn note_synced(&mut self, generation: u64, revision: u64);
    pub fn note_conflict(&mut self, server_version: u64); // also clears accept_expected
    /// Set SYNCHRONOUSLY in the OpenDocument prologue, right after
    /// replace_document minted the new generation and BEFORE the first
    /// await (sets open_pending = Some(target_generation) + open_pull_block):
    /// during bootstrap the baseline is None so pull_allowed would be true —
    /// without the block a 400 ms pull can overwrite the just-replaced local
    /// document while the open's probe/push is in flight.
    pub fn note_open_pending(&mut self, target_generation: u64);
    pub fn open_pending(&self) -> bool;
    /// Accept-remote resolution: forget the baseline, clear the conflict
    /// AND any undrained conflict latch (retaining the server version
    /// inside accept_expected), lift open_pull_block, and record `current`
    /// as the expected pair for the resolving pull — but KEEP open_pending:
    /// it clears when that pull's apply calls note_synced (bridge's cue to
    /// emit `opened`).
    pub fn resolve_accept_remote(&mut self, current: (u64, u64));
    /// Wiring escape hatch for a broken accept window: returns the retained
    /// server version when accept_expected is set and `current` moved past
    /// it (an edit landed before the resolving pull applied). The pull tick
    /// checks this; Some(v) means "re-enter the conflict flow": call
    /// note_conflict(v) (which clears accept_expected) so the user decides
    /// again — the gate can never wedge at baseline=None/conflict=None.
    pub fn accept_window_broken(&self, current: (u64, u64)) -> Option<u64>;
    /// CONSUMABLE edge latches — the bridge's tick observer drains these
    /// instead of comparing cached values (a compare-based observer misses
    /// edges that rise AND fall between ticks, e.g. a fast open completing
    /// before the next tick; a latch cannot lose the event):
    /// set by note_synced when it clears a pending open (holds the open's
    /// generation) — drain to emit `opened`.
    pub fn take_opened_edge(&mut self) -> Option<u64>;
    /// set by note_conflict (holds the server version) — drain to emit
    /// `sync-conflict`. CLEARED by note_synced and resolve_accept_remote:
    /// a conflict resolved before the observer drained the latch must NOT
    /// be announced (stale emission would pop a ghost dialog on the host).
    /// A back-to-back second conflict simply sets the latch again after
    /// the resolution cleared it — still reported.
    pub fn take_conflict_edge(&mut self) -> Option<u64>;
    /// Pulls allowed when open_pull_block is clear, no conflict pending,
    /// AND no unpushed local edits. None baseline (mount) allows the
    /// initial pull — EXCEPT when accept_expected is set: then the pull is
    /// allowed only while `current == accept_expected` (an intervening edit
    /// moves the pair; the wiring observes the blocked pull and re-notes
    /// the conflict so the user decides again).
    pub fn pull_allowed(&self, current: (u64, u64)) -> bool;
    /// Push due when NO conflict is pending (a stored conflict suspends the
    /// periodic retry until explicit resolution — spec's "no silent retry"),
    /// a baseline exists (daemon-authority: never push before the first
    /// apply), and the current pair moved past it.
    pub fn needs_push(&self, current: (u64, u64)) -> bool;
    pub fn conflict(&self) -> Option<u64>;
    pub fn periodic_push_allowed(len: usize) -> bool; // len <= 2 MiB cap
    pub fn snapshot_push_allowed(_len: usize) -> bool { true } // uncapped
}
```

- Produces（`bridge_protocol.rs`）——协议编解码，serde 实现（`op-editor-core` 已依赖 serde/serde_json；不再手写扫描器，review 要求可靠解析）：

```rust
#[derive(Debug, PartialEq)]
pub enum BridgeInbound {
    Init { token: String },
    OpenDocument { json: String },
    Snapshot { purpose: String, request_id: String },
    SaveCommitted { generation: u64, revision: u64 },
    ResolveConflict { mode: ConflictMode, request_id: String },
}
#[derive(Debug, PartialEq)]
pub enum ConflictMode { UseLocal, AcceptRemote }

impl BridgeInbound {
    /// None for non-bridge / malformed messages (foreign postMessage traffic
    /// like react-devtools must be ignored, never an error).
    pub fn parse(raw: &str) -> Option<Self>;
}

pub fn event_ready(generation: u64, revision: u64) -> String;
pub fn event_dirty_changed(generation: u64, revision: u64, dirty: bool) -> String;
pub fn event_opened(generation: u64) -> String;
pub fn event_snapshot_result(request_id: &str, doc_json: &str, generation: u64, revision: u64) -> String;
pub fn event_snapshot_conflict(request_id: &str, server_version: u64) -> String;
pub fn event_sync_conflict(generation: u64, revision: u64, server_version: u64) -> String;
pub fn event_conflict_resolved(request_id: &str) -> String;
```

  消息 `type` 值：`op-bridge/init`、`op-bridge/open-document`、`op-bridge/snapshot`、`op-bridge/save-committed`、`op-bridge/resolve-conflict`（入站）；`op-bridge/ready`、`op-bridge/dirty-changed`、`op-bridge/opened`、`op-bridge/snapshot-result`、`op-bridge/snapshot-conflict`、`op-bridge/sync-conflict`、`op-bridge/conflict-resolved`（出站）。字段命名 camelCase（`requestId`、`serverVersion`、`docJson`）。
- Produces（`web_sync.rs`）：`pub fn last_version(&self) -> u64`；`pub fn wrap_push_body_with_base(doc_json: &str, base_version: u64) -> String`（在现有 `wrap_push_body` 的 JSON 上加 `"baseVersion":N`）；`pub fn parse_push_conflict(resp: &str) -> Option<u64>`（`version-conflict` body → 服务端版本）。
- Consumes: 无（纯新增）。

- [ ] **Step 1: 写失败测试**

```rust
// sync_gate.rs
#[test]
fn edit_during_inflight_push_stays_gated() {
    let mut g = SyncGate::default();
    assert!(g.pull_allowed((1, 0))); // mount: initial pull allowed
    g.note_synced(1, 0);             // first apply baselines
    // edit A -> revision 1: push due, pull gated
    assert!(g.needs_push((1, 1)));
    assert!(!g.pull_allowed((1, 1)));
    // push A in flight; edit B bumps revision to 2; A's confirmation
    // carries the pair captured at serialization time (1, 1):
    g.note_synced(1, 1);
    assert!(!g.pull_allowed((1, 2))); // B still unpushed — gate stays closed
    assert!(g.needs_push((1, 2)));
    g.note_synced(1, 2); // push B confirmed
    assert!(g.pull_allowed((1, 2)));
}

#[test]
fn ui_only_changes_do_not_close_the_gate() {
    // revision is content-only (state.rs): an unchanged pair keeps pulls open
    let mut g = SyncGate::default();
    g.note_synced(1, 5);
    assert!(g.pull_allowed((1, 5)));
    assert!(!g.needs_push((1, 5))); // no deadlock: nothing to push, gate open
}

#[test]
fn never_pushes_before_first_sync() {
    let g = SyncGate::default();
    assert!(!g.needs_push((0, 3))); // daemon authority preserved
}

#[test]
fn open_pending_gates_pulls_through_bootstrap() {
    let mut g = SyncGate::default();
    assert!(g.pull_allowed((1, 0))); // bootstrap: baseline None, pulls open
    g.note_open_pending(2);          // prologue: replace_document minted gen 2
    assert!(!g.pull_allowed((2, 0))); // pull may not overwrite the open
    g.note_synced(2, 0);             // open's own push confirmed
    assert!(!g.open_pending());
    assert!(g.pull_allowed((2, 0)));
}

#[test]
fn fast_open_completion_is_latched_not_lost() {
    // open rises AND completes between two observer ticks — a compare-based
    // observer would see false→false and drop `opened`; the latch cannot:
    let mut g = SyncGate::default();
    g.note_open_pending(2);
    g.note_synced(2, 0);
    assert_eq!(g.take_opened_edge(), Some(2)); // observer drains exactly once
    assert_eq!(g.take_opened_edge(), None);
}

#[test]
fn back_to_back_conflicts_both_latch() {
    let mut g = SyncGate::default();
    g.note_synced(1, 0);
    g.note_conflict(7);
    assert_eq!(g.take_conflict_edge(), Some(7));
    g.resolve_accept_remote((1, 1));
    g.note_conflict(8); // second conflict lands before any observer tick
    assert_eq!(g.take_conflict_edge(), Some(8)); // not lost
    assert_eq!(g.take_conflict_edge(), None);
}

#[test]
fn resolved_conflict_latch_is_not_announced() {
    // conflict latched, then resolved BEFORE the observer drained it —
    // a stale emission would pop a ghost dialog on the host:
    let mut g = SyncGate::default();
    g.note_synced(1, 0);
    g.note_conflict(7);
    g.resolve_accept_remote((1, 1)); // resolution clears the pending latch
    assert_eq!(g.take_conflict_edge(), None);

    g.note_conflict(9);
    g.note_synced(1, 2); // UseLocal-style completion also clears it
    assert_eq!(g.take_conflict_edge(), None);
}

#[test]
fn older_push_confirmation_cannot_complete_a_pending_open() {
    // Interleaving: a periodic push was in flight (serialized at gen 1)
    // when OpenDocument minted gen 2 and started waiting on push_busy.
    let mut g = SyncGate::default();
    g.note_synced(1, 3);             // steady state before the open
    g.note_open_pending(2);          // open targets generation 2
    g.note_synced(1, 4);             // the OLD push's confirmation lands
    assert!(g.open_pending());       // must NOT complete the open (gen 1 < 2)
    assert!(!g.pull_allowed((2, 0)));
    g.note_synced(2, 0);             // the open's own confirmation
    assert!(!g.open_pending());
}

#[test]
fn accept_remote_keeps_open_pending_until_the_pull_applies() {
    let mut g = SyncGate::default();
    g.note_open_pending(2);
    g.note_conflict(9);              // open's push conflicted twice
    g.resolve_accept_remote((2, 0));
    assert!(g.open_pending());       // still pending: opened not yet emittable
    assert!(g.pull_allowed((2, 0))); // the resolving pull may proceed
    g.note_synced(3, 0);             // remote applied (gen 3 >= target 2)
    assert!(!g.open_pending());      // bridge emits `opened` on this edge
}

#[test]
fn accept_remote_blocks_the_pull_if_an_edit_intervenes() {
    let mut g = SyncGate::default();
    g.note_synced(1, 1);
    g.note_conflict(9);
    g.resolve_accept_remote((1, 2)); // user accepted remote at pair (1,2)
    assert!(g.pull_allowed((1, 2))); // unchanged: pull may apply
    assert_eq!(g.accept_window_broken((1, 2)), None);
    // an edit lands BEFORE the pull applies — those bytes must not be lost:
    assert!(!g.pull_allowed((1, 3)));
    // the gate itself hands the wiring the retained server version so the
    // conflict flow can restart (no manual bookkeeping, no wedged state):
    assert_eq!(g.accept_window_broken((1, 3)), Some(9));
    g.note_conflict(9); // wiring re-enters the conflict flow
    assert_eq!(g.accept_window_broken((1, 3)), None); // window consumed
    assert!(!g.pull_allowed((1, 3)));
}

#[test]
fn generation_only_replacement_rebaselines_without_push() {
    // open-document with byte-identical content: generation moves, bytes
    // match the daemon. The wiring layer (Task 6) detects the byte match via
    // should_push and must re-baseline directly instead of pushing —
    // note_synced with the new pair reopens the gate:
    let mut g = SyncGate::default();
    g.note_synced(1, 0);
    assert!(g.needs_push((2, 0)));   // pair moved (generation bump)
    assert!(!g.pull_allowed((2, 0)));
    g.note_synced(2, 0);             // wiring re-baselines on identical bytes
    assert!(!g.needs_push((2, 0)));
    assert!(g.pull_allowed((2, 0))); // no deadlock
}

#[test]
fn oversize_or_failed_push_keeps_gate_closed() {
    let mut g = SyncGate::default();
    g.note_synced(1, 0);
    // edit -> (1,1); periodic tick skipped the push (oversize) — no
    // confirmation, baseline unchanged, gate must stay closed:
    assert!(!SyncGate::periodic_push_allowed(3 * 1024 * 1024));
    assert!(!g.pull_allowed((1, 1)));
    // snapshot channel is uncapped; its confirmation advances the baseline:
    assert!(SyncGate::snapshot_push_allowed(3 * 1024 * 1024));
    g.note_synced(1, 1);
    assert!(g.pull_allowed((1, 1)));
}

#[test]
fn conflict_holds_gate_until_explicit_resolution() {
    let mut g = SyncGate::default();
    g.note_synced(1, 0);
    g.note_conflict(12); // (1,1) push rejected
    assert_eq!(g.conflict(), Some(12));
    assert!(!g.pull_allowed((1, 1)));
    // A stored conflict SUSPENDS the periodic push — no silent auto-retry
    // on the next 2s tick; only explicit resolution may push again:
    assert!(!g.needs_push((1, 1)));
    g.resolve_accept_remote((1, 1));
    assert_eq!(g.conflict(), None);
    assert!(g.pull_allowed((1, 1))); // baseline forgotten: next pull re-baselines
}

// bridge_protocol.rs
#[test]
fn parse_inbound_messages() {
    assert_eq!(
        BridgeInbound::parse(r#"{"type":"op-bridge/init","token":"t0k"}"#),
        Some(BridgeInbound::Init { token: "t0k".into() })
    );
    assert_eq!(
        BridgeInbound::parse(
            r#"{"type":"op-bridge/save-committed","generation":3,"revision":41}"#
        ),
        Some(BridgeInbound::SaveCommitted { generation: 3, revision: 41 })
    );
    assert_eq!(
        BridgeInbound::parse(
            r#"{"type":"op-bridge/resolve-conflict","mode":"use-local","requestId":"r1"}"#
        ),
        Some(BridgeInbound::ResolveConflict { mode: ConflictMode::UseLocal, request_id: "r1".into() })
    );
    assert_eq!(BridgeInbound::parse(r#"{"type":"react-devtools"}"#), None);
    assert_eq!(BridgeInbound::parse("not json"), None);
}

#[test]
fn snapshot_result_json_escapes_doc_payload() {
    let ev = event_snapshot_result("r1", r#"{"a":"b \" c"}"#, 2, 17);
    let v: serde_json::Value = serde_json::from_str(&ev).unwrap();
    assert_eq!(v["type"], "op-bridge/snapshot-result");
    assert_eq!(v["docJson"], r#"{"a":"b \" c"}"#);
    assert_eq!(v["generation"], 2);
}

// web_sync.rs
#[test]
fn push_body_with_base_and_conflict_roundtrip() {
    let body = WebSyncClient::wrap_push_body_with_base(r#"{"pages":[]}"#, 7);
    assert!(body.contains(r#""baseVersion":7"#));
    assert_eq!(
        WebSyncClient::parse_push_conflict(
            r#"{"ok":false,"error":"version-conflict","version":12}"#
        ),
        Some(12)
    );
    assert_eq!(WebSyncClient::parse_push_conflict(r#"{"ok":true,"version":9}"#), None);
}
```

- [ ] **Step 2: 确认失败** — Run: `cargo test -p op-editor-core sync_gate && cargo test -p op-editor-core bridge_protocol && cargo test -p op-editor-core push_body`；Expected: FAIL

- [ ] **Step 3: 实现**（接口块即实现骨架；`BridgeInbound::parse` 用 `serde_json::Value` 按 `type` 分发，未知 type/缺字段返回 `None`；event_* 用 `serde_json::json!` 构造后 `to_string`）

- [ ] **Step 4: 跑测试** — Run: `cargo test -p op-editor-core`；Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/op-editor-core/src/sync_gate.rs crates/op-editor-core/src/bridge_protocol.rs \
        crates/op-editor-core/src/web_sync.rs crates/op-editor-core/src/lib.rs
git commit -m "feat(editor): sync gate state machine and bridge protocol codec"
```

---

### Task 3: daemon — managed 启动契约（握手 JSON + `--port 0` + stdin-EOF 自杀）

**Files:**
- Modify: `crates/op-host-services/src/web_canvas_server.rs`（`parse_serve_web_args` :1118、`run_web_canvas` :1199-1290）
- Modify: `crates/op-host-web-server/src/main.rs`（`--serve-web` 臂）
- Modify: `crates/op-host-desktop/src/mcp_serve.rs`（:59-70 的 `--serve-web` 调用点——**不是** main.rs）
- Test: `crates/op-host-services/src/web_canvas_server_tests.rs`

**Interfaces:**
- Produces: CLI 双语法（同一 parser 支持）：
  - 旧（不变）：`--serve-web <port> [doc] [--host <addr>]`
  - 新（managed，flag 风格，与 spec 一致）：`--serve-web --managed --port <n|0> [--file <path>] [--host <addr>] [--allow-origin <origin>]...`
- Produces: `pub struct ServeWebOptions { pub port: u16, pub path: Option<PathBuf>, pub host: String, pub managed: bool, pub allow_origins: Vec<String> }`；`parse_serve_web_args(args) -> Result<ServeWebOptions, String>`；`run_web_canvas(options: ServeWebOptions) -> Result<(), String>`。两个旧调用点（desktop `mcp_serve.rs:59`、web-server `main.rs`）改为构造 `ServeWebOptions { managed: false, allow_origins: vec![], .. }`。
- Produces: managed 模式行为——
  1. bind 后 `listener.local_addr()` 取实际端口，stdout 单行握手：`{"ok":true,"port":<actual>,"token":"<hex32>","version":"<CARGO_PKG_VERSION>"}`；
  2. **仅 managed 模式**生成 per-instance token（非 managed 分支完全不触碰现有 `OPENPENCIL_MCP_TOKEN` shutdown 契约，见 `web_canvas_server.rs:1464` 与 `web_canvas_server_tests.rs:1238` 现有测试）；
  3. stdin 读线程：EOF/错误 → 置现有 `shutdown` AtomicBool → 用 `TcpStream::connect(listener.local_addr())`（**绑定地址原样回连**，兼容 IPv6/自定义 host）唤醒 accept 循环。
- Consumes: 现有 `shutdown` flag accept 循环（:1237-1243）。

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn parse_serve_web_args_legacy_positional_unchanged() {
    let o = parse_serve_web_args(vec!["3100".into(), "doc.op".into()].into_iter()).unwrap();
    assert_eq!((o.port, o.managed), (3100, false));
    assert_eq!(o.path.as_deref(), Some(std::path::Path::new("doc.op")));
    assert_eq!(o.host, "127.0.0.1");
}

#[test]
fn parse_serve_web_args_managed_flag_form() {
    let o = parse_serve_web_args(
        vec![
            "--managed".into(), "--port".into(), "0".into(),
            "--file".into(), "a.op".into(),
            "--allow-origin".into(), "vscode-webview://x".into(),
            "--allow-origin".into(), "vscode-webview://y".into(),
        ]
        .into_iter(),
    )
    .unwrap();
    assert!(o.managed);
    assert_eq!(o.port, 0);
    assert_eq!(o.path.as_deref(), Some(std::path::Path::new("a.op")));
    assert_eq!(o.allow_origins.len(), 2);
}

#[test]
fn handshake_line_is_single_line_json() {
    let line = handshake_json(41234, "aabbccdd00112233aabbccdd00112233");
    assert!(!line.contains('\n'));
    assert!(line.contains(r#""port":41234"#));
    assert!(line.contains(r#""token":"aabbccdd00112233aabbccdd00112233""#));
}
```

- [ ] **Step 2: 确认失败** — Run: `cargo test -p op-host-services parse_serve_web`；Expected: FAIL（返回类型不匹配）

- [ ] **Step 3: 实现**

```rust
// parser: peek first arg — starts_with("--") => flag form; else legacy
// positional. Both fill ServeWebOptions.

pub(crate) fn handshake_json(port: u16, token: &str) -> String {
    format!(
        r#"{{"ok":true,"port":{port},"token":"{token}","version":"{}"}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

fn random_token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h1 = s.build_hasher();
    h1.write_u128(std::time::UNIX_EPOCH.elapsed().map(|d| d.as_nanos()).unwrap_or(0));
    let mut h2 = s.build_hasher();
    h2.write_u64(std::process::id() as u64);
    format!("{:016x}{:016x}", h1.finish(), h2.finish())
}

// run_web_canvas(options):
let listener = TcpListener::bind((options.host.as_str(), options.port))
    .map_err(|e| format!("bind {}:{}: {e}", options.host, options.port))?;
let local_addr = listener.local_addr().map_err(|e| e.to_string())?;
let managed_token = options.managed.then(random_token);
if let Some(token) = &managed_token {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", handshake_json(local_addr.port(), token));
    let _ = out.flush();
    let shutdown_stdin = Arc::clone(&shutdown);
    std::thread::spawn(move || {
        use std::io::Read;
        let mut sink = [0u8; 64];
        let mut stdin = std::io::stdin();
        while matches!(stdin.read(&mut sink), Ok(n) if n > 0) {}
        shutdown_stdin.store(true, Ordering::Release);
        let _ = std::net::TcpStream::connect(local_addr); // wake the accept loop
    });
}
// managed_token 与 options.allow_origins 存入跨线程 state（Task 4 消费）。
```

- [ ] **Step 4: 跑测试 + 调用点编译** — Run: `cargo test -p op-host-services && cargo check --workspace`；Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/op-host-services/src/web_canvas_server.rs \
        crates/op-host-services/src/web_canvas_server_tests.rs \
        crates/op-host-web-server/src/main.rs crates/op-host-desktop/src/mcp_serve.rs
git commit -m "feat(web): managed serve-web contract with handshake and parent-death lease"
```

---

### Task 4: daemon — serve_one 层鉴权 + CORS 白名单

**Files:**
- Modify: `crates/op-host-services/src/web_canvas_server.rs`（**`serve_one` :1319 分发层**——`handle_web_canvas_request` 无 headers、且 SSE/AI 流/`/mcp` 在其之前分支，鉴权必须放 serve_one）
- Modify: `crates/op-host-services/src/web_static.rs`（:290 CORS 头）
- Modify: `crates/op-host-services/src/mcp_serve.rs`（:395 MCP HTTP 响应 CORS 头）
- Modify: `crates/op-host-services/src/ai_proxy.rs`（其响应组装处的 CORS 头）
- Modify: `crates/op-host-services/src/web_chat_standard.rs`（:144 `write_sse_headers` 直发的宽松 SSE CORS 头——同样接 `cors_origin_for`）
- Test: `crates/op-host-services/src/web_canvas_server_tests.rs`

**Interfaces:**
- Produces: 鉴权规则（仅 managed 模式生效；非 managed 全放行）：请求头 `X-OpenPencil-Token: <token>`。**豁免 = `OPTIONS *`（预检）+ web_static 层实际拥有的全部静态 GET 路由**——`/`、`/index.html`、`/pkg/*`、`/smoke/*`、`/canvaskit/*`、`/assets/*`（见 `web_static.rs:188` 起的路由表；CanvasKit wasm/iconify 目录都在这里，漏了编辑器根本起不来）。**接线与豁免表必须一致**：鉴权检查插在 serve_one 的静态路由处理返回之后——凡 web_static 服务掉的天然豁免，`RequestAuth::allows` 的豁免分支即为上述清单，二者语义等价。**其余一切请求**（含 `POST /` JSON-RPC 别名、`/mcp`、`/api/*`、SSE `/api/mcp/events`、AI 流式端点）缺失/错误 token → `401 {"ok":false,"error":"unauthorized"}`。
- Produces: `pub(crate) struct RequestAuth { pub managed: bool, pub token: String }`，`pub(crate) fn allows(&self, method: &str, path: &str, presented: Option<&str>) -> bool`。
- Produces: CORS——managed 模式下所有会发 `Access-Control-Allow-Origin` 的位置（web_static、mcp_serve HTTP、ai_proxy、canvas SSE writer）统一改走 `pub(crate) fn cors_origin_for(allow: &[String], origin: Option<&str>) -> Option<String>`：命中白名单回显该 Origin，未命中不发头；非 managed 保持现状 `*`。`OPTIONS` 预检 `Access-Control-Allow-Headers` 补 `X-OpenPencil-Token, Content-Type`。
- Consumes: Task 3 的 `managed_token`、`allow_origins`。

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn managed_auth_gates_by_method_and_path() {
    let auth = RequestAuth { managed: true, token: "tok123".into() };
    assert!(auth.allows("GET", "/", None));            // static shell
    assert!(auth.allows("GET", "/index.html", None));
    assert!(auth.allows("GET", "/pkg/op_host_web.js", None));
    assert!(auth.allows("GET", "/canvaskit/canvaskit.wasm", None)); // editor can't boot without
    assert!(auth.allows("GET", "/assets/iconify-catalog-brands.json", None));
    assert!(auth.allows("GET", "/smoke/step-1b.html", None));
    assert!(auth.allows("OPTIONS", "/api/mcp/document", None)); // preflight
    assert!(!auth.allows("POST", "/", None));           // JSON-RPC alias: privileged
    assert!(!auth.allows("GET", "/api/mcp/events", None)); // SSE: privileged
    assert!(!auth.allows("POST", "/mcp", Some("wrong")));
    assert!(auth.allows("POST", "/mcp", Some("tok123")));
}

#[test]
fn unmanaged_mode_keeps_open_behavior() {
    let auth = RequestAuth { managed: false, token: String::new() };
    assert!(auth.allows("POST", "/api/mcp/document", None));
}

#[test]
fn cors_echoes_only_allowlisted_origin() {
    let allow = vec!["vscode-webview://abc".to_string()];
    assert_eq!(
        cors_origin_for(&allow, Some("vscode-webview://abc")).as_deref(),
        Some("vscode-webview://abc")
    );
    assert_eq!(cors_origin_for(&allow, Some("http://evil.local")), None);
    assert_eq!(cors_origin_for(&allow, None), None);
}
```

- [ ] **Step 2: 确认失败**（新测试引用 `RequestAuth`/`cors_origin_for` 等未定义符号，**编译失败即为本步的红灯状态**——cargo 到不了 `running N tests`）

Run: `cargo test -p op-host-services managed_auth_gates`
Expected: 编译错误（cannot find … `RequestAuth`）

- [ ] **Step 3: 实现**

```rust
impl RequestAuth {
    pub fn allows(&self, method: &str, path: &str, presented: Option<&str>) -> bool {
        if !self.managed {
            return true;
        }
        // Mirrors web_static.rs's static route table (web_static.rs:188):
        // everything the static layer serves stays tokenless — the page
        // cannot know the token before the postMessage bootstrap.
        let static_get = method.eq_ignore_ascii_case("GET")
            && (path == "/"
                || path == "/index.html"
                || path.starts_with("/pkg/")
                || path.starts_with("/smoke/")
                || path.starts_with("/canvaskit/")
                || path.starts_with("/assets/"));
        let exempt = method.eq_ignore_ascii_case("OPTIONS") || static_get;
        exempt || presented == Some(self.token.as_str())
    }
}

pub(crate) fn cors_origin_for(allow: &[String], origin: Option<&str>) -> Option<String> {
    origin.filter(|o| allow.iter().any(|a| a == o)).map(str::to_string)
}
```

接线（`serve_one` :1319）：请求行 + headers 解析完成、**web_static 静态路由处理返回之后**（静态层服务掉的请求天然豁免——与 `allows` 的豁免清单语义一致，二者以 `web_static.rs:188` 的路由表为共同权威）、进入任何特权分支（SSE、AI 流、`/mcp`、`/api/*`、`POST /`）之前插入 `RequestAuth::allows` 检查，拒绝立即写 401 响应并关闭连接。header 提取沿用该函数现有的 header 解析代码路径。五处 CORS 产生点（web_static、mcp_serve、ai_proxy、web_chat_standard `write_sse_headers`、canvas 响应组装）接 `cors_origin_for`。

- [ ] **Step 4: 跑测试（含防 0 测试假绿检查）**

Run: `cargo test -p op-host-services managed_auth_gates && cargo test -p op-host-services unmanaged_mode_keeps && cargo test -p op-host-services cors_echoes_only && cargo test -p op-host-services && cargo clippy -p op-host-services --all-targets -- -D warnings`
Expected: 全 PASS；前三条 filter 命令输出各为 `running 1 test`（`running 0 tests` = filter 打错，视为失败）；全量绿 = 非 managed 兼容不变量成立

- [ ] **Step 5: Commit**

```bash
git add crates/op-host-services/src/web_canvas_server.rs \
        crates/op-host-services/src/web_canvas_server_tests.rs \
        crates/op-host-services/src/web_static.rs crates/op-host-services/src/mcp_serve.rs \
        crates/op-host-services/src/ai_proxy.rs crates/op-host-services/src/web_chat_standard.rs
git commit -m "feat(web): serve-one layer token auth and origin allowlist for managed daemon"
```

---

### Task 5: daemon — sync-reset 幂等守卫 + document POST 条件写（baseVersion）

**Files:**
- Modify: `crates/op-host-services/src/web_canvas_server.rs`（`sync-reset` 路由 :308、`reset_document` :168、document POST 处理 :266 附近）
- Test: `crates/op-host-services/src/web_canvas_server_tests.rs`（用现有 `fresh_state()` helper 与 `SYNC_BODY` 常量，:6-19）

**Interfaces:**
- Produces: `POST /api/mcp/sync-reset`——进程生命周期内第一次**成功**的 reset 消耗守卫；失败不消耗（可重试）；已消耗后返回 `200 {"ok":true,"skipped":true,"version":<current>}` 不动文档。`WebCanvasState` 加 `pub(crate) reset_consumed: bool`；`pub(crate) fn reset_document_guarded(&mut self) -> Result<ResetOutcome, String>`，`pub(crate) struct ResetOutcome { pub skipped: bool }`。
- Produces: `POST /api/mcp/document` body 可选顶层 `baseVersion: u64`——带且 ≠ 当前版本 → `409 {"ok":false,"error":"version-conflict","version":<current>}` 不写入；不带 → 现状无条件写（旧客户端/内部路径兼容）。入口签名 `pub(crate) fn apply_document_push(&mut self, body: &str, base_version_override: Option<u64>) -> Result<PushOutcome, String>`——`Err` 保留现有校验失败语义（malformed JSON/schema → HTTP 400 不变），`PushOutcome { pub applied: bool, pub current_version: u64 }` 只表达版本裁决。`baseVersion` 用 `serde_json` 提取（现有 document 解析路径本就是 serde，沿用即可，不要手写扫描）。
- Consumes: Task 4 鉴权（这些路由属特权路径）；`WebCanvasState` 现有版本字段（源码内已有单调 version，实现时以实际字段名为准并在守卫/条件写中复用，不新造版本计数）。

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn second_sync_reset_is_skipped_and_mutates_nothing() {
    let mut state = fresh_state();
    let first = state.reset_document_guarded().unwrap();
    assert!(!first.skipped);
    // capture full post-reset identity: a skipped reset must not move EITHER
    let v_before = state.document_version_for_test();
    let doc_before = serde_json::to_string(&state.editor.doc).unwrap(); // adapt to the real field path
    let second = state.reset_document_guarded().unwrap();
    assert!(second.skipped);
    assert_eq!(state.document_version_for_test(), v_before);
    assert_eq!(serde_json::to_string(&state.editor.doc).unwrap(), doc_before);
}

// A second valid body whose content DIFFERS from SYNC_BODY — rejected writes
// are asserted against it so "replace the doc but suppress the version bump"
// style bugs cannot pass.
const SYNC_BODY_ALT: &str = r##"{"document":{"version":"1.0.0","children":[{"id":"n77","type":"ellipse","name":"Rejected Ellipse","x":9,"y":9,"width":30,"height":30,"fill":[{"type":"solid","color":"#abcdef"}]}]},"sourceClientId":"web"}"##;

fn doc_fingerprint(state: &WebCanvasState) -> (u64, String) {
    (
        state.document_version_for_test(),
        serde_json::to_string(&state.editor.doc).unwrap(), // adapt to the real field path
    )
}

#[test]
fn failed_reset_errors_and_mutates_nothing() {
    let mut state = fresh_state();
    state.current_path = Some(std::path::PathBuf::from("/nonexistent/x.op"));
    let before = doc_fingerprint(&state);
    // the reset MUST fail here — a silent success is itself a bug:
    assert!(state.reset_document_guarded().is_err());
    assert!(!state.reset_consumed); // retryable
    assert_eq!(doc_fingerprint(&state), before); // version AND bytes untouched
}

#[test]
fn document_post_with_stale_base_version_conflicts() {
    let mut state = fresh_state();
    let v0 = state.document_version_for_test();
    let ok = state.apply_document_push(SYNC_BODY, Some(v0)).unwrap();
    assert!(ok.applied);
    // capture BEFORE the stale attempt: a rejected write must change nothing
    let before = doc_fingerprint(&state);
    // stale write carries DIFFERENT bytes — proves the document didn't move
    let stale = state.apply_document_push(SYNC_BODY_ALT, Some(v0)).unwrap();
    assert!(!stale.applied);
    assert_eq!(doc_fingerprint(&state), before); // no bump, no content swap
    assert_eq!(stale.current_version, before.0);
}

#[test]
fn document_post_without_base_version_keeps_legacy_behavior() {
    let mut state = fresh_state();
    assert!(state.apply_document_push(SYNC_BODY, None).unwrap().applied);
    assert!(state.apply_document_push(SYNC_BODY, None).unwrap().applied);
}

#[test]
fn malformed_document_push_stays_an_error() {
    let mut state = fresh_state();
    assert!(state.apply_document_push("{not json", None).is_err()); // still HTTP 400
}

#[test]
fn base_version_is_extracted_from_the_request_body() {
    let mut state = fresh_state();
    // No override: the stale baseVersion inside the body itself must conflict.
    let stale_in_body = SYNC_BODY.replacen(
        r#""sourceClientId":"web""#,
        r#""sourceClientId":"web","baseVersion":9999"#,
        1,
    );
    let out = state.apply_document_push(&stale_in_body, None).unwrap();
    assert!(!out.applied);
}
```

（`document_version_for_test()`：若版本字段非 pub，加 `#[cfg(test)]` 访问器；`apply_document_push` 内部复用现有 document POST 的 serde 解析与替换路径，把「解析 baseVersion + 比较 + 写入」收拢成这一个可单测入口，路由层薄调用并把 `Err` 映射为现有 400 响应。）

- [ ] **Step 2: 确认失败** — Run: `cargo test -p op-host-services sync_reset && cargo test -p op-host-services document_post`；Expected: FAIL

- [ ] **Step 3: 实现**（按 Interfaces 与测试语义；`baseVersion` 在现有 document POST 的 **serde 解析**处一并提取——`base_version_override` 参数仅供测试直入，路由路径以 body 内字段为准，override 为 None 时从 body 取）

- [ ] **Step 4: 跑测试** — Run: `cargo test -p op-host-services`；Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/op-host-services/src/web_canvas_server.rs \
        crates/op-host-services/src/web_canvas_server_tests.rs
git commit -m "feat(web): idempotent sync-reset and base-version conditional document writes"
```

---

### Task 6: wasm — SyncController 接线（拉取门控 + 条件推送 + 冲突挂起）

**Files:**
- Modify: `crates/op-host-web/src/live_sync_glue.rs`（`start` :54、`poll_version` :95、`push_document_if_changed` :195）
- Modify: `crates/op-host-web/src/canvaskit.rs`（`mount_ck` :894 起的 live-sync 启动调用点——`start()` 签名变化的唯一调用方）
- Test: 状态机逻辑已在 Task 2 原生覆盖；本任务是接线，验证 = wasm 编译门（运行时行为的 E2E 属 Plan 2 + 手动矩阵——Task 9 只测 daemon HTTP 契约，不测 wasm 接线）

**Interfaces:**
- Produces: `pub(crate) struct SyncController { pub gate: op_editor_core::sync_gate::SyncGate, pub client: WebSyncClient, pub push_busy: bool }`，包在 `pub(crate) type SharedSync = Rc<RefCell<SyncController>>`。由 `mount_ck` 创建并同时交给 `live_sync_glue::start(inner, sync: SharedSync)` 与 Task 7 的桥（两者共享同一实例——v1 的「桥摸不到局部 WebSyncClient」缺陷由此解决）。
- Produces: 行为接线（一切门控判定基于**当前 `(document_generation, document_revision)` 对**——从 `host().editor_state()` 读取，记 `current_pair()`；`take_doc_sync_dirty()` 仅作为「可能需要序列化」的廉价触发器保留，不再参与门控决策——它是 UI+文档的保守超集，误报不会再卡门）：
  - `poll_version` 开头 `if !gate.pull_allowed(current_pair()) { return; }`；
  - **apply 前二次校验**：`apply_document_response` 闭包应用前重读 `current_pair()` 再查一次 `pull_allowed`（请求飞行期间若产生了本地编辑必须放弃本次 apply，防异步响应覆盖新编辑）；apply 成功后 `gate.note_synced(post_apply_pair)`（replace 会 bump generation，取应用后的新对）+ 现有 `note_applied_snapshot` 逻辑保留（echo 抑制）；
  - push tick：`gate.needs_push(current_pair())` 是**唯一权威门**——false 即返回（冲突挂起时恒 false，周期推送因此挂起）；`take_doc_sync_dirty()` 仅作为提示位消费，永远不能作为序列化/推送的依据绕过该门；序列化时**同步捕获** `pushed_pair = current_pair()`；**哈希短路必须再基线**：`should_push` 判定字节与 daemon 基线相同时（如 generation-only 替换——`open-document` 灌入相同字节，generation 变了、内容没变），不发推送但**必须** `gate.note_synced(pair)` 直接再基线——否则 needs_push 恒真、推送恒被哈希跳过、拉取门永闭（死锁）；大小判断 `SyncGate::periodic_push_allowed(len)`（超限：跳过、基线不动、保留一次性 console warn——snapshot 通道兜底）；
  - 推送 body 用 `wrap_push_body_with_base(&doc_json, client.last_version())`；响应先过 `parse_push_conflict` → `Some(v)` 则 `gate.note_conflict(v)`（基线不动、不重试）；成功则 `client.mark_pushed(&doc_json, version)` + `gate.note_synced(pushed_pair)`（**用序列化时捕获的对，不是响应到达时的现值**——飞行中编辑 B 因此保持未推送判定）。
- Consumes: Task 1 `document_generation()`/`document_revision()`；Task 2 `SyncGate`/`WebSyncClient` 扩展；Task 5 的 409 语义。

- [ ] **Step 1: 接线实现**（判定逻辑全部在 Task 2 已原生测过；本任务是接线）

`live_sync_glue.rs` 关键改动形态：

```rust
pub(crate) fn start<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>, sync: SharedSync) {
    // 移除局部 WebSyncClient/push_busy —— 全部经 sync 访问；
    // 两个 tick 闭包各 clone 一份 sync。
}

fn current_pair<C: RepaintContext>(b: &C) -> (u64, u64) {
    let s = b.host().editor_state();
    (s.document_generation(), s.document_revision())
}

// poll_version 开头：先检查 accept 窗口是否被本地编辑打破（打破则带着保留的
// server version 重入冲突流，防 baseline=None/conflict=None 的死锁态），再查拉取门。
// 借用纪律：Rust 2021 中 if-let 检察对象里的 borrow() 临时量活到整个 body 结束，
// body 内再 borrow_mut() 会运行时 panic —— 必须先绑定 Option 再判：
let pair = current_pair(&*inner.borrow());
let broken = sync.borrow().gate.accept_window_broken(pair); // borrow ends HERE
if let Some(v) = broken {
    sync.borrow_mut().gate.note_conflict(v); // observer latch reports; user decides again
    return;
}
if !sync.borrow().gate.pull_allowed(pair) { return; }

// apply_document_response：借用纪律——现有 sync.sync(...) 闭包运行时 controller 已被
// 可变借用（live_sync_glue.rs:164 起的现有结构），闭包内再 borrow 会 panic。二次校验
// 必须放在**取得可变借用之前**，note_synced 放在闭包返回之后：
let gate_ok = sync.borrow().gate.pull_allowed(current_pair(inner_ref)); // ① immutable borrow, then drop
if !gate_ok {
    return; // a local edit landed while the fetch was in flight — abort apply
}
let applied = { /* ② existing sync.try_borrow_mut() + sync(...) closure, untouched */ };
if applied {
    // ③ closure has returned, mutable borrow released — commit the baseline:
    sync.borrow_mut().gate.note_synced(post_apply.0, post_apply.1);
}

// push_document_if_changed 内：needs_push 是唯一权威门（冲突挂起时它恒 false，
// 任何 dirty 触发都不得绕过）；dirty 位仅作为无变更时跳过序列化的优化。
let pair = current_pair(&*b);
let _ = b.host_mut().take_doc_sync_dirty(); // consume the hint; NOT a gate
if !sync.borrow().gate.needs_push(pair) { return; }
// ...serialize; pushed_pair = pair captured HERE, with the bytes...
if !sync.borrow().client.should_push(&doc_json) {
    // bytes already match the daemon baseline (e.g. generation-only replace):
    // no push to send — re-baseline directly or the gate deadlocks.
    sync.borrow_mut().gate.note_synced(pair.0, pair.1);
    return;
}
if !SyncGate::periodic_push_allowed(doc_json.len()) { /* warn-once; return */ }
let base = sync.borrow().client.last_version();
let body = WebSyncClient::wrap_push_body_with_base(&doc_json, base);
// on_response：
if let Some(server_v) = WebSyncClient::parse_push_conflict(&resp) {
    sync.borrow_mut().gate.note_conflict(server_v);
    return;
}
if let Some(version) = WebSyncClient::parse_push_response(&resp) {
    let mut s = sync.borrow_mut();
    s.client.mark_pushed(&doc_json, version);
    s.gate.note_synced(pushed_pair.0, pushed_pair.1); // the serialized pair
}
```

`canvaskit.rs`：live-sync 启动处创建 `SharedSync` 并传入（暂存到 mount 作用域变量，Task 7 桥复用）。

- [ ] **Step 2: 编译门**（`web` feature 是 wasm32-clean 桩，**不编译** live_sync_glue/canvaskit——必须用 `canvaskit` feature 检查）

Run: `cargo check --target wasm32-unknown-unknown -p op-host-web --no-default-features --features canvaskit && cargo test -p op-editor-core`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/op-host-web/src/live_sync_glue.rs crates/op-host-web/src/canvaskit.rs
git commit -m "feat(web): route live sync through the shared gate with conditional pushes"
```

---

### Task 7: wasm — postMessage 桥（token 引导 / dirty / snapshot / save-committed / 冲突解决）

**Files:**
- Create: `crates/op-host-web/src/vscode_bridge.rs`（DOM 接线薄层；协议编解码在 Task 2 的 `op-editor-core::bridge_protocol`）
- Modify: `crates/op-host-web/src/canvaskit.rs`（`mount_ck` :903 起——**启动顺序重排**，见下）
- Modify: `crates/op-host-web/src/live_sync.rs`（4 个请求 helper：`get` :41、`get_with_status` :66、`post_json` :92、`post_json_with_status` :118 统一附 token 头）
- Modify: `crates/op-host-web/src/web_model_catalog.rs`（:21 直发 daemon XHR——补 token）
- Modify: `crates/op-host-web/src/web_ai_transport.rs`（:93 直发 daemon XHR——补 token）
- Modify: `crates/op-host-web/src/iconify_web.rs`（:54 仅 daemon 侧 brand 请求补 token；**发向公网 Iconify CDN 的请求绝不附 token**——泄漏面）
- Modify: `crates/op-host-web/src/lib.rs`（`mod vscode_bridge;`）
- Modify: `crates/op-host-web/Cargo.toml`（web-sys features 补 `MessageEvent`）
- Modify: `crates/op-host-services/src/web_static/index.html`（:95 mount 前的 `sync-reset` fetch 移除）
- Test: 协议/状态机已在 Task 2 原生覆盖；**如实声明覆盖缺口（不假装 Task 9 会测）**——mount 顺序、token 引导、快照冲刷、监听器存活等 wasm 运行时行为在本 plan 内只有编译门；其端到端验证落在 Plan 2 的插件集成测试与 spec 手动矩阵（此为已接受的覆盖边界，Task 9 只覆盖 daemon HTTP 契约）

**Interfaces:**
- Produces: `pub(crate) fn install(inner: Rc<RefCell<Ck>>, sync: SharedSync)`——不返回句柄：监听器 Closure 用与本文件其它页面级监听器相同的 `Closure::forget` 模式**刻意泄漏**（桥与页面同生命周期；若返回 owning handle，`mount_ck` 返回时 drop 会拆掉桥）。window `message` 监听器：
  - **来源校验**：`event.source == window.parent`（`js_sys::Reflect` 比较）且 origin 锁定（首条合法 `init` 的 `event.origin` 记录后，后续不符即丢弃）；非桥消息（`BridgeInbound::parse` 返回 None）静默忽略；
  - `Init { token }` → 存 `thread_local BRIDGE_TOKEN` → 触发 daemon 引导序列（见启动顺序）→ 回 `event_ready(gen, rev)`；
  - `OpenDocument { json }` → **同步序幕（首个 await 之前，顺序固定）**：`replace_document`（generation 自增，得到目标 generation `G`）→ `gate.note_open_pending(G)`（open 完成从此 generation 作用域化——若 push_busy 正被某个飞行中的周期推送占用，该推送的确认携带的是 open 之前序列化的旧 generation 对，`note_synced` 按 `generation >= G` 判定不会误清 open 状态）+ 占用 `push_busy`（占用可能需等待当前推送结束，但状态已在同步序幕落好，等待期间 pull 已被 open_pull_block 挡住）→ **探测-条件推送**：`GET /api/mcp/version`（token 通道）取 daemon 当前版本 `V`（**不能用 `last_version()`**——bootstrap 期它是 0 而 sync-reset 已 bump 过版本，必 409），`wrap_push_body_with_base(&doc_json, V)` 条件推送 → 成功则原子 `client.mark_pushed(&doc_json, returned_version)` + `gate.note_synced(pair)`（一并清 open_pending/open_pull_block）——`opened` **不在此直发**：`note_synced` 清除 pending open 时置 opened 闩锁，观察者 `take_opened_edge()` 消费后发出（所有 `opened` 发射单点化且时序免疫）。**409 = 探测与推送之间有并发 MCP 写入落地**：重新探测并重试**一次**；仍 409 → `gate.note_conflict(v)`、**不发 `opened`**、释放 `push_busy`——冲突事件由**唯一的过渡发射器**（`gate.conflict()` None→Some 的 tick 观察者，见下文出站事件）发出，此处**不得**直接再发 `event_sync_conflict`（防重复）；宿主走既有 resolve-conflict 协议裁决（无版本覆盖会静默丢弃并发写入，违反 spec「两个版本都不丢」，明确禁止）；
  - `Snapshot { purpose, request_id }` → **独立检测，不依赖 tick**：先查 `gate.conflict()`——`Some(v)` 时不推送、直接回 `event_snapshot_conflict(request_id, v)`（冲突挂起期间 snapshot 行为由此定义：宿主必须先走 resolve-conflict）；无冲突则序列化 doc 并同步捕获 `pair`（原子性由此保证）→ `gate.needs_push(pair)` 为真则经 snapshot 通道推送（`SyncGate::snapshot_push_allowed` 无上限；带 baseVersion；尊重 `push_busy` 串行，busy 时排队至当前推送响应后重试）→ 确认后**原子更新两者**：`client.mark_pushed(&doc_json, returned_version)` + `gate.note_synced(pair)`（漏掉 mark_pushed 会让 `last_version()` 过期——poller 会把自己的快照当远端更新回放、下次推送用旧 baseVersion 制造假冲突）→ `event_snapshot_result(request_id, doc_json, gen, rev)`；推送冲突 → `gate.note_conflict(v)` + `event_snapshot_conflict(request_id, v)`；
  - `SaveCommitted { generation, revision }` → `mark_saved_revision_at`（过期静默丢弃）；
  - `ResolveConflict { mode, request_id }`：
    - `UseLocal` → 序列化本地并捕获 `pair`，取 `gate.conflict()` 的 server_version 作 baseVersion 重推（snapshot 通道）→ 成功同样**原子更新** `client.mark_pushed(&doc_json, returned_version)` + `gate.note_synced(pair)` + `event_conflict_resolved(request_id)`；若此前存在挂起的 open，`note_synced` 已清除 open_pending，`opened` 由边沿观察者发出（**不直发**——单点化）；再冲突则用新 server_version 重试一次，仍失败回 `event_snapshot_conflict`；
    - `AcceptRemote` → 先 `event_snapshot_result(request_id, <本地字节>, gen, rev)`（宿主借此落备份，spec「两个版本都不丢」）→ `gate.resolve_accept_remote(current_pair())`（拉取门对**该 pair** 重开、conflict 清除，但 **open_pending 保留**；`accept_expected` 记下此刻的对——若在 pull 应用前又有本地编辑落地，pair 移动、pull 被重新挡住，**接线层观察到 accept 窗口被打破时重新 `note_conflict`**，回到用户裁决，绝不覆盖新编辑）→ `event_conflict_resolved(request_id)`。**`opened` 此刻不发**（远端还没应用，发了就是谎报"可安全操作"）：pull 应用并 `note_synced` 时置 opened 闩锁，观察者消费后**在那时**补发 `event_opened(new_gen)`。
    - resolve 分支无需任何观察者缓存操作——冲突/opened 均为闩锁语义（见出站事件），背靠背的第二个冲突由 `note_conflict` 重新置闩，天然不丢。
  - 出站事件（桥的 tick 观察者是**唯一**发射点——三类事件一律不得在处理器内直发）：`dirty-changed`（`(generation, revision, is_dirty)` 三元组比较，状态型事件比较即可）；`sync-conflict` 与 `opened` 是**边沿型事件，一律经 SyncGate 的可消费闩锁**——观察者每 tick `take_conflict_edge()` / `take_opened_edge()`，Some 即发射（正常 open 成功、UseLocal 完成、AcceptRemote 后 pull apply 完成三条路径统一由 `note_synced` 置 opened 闩锁）。闩锁由状态变更方法自己置位、观察者消费，**不存在缓存比较**——快速完成（一个 tick 内 rise+fall）不丢事件、背靠背二次冲突不丢边沿、处理器无任何缓存纪律要负担。
- Produces: **mount_ck 启动顺序重排**——`mount_ck` 现状在 mount 早期就发起 credential/model/live-sync 等 daemon 请求（:894 起）。改为：
  1. 先安装桥监听器；
  2. `window.self != window.top`（iframe 内）→ `await` init（JsFuture + 2000ms 超时；超时按直开处理并 console warn）；直开浏览器（非 iframe）→ 立即继续；
  3. init 完成（或直开路径）后才启动 credential policy / model catalog / live-sync / indicator 等 daemon 依赖服务；
  4. `sync-reset`：managed（有 token）由 Rust 侧带 token 发起；直开路径保持原时机语义（移到 wasm 内、mount 后立即发，替代 index.html 里被删除的 fetch）——**400ms pull tick 必须在 reset 完成后才启动**（直开路径同样成立，防 reset 前拉到旧状态）。
- Produces: token 注入——`pub(crate) fn attach_daemon_headers(req: &XmlHttpRequest, url: &str)`：**仅当 `url` 以 `daemon_base()` 为前缀**才附 `X-OpenPencil-Token`（公网请求如 Iconify CDN 绝不携带，防 token 外泄）。`live_sync.rs` 四个 helper + `web_model_catalog.rs:21` + `web_ai_transport.rs:93` + `iconify_web.rs:54`（daemon 侧分支）统一接入。**步骤内审计**：`grep -rn "XmlHttpRequest\|window.fetch\|Request::new" crates/op-host-web/src` 列出全部请求点，逐一确认 daemon 目标走 helper/`attach_daemon_headers`、公网目标不带 token；审计结果（文件+行）记录在 commit message body。
- Consumes: Task 1 `mark_saved_revision_at`；Task 2 协议 + `SyncGate`；Task 6 `SharedSync`。

- [ ] **Step 1: 实现桥 + 重排启动顺序**（协议/状态机测试已在 Task 2；本任务为 DOM 接线）

- [ ] **Step 2: token 注入审计**

Run: `grep -rn "XmlHttpRequest\|window.fetch\|Request::new" crates/op-host-web/src | grep -v test`
Expected: 每个命中点确认走 `live_sync` helpers 或已补 `attach_daemon_headers`；清单进 commit body

- [ ] **Step 3: 编译门 + bundle 门**

Run: `cargo check --target wasm32-unknown-unknown -p op-host-web --no-default-features --features canvaskit && tools/check-wasm-bundle.sh`
Expected: PASS，bundle ≤ 6 MiB gzip

- [ ] **Step 4: Commit**

```bash
git add crates/op-host-web/src/vscode_bridge.rs crates/op-host-web/src/canvaskit.rs \
        crates/op-host-web/src/live_sync.rs crates/op-host-web/src/lib.rs \
        crates/op-host-web/src/web_model_catalog.rs crates/op-host-web/src/web_ai_transport.rs \
        crates/op-host-web/src/iconify_web.rs \
        crates/op-host-web/Cargo.toml crates/op-host-services/src/web_static/index.html
git commit -m "feat(web): postMessage bridge with token bootstrap and conflict resolution"
```

---

### Task 8: wasm — 保存基线竞态修复（dom_io）

**Files:**
- Modify: `crates/op-host-web/src/dom_io.rs`（:272 保存路径）
- Test: 语义由 Task 1 的 `ack_for_older_revision_does_not_mark_newer_edits_clean` 原生锁定；本任务为调用点修正

**Interfaces:**
- Produces: web 内建保存路径在**序列化文档字节的同一时刻**捕获 `(document_generation, document_revision)`，异步保存成功回调里调 `mark_saved_revision_at(g, r)`——写盘期间的新编辑不再被误标已保存（v1 用无参版本仍是竞态，review 缺陷 7）。
- Consumes: Task 1 `mark_saved_revision_at`。

- [ ] **Step 1: 实现**

```rust
// dom_io.rs save path — at serialization time:
let (snap_gen, snap_rev) = {
    let state = /* existing editor-state access at this site */;
    (state.document_generation(), state.document_revision())
};
// ... existing async save request ...
// in the success callback (:272 vicinity), replace the filename/dirty
// handling's baseline step with:
let _ = state.mark_saved_revision_at(snap_gen, snap_rev);
```

- [ ] **Step 2: 编译门** — Run: `cargo check --target wasm32-unknown-unknown -p op-host-web --no-default-features --features canvaskit`；Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add crates/op-host-web/src/dom_io.rs
git commit -m "fix(web): bind web save baseline to the serialized generation and revision"
```

---

### Task 9: 集成冒烟 — managed daemon 全契约端到端

**Files:**
- Create: `crates/op-host-web-server/tests/managed_daemon_smoke.rs`（**必须放本 crate** 的 integration test——`CARGO_BIN_EXE_op-host-web-server` 仅在定义该 bin 的 package 内可用）
- Test: 本文件即交付物

**Interfaces:**
- Consumes: Task 3-5 的全部 daemon 契约（握手/token 门/静态豁免/sync-reset 幂等/baseVersion/stdin-EOF/CORS 白名单与预检——两个测试函数合计覆盖）。
- Produces: CI 回归屏障：`cargo test -p op-host-web-server --test managed_daemon_smoke`。

- [ ] **Step 1: 写测试（一次写全；spawn 失败 = 测试失败，不 skip）**

```rust
//! End-to-end smoke for the managed serve-web contract.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Daemon { child: Child, port: u16, token: String }

const ALLOWED_ORIGIN: &str = "vscode-webview://smoke-test";

fn spawn_managed(file: Option<&str>) -> Daemon {
    let mut args = vec![
        "--serve-web", "--managed", "--port", "0",
        "--allow-origin", ALLOWED_ORIGIN,
    ];
    if let Some(f) = file { args.extend(["--file", f]); }
    let mut child = Command::new(env!("CARGO_BIN_EXE_op-host-web-server"))
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn op-host-web-server");
    let stdout = child.stdout.take().expect("stdout piped");
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line).expect("handshake line");
    let v: serde_json::Value = serde_json::from_str(&line).expect("handshake json");
    Daemon {
        child,
        port: v["port"].as_u64().expect("port") as u16,
        token: v["token"].as_str().expect("token").to_string(),
    }
}

/// Minimal HTTP/1.1 exchange over a raw TcpStream (no client dep).
fn http(port: u16, method: &str, path: &str, token: Option<&str>, body: Option<&str>) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let body = body.unwrap_or("");
    let token_header = token.map(|t| format!("X-OpenPencil-Token: {t}\r\n")).unwrap_or_default();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{token_header}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = String::new();
    s.read_to_string(&mut raw).expect("read");
    let status: u16 = raw.split_whitespace().nth(1).expect("status").parse().expect("status u16");
    let payload = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, payload)
}

fn wait_exit(child: &mut Child, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if child.try_wait().expect("try_wait").is_some() { return true; }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn read_version(port: u16, token: &str) -> u64 {
    let (code, body) = http(port, "GET", "/api/mcp/version", Some(token), None);
    assert_eq!(code, 200);
    let v: serde_json::Value = serde_json::from_str(&body).expect("version json");
    v["version"].as_u64().expect("version")
}

#[test]
fn managed_contract_end_to_end() {
    // --file load coverage: a canonical corpus fixture that MUST exist —
    // spawn/read failures are test failures, never skips.
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/jian/crates/jian-ops-schema/tests/corpus/nested-frame.op"
    );
    let fixture_text = std::fs::read_to_string(fixture).expect("fixture exists");
    // Probe a stable field to prove the daemon loaded the file. The corpus
    // root frame has no "name", so probe its "id" instead.
    let fx: serde_json::Value = serde_json::from_str(&fixture_text).expect("fixture json");
    let probe_id = fx["children"][0]["id"].as_str().expect("fixture root id");
    let probe = format!(r#""id":"{probe_id}""#);
    let mut d = spawn_managed(Some(fixture));

    // 1. token gate on /api and /mcp (missing + wrong), POST / alias included
    assert_eq!(http(d.port, "GET", "/api/mcp/version", None, None).0, 401);
    assert_eq!(http(d.port, "POST", "/mcp", Some("wrong"), Some("{}")).0, 401);
    assert_eq!(http(d.port, "POST", "/", None, Some("{}")).0, 401);

    // 2. authenticated /mcp succeeds (not only rejection coverage)
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let (code, mcp_body) = http(d.port, "POST", "/mcp", Some(&d.token), Some(init));
    assert_eq!(code, 200);
    assert!(mcp_body.contains(r#""result""#), "mcp initialize must answer: {mcp_body}");

    // 3. static shell tokenless: property under test is "not 401" —
    //    a bundle-less CI daemon may serve the help page, still not a 401.
    assert_ne!(http(d.port, "GET", "/", None, None).0, 401);

    // 4. --file content actually loaded
    let (code, doc_body) = http(d.port, "GET", "/api/mcp/document", Some(&d.token), None);
    assert_eq!(code, 200);
    assert!(doc_body.contains(&probe), "fixture content must be served: want {probe}");

    // 5. sync-reset idempotence
    let (_, first) = http(d.port, "POST", "/api/mcp/sync-reset", Some(&d.token), Some("{}"));
    assert!(!first.contains(r#""skipped":true"#), "first reset must run: {first}");
    let (_, second) = http(d.port, "POST", "/api/mcp/sync-reset", Some(&d.token), Some("{}"));
    assert!(second.contains(r#""skipped":true"#), "second reset must skip: {second}");

    // 6. baseVersion conflict — version RE-READ after the reset bumped it
    let current = read_version(d.port, &d.token);
    let stale = format!(
        r#"{{"document":{{"version":"1.0.0","children":[]}},"sourceClientId":"smoke","baseVersion":{}}}"#,
        current + 999
    );
    let (code, cbody) = http(d.port, "POST", "/api/mcp/document", Some(&d.token), Some(&stale));
    assert_eq!(code, 409, "stale baseVersion must conflict: {cbody}");
    assert!(cbody.contains("version-conflict"));

    // 7. matching baseVersion applies — re-read again (nothing wrote since,
    //    but the pattern must not depend on that)
    let current = read_version(d.port, &d.token);
    let fresh = format!(
        r#"{{"document":{{"version":"1.0.0","children":[]}},"sourceClientId":"smoke","baseVersion":{current}}}"#
    );
    assert_eq!(http(d.port, "POST", "/api/mcp/document", Some(&d.token), Some(&fresh)).0, 200);

    // 8. parent-death lease: dropping stdin must exit the daemon
    drop(d.child.stdin.take());
    assert!(wait_exit(&mut d.child, Duration::from_secs(5)), "daemon must exit on stdin EOF");
}

/// Raw exchange that also returns the response head (for header assertions).
fn http_with_origin(
    port: u16, method: &str, path: &str,
    token: Option<&str>, origin: Option<&str>, body: Option<&str>,
) -> (u16, String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let body = body.unwrap_or("");
    let token_h = token.map(|t| format!("X-OpenPencil-Token: {t}\r\n")).unwrap_or_default();
    let origin_h = origin.map(|o| format!("Origin: {o}\r\n")).unwrap_or_default();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{token_h}{origin_h}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = String::new();
    s.read_to_string(&mut raw).expect("read");
    let status: u16 = raw.split_whitespace().nth(1).expect("status").parse().expect("status u16");
    let mut parts = raw.splitn(2, "\r\n\r\n");
    let head = parts.next().unwrap_or("").to_string();
    let payload = parts.next().unwrap_or("").to_string();
    (status, head, payload)
}

#[test]
fn cors_allowlist_and_preflight_contract() {
    let mut d = spawn_managed(None);

    // allowlisted Origin is echoed back on an authed API response
    let (code, head, _) = http_with_origin(
        d.port, "GET", "/api/mcp/version", Some(&d.token), Some(ALLOWED_ORIGIN), None,
    );
    assert_eq!(code, 200);
    assert!(
        head.contains(&format!("Access-Control-Allow-Origin: {ALLOWED_ORIGIN}")),
        "allowlisted origin must be echoed: {head}"
    );

    // non-allowlisted Origin gets NO allow-origin header (and no wildcard)
    let (_, head, _) = http_with_origin(
        d.port, "GET", "/api/mcp/version", Some(&d.token), Some("http://evil.local"), None,
    );
    assert!(!head.contains("Access-Control-Allow-Origin"), "must not echo evil origin: {head}");

    // OPTIONS preflight is tokenless-exempt and advertises the token header
    let (code, head, _) = http_with_origin(
        d.port, "OPTIONS", "/api/mcp/document", None, Some(ALLOWED_ORIGIN), None,
    );
    assert_ne!(code, 401, "preflight must not require the token");
    assert!(
        head.to_ascii_lowercase().contains("x-openpencil-token"),
        "preflight must allow the token header: {head}"
    );

    drop(d.child.stdin.take());
    assert!(wait_exit(&mut d.child, Duration::from_secs(5)));
}
```

（`serde_json` 若不在 `op-host-web-server` 的 dev-dependencies：加 `[dev-dependencies] serde_json = { workspace = true }`。）

- [ ] **Step 2: 跑测试** — Run: `cargo test -p op-host-web-server --test managed_daemon_smoke`；Expected: PASS（红则回修 Task 3-5）

- [ ] **Step 3: 全 workspace 收尾门**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: 全 PASS

- [ ] **Step 4: Commit**

```bash
git add crates/op-host-web-server/tests/managed_daemon_smoke.rs crates/op-host-web-server/Cargo.toml
git commit -m "test(web): end-to-end smoke for the managed daemon contract"
```

---

## 任务依赖图

```
Task 1 (generation) ─────────┐
Task 2 (SyncGate+protocol) ──┼─► Task 6 (wasm sync 接线) ─► Task 7 (bridge) ─► Task 8 (dom_io)
Task 3 (handshake/lease) ─► Task 4 (auth/CORS) ─► Task 5 (reset guard + baseVersion)
Task 3-5 ─► Task 9 (integration smoke)
```

## 覆盖对照（spec Rust 工作包 → 任务）

| spec 条目 | 任务 |
|---|---|
| 1. wasm ⇄ 宿主消息桥（snapshot/save-committed/generation） | Task 1, 2, 7 |
| 2. 同步并发控制（门控/条件写/冲突二选一/竞态测试） | Task 2, 5, 6, 7 |
| 3. daemon 启动契约（port 0/握手/父死自杀/reset 守卫） | Task 3, 5 |
| 4. loopback 鉴权 + token 引导 + CORS | Task 4, 7 |
| 5. 推送上限（snapshot 通道豁免） | Task 2（判定）, 7（接线） |
| 保存确认 generation 作用域（stop-gate 修订） | Task 1, 8 |

## Plan 2/3 预告（不在本 plan 内）

- **Plan 2 — TS 插件本体**（`openpencil/editors/vscode/`）：DaemonPool、CustomEditorProvider（消费本 plan 桥协议，含冲突二选一对话框——`sync-conflict` → 弹窗 → `resolve-conflict`）、McpProxy、McpConfigurator、AiPanel、codegen 编排。
- **Plan 3 — 打包与发布**：平台矩阵 vsix、二进制/资产装配、Marketplace/OpenVSX/Trae 发布、CI。
