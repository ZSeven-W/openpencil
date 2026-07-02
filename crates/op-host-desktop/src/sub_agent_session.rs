//! Sub-agent design loops for `spawn_agents` (Task 3.1).
//!
//! When the top-level design-agent loop (`OPENPENCIL_DESIGN_AGENT_LOOP`)
//! calls the `spawn_agents` tool, the host launches N scoped design
//! `ChatSession`s — one per [`op_mcp::spawn_agents_tool::SpawnSpec`] — each
//! with its own agent identity + canvas badge, scoped to its container
//! node(s), and seeded with the parent-resolved styleguide + guideline
//! content. The subs run **sequentially** (one active at a time) and
//! **fire-and-forget**: the `spawn_agents` tool acks immediately so the
//! parent keeps working its own last section while the subs run via the
//! pump.
//!
//! ## Decoupling from `chat_session::pump`
//!
//! Threading `DesktopApp` (or a `Vec<SubAgentSession>`) through
//! `chat_session::pump` would borrow `host` and the sub sessions at the
//! same time. Instead, the spawn interception is split in two:
//!
//! 1. [`chat_session::drain_tool_requests`] detects `spawn_agents`,
//!    parses the specs, and **stashes** them via [`stash_pending_spawn`]
//!    — then acks the tool. (A module-level [`Mutex`] stash; the UI is
//!    single-threaded so the lock is always uncontended.)
//! 2. `app_handler.rs`, *after* the parent `chat_session::pump`, calls
//!    [`take_pending_spawn`] → [`launch_sub_agents`] → [`pump_sub_agents`].
//!
//! Borrow-clean: each step borrows `host` and the sub `Vec` separately,
//! and [`pump_sub_agents`] `std::mem::take`s the active session out before
//! pumping so `host` and the session are never borrowed simultaneously.
//!
//! ## Nested-spawn guard
//!
//! Only the top-level loop spawns. While subs are running (or a stash is
//! pending), a sub calling `spawn_agents` again is a no-op: the stash
//! refuses new specs whenever the host already owns sub-agents. The
//! interception caller passes `nested = host_has_sub_agents` so the
//! refusal is decided at the call site.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use op_ai::chat_provider::ChatRequest;
use op_host_native::WidgetHostNative;
use op_mcp::spawn_agents_tool::SpawnSpec;
use op_orchestrator::agent_identity::assign_agent_identities;

use op_ai_skills::style_guide::{select_style_guide, style_guide_registry, SelectOptions};
use op_editor_core::agent_indicators;

use crate::chat_session::{self, builtin_provider_with_design_tools, ChatSession};
use crate::design_loop_indicator::{
    collect_top_level_frame_ids, register_new_frames, DesignLoopIndicator,
};

/// One launched sub-agent: its scoped design `ChatSession`, its assigned
/// colour/name identity, and the canvas indicator that badges the frames
/// it builds.
///
/// `session` is an `Option` so [`pump_sub_agents`] can `std::mem::take` it
/// out, pump it against `host`, and put it back — never borrowing `host`
/// and the session at once.
///
/// The `indicator` (epoch + initial-frame snapshot) is begun **lazily**,
/// the first frame this sub becomes active. Beginning all epochs up-front
/// would be wrong: `agent_indicators::begin()` bumps a single global epoch
/// and only the latest epoch's registrations stick, so an earlier sub's
/// badges would be dropped. Sequential execution means only one sub is
/// active at a time, so only one epoch is ever live.
pub(crate) struct SubAgentSession {
    /// The scoped design turn. `None` once the turn has finished.
    pub session: Option<ChatSession>,
    /// Assigned agent identity (colour + name) for the canvas badge.
    pub identity: op_orchestrator::agent_identity::AgentIdentity,
    /// Canvas indicator — `None` until this sub becomes active (lazy
    /// epoch begin), `Some` while/after it runs.
    pub indicator: Option<DesignLoopIndicator>,
}

// ---------------------------------------------------------------------------
// Pending-spawn stash (host picks it up after the parent pump)
// ---------------------------------------------------------------------------

/// Module-level stash for specs parsed inside `drain_tool_requests` but
/// launched by `app_handler` after the parent pump. The UI thread is the
/// only accessor, so the lock is always uncontended; the `Mutex` just
/// satisfies `static` interior mutability without `unsafe`.
static PENDING_SPAWN: Mutex<Option<Vec<SpawnSpec>>> = Mutex::new(None);

/// Set while [`pump_sub_agents`] is pumping a sub's turn. The spawn
/// interception in `chat_session::drain_tool_requests` reads it via
/// [`nested_spawn_active`] to drop a SUB's `spawn_agents` call (only the
/// top-level loop spawns). UI-thread-only, so a plain atomic suffices.
static SUB_AGENT_ACTIVE: AtomicBool = AtomicBool::new(false);

/// True while a sub-agent turn is being pumped — used by the spawn
/// interception to refuse nested spawns.
pub(crate) fn nested_spawn_active() -> bool {
    SUB_AGENT_ACTIVE.load(Ordering::SeqCst)
}

/// Stash `specs` for the host to launch after the parent pump.
///
/// `nested` is true when the host already owns sub-agents — i.e. the
/// caller is a SUB-agent calling `spawn_agents` again. In that case the
/// stash is refused (nested spawns are a no-op; only the top-level loop
/// spawns) and this returns `false`. Otherwise the specs replace any
/// prior stash (last-writer-wins within a frame) and it returns `true`.
pub(crate) fn stash_pending_spawn(specs: Vec<SpawnSpec>, nested: bool) -> bool {
    if nested {
        return false;
    }
    *PENDING_SPAWN.lock().unwrap() = Some(specs);
    true
}

/// Take the stashed specs, if any. Called by `app_handler` each frame
/// after the parent pump.
pub(crate) fn take_pending_spawn() -> Option<Vec<SpawnSpec>> {
    PENDING_SPAWN.lock().unwrap().take()
}

// ---------------------------------------------------------------------------
// Scoped system prompt
// ---------------------------------------------------------------------------

/// Build the scoped system prompt for one sub-agent: the design-agent
/// protocol prompt, then the sub's assignment + container-node scoping,
/// then the parent-resolved styleguide content, then each resolved
/// guideline. The sub cannot search styleguides itself, so the parent
/// passes the NAMES and we resolve the content here (wired, not stubbed).
pub(crate) fn build_sub_agent_prompt(spec: &SpawnSpec) -> String {
    let mut prompt = op_ai_skills::design_agent_system_prompt().to_string();

    prompt.push_str("\n\n## Your sub-agent assignment\n\n");
    prompt.push_str(&spec.prompt);

    if !spec.container_nodes.is_empty() {
        prompt.push_str("\n\nScope: only build inside container node(s): ");
        prompt.push_str(&spec.container_nodes.join(", "));
        prompt.push('.');
    }

    // Styleguide — resolve the parent-passed name to its markdown content.
    if let Some(guide) = select_style_guide(
        style_guide_registry(),
        &SelectOptions {
            name: Some(spec.styleguide_name.clone()),
            tags: vec![],
            platform: None,
        },
    ) {
        prompt.push_str("\n\n## Style guide\n\n");
        prompt.push_str(&guide.content);
    }

    // Guidelines — resolve each parent-passed topic to its content.
    for name in &spec.guideline_names {
        if let Some(content) = op_ai_skills::guideline_for(name) {
            prompt.push_str("\n\n## Guideline: ");
            prompt.push_str(name);
            prompt.push_str("\n\n");
            prompt.push_str(&content);
        }
    }

    prompt
}

// ---------------------------------------------------------------------------
// Launch
// ---------------------------------------------------------------------------

/// Launch one scoped design `ChatSession` + indicator per spec.
///
/// Builds a fresh design-toolset provider per sub (each gets its own tool
/// channel), assigns N distinct agent identities, and seeds
/// `agents_running = (N, N)` so the chat header shows "N/N designing…".
///
/// Indicator epochs are NOT begun here — they start lazily in
/// [`pump_sub_agents`] when each sub becomes active (sequential execution
/// keeps exactly one epoch live at a time; see [`SubAgentSession`]).
///
/// Returns the launched sessions. Subs whose provider can't be built (no
/// ready builtin model) are skipped; `agents_running` reflects the number
/// actually launched. Returns an empty `Vec` when nothing launched.
pub(crate) fn launch_sub_agents(
    host: &mut WidgetHostNative,
    specs: Vec<SpawnSpec>,
) -> Vec<SubAgentSession> {
    let identities = assign_agent_identities(specs.len());
    let mut subs = Vec::with_capacity(specs.len());

    for (spec, identity) in specs.into_iter().zip(identities) {
        // A fresh provider + tool channel per sub; skip if no ready
        // builtin model is configured (same gate as the parent loop).
        let Some((provider, tool_rx)) = builtin_provider_with_design_tools(host) else {
            continue;
        };

        let system_prompt = build_sub_agent_prompt(&spec);
        // Same design-turn thinking policy as the single design-agent loop:
        // reasoning models that would burn their budget on `<think>` and draw
        // nothing are forced thinking-off (see `design_turn_thinking_mode`).
        let thinking = chat_session::design_turn_thinking_mode(host);
        let effort = host.editor_state().chat.effort_level;
        let req = ChatRequest {
            system_prompt,
            user_message: spec.prompt.clone(),
            history: vec![],
            max_output_tokens: 8192,
            thinking,
            effort,
            attachments: vec![],
            model: None,
        };
        let session = ChatSession::start_with_tools(provider, req, Some(tool_rx));

        subs.push(SubAgentSession {
            session: Some(session),
            identity,
            indicator: None,
        });
    }

    let n = subs.len();
    host.editor_state_mut().chat.agents_running = (n, n);
    subs
}

/// Abort every sub-agent loop (MT.3 close-of-running-tab): end the active
/// sub's live indicator epoch so the canvas badge glow doesn't get stuck,
/// drop all sessions, and reset the cursor. The caller is expected to clear
/// `agents_running` on the affected tab separately (here the tab is being
/// removed, so its badge state goes away with it).
pub(crate) fn abort_all(subs: &mut Vec<SubAgentSession>, active: &mut usize) {
    if let Some(sub) = subs.get(*active) {
        if let Some(ind) = sub.indicator.as_ref() {
            agent_indicators::end_if_epoch(ind.epoch);
        }
    }
    subs.clear();
    *active = 0;
}

// ---------------------------------------------------------------------------
// Sequential pump
// ---------------------------------------------------------------------------

/// Pump the single active sub-agent (sequential execution), badge its new
/// frames, and advance when it finishes. Runs every frame from
/// `app_handler` AFTER the parent pump + `pump_indicator`.
///
/// Borrow handling: the active session is `std::mem::take`n out of the sub
/// before [`chat_session::pump`] runs against `host`, then put back — so
/// `host` and `subs[active].session` are never borrowed at the same time.
///
/// Lifecycle per frame:
/// - Pump the active sub. Its tool calls apply to the shared `EditorState`
///   via the same UI-thread path the parent uses, so canvas writes merge
///   automatically.
/// - Register any new frames under the active sub's epoch/identity (badge).
/// - When the active sub's session has finished, end its epoch and advance
///   to the next; decrement `agents_running.0`.
/// - When all subs are done, clear the `Vec`, reset `active`, set
///   `agents_running = (0, 0)`.
///
/// Returns true when anything changed (caller redraws).
pub(crate) fn pump_sub_agents(
    host: &mut WidgetHostNative,
    subs: &mut Vec<SubAgentSession>,
    active: &mut usize,
    running_tab: Option<usize>,
) -> bool {
    if subs.is_empty() {
        return false;
    }

    let mut changed = false;

    if *active < subs.len() {
        // Lazily begin this sub's indicator epoch the first frame it is
        // active. Sequential execution keeps exactly one epoch live, so an
        // earlier sub's badges aren't clobbered (see `SubAgentSession`).
        // The initial-frame snapshot is taken NOW (after prior subs
        // finished), so only THIS sub's new frames get badged.
        if subs[*active].indicator.is_none() {
            let epoch = agent_indicators::begin();
            let initial_frame_ids = collect_top_level_frame_ids(host.editor_state());
            let identity = subs[*active].identity.clone();
            subs[*active].indicator = Some(DesignLoopIndicator {
                epoch,
                color: identity.color,
                name: identity.name,
                initial_frame_ids,
            });
        }

        // Take the active session out so `host` and the session aren't
        // borrowed simultaneously while pumping.
        let mut session = subs[*active].session.take();
        // Mark sub-agent context so a nested `spawn_agents` from this
        // sub's tool loop is refused by the interception (no recursion).
        SUB_AGENT_ACTIVE.store(true, Ordering::SeqCst);
        // Sub-agent transcript deltas bind to the same tab the parent design
        // loop started on (MT.3 session-per-tab).
        let pumped = chat_session::pump(host, &mut session, running_tab);
        SUB_AGENT_ACTIVE.store(false, Ordering::SeqCst);
        if pumped {
            changed = true;
        }
        // Put it back (pump sets it to `None` when the turn finishes).
        subs[*active].session = session;

        // Badge any frames the active sub just created.
        if let Some(ind) = subs[*active].indicator.as_ref() {
            register_new_frames(ind, host.editor_state());
        }

        // Advance when the active sub finished its turn. Natural finish:
        // queued reveals drain gracefully before the overlay clears.
        if subs[*active].session.is_none() {
            if let Some(ind) = subs[*active].indicator.as_ref() {
                agent_indicators::finish_if_epoch(ind.epoch);
            }
            *active += 1;
            // Reflect one fewer running agent in the N/M header.
            let total = host.editor_state().chat.agents_running.1;
            let remaining = subs.len().saturating_sub(*active);
            host.editor_state_mut().chat.agents_running = (remaining, total);
            changed = true;
        }
    }

    // All subs finished — clear and reset the counter.
    if *active >= subs.len() {
        subs.clear();
        *active = 0;
        host.editor_state_mut().chat.agents_running = (0, 0);
        changed = true;
    }

    changed
}

// ---------------------------------------------------------------------------
// Spec parsing for the interception branch
// ---------------------------------------------------------------------------

/// Parse a `spawn_agents` tool call's raw `args_json` into specs.
///
/// The model emits the tool args as a JSON object whose `config` field is
/// the agent array. `op_mcp::spawn_agents_tool::parse_spawn_config` reads
/// `config` as a JSON-string, so we normalise: extract the `config` value,
/// re-serialise an array to a string (a string value is used verbatim),
/// and dispatch into the shared validator.
pub(crate) fn parse_spawn_args(args_json: &str) -> Result<Vec<SpawnSpec>, String> {
    use std::collections::BTreeMap;

    let value: serde_json::Value = serde_json::from_str(args_json.trim())
        .map_err(|e| format!("spawn_agents args must be a JSON object: {e}"))?;

    let config = value
        .get("config")
        .ok_or_else(|| "spawn_agents requires a non-empty config array".to_string())?;

    let config_str = match config {
        // Already a JSON-string of the array — use verbatim.
        serde_json::Value::String(s) => s.clone(),
        // An array/object — re-serialise to the string form the
        // validator expects.
        other => other.to_string(),
    };

    let mut args = BTreeMap::new();
    args.insert("config".to_string(), config_str);
    op_mcp::spawn_agents_tool::parse_spawn_config(&args)
}

#[cfg(test)]
#[path = "sub_agent_session_tests.rs"]
mod tests;
