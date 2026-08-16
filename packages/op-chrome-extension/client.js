/**
 * OpenPencil delivery client — the `fetch` arm of both import paths.
 *
 * Every decision this file used to make now lives in Rust
 * (`crates/op-chrome-extension-core/src/{endpoint,ingress,transfer}.rs`):
 * which endpoints may be addressed, which URL each ingress has, what the
 * `/mcp` envelope looks like, whether a capture is over the body cap, and
 * what a reply means. What is left here is what only a browser can do —
 * issuing the request, timing it out, and reading the body.
 *
 * Two ingresses, tried in order:
 *
 * 1. `POST /api/import/web-snapshot` — the insert-only snapshot route the
 *    running desktop editor exposes on its live MCP endpoint
 *    (`crates/op-host-services/src/mcp_live/snapshot_ingest.rs`). It is the
 *    only route on that endpoint reachable from a `chrome-extension://`
 *    origin, and it needs no token. The request body IS the snapshot JSON.
 * 2. `POST /mcp` — a plain MCP `tools/call` for `import_web_snapshot`. This
 *    covers exactly one host: the UNMANAGED headless daemon
 *    (`openpencil-desktop --serve-web <port>`), whose `/mcp` needs no token.
 *    It is NOT a compatibility path for older desktop builds — a desktop app
 *    that predates the ingest route answers 404 there and 403 on `/mcp` (its
 *    admission gate refuses extension origins outright), and the fallback
 *    only runs on a 404.
 *
 * Neither request is subject to CORS. What makes that true is
 * `manifest.json`'s loopback `host_permissions`: a `fetch` from an extension
 * page to a host the extension has permission for is a privileged request,
 * so Chrome sends no preflight and requires no `Access-Control-Allow-Origin`
 * on the reply. (Keeping to `Content-Type` alone would only matter if we
 * were relying on the CORS simple-request rules, which we are not.) The
 * editor's own origin checks are unaffected — those are its admission gate,
 * not the browser's.
 *
 * And one remote ingress, chosen instead of the two above when the user set
 * delivery to their account:
 *
 * 3. `POST <hub>/api/v1/snapshots` — the Hub's per-user capture inbox. Same
 *    no-CORS mechanics (both hub origins are in `host_permissions`), same
 *    session cookie the account row already relies on, plus the session's
 *    `X-CSRF-Token`. The Hub admits a `chrome-extension://` origin on this
 *    route deliberately (`auth.ExtensionCapableMutation`). Everything about
 *    the request that is a decision — the envelope, the size ceiling, what
 *    each status means — is in the core's `hub` / `hub_reply` modules.
 */

import { getCore } from './core-registry.js';

/**
 * Endpoint the popup pre-fills. Read through the core so the default and the
 * rules that validate it cannot drift apart.
 */
export function defaultEndpoint() {
  return getCore().defaultEndpoint();
}

/**
 * Normalize a user-typed endpoint to `host:port`, or null when it is
 * unparseable or names anything but loopback.
 *
 * The rule itself — loopback literals only, `localhost` rewritten to
 * `127.0.0.1` — is enforced in Rust; see `endpoint.rs` for why.
 *
 * @returns {string | null}
 */
export function normalizeEndpoint(raw) {
  // wasm-bindgen maps a Rust `None` onto `undefined`; the callers test for
  // null, so normalize the absence too.
  return getCore().normalizeEndpoint(raw) ?? null;
}

function requestError(code, detail) {
  const error = new Error(detail || code);
  error.code = code;
  error.detail = detail;
  return error;
}

async function postJson(url, body) {
  const timeoutMs = getCore().requestTimeoutMs();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    // The timer covers the response body too, not just the headers: a server
    // that answers and then stalls mid-body would hang the popup just as
    // effectively as one that never answers at all.
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
      signal: controller.signal,
    });
    return { status: response.status, text: await response.text() };
  } catch (cause) {
    // Our own abort is a hung server, not an absent one — the two want
    // different advice, so they carry different codes.
    if (controller.signal.aborted) {
      throw requestError('timeout', String(timeoutMs / 1000));
    }
    // A refused connection, a closed app, or a wrong port all land here as a
    // bare TypeError; the caller turns it into the "launch OpenPencil" hint.
    throw requestError('offline', String(cause && cause.message ? cause.message : cause));
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Build the `/mcp` request body around `snapshot`.
 *
 * The envelope's shape — method, tool name, key order, and the fact that the
 * snapshot rides as a JSON string the tool parses itself — comes from the
 * core as a template with one placeholder. Only the splice happens here, and
 * it has to: `JSON.stringify` is the one escaper that round-trips a lone
 * surrogate (as `\udXXX`), which pages do produce in titles and `alt` text
 * and which a Rust `&str` cannot even represent — the previous
 * `buildMcpEnvelope(snapshot)` had wasm-bindgen rewrite each one to `U+FFFD`
 * on the way in. Splicing here also keeps the snapshot from being copied
 * into and back out of the wasm heap, which for a 32 MB capture was the
 * difference between two live copies and four.
 */
function buildMcpEnvelope(core, snapshot) {
  const parts = core.mcpEnvelopeTemplate().split(core.snapshotPlaceholder());
  if (parts.length !== 2) {
    throw requestError('import', 'the logic core returned a malformed request template');
  }
  return parts[0] + JSON.stringify(snapshot) + parts[1];
}

/**
 * POST `body` to `url` and let the core say what the answer means.
 *
 * The reply body is handed over as raw text: parsing it, digging the failure
 * message out of either shape, and choosing between "fall back", "imported"
 * and "refused" are all core decisions.
 */
async function sendVia(url, body, classify) {
  const reply = await postJson(url, body);
  return JSON.parse(classify(reply.status, reply.text));
}

/**
 * Import `snapshot` (the extractor's JSON text) into the OpenPencil instance
 * listening on `endpoint` (`host:port`).
 *
 * @returns {Promise<{ nodeCount: number, warnings: string[] }>}
 * @throws {Error} with `code` of `offline` / `timeout` / `tooLarge` /
 *   `forbidden` / `import`.
 */
export async function importSnapshot(endpoint, snapshot) {
  const core = getCore();
  // UTF-16 length under-counts UTF-8 bytes for non-ASCII text and is never
  // larger, so this only ever rejects captures the server would reject too.
  if (core.snapshotTooLarge(snapshot.length)) {
    throw requestError('tooLarge', String(core.maxSnapshotMb()));
  }

  let outcome = await sendVia(core.ingestUrl(endpoint), snapshot, core.classifyIngestReply);
  if (outcome.outcome === 'fallback') {
    // The envelope JSON-escapes the snapshot, which can inflate a capture
    // that passed the raw-size precheck past the endpoint's 64 MiB
    // whole-body cap — check the BUILT size so the user gets the actionable
    // message instead of uploading it all and being refused.
    const envelope = buildMcpEnvelope(core, snapshot);
    if (core.mcpEnvelopeTooLarge(envelope.length)) {
      throw requestError('tooLarge', String(core.maxSnapshotMb()));
    }
    outcome = await sendVia(core.mcpUrl(endpoint), envelope, core.classifyMcpReply);
  }
  if (outcome.outcome === 'error') {
    throw requestError(outcome.code, outcome.detail);
  }
  return { nodeCount: outcome.nodeCount, warnings: outcome.warnings };
}

/* ----------------------------------------------- the account snapshot inbox */

/**
 * Build the `POST /api/v1/snapshots` body around `snapshot`.
 *
 * The core hands over the envelope with one placeholder where the document
 * goes, exactly as it does for `/mcp` — but here the document is spliced in
 * RAW rather than through `JSON.stringify`, because the Hub wants an object in
 * that slot and not a string.
 *
 * That is safe for a reason that belongs to the server, not to a guess about
 * the extractor: the envelope always carries exactly five fields, and the
 * Hub's decoder refuses a sixth key, a duplicate key, and any trailing content
 * after the closing brace. So text spliced into the `snapshot` slot can
 * produce a valid document or a `400`, and nothing in between — it cannot
 * reach `name`, `source_url`, `kind` or `captured_at`, all of which are built
 * and escaped inside the core. Reparsing 32 MB on the popup's thread to prove
 * the same thing would cost a great deal and buy nothing.
 */
function buildHubEnvelope(core, snapshot, meta) {
  const template = core.hubEnvelopeTemplate(
    String((meta && meta.title) || ''),
    String((meta && meta.url) || ''),
    Date.now(),
    // `getTimezoneOffset` counts minutes to ADD to local time to reach UTC, so
    // it runs the other way round from every other offset notation. This is
    // only ever used for the human-readable stamp inside the snapshot's name;
    // the wire instant the core builds is always UTC.
    -new Date().getTimezoneOffset(),
  );
  const parts = template.split(core.hubSnapshotPlaceholder());
  if (parts.length !== 2) {
    throw requestError('rejected', 'the logic core returned a malformed request template');
  }
  return parts[0] + snapshot + parts[1];
}

/**
 * POST the create request and return `{status, text, retryAfter}`.
 *
 * Separate from {@link postJson} on three counts, each of which is the Hub's
 * doing: the session cookie has to travel (`credentials: 'include'`), the
 * session's CSRF token has to be presented, and the timeout is the core's
 * upload budget rather than the local import's — this request crosses the
 * public internet carrying up to 32 MiB.
 */
async function postHubJson(url, body, csrfToken) {
  const timeoutMs = getCore().hubUploadTimeoutMs();
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(url, {
      method: 'POST',
      credentials: 'include',
      // A create is never worth serving from the HTTP cache, and a redirect
      // would take a cookie-bearing request somewhere nobody asked for.
      cache: 'no-store',
      redirect: 'error',
      headers: { 'Content-Type': 'application/json', 'X-CSRF-Token': csrfToken },
      body,
      signal: controller.signal,
    });
    return {
      status: response.status,
      text: await response.text(),
      // Present on 429 and on the busy-slot 503; the core turns it into a wait.
      retryAfter: response.headers.get('Retry-After') ?? '',
    };
  } catch (cause) {
    // The same two codes `account.js` uses, so a hub that is unreachable reads
    // the same way whether it was the session probe or an upload that found
    // out.
    if (controller.signal.aborted) {
      throw requestError('hubTimeout', String(timeoutMs / 1000));
    }
    throw requestError('hubOffline', String(cause && cause.message ? cause.message : cause));
  } finally {
    clearTimeout(timer);
  }
}

/**
 * File `snapshot` in the signed-in user's OpenPencil Hub inbox.
 *
 * @param {string} hubOrigin the region's hub origin, from the core.
 * @param {string} csrfToken the live session's token — never a stored one.
 * @param {string} snapshot the extractor's JSON text.
 * @param {{title?: string, url?: string}} meta the captured page's title and URL.
 * @returns {Promise<{id: string, name: string, bytes: number, expiresAt: string}>}
 * @throws {Error} with `code` of `tooLarge` / `signedOut` / `forbidden` /
 *   `quota` / `rateLimited` / `rejected` / `unavailable` / `hubOffline` /
 *   `hubTimeout`. A `rateLimited` error additionally carries
 *   `retryAfterSeconds`.
 */
export async function sendSnapshotToAccount(hubOrigin, csrfToken, snapshot, meta) {
  const core = getCore();
  try {
    if (typeof csrfToken !== 'string' || csrfToken === '') {
      throw requestError('signedOut', 'no session');
    }
    // The hub's cap is its own 32 MiB (smaller than the local ingress cap),
    // less the envelope wrapped around the document — a capture that fits
    // locally would otherwise be uploaded in full and then refused.
    if (core.hubSnapshotTooLarge(snapshot.length)) {
      throw requestError('tooLarge', String(core.hubMaxSnapshotMb()));
    }
    const reply = await postHubJson(
      core.hubSnapshotsUrl(hubOrigin),
      buildHubEnvelope(core, snapshot, meta),
      csrfToken,
    );
    const outcome = JSON.parse(core.classifyHubReply(reply.status, reply.text, reply.retryAfter));
    if (outcome.outcome === 'error') {
      const error = requestError(outcome.code, outcome.detail);
      if (typeof outcome.retryAfterSeconds === 'number') {
        error.retryAfterSeconds = outcome.retryAfterSeconds;
      }
      throw error;
    }
    return {
      id: outcome.id,
      name: outcome.name,
      bytes: outcome.bytes,
      expiresAt: outcome.expiresAt,
    };
  } catch (error) {
    // Tag every failure from this path. `forbidden` in particular means two
    // different things — "this editor has no extension ingress" for the local
    // route, "origin or CSRF" for the hub — and a caller that handles both
    // destinations must be able to tell which one it is holding without
    // guessing from the message.
    if (error && typeof error === 'object') error.account = true;
    throw error;
  }
}
