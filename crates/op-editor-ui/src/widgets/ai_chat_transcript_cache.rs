//! Per-frame transcript layout cache.
//!
//! The transcript layout (`build_item` per message) is a pure function of the
//! message list + body rect + locale. It does NOT depend on the scroll offset
//! (scroll only shifts every rect vertically) nor on design-hover (hover only
//! toggles the paint-time `copy_visible` affordance, never a rect). Previously
//! the paint path rebuilt the whole transcript twice per frame — once for
//! `transcript_content_height`, once for `paint_transcript_with_selection` — and
//! every mouse-move hit-test / selection probe rebuilt it again. During
//! chat/codegen streaming the panel repaints continuously, so this was the
//! hottest UI path (per-message `wrap_units` Vecs, i18n `.to_string()`,
//! `.replace()`, title clones on every message, on-screen or not).
//!
//! This caches ONE canonical build (laid out from the body top, scroll 0) plus
//! its total content height in a thread-local, keyed on a cheap fingerprint of
//! every layout input. Height, paint, hit-test and selection all read the same
//! cached build; paint applies the scroll via a `translate` and the hit-tests
//! shift the query point, so scroll is deliberately excluded from the key (a
//! scroll-only change — the common case while dragging the wheel or moving the
//! mouse — still hits the cache).
//!
//! The key changes whenever ANY layout-relevant field of ANY message changes —
//! including a streaming message whose `content`/`thinking` grows each frame — so
//! a streaming turn never serves stale layout. There is no revision counter on
//! `ChatMessage`, so the fingerprint hashes the fields `build_item` consumes.
//!
//! ## Owner-scoping protocol (the FULL rules)
//!
//! The single slot carries an `owner` id so the 0-hash display-frame hint
//! ([`with_current_canonical`]) never pairs one panel's cached geometry with a
//! different panel's live messages after a tab / host / session switch. The rules
//! are deliberately simple — "the last resolver owns the slot":
//!
//! 1. **Every resolve sets the owner** — on a fresh rebuild AND on a cache hit
//!    ([`cached_canonical_transcript_owned`]). Whichever panel most recently
//!    resolved the slot owns it. This is what lets a later panel *claim* a slot
//!    that an earlier (even byte-identical) panel or an accidental UNOWNED resolve
//!    left behind: a stale owner can never permanently block a subsequent claim.
//! 2. **UNOWNED (`0`) can never read.** [`with_current_canonical`] with `owner ==
//!    UNOWNED` ALWAYS yields `f(None)` — the sentinel is not a real panel, so an
//!    unstamped resolve must not be able to read a build back. A read only sees
//!    the slot when `slot_owner == owner` and `owner != UNOWNED`.
//! 3. **After a switch the hint reads `None` until the new panel's first paint.**
//!    A host rotates its `chat_panel_owner` when the active chat session changes.
//!    Rotation happens at TWO layers: (a) synchronously at every host-side
//!    session-mutation site (tab switch / new tab / close tab) via
//!    `force_rotate_chat_owner`, so a `CursorMoved` arriving before the next
//!    paint cannot pair the previous session's geometry with the new session's
//!    messages, AND (b) as an index-poll backstop
//!    (`rotate_chat_owner_if_session_changed`) at the paint / cursor-probe entry,
//!    for mutation paths that bypass the host. The fresh owner does not match the
//!    slot until that panel next paints (which resolves + re-tags), so the cursor
//!    hint is the default arrow for one frame — the documented, intended
//!    isolation.
//!
//! ### Residual coverage gap (documented honestly)
//!
//! The index poll compares only `chat.active_index()`, and `ChatSessions`
//! carries no stable per-session id / creation counter (op-editor-core is out of
//! scope to grow one). A mutation that REPLACES the session at the same numeric
//! index therefore slips past the poll. The forced rotation at host mutation
//! sites closes this for every host-driven case (close active tab 0 → next
//! session installed at index 0; close the sole tab → replaced in place). The
//! remaining uncovered case is a session swap that does NOT flow through a host
//! mutation site AND leaves the index unchanged (e.g. a hypothetical MCP-driven
//! session replace-at-same-index; none exists in the repository today). The
//! desktop-side ⌘T `new_chat_tab` / tab-close `close_chat_tab` mutators rotate
//! synchronously via the public `WidgetHostNative::force_rotate_chat_owner`.
//! The uncovered case is benign display-only: the worst outcome is a one-frame stale cursor
//! shape that self-corrects on the next paint — never a panic or a wrong hit.
//!
//! Only production entry points ever supply a real owner; the sole UNOWNED
//! resolve is [`unowned_for_tests`], which is `#[cfg(test)]`-only precisely so a
//! future production call site cannot silently resolve without an owner. A
//! `debug_assert!` on [`cached_canonical_transcript_owned`] (active only in
//! dev builds of the real host apps — see its body) is the runtime backstop.
//!
//! Precedent: `op-pen-loader/src/measure_cache.rs` (thread-local memo backend)
//! and the layer panel's `visible_row_range` (paint virtualization).

use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use op_editor_core::chat::{ChatMessage, ChatRole};

use super::ai_chat_transcript::{build_item, TranscriptItem, MSG_GAP};
use crate::Rect;

/// Process-global allocator of opaque per-panel-instance owner ids. Each host's
/// persistent widget-host state pulls ONE id at construction (and ROTATES it via
/// [`next_panel_owner`] when the active chat session changes), then passes it to
/// every transcript-cache resolve, so the single thread-local cache slot is
/// SCOPED to whichever panel instance last resolved it. A different panel — a
/// tab/host/session switch, or two tests sharing a worker thread — therefore
/// never cross-pairs one transcript's cached geometry with another's live
/// messages via the 0-hash [`with_current_canonical`] hint path.
///
/// `0` is reserved as the `UNOWNED` sentinel (the default an un-scoped
/// construction carries), so the counter starts at `1` and never hands it out.
static NEXT_PANEL_OWNER: AtomicU64 = AtomicU64::new(1);

/// Sentinel owner for panels that were built without an explicit host-supplied
/// id (plain `from_editor` / `from_editor_at` — used by unit tests and any
/// path that never touches the display-frame hint). Distinct from every id
/// [`next_panel_owner`] returns, and [`with_current_canonical`] refuses to read
/// the slot under it, so an unstamped resolve can never read a build back.
pub(crate) const UNOWNED: u64 = 0;

/// Allocate a fresh, process-unique panel-owner id. Called once per persistent
/// widget-host instance; the returned id is stable for that host's lifetime.
pub(crate) fn next_panel_owner() -> u64 {
    NEXT_PANEL_OWNER.fetch_add(1, Ordering::Relaxed)
}

/// A canonical (scroll-0) transcript layout plus its total content height.
/// Every consumer derives its view from this: paint translates by the scroll,
/// hit-tests shift the query point, height reads `total_height` directly.
pub(crate) struct CanonicalTranscript {
    pub items: Vec<TranscriptItem>,
    pub total_height: f32,
}

/// Lay out the full transcript once from the body top (scroll 0) and record its
/// total content height. This is the single build every per-frame consumer
/// shares: paint translates it by the scroll, hit-tests shift the query point,
/// and `transcript_content_height` reads `total_height`.
fn build_canonical_transcript(
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
) -> CanonicalTranscript {
    if messages.is_empty() {
        return CanonicalTranscript {
            items: Vec::new(),
            total_height: 0.0,
        };
    }
    let mut items = Vec::with_capacity(messages.len());
    let mut top = body_rect.origin.y;
    for (i, msg) in messages.iter().enumerate() {
        let (item, bottom) = build_item(msg, i, top, body_rect, locale);
        items.push(item);
        top = bottom + MSG_GAP;
    }
    // `top` overshot by one MSG_GAP after the last message.
    let total_height = (top - MSG_GAP - body_rect.origin.y).max(0.0);
    CanonicalTranscript {
        items,
        total_height,
    }
}

/// Identity of a cached canonical build. Scroll offset and design-hover are
/// intentionally absent — both are applied by consumers without a rebuild.
#[derive(PartialEq)]
struct CanonicalKey {
    fingerprint: u64,
    msg_count: usize,
    body_x: u32,
    body_y: u32,
    body_w: u32,
    body_h: u32,
    locale: op_editor_core::Locale,
}

impl CanonicalKey {
    fn new(messages: &[ChatMessage], body: Rect, locale: op_editor_core::Locale) -> Self {
        Self {
            fingerprint: fingerprint_messages(messages),
            msg_count: messages.len(),
            body_x: body.origin.x.to_bits(),
            body_y: body.origin.y.to_bits(),
            body_w: body.size.x.to_bits(),
            body_h: body.size.y.to_bits(),
            locale,
        }
    }
}

fn role_tag(role: ChatRole) -> u8 {
    match role {
        ChatRole::User => 0,
        ChatRole::Assistant => 1,
    }
}

/// Cheap SipHash fingerprint over EVERY field `build_item` reads. Image *bytes*
/// are excluded — layout only depends on the thumbnail count, and paint reads
/// the live `messages` slice for the pixels — but everything that can move a
/// rect or change a wrapped line count is folded in. A 64-bit SipHash makes a
/// stale-serving collision astronomically unlikely; `msg_count` is stored
/// alongside as a second, structural discriminator.
fn fingerprint_messages(messages: &[ChatMessage]) -> u64 {
    // Count every full-transcript hash so tests can prove each paint pass and
    // input event fingerprints the transcript at most once (the accepted floor)
    // rather than once per helper it fans out to.
    FINGERPRINT_COUNT.with(|c| c.set(c.get() + 1));
    let mut h = DefaultHasher::new();
    messages.len().hash(&mut h);
    for m in messages {
        role_tag(m.role).hash(&mut h);
        m.content.hash(&mut h);
        m.thinking.hash(&mut h);
        m.tool_calls.len().hash(&mut h);
        for tc in &m.tool_calls {
            tc.name.hash(&mut h);
            tc.args.hash(&mut h);
            // The interleaved flow keys card POSITIONS off the offset — a
            // late-stamped offset must invalidate the cached build.
            tc.content_offset.hash(&mut h);
        }
        m.activities.hash(&mut h);
        // Only the thumbnail count feeds layout; the bytes are painted from the
        // live slice, so hashing the length is both sufficient and cheap.
        m.images.len().hash(&mut h);
        m.thinking_collapsed.hash(&mut h);
        m.tools_collapsed.hash(&mut h);
        m.tool_call_expanded_overrides.hash(&mut h);
        m.design_block_expanded_overrides.hash(&mut h);
        m.action_step_expanded_overrides.hash(&mut h);
        m.streaming.hash(&mut h);
        // Sub-agent identity header adds a layout row — its arrival
        // mid-run must invalidate the cached build.
        m.agent_name.hash(&mut h);
        m.agent_color.hash(&mut h);
    }
    h.finish()
}

thread_local! {
    /// The single canonical build slot, tagged with the `owner` of whichever
    /// panel instance last resolved it. EVERY resolve (rebuild or cache hit)
    /// re-stamps this owner — "the last resolver owns the slot" — so a stale or
    /// UNOWNED owner can never permanently block a later panel's claim. The
    /// display-frame hint read ([`with_current_canonical`]) then serves the slot
    /// only to that same owner (and never to UNOWNED).
    static CACHE: RefCell<Option<(u64, CanonicalKey, Rc<CanonicalTranscript>)>> =
        const { RefCell::new(None) };
    /// Observable rebuild counter — increments only when a fresh build runs.
    /// Lets tests prove a cache hit does not recompute.
    static BUILD_COUNT: Cell<u64> = const { Cell::new(0) };
    /// Observable fingerprint counter — increments on every full-transcript
    /// hash (whether or not it hits the cache). Lets tests prove the paint and
    /// hit/selection entry points resolve the canonical build once per pass and
    /// thread it down, instead of re-hashing in each helper.
    static FINGERPRINT_COUNT: Cell<u64> = const { Cell::new(0) };
}

/// TEST-ONLY unscoped resolve: forwards the [`UNOWNED`] sentinel so the stored
/// build can never be read back via the display-frame hint (which refuses
/// UNOWNED). Named `*_for_tests` and gated `#[cfg(test)]` on purpose — there is
/// no production-nameable unowned resolve, so a future host call site cannot
/// silently resolve without an owner; it MUST go through
/// [`cached_canonical_transcript_owned`] with a real owner (supplied by
/// `AIChatPlaceholder::owned_by`). The `Rc` is cloned out of the cell so no
/// borrow is held while the caller iterates.
#[cfg(test)]
pub(crate) fn unowned_for_tests(
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
) -> Rc<CanonicalTranscript> {
    cached_canonical_transcript_owned(UNOWNED, messages, body_rect, locale)
}

/// Owner-scoped resolve. The resulting slot is ALWAYS tagged with `owner` —
/// whether this call rebuilt or hit the cache — so the most recent resolver owns
/// the slot ("last resolver owns"). That is what makes ownership claimable: a
/// later panel that resolves an identical (byte-for-byte) key still takes the
/// slot from whoever painted it, and an accidental UNOWNED resolve can never
/// leave the slot permanently unreadable to a real panel. The owner's later
/// [`with_current_canonical`] hint read then matches; a different panel's does
/// not. Panels pass their host-supplied `AIChatPlaceholder::owner`; the sole
/// [`unowned_for_tests`] wrapper passes [`UNOWNED`] for test callers.
pub(crate) fn cached_canonical_transcript_owned(
    owner: u64,
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
) -> Rc<CanonicalTranscript> {
    // Enforcement: a real resolve must carry a real panel owner. The only
    // UNOWNED-forwarding caller is `unowned_for_tests` (`#[cfg(test)]`), so a
    // resolve reaching here with UNOWNED in a real build means a host construction
    // site forgot `.owned_by(chat_panel_owner)` — the display-frame hint would
    // then be unreadable for that panel forever. `not(test)` gates this OUT of
    // op-editor-ui's own 269 unit tests (which legitimately resolve default,
    // UNOWNED panels); `debug_assertions` gates it out of release. It therefore
    // fires only in dev builds of the real host apps (op-host-native/-web/-desktop).
    #[cfg(all(debug_assertions, not(test)))]
    debug_assert!(
        owner != UNOWNED,
        "transcript-cache resolve reached with the UNOWNED sentinel — a host \
         build site is missing `.owned_by(chat_panel_owner)`"
    );
    let key = CanonicalKey::new(messages, body_rect, locale);
    CACHE.with(|cell| {
        if let Some((slot_owner, cached_key, cached)) = cell.borrow_mut().as_mut() {
            // Serve any key-matching build regardless of the previous owner
            // (identical inputs produce an identical layout) AND re-stamp the
            // slot to the resolving owner: the last resolver owns it. This is the
            // protocol that lets a later panel claim the slot and defuses any
            // deadlock where a stale/UNOWNED owner would otherwise block reads.
            if *cached_key == key {
                *slot_owner = owner;
                return cached.clone();
            }
        }
        let built = Rc::new(build_canonical_transcript(messages, body_rect, locale));
        BUILD_COUNT.with(|c| c.set(c.get() + 1));
        *cell.borrow_mut() = Some((owner, key, built.clone()));
        built
    })
}

/// Read-only accessor over the currently STORED canonical build, scoped to
/// `owner`.
///
/// Invokes `f` with the build most recently resolved on this thread BY THE SAME
/// panel instance (`owner`) — which, because paint resolves the canonical every
/// frame, is the LAST PAINTED layout — together with its key fingerprint. It
/// returns `f(None)` when nothing has been built yet, when the stored build
/// belongs to a DIFFERENT panel (`slot_owner != owner`), OR when `owner` is the
/// [`UNOWNED`] sentinel (which never names a real panel, so an unstamped resolve
/// must not be able to read a build back). The owner guard is what restores the
/// "None before THIS panel's first resolve" contract and stops the 0-hash hint
/// path from pairing another transcript's cached geometry with this panel's live
/// messages after a tab/host/session switch.
///
/// This is a PURE READ: it hashes nothing, never rebuilds, and never touches
/// `FINGERPRINT_COUNT` / `BUILD_COUNT`.
///
/// Semantics callers accept: the result reflects the last-resolved (= displayed)
/// frame, not a freshly hashed snapshot of the live message list. Hit-testing a
/// cursor hint against this build is the semantically correct choice — the user
/// points at what is on screen — and any staleness self-corrects on the next
/// paint, which re-resolves the canonical. Use this for cheap display-frame
/// hit-tests (e.g. the native cursor-shape hint); use
/// [`unowned_for_tests`] when you need the build for the *live* inputs.
///
/// The `Rc` is cloned out of the cell (a refcount bump, still no hashing) so no
/// borrow is held while `f` runs.
pub(crate) fn with_current_canonical<R>(
    owner: u64,
    f: impl FnOnce(Option<(u64, &CanonicalTranscript)>) -> R,
) -> R {
    // The UNOWNED sentinel is never a real panel: it must never read the slot,
    // so an unstamped resolve (owner defaulted to UNOWNED) can't pair a build
    // back with live messages. This is a hard guard, above the slot lookup.
    if owner == UNOWNED {
        return f(None);
    }
    let current = CACHE.with(|cell| {
        cell.borrow().as_ref().and_then(|(slot_owner, key, build)| {
            (*slot_owner == owner).then(|| (key.fingerprint, build.clone()))
        })
    });
    match current {
        Some((fingerprint, build)) => f(Some((fingerprint, &build))),
        None => f(None),
    }
}

/// Number of fresh canonical builds performed so far on this thread — a
/// monotonic counter used by tests to assert cache hits do not recompute.
#[cfg(test)]
pub(crate) fn transcript_build_count() -> u64 {
    BUILD_COUNT.with(Cell::get)
}

/// Number of full-transcript fingerprints computed so far on this thread — a
/// monotonic counter used by tests to assert an entry point hashes the
/// transcript at most once per paint pass / input event.
#[cfg(test)]
pub(crate) fn transcript_fingerprint_count() -> u64 {
    FINGERPRINT_COUNT.with(Cell::get)
}
