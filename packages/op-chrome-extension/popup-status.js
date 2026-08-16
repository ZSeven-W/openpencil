/**
 * The popup's feedback surface: one status line, one detail line, and the
 * mapping from a thrown error onto the sentence the user reads.
 *
 * Split out of `popup.js` for two reasons beyond its length. The mapping is
 * the part of the popup that grows with every destination — three ingresses
 * and a dozen failure codes now — while the controller around it does not, and
 * keeping every message key in one file is what makes it possible to check
 * that none of them is unreachable. And nothing here touches `chrome.*`: it is
 * DOM, `i18n.js`, and the core's published ceilings.
 *
 * Messages are held as *references* (`{ key, args }`) rather than as rendered
 * text, so switching language re-renders whatever is on screen exactly instead
 * of leaving one stale sentence among translated buttons. That is also the
 * shape `background.js` stores a pick result in, which is why `tRef` recurses.
 */

import { getCore } from './core-registry.js';
import { t, tRef } from './i18n.js';

const statusArea = document.getElementById('status');
const statusMessage = document.getElementById('status-message');
const statusDetail = document.getElementById('status-detail');

/**
 * What the status area is currently saying, held as message references rather
 * than rendered text.
 */
let shown = null;

/** A message reference: `{ key, args }`, renderable in any locale. */
export function msg(key, args) {
  return { key, args: args || [] };
}

/**
 * Render the feedback surface.
 *
 * `tone` drives the coloured rule down the left edge — `idle`, `working`,
 * `ok`, `error` — and `progress` (0..1) turns that same rule into a
 * determinate fill, which is why there is no separate spinner to keep in
 * sync. `detail` is the secondary line, given as an array of parts (strings
 * or message references): sizes, node counts, importer warnings, server
 * messages.
 */
export function setStatus(message, tone, options) {
  const { detail, progress } = options || {};
  shown = { message, tone, detail, progress };
  renderStatus();
}

/** Re-render the current message — after a language change, in place. */
export function renderStatus() {
  if (!shown) return;
  const parts = Array.isArray(shown.detail) ? shown.detail : [];
  statusArea.dataset.tone = shown.tone || 'idle';
  statusMessage.textContent = tRef(shown.message);
  statusDetail.textContent = parts.map(tRef).join('\n');
  statusDetail.hidden = parts.length === 0;
  if (typeof shown.progress === 'number') {
    statusArea.style.setProperty('--progress', String(Math.max(0, Math.min(1, shown.progress))));
  } else {
    statusArea.style.removeProperty('--progress');
  }
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** The detail parts for a finished capture: size, then any caveats. */
export function captureDetail(meta, extra) {
  const parts = [formatBytes(meta.bytes)];
  if (meta.truncated) parts.push(msg('statusTruncated'));
  if (extra && extra.length > 0) parts.push(...extra);
  return parts;
}

/** Failures of the capture itself, before any destination is involved. */
export function reportCaptureError(error) {
  switch (error && error.code) {
    case 'noTab':
      return setStatus(msg('errorNoTab'), 'error');
    case 'restricted':
      return setStatus(msg('errorRestrictedTab'), 'error');
    case 'empty':
      return setStatus(msg('errorNoSnapshot'), 'error');
    case 'chunkLost':
      return setStatus(msg('errorChunkLost'), 'error');
    case 'wasmMissing':
      return setStatus(msg('errorCoreMissing', [String(error.message)]), 'error');
    case 'wasmInit':
      // The build product is there and would not start. "Run the build
      // script" would be the wrong advice.
      return setStatus(msg('errorCoreInit', [String(error.message)]), 'error');
    case 'pickUnavailable':
      return setStatus(msg('errorPickUnavailable', [String(error.message)]), 'error');
    default:
      return setStatus(
        msg('errorInjection', [String(error && error.message ? error.message : error)]),
        'error',
      );
  }
}

/** Failures of the local ingress, which name the endpoint that refused. */
export function reportDeliveryError(error, endpoint) {
  switch (error && error.code) {
    case 'offline':
      return setStatus(msg('errorOffline', [endpoint]), 'error');
    case 'timeout':
      return setStatus(msg('errorTimeout', [endpoint, String(error.detail || '')]), 'error');
    case 'tooLarge':
      return setStatus(msg('errorTooLarge', [String(error.detail || '')]), 'error');
    case 'forbidden':
      return setStatus(msg('errorNoIngress', [String(error.detail || '')]), 'error');
    case 'import':
      return setStatus(msg('errorImport', [String(error.detail || error.message)]), 'error');
    default:
      return reportCaptureError(error);
  }
}

/**
 * Failures of the account upload.
 *
 * Every code here comes from the core's classification of one Hub status, so
 * this switch and `hub_reply.rs`'s enum are the same list read from two ends.
 * The quota ceilings are read from the core rather than retyped: they are part
 * of the Hub's contract, and a number that drifted would be worse than no
 * number at all.
 */
export function reportAccountError(error) {
  const detail = String((error && (error.detail || error.message)) || error);
  switch (error && error.code) {
    case 'signedOut':
      return setStatus(msg('errorAccountSignedOut'), 'error');
    case 'forbidden':
      return setStatus(msg('errorAccountForbidden', [detail]), 'error');
    case 'quota':
      return setStatus(
        msg('errorAccountQuota', [
          String(getCore().hubQuotaItems()),
          String(getCore().hubQuotaTotalMb()),
          detail,
        ]),
        'error',
      );
    case 'rateLimited':
      return setStatus(rateLimitMessage(error.retryAfterSeconds, detail), 'error');
    // The hub's own cap (32 MiB) is smaller than the local ingest cap, so
    // this capture may still import locally — say so instead of the local
    // message's "download the file".
    case 'tooLarge':
      return setStatus(msg('errorAccountTooLarge', [detail]), 'error');
    case 'rejected':
      return setStatus(msg('errorAccountRejected', [detail]), 'error');
    case 'unavailable':
      return setStatus(msg('errorAccountUnavailable', [detail]), 'error');
    case 'hubOffline':
    case 'hubTimeout':
    case 'hubNotPermitted':
      return setStatus(msg('accountUnavailable', [detail]), 'error');
    default:
      return reportCaptureError(error);
  }
}

/**
 * "Try again in about N minutes", or seconds when the wait is short.
 *
 * A wait the hub did not state falls back to the message without a number:
 * `Retry-After` in its HTTP-date form is not turned into a duration (the core
 * has no clock), and inventing one would be worse than saying nothing.
 */
function rateLimitMessage(seconds, detail) {
  if (typeof seconds !== 'number') return msg('errorAccountRateLimited', [detail]);
  if (seconds < 60) return msg('errorAccountRateLimitedSeconds', [String(seconds)]);
  return msg('errorAccountRateLimitedMinutes', [String(Math.ceil(seconds / 60))]);
}

/** Localize every `data-i18n` node in the document, plus the title. */
export function localizeDocument() {
  for (const node of document.querySelectorAll('[data-i18n]')) {
    node.textContent = t(node.dataset.i18n);
  }
  for (const node of document.querySelectorAll('[data-i18n-aria]')) {
    node.setAttribute('aria-label', t(node.dataset.i18nAria));
  }
  document.title = t('extName');
}
