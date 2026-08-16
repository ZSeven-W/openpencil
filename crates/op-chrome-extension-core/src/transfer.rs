//! Chunked snapshot readback: slice planning, integrity verification, and
//! incremental truncation detection.
//!
//! Snapshot text is pulled back from the tab in slices instead of one giant
//! return value: an extractor result can reach tens of MB (it embeds
//! rasterized images up to 24 MB) and `chrome.scripting.executeScript` has to
//! structured-clone the result across a process boundary. 4 MiB per hop stays
//! far below any IPC limit while keeping the number of round trips small
//! (12 for a maximum 48 MB page).
//!
//! # Why the snapshot text never leaves JS
//!
//! Slicing is by UTF-16 index, so a slice boundary can fall between the two
//! halves of a surrogate pair. JS concatenation rejoins such a pair exactly.
//! A Rust `String` cannot hold a lone surrogate, so a chunk crossing the
//! wasm-bindgen boundary has any unpaired half rewritten to `U+FFFD` — which
//! would silently corrupt the snapshot if the reassembled text came back out
//! of wasm. The joined text therefore stays on the JS side, and this module
//! owns everything else: which slice to ask for next, whether the answer was
//! acceptable, whether the total adds up, and whether the payload carries the
//! extractor's `truncated` flag.
//!
//! The same rule governs the `/mcp` request body: the snapshot is embedded
//! there as a JSON string literal, and it is JS — not this crate — that does
//! the embedding, so a lone surrogate the page produced survives as the
//! `\udXXX` escape `JSON.stringify` emits. The Rust side hands over the
//! envelope *shape* only; see [`crate::ingress::mcp_envelope_template`].
//!
//! Slices still cross the boundary, but only to be scanned, and the `U+FFFD`
//! substitution is harmless for that. The authoritative slice length is the
//! `String.prototype.length` JS measured on the intact slice and passes in
//! alongside it, and the truncation pattern is pure ASCII, so a mangled
//! astral character can neither create nor destroy a match.

/// UTF-16 code units pulled back per `executeScript` round trip.
pub const CHUNK_CHARS: u32 = 4 * 1024 * 1024;

/// Mirror of the ingest route's own body cap
/// (`mcp_live::snapshot_ingest::MAX_SNAPSHOT_BODY`), in megabytes. The server
/// refuses a larger declared `Content-Length` with `413` before reading
/// anything — but that refusal is scoped to the accepted origin and a browser
/// will not let us read a body we were not granted, so an over-cap POST would
/// surface as an opaque network failure. Checking client-side turns it into
/// the one message that helps: use `Download JSON` and import the file.
pub const MAX_SNAPSHOT_MB: u32 = 48;

/// Give up on an HTTP request after this long. Without it a server that
/// accepts the connection and then never answers (a wedged editor, a socket
/// black hole) leaves the popup spinning with its buttons disabled until the
/// user closes it. Sized ABOVE the editor's own 15 s UI-thread ack budget —
/// they must not be equal, or a maximum-size import that uses the editor's
/// whole budget times out on both ends at once and the client error races
/// the server's honest one.
pub const REQUEST_TIMEOUT_MS: u32 = 30_000;

/// Whether a snapshot of `chars` UTF-16 code units is over the ingest cap.
///
/// UTF-16 length under-counts UTF-8 bytes for non-ASCII text and is never
/// larger, so this only ever rejects captures the server would reject too; a
/// borderline non-ASCII one still gets the server's 413.
pub fn snapshot_too_large(chars: f64) -> bool {
    chars > f64::from(MAX_SNAPSHOT_MB) * 1024.0 * 1024.0
}

/// The editor endpoint's whole-body cap (`mcp_serve::MAX_BODY`), in
/// megabytes. Bounds the `/mcp` FALLBACK envelope, not the raw snapshot:
/// `tools/call` wraps the snapshot as a JSON string, and the re-escaping of
/// quotes/backslashes can inflate a snapshot that passed
/// [`snapshot_too_large`] past this outer limit (worst case approaches 2×,
/// so a 33+ MiB quote-heavy capture can exceed 64 MiB). Checked client-side
/// after the envelope is built so the failure is the actionable `tooLarge`
/// message instead of a post-upload refusal.
pub const MCP_FALLBACK_BODY_MB: u32 = 64;

/// Whether an `/mcp` envelope of `chars` UTF-16 code units is over that cap.
pub fn mcp_envelope_too_large(chars: f64) -> bool {
    chars > f64::from(MCP_FALLBACK_BODY_MB) * 1024.0 * 1024.0
}

/// Plan + verify the slice-by-slice readback of one snapshot.
///
/// Drive it as: while `!done()`, ask the tab for `next_length()` units at
/// `next_offset()`, then hand the answer to [`ChunkPlan::accept`]. When
/// `done()`, call [`ChunkPlan::verify_total`] with the length of the joined
/// string. Any `false` return means the capture was lost mid-transfer (the
/// tab navigated, the parked global went away, a slice came back short) and
/// the caller must report `chunkLost` rather than use a partial snapshot.
#[derive(Debug)]
pub struct ChunkPlan {
    expected: u32,
    transferred: u32,
    chunk_size: u32,
    scanner: TruncationScanner,
    failed: bool,
}

impl ChunkPlan {
    /// Plan a readback of `expected` UTF-16 code units.
    pub fn new(expected: u32) -> Self {
        Self {
            expected,
            transferred: 0,
            chunk_size: CHUNK_CHARS,
            scanner: TruncationScanner::new(),
            failed: false,
        }
    }

    /// Offset of the next slice to request.
    pub fn next_offset(&self) -> u32 {
        self.transferred
    }

    /// Maximum length of the next slice to request.
    pub fn next_length(&self) -> u32 {
        self.chunk_size
    }

    /// Units accepted so far.
    pub fn transferred(&self) -> u32 {
        self.transferred
    }

    /// Whether every expected unit has arrived.
    pub fn done(&self) -> bool {
        !self.failed && self.transferred >= self.expected
    }

    /// Whether the payload seen so far carries `"truncated": true`.
    pub fn truncated(&self) -> bool {
        self.scanner.found()
    }

    /// Record one slice.
    ///
    /// `chunk` is the slice text (used only to scan for the truncation flag)
    /// and `chunk_chars` is the length JS measured on it — negative when the
    /// tab answered with something that was not a string at all. Returns
    /// `false` on a lost or nonsensical slice; the plan then stays failed.
    ///
    /// A length that is not a whole number ≥ 1 is refused outright. Only
    /// `String.prototype.length` is ever supposed to arrive here, so anything
    /// else means the caller is not the caller this was written for — and a
    /// fractional value in `(0, 1)` would truncate to zero units transferred
    /// and spin the read loop forever, which is the one failure mode this
    /// type exists to make impossible.
    pub fn accept(&mut self, chunk: &str, chunk_chars: f64) -> bool {
        // The finiteness test comes first so a NaN length — which no sane
        // caller produces, but which would otherwise floor to 0 and spin the
        // read loop forever — lands in the failure arm rather than sneaking
        // past a `< 1.0` comparison that NaN always answers `false` to.
        if self.failed
            || !chunk_chars.is_finite()
            || chunk_chars < 1.0
            || chunk_chars.fract() != 0.0
        {
            self.failed = true;
            return false;
        }
        // An over-long slice means the tab is not answering the question we
        // asked; treat it as a lost capture rather than trusting the surplus.
        let chars = chunk_chars as u64;
        if chars > u64::from(self.chunk_size)
            || u64::from(self.transferred) + chars > u64::from(self.expected)
        {
            self.failed = true;
            return false;
        }
        self.transferred += chars as u32;
        self.scanner.push(chunk);
        true
    }

    /// Final integrity check: the joined text must be exactly as long as the
    /// tab said the snapshot was.
    pub fn verify_total(&self, total_chars: u32) -> bool {
        !self.failed && total_chars == self.expected
    }
}

/// Incremental matcher for the extractor's top-level `"truncated": true`.
///
/// Matched with a pattern that accepts both the indented form the asset emits
/// today (`"truncated": true`) and a compact one, so a change to the asset's
/// formatting cannot silently turn the notice off.
///
/// Deliberately not a JSON parse: nothing else in the extension parses the
/// snapshot (it is POSTed as an opaque body, and it is the Rust importer that
/// parses it), so a parse here would be a second full traversal plus a full
/// object graph in memory for a payload that routinely runs to tens of MB —
/// to read one boolean. The cost of a false positive is one extra "capture is
/// partial" line.
///
/// Being incremental is what keeps peak memory bounded: each 4 MiB slice is
/// scanned as it arrives and dropped, so the whole snapshot never has to
/// exist inside the wasm heap.
#[derive(Debug, Default)]
struct TruncationScanner {
    state: MatchState,
    found: bool,
}

/// The literal being matched before the colon.
///
/// Its only interior repeat of its own prefix is the closing quote, so the
/// whole KMP failure function collapses to two cases: a mismatch anywhere
/// inside the key re-anchors at [`TruncationScanner::restart`], and a
/// mismatch *after* the key completed falls back to `Key(1)` — the key's
/// single proper border, the opening quote that its last character also is.
/// See [`TruncationScanner::step`]'s `PreColon` arm.
const KEY: &[u8] = b"\"truncated\"";
const TRUE_LITERAL: &[u8] = b"true";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchState {
    /// Matching [`KEY`], `usize` characters in.
    Key(usize),
    /// Key matched; skipping whitespace, expecting `:`.
    PreColon,
    /// Colon seen; skipping whitespace, expecting `true`.
    PostColon,
    /// Matching [`TRUE_LITERAL`], `usize` characters in.
    True(usize),
}

impl Default for MatchState {
    fn default() -> Self {
        MatchState::Key(0)
    }
}

impl TruncationScanner {
    fn new() -> Self {
        Self::default()
    }

    fn found(&self) -> bool {
        self.found
    }

    fn push(&mut self, text: &str) {
        if self.found {
            return;
        }
        for c in text.chars() {
            if self.step(c) {
                self.found = true;
                return;
            }
        }
    }

    /// Feed one character; returns true when the pattern just completed.
    fn step(&mut self, c: char) -> bool {
        match self.state {
            MatchState::Key(i) => {
                if byte_at(KEY, i) == Some(c) {
                    let next = i + 1;
                    self.state = if next == KEY.len() {
                        MatchState::PreColon
                    } else {
                        MatchState::Key(next)
                    };
                } else {
                    self.restart(c);
                }
            }
            MatchState::PreColon => {
                if c == ':' {
                    self.state = MatchState::PostColon;
                } else if !crate::js_text::is_js_space(c) {
                    // The key just matched in full, so its last character was
                    // the quote that also begins it: the longest prefix still
                    // live is `"`, i.e. `Key(1)`. Dropping all the way back to
                    // `Key(0)` would lose that quote and miss
                    // `{"truncated"truncated": true}` — the second key starts
                    // at the quote the first one ended with. Re-run this
                    // character against `KEY[1]`; `Key` is the only state the
                    // recursion can reach, and it never recurses again.
                    self.state = MatchState::Key(1);
                    return self.step(c);
                }
            }
            MatchState::PostColon => {
                if c == 't' {
                    self.state = MatchState::True(1);
                } else if !crate::js_text::is_js_space(c) {
                    self.restart(c);
                }
            }
            MatchState::True(i) => {
                if byte_at(TRUE_LITERAL, i) == Some(c) {
                    let next = i + 1;
                    if next == TRUE_LITERAL.len() {
                        self.state = MatchState::default();
                        return true;
                    }
                    self.state = MatchState::True(next);
                } else {
                    self.restart(c);
                }
            }
        }
        false
    }

    /// Re-anchor after a mismatch *inside* an unfinished match.
    ///
    /// The only character that can begin a fresh match is `"`, and the key
    /// carries no other `"` before its last, so no partial match of the key
    /// can end with a live prefix longer than that one quote: restarting at
    /// "one character in" whenever the mismatching character is itself a
    /// quote is exact for these states. The one state this does NOT cover is
    /// [`MatchState::PreColon`], reached only after the key matched in full —
    /// there the last consumed character *was* the closing quote and must be
    /// kept; that arm handles its own fallback.
    fn restart(&mut self, c: char) {
        self.state = if c == '"' {
            MatchState::Key(1)
        } else {
            MatchState::default()
        };
    }
}

/// ASCII byte `i` of `literal` as a `char`, or `None` past the end.
fn byte_at(literal: &[u8], i: usize) -> Option<char> {
    literal.get(i).map(|b| *b as char)
}
