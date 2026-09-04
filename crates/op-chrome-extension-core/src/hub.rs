//! The account inbox: the `POST /api/v1/snapshots` create request.
//!
//! This is the second delivery target, and the only one that leaves the
//! machine. The Hub's contract is pinned here, field for field, from
//! `op-hub/backend/internal/`:
//!
//! | What | Where it is decided in op-hub |
//! | --- | --- |
//! | route + method | `httpapi/snapshot_routes.go:20,71-86` |
//! | auth: session cookie + role | `httpapi/snapshot_routes.go:72` → `auth/middleware.go:11-51` |
//! | auth: `Origin` + `X-CSRF-Token` | `httpapi/snapshot_routes.go:285-304`, `auth/service.go:139-176` |
//! | `Content-Type: application/json` (charset=utf-8 optional) | `httpapi/snapshot_routes.go:389-405` |
//! | no query string on the target | `httpapi/router.go:282-285` |
//! | body cap 32 MiB | `httpapi/snapshot_routes.go:26`, `httpapi/security.go:59-65` |
//! | envelope: exactly 5 fields, no duplicates, no trailing bytes | `snapshots/decode.go:30-117` |
//! | `name` ≤ 200 runes, trimmed, no control or bidi characters | `snapshots/validation.go:73-105` |
//! | `source_url` ≤ 2048 bytes, `http(s)` only, or absent | `snapshots/validation.go:107-124` |
//! | `captured_at` RFC 3339 **UTC**, ±24 h of server time | `snapshots/decode.go:129-142`, `validation.go:128-137` |
//! | quota 50 items / 200 MiB, retention 30 d, 20 uploads/h | `snapshots/types.go:102-119` |
//!
//! # Why the snapshot is spliced in by JavaScript, again
//!
//! Same reasoning as [`crate::ingress::mcp_envelope_template`], with one
//! difference that matters: the Hub wants the document as a JSON **object**,
//! not as a string. So JavaScript splices the extractor's raw JSON text in
//! rather than the result of `JSON.stringify`.
//!
//! That is safe, and it is the Hub's strict decoder that makes it safe rather
//! than any assumption about what the extractor emits. [`create_envelope_template`]
//! always emits exactly five fields — `source_url` is written as `null` when
//! there is nothing to send, never omitted — so the envelope's field count is
//! a constant. `decodeEnvelope` refuses a sixth field, refuses a duplicate
//! key, and refuses trailing content after the closing brace. Text spliced
//! into the `snapshot` slot can therefore only produce a valid document or a
//! `400`; it cannot add, replace, or shadow `name`, `source_url`, `kind` or
//! `captured_at`. Everything else in the envelope is built here and escaped
//! through `op_util::json_escape`.
//!
//! The alternative — `JSON.stringify(JSON.parse(snapshot))` — would parse a
//! 32 MiB document into a JS object graph and serialize it again, on the
//! popup's thread, to learn nothing the server does not check anyway.

use op_util::json_escape::escape_json_quoted;

use crate::hub_time::{format_local_stamp, format_rfc3339_utc, unix_seconds_from_ms};
use crate::js_text::{is_js_space, js_trim};
/// The Hub's own request-body / document cap, in MiB.
///
/// A SERVER contract (`snapshot_routes.go::maxSnapshotBodyBytes`,
/// `snapshots::DefaultMaxDocumentBytes`), deliberately NOT tied to the local
/// ingest cap (`crate::transfer::MAX_SNAPSHOT_MB`): the local editor can raise
/// its ceiling in lockstep with the extractor's node budget, but a capture
/// only fits in the account inbox when the Go backend accepts it.
pub const HUB_MAX_SNAPSHOT_MB: u32 = 32;

/// The inbox route. Bare path, no query — the Hub refuses a request target
/// carrying one (`exactRequestTarget`).
const SNAPSHOTS_PATH: &str = "/api/v1/snapshots";

/// The only producer this inbox accepts today (`snapshots.KindWebSnapshot`).
const KIND: &str = "web-snapshot";

/// Marker standing in for the snapshot document inside
/// [`create_envelope_template`]. Distinct from [`crate::ingress`]'s marker so
/// the two templates can never be spliced with each other's placeholder.
pub const SNAPSHOT_PLACEHOLDER: &str = "__OPENPENCIL_HUB_SNAPSHOT__";

/// Longest display name the Hub accepts, in Unicode scalar values
/// (`snapshots/validation.go::maxNameRunes`). Go counts runes, so this is
/// counted in `char`s and not in UTF-16 code units.
const MAX_NAME_RUNES: usize = 200;

/// Longest `source_url` the Hub accepts, in bytes.
const MAX_SOURCE_URL_BYTES: usize = 2048;

/// Name used when a page title sanitises down to nothing. Deliberately not
/// translated: it is stored server-side and read later, possibly from another
/// device in another language, so it is one stable English word rather than
/// whichever locale the popup happened to be in.
const FALLBACK_STEM: &str = "Web capture";

/// Separator between the title stem and the timestamp in a snapshot name.
const NAME_SEPARATOR: &str = " — ";

/// Runes the ` — YYYY-MM-DD HH:MM` suffix always costs.
const NAME_SUFFIX_RUNES: usize = 3 + 16;

/// Per-user ceiling on stored snapshots (`snapshots::DefaultMaxItems`).
/// Rendered in the "your inbox is full" message, so it lives beside the rest
/// of the contract instead of being retyped into fifteen locale catalogs.
pub const QUOTA_ITEMS: u32 = 50;

/// Per-user ceiling on stored bytes, in MiB (`snapshots::DefaultMaxTotalBytes`).
pub const QUOTA_TOTAL_MB: u32 = 200;

/// Bytes reserved for everything in the envelope that is not the snapshot.
///
/// The Hub caps the **request body** at 32 MiB and the **document** at 32 MiB
/// (`snapshot_routes.go::maxSnapshotBodyBytes`, `snapshots::DefaultMaxDocumentBytes`),
/// so a document that exactly reaches the local ingress cap would push the
/// envelope over the request cap and come back `413` after the whole upload
/// had been sent. The envelope is at most ~2.4 KiB (a 2048-byte URL, a
/// 200-rune name, ~120 fixed bytes); 4 KiB is that with room to spare.
const ENVELOPE_OVERHEAD_BYTES: f64 = 4096.0;

/// Milliseconds before an upload to the Hub is abandoned.
///
/// Deliberately far above the local import's 15 s and the session probe's
/// 8 s: this one crosses the public internet carrying up to 32 MiB. Twenty
/// megabytes on a 3 Mbit/s uplink is roughly a minute, and a user who chose
/// "my account" would rather wait than watch a timeout that only means their
/// connection is ordinary.
pub const UPLOAD_TIMEOUT_MS: u32 = 120_000;

/// Absolute URL of the inbox create route on `origin`.
pub fn snapshots_url(origin: &str) -> String {
    format!("{origin}{SNAPSHOTS_PATH}")
}

/// Whether a snapshot of `chars` UTF-16 code units cannot fit in the Hub's
/// request body once the envelope is wrapped around it.
///
/// The cap is the Hub's own 32 MiB (`snapshots/types.go:102-105`) minus
/// [`ENVELOPE_OVERHEAD_BYTES`]. Since the local ingest ceiling grew past it,
/// a capture in the 32–48 MiB band imports locally but not into the account —
/// which is why this check reports against [`HUB_MAX_SNAPSHOT_MB`], not the
/// local cap. UTF-16 length under-counts UTF-8 bytes and is never larger, so
/// this only rejects captures the server would reject too.
pub fn snapshot_too_large(chars: f64) -> bool {
    chars > f64::from(HUB_MAX_SNAPSHOT_MB) * 1024.0 * 1024.0 - ENVELOPE_OVERHEAD_BYTES
}

/// The display name for a page titled `title`, captured at `captured_at_ms`.
///
/// Shape: `<sanitised title> — 2026-08-04 17:20`. The title half is treated as
/// hostile, exactly as [`crate::filename`] treats it: it is a page title, and
/// this string is rendered in the account portal's list beside other people's
/// captures. Control characters, C1 bytes, and the bidi/zero-width run the Hub
/// rejects (`displayControl`) all collapse to spaces before anything else
/// happens, so no title can disguise its entry in that list — and no title can
/// produce a name the Hub then refuses, which would surface as an unexplained
/// `400`.
pub fn snapshot_name(title: &str, captured_at_ms: f64, tz_offset_minutes: f64) -> String {
    let seconds = unix_seconds_from_ms(captured_at_ms);
    let stamp = format_local_stamp(seconds, clamp_offset(tz_offset_minutes));
    let stem = name_stem(title);
    format!("{stem}{NAME_SEPARATOR}{stamp}")
}

/// The RFC 3339 UTC instant for `captured_at`.
pub fn captured_at(captured_at_ms: f64) -> String {
    format_rfc3339_utc(unix_seconds_from_ms(captured_at_ms))
}

/// The create-request envelope, with [`SNAPSHOT_PLACEHOLDER`] where the
/// snapshot document goes.
///
/// `source_url` that fails [`source_url`] is written as `null` rather than
/// dropped: the field count is what makes splicing the document safe (see the
/// module header), so it is a constant five.
pub fn create_envelope_template(
    title: &str,
    source_url_raw: &str,
    captured_at_ms: f64,
    tz_offset_minutes: f64,
) -> String {
    let mut out = String::with_capacity(512);
    out.push_str(r#"{"kind":"#);
    out.push_str(&escape_json_quoted(KIND));
    out.push_str(r#","name":"#);
    out.push_str(&escape_json_quoted(&snapshot_name(
        title,
        captured_at_ms,
        tz_offset_minutes,
    )));
    out.push_str(r#","source_url":"#);
    match source_url(source_url_raw) {
        Some(url) => out.push_str(&escape_json_quoted(&url)),
        None => out.push_str("null"),
    }
    out.push_str(r#","captured_at":"#);
    out.push_str(&escape_json_quoted(&captured_at(captured_at_ms)));
    out.push_str(r#","snapshot":"#);
    out.push_str(SNAPSHOT_PLACEHOLDER);
    out.push('}');
    out
}

/// The capture's origin URL, or `None` when it must not be sent.
///
/// Mirrors the Hub's `ValidSourceURL` and then narrows it: the Hub also
/// requires the value to survive a Go `url.Parse` / `String()` round trip
/// unchanged, which no client can reproduce without a URL parser. Rather than
/// approximate that, this accepts only what a browser's `location.href`
/// actually produces — an `http`/`https` URL of visible ASCII with no
/// userinfo, where every character Go would have re-escaped is already
/// percent-encoded. A URL outside that set is withheld (the field is optional)
/// instead of risking a `400` that would cost the user their whole capture.
pub fn source_url(raw: &str) -> Option<String> {
    let value = js_trim(raw);
    let rest = ["http://", "https://"]
        .iter()
        .find_map(|scheme| value.strip_prefix(scheme))?;
    if value.len() > MAX_SOURCE_URL_BYTES {
        return None;
    }
    // Visible ASCII only. This is also what rules out spaces, tabs, newlines,
    // NULs and every non-ASCII byte in one test.
    if !value.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        return None;
    }
    // Characters a Go `URL.String()` would re-encode, so a value containing
    // one could never equal the parsed round trip.
    if value.bytes().any(|b| {
        matches!(
            b,
            b'"' | b'<' | b'>' | b'\\' | b'^' | b'`' | b'{' | b'}' | b'|'
        )
    }) {
        return None;
    }
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .filter(|authority| !authority.is_empty())?;
    // `user:password@host` is refused by the Hub (`parsed.User != nil`), and a
    // browser strips credentials from `location.href` anyway.
    if authority.contains('@') {
        return None;
    }
    Some(value.to_owned())
}

/// A timezone offset in minutes, or 0 for a value that is not one.
fn clamp_offset(minutes: f64) -> i32 {
    if !minutes.is_finite() {
        return 0;
    }
    let rounded = minutes.trunc();
    if (-14.0 * 60.0..=14.0 * 60.0).contains(&rounded) {
        rounded as i32
    } else {
        0
    }
}

/// The title half of a snapshot name: hostile text made safe and bounded.
fn name_stem(title: &str) -> String {
    let collapsed = collapse_to_spaces(title);
    let trimmed = js_trim(&collapsed);
    let budget = MAX_NAME_RUNES - NAME_SUFFIX_RUNES;
    let truncated = truncate_runes(trimmed, budget).trim_end_matches(is_js_space);
    if truncated.is_empty() {
        FALLBACK_STEM.to_owned()
    } else {
        truncated.to_owned()
    }
}

/// Replace every character the Hub refuses — and every whitespace run — with a
/// single space.
fn collapse_to_spaces(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut in_run = false;
    for c in title.chars() {
        if is_js_space(c) || c.is_control() || is_c1(c) || is_display_control(c) {
            if !in_run {
                out.push(' ');
                in_run = true;
            }
        } else {
            in_run = false;
            out.push(c);
        }
    }
    out
}

/// The C1 range `U+0080..=U+009F`, which `char::is_control` already covers but
/// which the Hub names separately; kept explicit so the two lists read the
/// same way.
fn is_c1(c: char) -> bool {
    matches!(c, '\u{80}'..='\u{9f}')
}

/// Characters that can reorder or hide the text around them —
/// `snapshots/validation.go::displayControl`, verbatim.
fn is_display_control(c: char) -> bool {
    matches!(c, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{feff}')
}

/// Truncate to at most `limit` Unicode scalar values — Go's
/// `utf8.RuneCountInString`, which is what the Hub bounds the name by.
fn truncate_runes(s: &str, limit: usize) -> &str {
    match s.char_indices().nth(limit) {
        Some((offset, _)) => &s[..offset],
        None => s,
    }
}
