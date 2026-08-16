/**
 * Capture pipeline: run the canonical OpenPencil DOM extractor in the active
 * tab and pull its JSON back into the extension.
 *
 * The extractor (`vendor/snapshot-extractor.js`, a byte-for-byte copy of
 * `crates/op-html/assets/snapshot-extractor.js`) is a console-oriented IIFE:
 * it returns nothing, copies its output to the clipboard, and triggers a
 * `snapshot.json` download. We therefore run it between two small harness
 * injections that patch `URL.createObjectURL`, `HTMLAnchorElement.click` and
 * `navigator.clipboard.writeText` in the extension's ISOLATED world (the page
 * never sees those patches), capture the Blob the extractor builds, and
 * restore the originals. The extractor file itself is never modified — a
 * drift check (`scripts/check-extractor-sync.sh`) fails the moment the copy
 * and the Rust asset disagree.
 *
 * The extractor reads geometry and styles through `getComputedStyle` /
 * `getBoundingClientRect`, which the isolated world can do, so no MAIN-world
 * injection (and no page-CSP exposure) is needed.
 *
 * Snapshot text is pulled back in slices instead of one giant return value:
 * an extractor result can reach tens of MB (it embeds rasterized images up to
 * 24 MB) and `chrome.scripting.executeScript` has to structured-clone the
 * result across a process boundary. 4 MiB per hop stays far below any IPC
 * limit while keeping the number of round trips small (12 for a maximum 48 MB page).
 * Which slice to ask for, whether the answer was acceptable, whether the
 * total adds up, and whether the payload carries the extractor's `truncated`
 * flag are all decided by the Rust core's `ChunkPlan`; this file only moves
 * the bytes. The joining stays here because slicing is by UTF-16 index and
 * only JS can rejoin a split surrogate pair exactly.
 *
 * The functions injected into the tab necessarily stay JavaScript — they are
 * serialized and evaluated inside the page's process.
 */

import { getCore } from './core-registry.js';

const EXTRACTOR_FILE = 'vendor/snapshot-extractor.js';
/** Sentinel object URL, so the harness only swallows the extractor's own anchor. */
const CAPTURE_URL = 'blob:openpencil-capture';
/** Hard ceiling on the lazy-content scroll march before a full-page capture. */
const SETTLE_BUDGET_MS = 7000;

/* -------------------------------------------------------------------------
 * Functions below run inside the tab (ISOLATED world). They must be
 * self-contained: `chrome.scripting.executeScript` serializes the function
 * source, so they cannot close over anything in this module.
 * ---------------------------------------------------------------------- */

function beginCapture(captureUrl, keepPickedRoot) {
  const state = (globalThis.openpencilCapture ||= {});
  state.blob = null;
  state.text = null;
  // The extractor reads its root from this global. A full-page capture must
  // clear whatever a previous element pick left behind, or it would silently
  // capture that old subtree again; a pick capture must keep it.
  if (!keepPickedRoot) delete globalThis.openpencilSnapshotOptions;
  // An element-pick overlay can still be armed when a capture starts (the
  // user reopened the popup instead of picking). It is page DOM in this
  // isolated world, so tear it down or it gets captured as content. After a
  // committed pick the teardown already ran and this is a no-op; the
  // abandoned `awaitPick` injection settles through its own timeout.
  const pick = globalThis.openpencilPick;
  if (pick && typeof pick.teardown === 'function') pick.teardown();
  if (!state.originals) {
    state.originals = {
      createObjectURL: URL.createObjectURL,
      click: HTMLAnchorElement.prototype.click,
      writeText: navigator.clipboard ? navigator.clipboard.writeText : null,
    };
  }
  const originals = state.originals;
  URL.createObjectURL = function (object) {
    if (object instanceof Blob) {
      state.blob = object;
      return captureUrl;
    }
    return originals.createObjectURL.call(URL, object);
  };
  HTMLAnchorElement.prototype.click = function () {
    if (this.href === captureUrl) return undefined;
    return originals.click.call(this);
  };
  if (navigator.clipboard && originals.writeText) {
    navigator.clipboard.writeText = () => Promise.resolve();
  }
  return true;
}

async function finishCapture() {
  const state = globalThis.openpencilCapture;
  if (!state || !state.originals) return { ok: false, reason: 'state' };
  const originals = state.originals;
  URL.createObjectURL = originals.createObjectURL;
  HTMLAnchorElement.prototype.click = originals.click;
  if (navigator.clipboard && originals.writeText) {
    navigator.clipboard.writeText = originals.writeText;
  }
  // A zero-byte blob is not a capture. The extractor bails out before
  // building one when there is nothing visible to snapshot, so reaching here
  // with an empty Blob means something in the page replaced or emptied it —
  // either way there is no snapshot, and reporting success would post an
  // empty body and surface as an opaque importer failure instead.
  if (!state.blob || state.blob.size === 0) return { ok: false, reason: 'empty' };
  const blob = state.blob;
  const text = await blob.text();
  if (text.length === 0) return { ok: false, reason: 'empty' };
  state.blob = null;
  state.text = text;
  // The extractor's own `truncated` flag is NOT read here. It is detected by
  // the core's incremental matcher as the slices come back (see `ChunkPlan`
  // in `crates/op-chrome-extension-core/src/transfer.rs`), which keeps the
  // rule in one place and off the page-injected side of the boundary.
  return {
    ok: true,
    bytes: blob.size,
    chars: text.length,
    title: document.title,
    url: location.href,
  };
}

function readChunk(offset, length) {
  const state = globalThis.openpencilCapture;
  if (!state || typeof state.text !== 'string') return null;
  return state.text.slice(offset, offset + length);
}

/**
 * Walk the viewport through the whole page so lazy content actually exists
 * before the extractor reads the DOM, then park the page at the top.
 *
 * Three problems, one scroll journey:
 * - `loading="lazy"` images and IntersectionObserver reveals only load when
 *   the viewport approaches them — a capture taken from the top of a long
 *   page otherwise ships placeholders for everything below the fold.
 * - `content-visibility: auto` sections have no laid-out geometry until
 *   they have been near the viewport once.
 * - `position: fixed` / `sticky` chrome sits wherever the CURRENT scroll
 *   put it; capturing from the very top is the one scroll position where
 *   viewport coordinates and resting page coordinates agree.
 *
 * The march is bounded (deadline + step cap) so an infinite feed cannot
 * hang the capture: whatever loaded within the budget is what gets
 * captured. Returns the original scroll offset for `restoreScroll`.
 */
async function settlePageForCapture(maxMs) {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const scroller = document.scrollingElement || document.documentElement;
  const original = { x: window.scrollX, y: window.scrollY };
  const deadline = Date.now() + maxMs;
  const step = Math.max(200, window.innerHeight * 0.85);
  let y = 0;
  let steps = 0;
  // Re-read scrollHeight every pass: it grows as lazy sections materialize.
  while (y < scroller.scrollHeight && Date.now() < deadline && steps < 120) {
    window.scrollTo(0, y);
    y += step;
    steps += 1;
    // oxlint-disable-next-line no-await-in-loop -- the pause IS the point:
    // lazy loaders watch the viewport, not a synchronous scroll sweep.
    await sleep(90);
  }
  // Give the images the march started a bounded chance to finish.
  while (Date.now() < deadline) {
    const pending = Array.prototype.filter.call(
      document.images,
      (image) => !image.complete,
    );
    if (pending.length === 0) break;
    // oxlint-disable-next-line no-await-in-loop
    await sleep(150);
  }
  window.scrollTo(0, 0);
  await sleep(180);
  return original;
}

/** Put the page back where the user left it. */
function restoreScroll(position) {
  if (position && typeof position.y === 'number') {
    window.scrollTo(position.x || 0, position.y);
  }
  return true;
}

/**
 * Teardown, safe to run at any point after `beginCapture`: restore the
 * patched globals and drop the parked snapshot. Restoring is idempotent —
 * `beginCapture` only ever records `originals` once — so this is correct
 * both after `finishCapture` (which already restored) and instead of it.
 */
function endCapture() {
  // Drop the picked root unconditionally: it belongs to exactly one capture,
  // and leaving it parked would silently narrow the next one.
  delete globalThis.openpencilSnapshotOptions;
  const state = globalThis.openpencilCapture;
  if (!state) return true;
  const originals = state.originals;
  if (originals) {
    URL.createObjectURL = originals.createObjectURL;
    HTMLAnchorElement.prototype.click = originals.click;
    if (navigator.clipboard && originals.writeText) {
      navigator.clipboard.writeText = originals.writeText;
    }
  }
  state.text = null;
  state.blob = null;
  return true;
}

/* ---------------------------------------------------------------------- */

async function runInTab(tabId, injection) {
  const results = await chrome.scripting.executeScript({
    target: { tabId, frameIds: [0] },
    ...injection,
  });
  return results && results.length > 0 ? results[0].result : undefined;
}

/**
 * Capture the active tab.
 *
 * With `pickedRoot`, the extractor narrows to whatever element the picker
 * parked on `globalThis.openpencilSnapshotOptions` in the tab's isolated
 * world (see `picker.js`); without it, the whole page is captured and any
 * stale parked root is discarded first.
 *
 * @param {number} tabId
 * @param {(transferred: number, total: number) => void} [onProgress]
 * @param {{ pickedRoot?: boolean }} [options]
 * @returns {Promise<{ text: string, meta: object }>}
 * @throws {Error} with `code` set to one of `restricted` / `empty` /
 *   `chunkLost` — the popup maps those onto localized messages.
 */
export async function capturePage(tabId, onProgress, options) {
  const core = getCore();
  const pickedRoot = Boolean(options && options.pickedRoot);
  // Full-page captures walk the viewport through the page first so lazy
  // content loads and fixed chrome rests at the top (see
  // `settlePageForCapture`). A picked element was chosen on screen, already
  // loaded — scrolling out from under the user's pick would be worse than
  // capturing it as seen.
  let scrollBack = null;
  if (!pickedRoot) {
    scrollBack = await runInTab(tabId, {
      func: settlePageForCapture,
      args: [SETTLE_BUDGET_MS],
    }).catch(() => null);
  }
  await runInTab(tabId, { func: beginCapture, args: [CAPTURE_URL, pickedRoot] });
  try {
    await runInTab(tabId, { files: [EXTRACTOR_FILE] });
    const meta = await runInTab(tabId, { func: finishCapture });
    if (!meta || !meta.ok) {
      // `state` means the isolated-world global we parked the harness on is
      // gone — the tab navigated out from under the run. That is the same
      // "you lost the capture, try again" situation as a dropped slice, not
      // "this page has nothing to capture".
      const lostState = !meta || meta.reason === 'state';
      const error = new Error(lostState ? 'capture state lost' : 'no snapshot');
      error.code = lostState ? 'chunkLost' : 'empty';
      throw error;
    }

    const plan = new core.ChunkPlan(meta.chars);
    try {
      const parts = [];
      while (!plan.done) {
        // Sequential on purpose: slices must arrive in order, and the point
        // of chunking is to keep exactly one in flight.
        // oxlint-disable-next-line no-await-in-loop
        const chunk = await runInTab(tabId, {
          func: readChunk,
          args: [plan.nextOffset, plan.nextLength],
        });
        // A non-string answer means the tab lost the parked snapshot. `-1`
        // is how the core is told that; a real slice reports the length JS
        // measured on it, which is authoritative — the copy the core sees
        // has had any surrogate half split by the slice boundary rewritten,
        // so its own length would be wrong.
        const text = typeof chunk === 'string' ? chunk : '';
        if (!plan.accept(text, typeof chunk === 'string' ? chunk.length : -1)) {
          const error = new Error('snapshot transfer interrupted');
          error.code = 'chunkLost';
          throw error;
        }
        parts.push(text);
        if (onProgress) onProgress(plan.transferred, meta.chars);
      }

      // Joined here rather than in the core: a slice boundary can fall
      // between the halves of a surrogate pair, and only JS can rejoin one
      // exactly (a Rust `String` cannot hold an unpaired half).
      const text = parts.join('');
      if (!plan.verifyTotal(text.length)) {
        const error = new Error('snapshot transfer incomplete');
        error.code = 'chunkLost';
        throw error;
      }
      return { text, meta: { ...meta, truncated: plan.truncated } };
    } finally {
      // Release the plan's wasm-side allocation instead of waiting for the
      // popup to be torn down.
      plan.free();
    }
  } finally {
    // Every exit path — including a rejection from `finishCapture` itself,
    // which is the call that would otherwise have done the restoring — must
    // leave the tab with its own `URL.createObjectURL` / anchor `click` /
    // clipboard back in place and the parked snapshot freed.
    await runInTab(tabId, { func: endCapture }).catch(() => undefined);
    if (scrollBack) {
      await runInTab(tabId, { func: restoreScroll, args: [scrollBack] }).catch(
        () => undefined,
      );
    }
  }
}
