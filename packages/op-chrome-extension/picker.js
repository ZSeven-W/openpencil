/**
 * Element picker: let the user point at one element in the page, and hand its
 * subtree back to the capture pipeline.
 *
 * The whole picker runs inside the tab, injected by
 * `chrome.scripting.executeScript` into the extension's ISOLATED world. That
 * placement is what makes it safe: the overlay is built from elements this
 * world owns, so the page cannot see them, and nothing on the target element
 * is ever mutated — the highlight is a separate fixed-position box drawn on
 * top, not an outline written onto the element's own style. A page that
 * rewrites its DOM mid-pick therefore cannot lose a style we set, because we
 * set none.
 *
 * Two functions are injected, in sequence:
 *
 * 1. {@link startPick} arms the picker and returns immediately. It cannot
 *    return the selection, because `executeScript` resolves as soon as the
 *    injected function returns and the user has not clicked yet.
 * 2. {@link awaitPick} resolves once the user commits or cancels. It is a
 *    second injection whose promise `executeScript` waits on.
 *
 * Splitting it that way keeps the popup's own lifetime out of the picture:
 * Chrome tears the popup down the moment the user clicks into the page, and
 * `chrome.scripting` calls from a torn-down popup never settle. The pick is
 * therefore driven from the service worker (`background.js`), which outlives
 * the popup.
 *
 * The functions below are serialized to source and evaluated in the tab, so
 * they must be self-contained — no imports, no closures over this module.
 */

/** Global the picker parks its state on, in the isolated world. */
const PICK_STATE = 'openpencilPick';

/* -------------------------------------------------------------------------
 * Injected into the tab. Self-contained by requirement.
 * ---------------------------------------------------------------------- */

/**
 * Arm the picker. Returns `true` once the overlay is live.
 *
 * @param {string} stateKey
 * @param {{hint: string, cancel: string}} labels — already localized; the
 *   page process has no access to `chrome.i18n`.
 */
function startPick(stateKey, labels) {
  const previous = globalThis[stateKey];
  if (previous && typeof previous.teardown === 'function') previous.teardown();

  const doc = document;
  const host = doc.createElement('div');
  // The extractor skips elements carrying this marker: the overlay is
  // capture chrome, never page content. Belt to `beginCapture`'s braces — a
  // full-page capture can start while the picker is still armed (popup
  // reopened without picking), and the overlay must not import as content.
  host.setAttribute('data-openpencil-ui', '');
  // A max z-index and `position: fixed` put the overlay above page content
  // without touching any page element. `pointer-events: none` keeps
  // `elementFromPoint` and the page's own hover states honest — the overlay
  // must never be what the cursor is over.
  host.style.cssText = [
    'position:fixed',
    'inset:0',
    'z-index:2147483647',
    'pointer-events:none',
    'contain:strict',
  ].join(';');

  const box = doc.createElement('div');
  box.style.cssText = [
    'position:fixed',
    'box-sizing:border-box',
    'border:2px solid #29a8db',
    'background:rgba(41,168,219,0.14)',
    'border-radius:2px',
    'display:none',
  ].join(';');

  const tag = doc.createElement('div');
  tag.style.cssText = [
    'position:fixed',
    'padding:3px 6px',
    'background:#29a8db',
    'color:#ffffff',
    'font:600 11px/1.4 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace',
    'border-radius:3px',
    'white-space:nowrap',
    'display:none',
  ].join(';');

  const banner = doc.createElement('div');
  banner.style.cssText = [
    'position:fixed',
    'top:12px',
    'left:50%',
    'transform:translateX(-50%)',
    'padding:7px 12px',
    'background:#0e1620',
    'color:#ffffff',
    'font:500 12px/1.4 system-ui,-apple-system,"Segoe UI",Roboto,"PingFang SC",sans-serif',
    'border-radius:6px',
    'max-width:80vw',
    'text-align:center',
  ].join(';');
  banner.textContent = `${labels.hint} · ${labels.cancel}`;

  host.append(box, tag, banner);
  (doc.body || doc.documentElement).append(host);

  const state = {
    host,
    settle: null,
    hovered: null,
    teardown: null,
  };
  globalThis[stateKey] = state;

  function elementAt(x, y) {
    let found = doc.elementFromPoint(x, y);
    if (!found || found === doc.documentElement) return null;
    // The overlay is `pointer-events: none`, so it should never be returned;
    // guard anyway in case a page stylesheet forces pointer events on.
    if (host.contains(found)) return null;
    // `elementFromPoint` stops at a shadow HOST, which made everything inside
    // a web component unpickable — micro-frontend sub-apps mounted in an open
    // shadow root (wujie, qiankun strictStyleIsolation) could only be picked
    // as one opaque block. Descend through open shadow roots the way DevTools
    // inspect does; closed roots keep the host, which the extractor cannot
    // see into anyway. Bounded: nested hosts, not a general cycle risk.
    //
    // Slot-bearing shadow roots are NOT descended: slotted (light) children
    // are not `childNodes` of anything inside the shadow tree, so a pick that
    // landed on shadow chrome would capture the component minus its content.
    // The host is the one node whose extraction merges both trees, and the
    // extractor already walks its shadow root.
    try {
      for (let depth = 0; found.shadowRoot && depth < 32; depth += 1) {
        if (found.shadowRoot.querySelector('slot')) break;
        const inner = found.shadowRoot.elementFromPoint(x, y);
        if (!inner || inner === found) break;
        found = inner;
      }
    } catch {
      // A hostile shadowRoot getter or hit-test failure falls back to the
      // host; never let a throw escape the armed picker's event handlers.
    }
    return found;
  }

  function draw(element) {
    state.hovered = element;
    if (!element) {
      box.style.display = 'none';
      tag.style.display = 'none';
      return;
    }
    const rect = element.getBoundingClientRect();
    box.style.display = 'block';
    box.style.left = `${rect.left}px`;
    box.style.top = `${rect.top}px`;
    box.style.width = `${rect.width}px`;
    box.style.height = `${rect.height}px`;

    tag.textContent = `${element.tagName.toLowerCase()} · ${Math.round(rect.width)}×${Math.round(rect.height)}`;
    tag.style.display = 'block';
    // Sit the label under the box, or above it when there is no room below.
    const below = rect.bottom + 4;
    const labelHeight = tag.offsetHeight || 20;
    tag.style.top = `${below + labelHeight < innerHeight ? below : Math.max(0, rect.top - labelHeight - 4)}px`;
    tag.style.left = `${Math.max(0, Math.min(rect.left, innerWidth - tag.offsetWidth - 4))}px`;
  }

  function onMove(event) {
    draw(elementAt(event.clientX, event.clientY));
  }

  function onKeyDown(event) {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    finish({ ok: false, reason: 'cancelled' });
  }

  // Every pointer event is swallowed in the capture phase so the pick cannot
  // also activate a link, submit a form, or open a menu. It stays nested
  // despite capturing nothing: `startPick` is serialized to source and
  // evaluated inside the tab, so anything it references must be inside it.
  // oxlint-disable-next-line unicorn/consistent-function-scoping
  function swallow(event) {
    event.preventDefault();
    event.stopPropagation();
  }

  function onPointerDown(event) {
    swallow(event);
    const element = elementAt(event.clientX, event.clientY);
    if (!element) return;
    // Park the chosen element where the extractor reads its root from. The
    // capture runs as a separate injection, so it cannot be passed a live
    // reference any other way.
    globalThis.openpencilSnapshotOptions = { root: element };
    finish({
      ok: true,
      tag: element.tagName.toLowerCase(),
      width: Math.round(element.getBoundingClientRect().width),
      height: Math.round(element.getBoundingClientRect().height),
    });
  }

  const listeners = [
    ['pointermove', onMove, true],
    ['pointerdown', onPointerDown, true],
    ['pointerup', swallow, true],
    ['click', swallow, true],
    ['contextmenu', swallow, true],
    ['keydown', onKeyDown, true],
  ];
  for (const [type, handler, capture] of listeners) {
    addEventListener(type, handler, capture);
  }
  // A navigation drops the isolated world with it, but a same-document route
  // change does not, and neither does a bfcache restore. Both leave an
  // overlay the user cannot dismiss, so treat them as a cancel.
  const onLeave = () => finish({ ok: false, reason: 'cancelled' });
  addEventListener('pagehide', onLeave, true);
  addEventListener('popstate', onLeave, true);

  function teardown() {
    for (const [type, handler, capture] of listeners) {
      removeEventListener(type, handler, capture);
    }
    removeEventListener('pagehide', onLeave, true);
    removeEventListener('popstate', onLeave, true);
    host.remove();
    if (globalThis[stateKey] === state) delete globalThis[stateKey];
  }
  state.teardown = teardown;

  function finish(result) {
    teardown();
    state.result = result;
    if (state.settle) state.settle(result);
  }

  return true;
}

/**
 * Resolve once the armed picker settles.
 *
 * Returns `{ ok: true, tag, width, height }` on a selection,
 * `{ ok: false, reason: 'cancelled' | 'state' | 'timeout' }` otherwise.
 *
 * @param {string} stateKey
 * @param {number} timeoutMs
 */
function awaitPick(stateKey, timeoutMs) {
  const state = globalThis[stateKey];
  // No state at all means the arming injection never landed, or the page
  // navigated between the two calls and took the isolated world with it.
  if (!state) return Promise.resolve({ ok: false, reason: 'state' });
  if (state.result) return Promise.resolve(state.result);
  return new Promise((resolve) => {
    let done = false;
    const settle = (result) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      resolve(result);
    };
    // Without this the overlay would outlive any interest in it — a user who
    // walks away leaves the page unusable until a reload.
    const timer = setTimeout(() => {
      if (typeof state.teardown === 'function') state.teardown();
      settle({ ok: false, reason: 'timeout' });
    }, timeoutMs);
    state.settle = settle;
  });
}

/** Tear the overlay down from outside, e.g. after a failed capture. */
function cancelPick(stateKey) {
  const state = globalThis[stateKey];
  if (state && typeof state.teardown === 'function') state.teardown();
  delete globalThis.openpencilSnapshotOptions;
  return true;
}

/* ---------------------------------------------------------------------- */

/** How long an armed picker waits before giving the page back to the user. */
const PICK_TIMEOUT_MS = 120_000;

async function runInTab(tabId, injection) {
  const results = await chrome.scripting.executeScript({
    target: { tabId, frameIds: [0] },
    ...injection,
  });
  return results && results.length > 0 ? results[0].result : undefined;
}

/**
 * Run one pick in `tabId`.
 *
 * On success the chosen element is left parked on
 * `globalThis.openpencilSnapshotOptions` in the tab's isolated world, which
 * is where the extractor reads its root from — so the caller's next step is
 * `capturePage`, with no argument threading in between.
 *
 * @param {number} tabId
 * @param {{hint: string, cancel: string}} labels
 * @returns {Promise<{tag: string, width: number, height: number}>}
 * @throws {Error} with `code` of `cancelled` / `chunkLost`.
 */
export async function pickElement(tabId, labels) {
  await runInTab(tabId, { func: startPick, args: [PICK_STATE, labels] });
  let result;
  try {
    result = await runInTab(tabId, { func: awaitPick, args: [PICK_STATE, PICK_TIMEOUT_MS] });
  } catch {
    // The tab navigated while we were waiting; the overlay went with it.
    await clearPick(tabId);
    const error = new Error('pick lost');
    error.code = 'chunkLost';
    throw error;
  }
  if (!result || !result.ok) {
    const lost = !result || result.reason === 'state';
    const error = new Error(lost ? 'pick state lost' : 'pick cancelled');
    error.code = lost ? 'chunkLost' : 'cancelled';
    throw error;
  }
  return result;
}

/** Remove any overlay and parked selection left in `tabId`. */
export async function clearPick(tabId) {
  await runInTab(tabId, { func: cancelPick, args: [PICK_STATE] }).catch(() => undefined);
}
