/**
 * Service worker: owns long-lived actions which cannot live in the popup.
 *
 * Chrome closes a popup the moment the user clicks into the page, and every
 * `chrome.*` call from a torn-down popup stops settling. Picking an element
 * *requires* that click, so the popup can only start the flow — arming the
 * picker, capturing the chosen subtree, delivering it, and reporting the
 * outcome all have to happen somewhere that outlives it. That is here.
 *
 * Because there is no UI left to report into, the outcome is delivered two
 * ways, both of which the user sees without being asked to keep anything
 * open:
 *
 * * a badge on the toolbar icon — `✓` or `✗`, coloured;
 * * a stored result line the popup renders the next time it opens, at which
 *   point the badge is cleared.
 *
 * The delivery mode is not asked for again either: the pick reuses whichever
 * of "send" or "download" the user last chose in the popup, persisted under
 * `lastAction`, and — for a send — whichever destination is stored under
 * `deliveryTarget`. One button, one predictable behaviour.
 *
 * # The account destination, from here
 *
 * Uploading to the hub needs the session's CSRF token, and that token lives in
 * POPUP memory by design: it is never written to storage, so there is nothing
 * here to read. The worker therefore obtains its own by running the same
 * session probe the popup runs (`account.js`'s `fetchSession`), which also
 * settles the question that matters more — whether the session is still live
 * at the moment of the pick, minutes or hours after the popup last looked.
 *
 * That is why `account.js` is imported here rather than duplicated: it depends
 * on nothing but `core-registry.js`, so pulling it into the worker's graph
 * adds no dynamic `import()` (which `ServiceWorkerGlobalScope` forbids, see
 * `scripts/check-sw-imports.mjs`). The popup's `wasm-core.js` loader stays out
 * of that graph, as it always has.
 */

import { deliveryTarget, fetchSession, hubOrigin, storedRegion } from './account.js';
import { capturePage } from './capture.js';
import { importSnapshot, sendSnapshotToAccount } from './client.js';
import {
  coreError,
  getCore,
  hasCore,
  invalidateCore,
  isCoreTrap,
  setCore,
} from './core-registry.js';
import { initLocale, t } from './i18n.js';
import { createDesignMd, hasFreshWorkerResult } from './design-md.js';
import { clearPick, pickElement } from './picker.js';

// STATIC import, and it has to be. `import()` is disallowed on
// `ServiceWorkerGlobalScope` by the HTML specification (w3c/ServiceWorker
// issue 1356) — not merely after startup, but at all: Chrome rejects the call
// itself, whenever it is made. So the worker cannot use the popup's
// `wasm-core.js` loader, and must name the shim at parse time instead. The
// path is fixed at build time (`scripts/build-wasm.sh` writes exactly this
// file), so there is nothing to resolve dynamically.
//
// The consequence is deliberate: in a checkout where `wasm/` has not been
// built, this import fails and the worker does not register at all. That is
// the honest outcome — the extension is unbuilt — and the popup, which
// imports none of this graph, still comes up on its own and says which script
// to run. `scripts/check-sw-imports.mjs` fails the build if a dynamic
// `import(` ever appears in this file or anything it reaches.
import initCore, * as coreExports from './wasm/op_chrome_extension_core.js';

const ENDPOINT_KEY = 'endpoint';
const LAST_ACTION_KEY = 'lastAction';
const PICK_RESULT_KEY = 'pickResult';
/** Independent from pickResult: design.md never changes pick delivery state. */
const DESIGN_RESULT_KEY = 'designResult';

/** The `.wasm` beside the shim, addressed absolutely for the worker. */
const CORE_WASM = 'wasm/op_chrome_extension_core_bg.wasm';

/** In-flight initialization, so concurrent wakes share one instantiation. */
let initializing = null;

/**
 * Initialize the statically imported core. Idempotent.
 *
 * Only the *instantiation* is asynchronous here, and async work is fine in a
 * worker — it is dynamic `import()` alone that is banned. The `.wasm` URL is
 * passed explicitly rather than left to the shim's `import.meta.url` default,
 * so the fetch is unambiguous no matter how the worker was started.
 *
 * A worker is evicted when idle and re-evaluated on the next event, so this
 * runs again per wake; it costs a fetch from local disk plus a compile.
 */
function ensureCore() {
  if (hasCore()) return Promise.resolve(getCore());
  if (!initializing) {
    initializing = (async () => {
      await initCore({ module_or_path: chrome.runtime.getURL(CORE_WASM) });
      return setCore(coreExports);
    })().catch((cause) => {
      initializing = null;
      // The shim resolved (a static import that did not resolve would have
      // stopped the worker registering), so this can only be an
      // initialization failure — never "the extension has not been built".
      throw coreError('wasmInit', cause);
    });
  }
  return initializing;
}

/** Badge freshness window, evaluated whenever the worker next wakes. */
const BADGE_MS = 90_000;

const BADGE = {
  ok: { text: '✓', color: '#128a63' },
  error: { text: '✗', color: '#c0392b' },
};

let badgeGeneration = 0;

/**
 * Record the outcome for the next popup, and flash it on the toolbar icon.
 *
 * The message is stored as a key plus substitutions rather than as rendered
 * text: the popup renders it, so a language change between the pick and the
 * next popup open still shows the right language. A substitution may itself
 * be a `{ key, args }` reference — `statusPicked` nests the delivery message
 * that way rather than rendering it here, which is what keeps this whole
 * record language-free.
 *
 * The badge is exempt: `✓` / `✗` need no locale.
 */
async function reportResult(storageKey, tone, key, args, detail) {
  badgeGeneration += 1;
  const now = Date.now();
  await chrome.storage.local.set({
    [storageKey]: {
      tone,
      key,
      args: args || [],
      detail: detail || null,
      at: now,
      expiresAt: now + BADGE_MS,
    },
  });
  const badge = BADGE[tone];
  await chrome.action.setBadgeBackgroundColor({ color: badge.color });
  await chrome.action.setBadgeText({ text: badge.text });
}

const report = (tone, key, args) => reportResult(PICK_RESULT_KEY, tone, key, args);
const reportDesign = (tone, key, args, detail) =>
  reportResult(DESIGN_RESULT_KEY, tone, key, args, detail);

/** A worker kept alive by chrome calls may still receive a second popup press. */
let designInFlight = false;
let pickInFlight = false;

/** Clear a stale badge on wake without requesting the alarms permission. */
async function clearExpiredBadge() {
  const generation = badgeGeneration;
  const stored = await chrome.storage.local.get([PICK_RESULT_KEY, DESIGN_RESULT_KEY]);
  if (generation !== badgeGeneration) return;
  if (!hasFreshWorkerResult([stored[PICK_RESULT_KEY], stored[DESIGN_RESULT_KEY]], Date.now())) {
    await chrome.action.setBadgeText({ text: '' });
  }
}

clearExpiredBadge().catch(() => undefined);

/**
 * Turn snapshot text into something `chrome.downloads` can fetch.
 *
 * MV3 service workers do not expose `URL.createObjectURL`, so the popup's
 * blob-URL route is unavailable here and the bytes have to travel inside the
 * URL itself. Base64 costs 4/3; percent-encoding would cost nearer 3× on
 * JSON, which is all quotes and braces.
 */
function dataUrl(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  // `String.fromCharCode(...bytes)` blows the argument limit on anything
  // large, so build the binary string in bounded slices.
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode.apply(null, bytes.subarray(index, index + 0x8000));
  }
  return `data:application/json;base64,${btoa(binary)}`;
}

/**
 * Resolve the account destination for a send, or null when this is not one.
 *
 * Two calls to the core's rule, with the hub probe between them. The first is
 * a cheap pre-check — `deliveryTarget(true)` answers "account" only when the
 * user actually stored that choice — so a user who never touched the setting
 * never pays for a network round trip on a pick. The second re-asks with what
 * the probe found, which is where an expired session collapses back to the
 * local editor, in the core, exactly as it does in the popup.
 *
 * @returns {Promise<{origin: string, csrfToken?: string, fellBack?: boolean} | null>}
 */
async function accountDelivery() {
  if ((await deliveryTarget(true)) !== 'account') return null;
  const origin = await hubOrigin(await storedRegion());
  let view;
  try {
    view = await fetchSession(origin);
  } catch {
    // An unreachable hub is not a reason to lose the capture: it resolves the
    // same way an expired session does, and the report says where it went.
    view = { state: 'error' };
  }
  if ((await deliveryTarget(view.state === 'signedIn')) !== 'account') {
    return { origin, fellBack: true };
  }
  return { origin, csrfToken: view.csrfToken };
}

async function deliver(mode, text, meta, endpoint, account) {
  if (mode === 'download') {
    // Same conversion as the popup: hand back a ready-to-open `.op` document,
    // not the raw snapshot. An empty capture surfaces as an actionable error
    // rather than a broken file.
    const converted = JSON.parse(getCore().snapshotToOpDocument(text, String(meta.title || '')));
    if (!converted.ok) {
      const error = new Error(String(converted.error || 'empty capture'));
      error.code = 'empty';
      throw error;
    }
    const filename = getCore().opFilename(String(meta.title || ''));
    try {
      await chrome.downloads.download({ url: dataUrl(converted.op), filename, saveAs: false });
    } catch (cause) {
      const error = new Error(String((cause && cause.message) || cause));
      error.code = 'download';
      throw error;
    }
    return { key: 'statusDownloaded', args: [filename] };
  }
  if (account && account.csrfToken) {
    const filed = await sendSnapshotToAccount(account.origin, account.csrfToken, text, meta);
    return { key: 'statusSentToAccount', args: [filed.name] };
  }
  const outcome = await importSnapshot(endpoint, text);
  // A pick the user aimed at their account that ended up here went local
  // because the session was gone. Saying so is the difference between a
  // surprise and an explanation.
  return {
    key: account && account.fellBack ? 'statusSentLocalFallback' : 'statusImported',
    args: [String(outcome.nodeCount)],
  };
}

/**
 * Map a thrown error onto the same message keys the popup uses, so a failure
 * reads identically whether it happened in the popup or out here.
 */
function failureMessage(error, endpoint) {
  const code = error && error.code;
  const detail = String((error && (error.detail || error.message)) || error);
  // `forbidden` means two different things depending on where the capture was
  // going — "this editor has no extension ingress" locally, "origin or CSRF"
  // at the hub — so the hub's failures are routed by the tag `client.js` puts
  // on them rather than by the code alone.
  if (error && error.account) return accountFailureMessage(code, detail, error);
  switch (code) {
    case 'cancelled':
      return { key: 'statusPickCancelled', args: [] };
    case 'restricted':
      return { key: 'errorRestrictedTab', args: [] };
    case 'noTab':
      return { key: 'errorNoTab', args: [] };
    case 'empty':
      return { key: 'errorNoSnapshot', args: [] };
    case 'chunkLost':
      return { key: 'errorChunkLost', args: [] };
    case 'offline':
      return { key: 'errorOffline', args: [endpoint] };
    case 'timeout':
      return { key: 'errorTimeout', args: [endpoint, detail] };
    case 'tooLarge':
      return { key: 'errorTooLarge', args: [detail] };
    case 'forbidden':
      return { key: 'errorNoIngress', args: [detail] };
    case 'import':
      return { key: 'errorImport', args: [detail] };
    case 'download':
      return { key: 'errorDownload', args: [detail] };
    case 'wasmInit':
    case 'wasmMissing':
      // Never `errorCoreMissing` from here. The worker only runs at all when
      // its static import of the shim resolved, so the build product IS
      // present; telling the user to build it would send them to fix
      // something that is not broken.
      return { key: 'errorCoreInit', args: [detail] };
    default:
      return { key: 'errorInjection', args: [detail] };
  }
}

/**
 * The same mapping for a failed upload to the account.
 *
 * Deliberately the message set the popup uses for these codes, so the outcome
 * of a pick reads exactly like the outcome of the button.
 */
function accountFailureMessage(code, detail, error) {
  switch (code) {
    case 'signedOut':
      return { key: 'errorAccountSignedOut', args: [] };
    case 'forbidden':
      return { key: 'errorAccountForbidden', args: [detail] };
    case 'quota':
      return {
        key: 'errorAccountQuota',
        args: [String(getCore().hubQuotaItems()), String(getCore().hubQuotaTotalMb()), detail],
      };
    case 'rateLimited':
      return accountRateLimitMessage(error.retryAfterSeconds, detail);
    // NOT `errorTooLarge`: the hub's 32 MiB cap is smaller than the local
    // ingest cap, so a capture in between imports fine locally — the local
    // message's "download the file" remedy would be wrong here.
    case 'tooLarge':
      return { key: 'errorAccountTooLarge', args: [detail] };
    case 'rejected':
      return { key: 'errorAccountRejected', args: [detail] };
    case 'unavailable':
      return { key: 'errorAccountUnavailable', args: [detail] };
    case 'hubOffline':
    case 'hubTimeout':
    case 'hubNotPermitted':
      return { key: 'accountUnavailable', args: [detail] };
    default:
      return { key: 'errorAccountUnavailable', args: [detail] };
  }
}

/** "Try again in about N minutes", or seconds, or without a number. */
function accountRateLimitMessage(seconds, detail) {
  if (typeof seconds !== 'number') return { key: 'errorAccountRateLimited', args: [detail] };
  if (seconds < 60) return { key: 'errorAccountRateLimitedSeconds', args: [String(seconds)] };
  return { key: 'errorAccountRateLimitedMinutes', args: [String(Math.ceil(seconds / 60))] };
}

/**
 * Keep the worker alive across the pick.
 *
 * Everything else in the flow is a steady stream of `chrome.*` calls, each of
 * which resets the idle timer. Waiting for the user to click is the one gap
 * that can run for minutes with no API traffic at all, and an evicted worker
 * would drop the pick silently — the overlay would stay up over a page whose
 * click goes nowhere.
 */
function keepAlive() {
  // Chrome 110+ resets the MV3 idle timer on extension API calls. The
  // manifest pins that minimum specifically so this is a guarantee, not a
  // best-effort trick on older workers.
  const timer = setInterval(() => {
    chrome.runtime.getPlatformInfo().catch(() => undefined);
  }, 20_000);
  return () => clearInterval(timer);
}

async function runPick(tabId) {
  // The overlay's two lines are the only text this worker renders itself, and
  // they go onto the user's page — so they follow the language chosen in the
  // popup, not the browser's. A worker is re-evaluated after each eviction,
  // hence the load here rather than at module scope.
  await initLocale();
  const stored = await chrome.storage.local.get([ENDPOINT_KEY, LAST_ACTION_KEY]);
  const endpoint = stored[ENDPOINT_KEY] || getCore().defaultEndpoint();
  const mode = stored[LAST_ACTION_KEY] === 'download' ? 'download' : 'send';

  const stopKeepAlive = keepAlive();
  let picked;
  try {
    picked = await pickElement(tabId, {
      hint: t('pickHint'),
      cancel: t('pickCancelHint'),
    });
  } finally {
    stopKeepAlive();
  }

  const { text, meta } = await capturePage(tabId, undefined, { pickedRoot: true });
  const account = mode === 'send' ? await accountDelivery() : null;
  // An upload to the hub can run for a minute or more with no `chrome.*` call
  // in it, which is exactly the gap that gets a worker evicted — the same
  // reason the pick itself is wrapped.
  const stopDeliveryKeepAlive = keepAlive();
  let delivered;
  try {
    delivered = await deliver(mode, text, meta, endpoint, account);
  } finally {
    stopDeliveryKeepAlive();
  }
  await report('ok', 'statusPicked', [
    `${picked.tag} ${picked.width}×${picked.height}`,
    { key: delivered.key, args: delivered.args },
  ]);
}

/** Generate design.md without consulting or changing lastAction. */
async function runDesign(tabId) {
  const stored = await chrome.storage.local.get(ENDPOINT_KEY);
  const endpoint = stored[ENDPOINT_KEY] || getCore().defaultEndpoint();
  const outcome = await createDesignMd(tabId, endpoint);
  const detail = {
    key: outcome.truncated ? 'designEvidenceTruncated' : 'designEvidenceSummary',
    args: [String(outcome.elementCount)],
  };
  if (outcome.intelligent) {
    await reportDesign('ok', 'statusDesignSmartSuccess', [], detail);
  } else {
    await reportDesign('ok', 'statusDesignFallbackSuccess', [outcome.reason], detail);
  }
}

function designFailureMessage(error) {
  const detail = String((error && (error.detail || error.message)) || error);
  switch (error && error.code) {
    case 'restricted':
      return { key: 'errorRestrictedTab', args: [] };
    case 'wasmInit':
    case 'wasmMissing':
      return { key: 'errorCoreInit', args: [detail] };
    default:
      return { key: 'errorDesign', args: [detail] };
  }
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (!message || (message.type !== 'pick' && message.type !== 'designMd')) return undefined;
  const tabId = message.tabId;
  if (designInFlight || pickInFlight) {
    sendResponse({ ok: false, busy: true });
    return false;
  }
  if (message.type === 'designMd') designInFlight = true;
  else pickInFlight = true;
  // The popup is about to close; acknowledge before it does so it can.
  sendResponse({ ok: true });
  (async () => {
    try {
      const resultKey = message.type === 'designMd' ? DESIGN_RESULT_KEY : PICK_RESULT_KEY;
      await chrome.storage.local.remove(resultKey);
      await chrome.action.setBadgeText({ text: '' });
      await ensureCore();
      if (message.type === 'designMd') {
        const stopKeepAlive = keepAlive();
        try {
          await runDesign(tabId);
        } finally {
          stopKeepAlive();
        }
      } else {
        await runPick(tabId);
      }
    } catch (error) {
      if (isCoreTrap(error)) invalidateCore();
      if (message.type === 'designMd') {
        const failure = designFailureMessage(error);
        await reportDesign('error', failure.key, failure.args);
      } else {
        await clearPick(tabId).catch(() => undefined);
        const stored = await chrome.storage.local.get(ENDPOINT_KEY);
        const failure = failureMessage(error, stored[ENDPOINT_KEY] || '');
        // A cancel is the user's own decision, not a failure to flag in red.
        const tone = failure.key === 'statusPickCancelled' ? 'ok' : 'error';
        await report(tone, failure.key, failure.args);
      }
    } finally {
      if (message.type === 'designMd') designInFlight = false;
      else pickInFlight = false;
    }
  })();
  // Nothing further is sent, but keeping the channel nominally open costs
  // nothing and documents that the work outlives this handler.
  return false;
});
