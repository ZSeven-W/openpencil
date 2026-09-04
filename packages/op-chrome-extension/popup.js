/**
 * Popup controller: capture the active tab, then import or download it.
 *
 * This file owns the browser-facing half of the extension — `chrome.tabs`,
 * `chrome.downloads`, `chrome.storage` and the popup DOM. The rules it applies
 * (which endpoints are addressable, what a download is named) come from the
 * Rust core; nothing here re-derives them. UI text comes from `i18n.js`, not
 * `chrome.i18n`, because the language is the user's choice here rather than
 * the browser's — see that file's header.
 *
 * The one flow that does NOT run here is the element pick: Chrome closes the
 * popup as soon as the user clicks into the page, so arming the picker,
 * capturing, delivering and reporting all belong to the service worker
 * (`background.js`). This file starts that flow and, on its next open, shows
 * whatever the worker recorded.
 *
 * The status line and every sentence that can appear in it live next door in
 * `popup-status.js`: the mapping from a failure code to a message grows with
 * each destination, while this controller does not, and keeping all of it in
 * one file is what makes it checkable.
 */

import {
  cacheAccount,
  cachedAccount,
  clearCachedAccount,
  deliveryTarget,
  fetchSession,
  hubOrigin,
  openAccountPage,
  openSignIn,
  persistDeliveryTarget,
  persistRegion,
  signOut,
  storedRegion,
} from './account.js';
import { capturePage } from './capture.js';
import {
  defaultEndpoint,
  importSnapshot,
  normalizeEndpoint,
  sendSnapshotToAccount,
} from './client.js';
import { coreError, getCore, invalidateCore, isCoreTrap } from './core-registry.js';
import {
  currentLocale,
  initLocale,
  isSupportedLocale,
  LOCALE_KEY,
  persistLocale,
  setLocale,
  SUPPORTED_LOCALES,
  t,
} from './i18n.js';
import {
  captureDetail,
  localizeDocument,
  msg,
  renderStatus,
  reportAccountError,
  reportCaptureError,
  reportDeliveryError,
  setStatus,
} from './popup-status.js';
import { showPendingWorkerResult, startWorkerAction } from './popup-worker-actions.js';
// The popup's own loader — dynamic, so an unbuilt checkout still renders an
// actionable message instead of a blank window. The service worker MUST NOT
// import this file; see `core-registry.js`.
import { loadCore } from './wasm-core.js';

const ENDPOINT_KEY = 'endpoint';
/** Which delivery the user last chose; the element pick reuses it. */
const LAST_ACTION_KEY = 'lastAction';
/** Schemes Chrome lets `activeTab` + `scripting` reach. */
const INJECTABLE = /^(https?|file):/i;

const captureButton = document.getElementById('capture');
const designButton = document.getElementById('design-md');
const pickButton = document.getElementById('pick');
const downloadButton = document.getElementById('download');
const endpointInput = document.getElementById('endpoint');
const endpointSummary = document.getElementById('endpoint-summary');
const localeSelect = document.getElementById('locale');
const regionSelect = document.getElementById('region');
const deliveryRow = document.getElementById('delivery-row');
const deliverySelect = document.getElementById('delivery');
const accountRow = document.getElementById('account');

/** Last successful capture in this popup session: `{ text, meta }`. */
let captured = null;
let busy = false;

/**
 * The account row's model: `{ displayName, avatarUrl, region }`, or null when
 * signed out. It may start out as the CACHED row from the last open — a
 * picture of the last session, replaced (or cleared) as soon as the live
 * probe answers.
 */
let account = null;

/**
 * The session's CSRF token, required by the hub's mutating endpoints — the
 * logout call and, now, every snapshot filed in the account inbox.
 *
 * In memory for the lifetime of this popup only, and set exclusively by a
 * live probe — never restored from storage, never written to it. See
 * `account.js` for the whole trust model.
 */
let csrfToken = null;

/** The hub origin the current region resolves to. */
let currentHubOrigin = '';
/** Guards the sign-in / sign-out controls against a double click. */
let accountBusy = false;

function setBusy(next) {
  busy = next;
  for (const button of [captureButton, designButton, pickButton, downloadButton]) {
    button.disabled = next;
  }
  // The account controls navigate away from the popup, which would abandon a
  // capture mid-transfer, so they go down with all four action buttons.
  if (!accountRow.hidden) renderAccount();
}

async function activeTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab || typeof tab.id !== 'number')
    throw Object.assign(new Error('no tab'), { code: 'noTab' });
  if (!INJECTABLE.test(tab.url || '')) {
    throw Object.assign(new Error('restricted tab'), { code: 'restricted' });
  }
  return tab;
}

/** Capture the page unless this popup session already holds a snapshot. */
async function ensureCapture() {
  if (captured) return captured;
  const tab = await activeTab();
  setStatus(msg('statusCapturing'), 'working', { progress: 0 });
  captured = await capturePage(tab.id, (transferred, total) => {
    const fraction = total === 0 ? 1 : transferred / total;
    setStatus(msg('statusTransferring', [`${Math.round(fraction * 100)}%`]), 'working', {
      progress: fraction,
    });
  });
  return captured;
}

/** Read + validate the endpoint box, storing the normalized form. */
async function commitEndpoint() {
  const endpoint = normalizeEndpoint(endpointInput.value);
  if (!endpoint) {
    setStatus(msg('errorEndpoint'), 'error');
    return null;
  }
  endpointInput.value = endpoint;
  endpointSummary.textContent = endpoint;
  await chrome.storage.local.set({ [ENDPOINT_KEY]: endpoint });
  return endpoint;
}

/** Remember which delivery to reuse when the user picks an element. */
function rememberAction(action) {
  return chrome.storage.local.set({ [LAST_ACTION_KEY]: action });
}

/**
 * Send the capture wherever the user chose.
 *
 * The choice is resolved by the core rather than read from the `<select>`:
 * the stored value, the session state and whether the account target exists at
 * all are one rule in one place (`delivery.rs`), and a session that expired
 * while this popup was open collapses back to the local editor there.
 */
async function onCapture() {
  if (busy) return;
  const target = await deliveryTarget(account !== null);
  if (target === 'account') return sendToAccount();
  return sendToLocal();
}

async function sendToLocal() {
  const endpoint = await commitEndpoint();
  if (!endpoint) return;

  setBusy(true);
  try {
    const { text, meta } = await ensureCapture();
    setStatus(msg('statusSending'), 'working');
    const outcome = await importSnapshot(endpoint, text);
    await rememberAction('send');
    setStatus(msg('statusImported', [String(outcome.nodeCount)]), 'ok', {
      detail: captureDetail(meta, outcome.warnings),
    });
  } catch (error) {
    if (isCoreTrap(error)) invalidateCore();
    reportDeliveryError(error, endpoint);
  } finally {
    setBusy(false);
  }
}

/**
 * Upload the capture to the signed-in account's inbox.
 *
 * The CSRF token is the one value this flow needs that a cached row cannot
 * supply, so a popup whose opening probe has not landed yet re-probes here
 * rather than failing. That probe is also what keeps the account row honest:
 * a session that ended in another tab is discovered before a 32 MB upload is
 * started, not after.
 */
async function sendToAccount() {
  setBusy(true);
  try {
    const token = await ensureCsrfToken();
    if (!token) return;
    const { text, meta } = await ensureCapture();
    setStatus(msg('statusSendingAccount'), 'working');
    const filed = await sendSnapshotToAccount(currentHubOrigin, token, text, meta);
    await rememberAction('send');
    setStatus(msg('statusSentToAccount', [filed.name]), 'ok', { detail: captureDetail(meta) });
  } catch (error) {
    if (isCoreTrap(error)) invalidateCore();
    // A session that ended between the probe and the upload leaves a row this
    // popup has stopped believing in; clearing it is what sends the next press
    // to the local editor instead of repeating a request that cannot succeed.
    if (error && error.code === 'signedOut') await forgetAccount();
    reportAccountError(error);
  } finally {
    setBusy(false);
  }
}

/**
 * The live session's CSRF token, re-probing when this popup has not seen one.
 *
 * Returns null after writing the status line, so callers can simply stop.
 */
async function ensureCsrfToken() {
  // `refreshAccount` sets this before it paints anything, and nothing can
  // select the account destination before that has happened — but a URL built
  // against an empty origin would be a relative one, and the whole point of
  // pinning the hub set is that no request goes anywhere unnamed.
  if (!currentHubOrigin) currentHubOrigin = await hubOrigin(await storedRegion());
  if (csrfToken) return csrfToken;
  let view;
  try {
    view = await fetchSession(currentHubOrigin);
  } catch (error) {
    setStatus(
      msg('accountUnavailable', [String(error?.detail ?? error?.message ?? error)]),
      'error',
    );
    return null;
  }
  if (view.state !== 'signedIn') {
    // Only a definite `signedOut` clears the row; an `error` means the hub
    // said something we cannot act on, and dropping the session over that
    // would be the same mistake `refreshAccount` refuses to make.
    if (view.state === 'signedOut') await forgetAccount();
    setStatus(msg('errorAccountSignedOut'), 'error');
    return null;
  }
  csrfToken = view.csrfToken;
  account = {
    displayName: view.displayName,
    avatarUrl: view.avatarUrl ?? null,
    region: regionSelect.value,
  };
  await cacheAccount(view, regionSelect.value);
  renderAccount();
  await syncDelivery();
  return csrfToken;
}

/** Drop every trace of the session this popup was holding. */
async function forgetAccount() {
  account = null;
  csrfToken = null;
  await clearCachedAccount();
  renderAccount();
  await syncDelivery();
}

/**
 * Revoke `objectUrl` once download `id` reaches a terminal state.
 *
 * `chrome.downloads.download` resolves as soon as the download STARTS, so
 * revoking right after it would cut a multi-MB snapshot short. A timer backs
 * the listener up in case the event never arrives.
 */
function revokeWhenDownloadSettles(id, objectUrl) {
  let done = false;
  const finish = () => {
    if (done) return;
    done = true;
    chrome.downloads.onChanged.removeListener(onChanged);
    URL.revokeObjectURL(objectUrl);
  };
  function onChanged(delta) {
    if (delta.id === id && delta.state && delta.state.current !== 'in_progress') finish();
  }
  chrome.downloads.onChanged.addListener(onChanged);
  setTimeout(finish, 120_000);
}

async function onDownload() {
  if (busy) return;
  setBusy(true);
  try {
    const { text, meta } = await ensureCapture();
    // Convert the raw snapshot into a ready-to-open `.op` document (the same
    // conversion `op import:snapshot` runs), so the download opens directly by
    // double-click with no CLI step.
    const converted = JSON.parse(getCore().snapshotToOpDocument(text, String(meta.title || '')));
    if (!converted.ok) {
      const error = new Error(String(converted.error || 'empty capture'));
      error.code = 'empty';
      throw error;
    }
    // The page title is attacker-controlled, so the name is sanitised by the
    // core (`filename.rs`) — no title can produce a `..` component or a
    // dot-file.
    const filename = getCore().opFilename(String(meta.title || ''));
    // `application/octet-stream`, NOT json: Chrome's download pipeline
    // second-guesses an unknown suffix against the blob's MIME type and
    // rewrote `<title>.op` to `<title>.json`. A generic byte stream keeps
    // the `.op` name the core built.
    const objectUrl = URL.createObjectURL(
      new Blob([converted.op], { type: 'application/octet-stream' }),
    );
    const id = await chrome.downloads.download({ url: objectUrl, filename, saveAs: false });
    revokeWhenDownloadSettles(id, objectUrl);
    await rememberAction('download');
    setStatus(msg('statusDownloaded', [filename]), 'ok', {
      detail: captureDetail(meta, converted.warnings),
    });
  } catch (error) {
    if (isCoreTrap(error)) invalidateCore();
    if (error && (error.code === 'offline' || error.code === 'import')) {
      setStatus(msg('errorDownload', [String(error.message)]), 'error');
    } else if (error && error.code) {
      reportCaptureError(error);
    } else {
      setStatus(
        msg('errorDownload', [String(error && error.message ? error.message : error)]),
        'error',
      );
    }
  } finally {
    setBusy(false);
  }
}

/**
 * Hand the element pick to the service worker, then get out of the way.
 *
 * The popup cannot stay open across the click that selects an element, so it
 * closes itself deliberately rather than being closed mid-sentence. The
 * worker reports back through the toolbar badge and through `pickResult`,
 * which {@link showPendingPickResult} renders on the next open.
 */
async function onPick() {
  if (busy) return;
  const endpoint = await commitEndpoint();
  if (!endpoint) return;

  setBusy(true);
  try {
    const tab = await activeTab();
    try {
      await startWorkerAction('pick', tab.id);
    } catch (cause) {
      if (cause && cause.code === 'actionBusy') throw cause;
      // No worker answered. The usual reason is that its static import of the
      // core did not resolve, so it never registered — which is a different
      // problem from "this tab cannot be captured", and the only one of the
      // capture buttons this can affect.
      throw coreError('pickUnavailable', cause);
    }
    window.close();
  } catch (error) {
    setBusy(false);
    if (error && error.code === 'actionBusy') setStatus(msg('designBusy'), 'error');
    else reportCaptureError(error);
  }
}

/**
 * Design-system extraction is worker-owned because the local AI request may
 * outlive this popup by two minutes. It is independent of lastAction and of
 * the element pick's pending result.
 */
async function onDesignMd() {
  if (busy) return;
  const endpoint = await commitEndpoint();
  if (!endpoint) return;
  setBusy(true);
  try {
    const tab = await activeTab();
    setStatus(msg('statusDesignWorking'), 'working');
    await startWorkerAction('designMd', tab.id);
    window.close();
  } catch (error) {
    setBusy(false);
    if (error && (error.code === 'noTab' || error.code === 'restricted')) {
      reportCaptureError(error);
    } else if (error && error.code === 'actionBusy') {
      setStatus(msg('designBusy'), 'error');
    } else {
      setStatus(
        msg('errorDesign', [String(error && error.message ? error.message : error)]),
        'error',
      );
    }
  }
}

/* ------------------------------------------------------------ the account */

/**
 * Build one control for the account row.
 *
 * Everything in this row is assembled node by node with `textContent`. None
 * of it is ever handed to `innerHTML`: the display name comes from another
 * user's account profile, and the popup renders it inside the extension's own
 * origin, where a script tag would run with the extension's permissions.
 */
function accountButton(label, className, onClick) {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = `account-link ${className}`;
  button.textContent = label;
  button.disabled = busy || accountBusy;
  button.addEventListener('click', onClick);
  return button;
}

/**
 * Paint the account row from {@link account}.
 *
 * Signed out: one quiet "sign in" button. Signed in: the avatar (when there
 * is one that survived the core's `https://`-only check), the display name,
 * a region badge, and a sign-out control.
 */
function renderAccount() {
  accountRow.replaceChildren();
  accountRow.hidden = false;

  if (!account) {
    accountRow.append(accountButton(t('accountSignIn'), 'account-signin', onSignIn));
    return;
  }

  if (account.avatarUrl) {
    const avatar = document.createElement('img');
    avatar.className = 'account-avatar';
    avatar.src = account.avatarUrl;
    avatar.alt = '';
    // The avatar is decoration beside a name the row already states.
    avatar.setAttribute('aria-hidden', 'true');
    // The hub does not need to know which of its users opened this popup on
    // which page, and an avatar request is not worth a referrer.
    avatar.referrerPolicy = 'no-referrer';
    // A CDN that is down, a blocked image, a URL that 404s: none of those
    // should leave a broken-image glyph in the header.
    avatar.addEventListener('error', () => avatar.remove(), { once: true });
    accountRow.append(avatar);
  }

  const name = document.createElement('span');
  name.className = 'account-name';
  name.textContent = account.displayName;
  name.title = account.displayName;

  const region = document.createElement('span');
  region.className = 'account-region';
  region.textContent = t(account.region === 'global' ? 'regionGlobal' : 'regionCn');

  accountRow.append(name, region, accountButton(t('accountSignOut'), 'account-signout', onSignOut));
}

/** Show or hide the delivery row, and keep its selection legal. */
async function syncDelivery() {
  const signedIn = account !== null;
  deliveryRow.hidden = !getCore().deliveryRowVisible(signedIn);
  deliverySelect.value = await deliveryTarget(signedIn);
}

/**
 * Ask the hub who is signed in, and repaint.
 *
 * The cached row is painted first so the header does not flash empty on
 * every open, then the probe either confirms it, replaces it, or clears it.
 * An `error` state deliberately changes nothing: a hub that is briefly
 * unreachable must not look like a sign-out.
 */
async function refreshAccount() {
  const region = await storedRegion();
  regionSelect.value = region;
  currentHubOrigin = await hubOrigin(region);

  const cached = await cachedAccount();
  // A cached row from the OTHER region belongs to a different identity.
  account = cached && cached.region === region ? cached : null;
  renderAccount();
  await syncDelivery();

  let view;
  try {
    view = await fetchSession(currentHubOrigin);
  } catch (error) {
    // Silent on open: the account row is optional, and the popup's status
    // area belongs to the capture the user came here for. The stale row (if
    // any) stays up rather than being replaced by an error.
    return { state: 'error', detail: String(error?.detail ?? error?.message ?? error) };
  }

  if (view.state === 'signedIn') {
    account = {
      displayName: view.displayName,
      avatarUrl: view.avatarUrl ?? null,
      region,
    };
    csrfToken = view.csrfToken;
    await cacheAccount(view, region);
  } else if (view.state === 'signedOut') {
    account = null;
    csrfToken = null;
    await clearCachedAccount();
  }

  renderAccount();
  await syncDelivery();
  return view;
}

/**
 * Start the sign-in.
 *
 * The tab that opens takes focus, which closes this popup — so the status
 * line is written first and the sentence it leaves behind ("finish signing
 * in, then reopen") is what the user reads on the way out.
 */
async function onSignIn() {
  if (accountBusy) return;
  accountBusy = true;
  renderAccount();
  try {
    setStatus(msg('accountSignInOpened'), 'working');
    await openSignIn(currentHubOrigin);
  } catch (error) {
    setStatus(msg('accountUnavailable', [String(error?.message ?? error)]), 'error');
  } finally {
    accountBusy = false;
    renderAccount();
  }
}

/**
 * Sign out.
 *
 * The local state is cleared whatever the hub says: the extension has
 * stopped believing in this session, and a row that outlived the user's
 * decision would be a lie. When the hub refuses the extension-origin logout
 * — which is what today's op-hub does, see `account.js` — the account page
 * opens so the user can finish there.
 */
async function onSignOut() {
  if (accountBusy) return;
  accountBusy = true;
  renderAccount();
  const token = csrfToken;
  try {
    const ended = await signOut(currentHubOrigin, token);
    account = null;
    csrfToken = null;
    await clearCachedAccount();
    if (ended) {
      setStatus(msg('accountSignedOut'), 'ok');
    } else {
      setStatus(msg('accountSignOutManual'), 'working');
      await openAccountPage(currentHubOrigin);
    }
  } finally {
    accountBusy = false;
    renderAccount();
    await syncDelivery();
  }
}

/**
 * Fill the region selector.
 *
 * Unlike the language rows — which are each language's own name, so they are
 * not translated — the two regions are product terms and are read in the
 * popup's language.
 */
function buildRegionSelect() {
  regionSelect.replaceChildren();
  for (const [value, key] of [
    ['cn', 'regionCn'],
    ['global', 'regionGlobal'],
  ]) {
    const option = document.createElement('option');
    option.value = value;
    option.textContent = t(key);
    regionSelect.append(option);
  }
}

/**
 * Fill the delivery selector.
 *
 * Both destinations are selectable now that the hub answers the inbox route.
 * The account option is still gated on the core's `accountDeliveryAvailable`
 * rather than on this file's opinion: if that flag is ever turned off — a hub
 * regression, a region that has not deployed the route — the option stops
 * being offered here without any other edit, exactly as it did while the API
 * was still a proposal.
 */
function buildDeliverySelect() {
  const cloudReady = getCore().accountDeliveryAvailable();
  deliverySelect.replaceChildren();
  for (const [value, key] of [
    ['local', 'deliveryLocal'],
    ['account', 'deliveryAccount'],
  ]) {
    if (value === 'account' && !cloudReady) continue;
    const option = document.createElement('option');
    option.value = value;
    option.textContent = t(key);
    deliverySelect.append(option);
  }
}

/**
 * Fill the language selector.
 *
 * Option labels are each language's own name, so they are NOT translated —
 * a user looking for their language reads it in that language, which is the
 * whole point of a switcher they can find while the UI is in a script they
 * cannot read.
 */
function buildLocaleSelect() {
  localeSelect.replaceChildren();
  for (const { code, name } of SUPPORTED_LOCALES) {
    const option = document.createElement('option');
    option.value = code;
    option.textContent = name;
    localeSelect.append(option);
  }
  localeSelect.value = currentLocale();
}

/** Re-render every string in place. No popup restart, no flash of English. */
async function applyLocale(code) {
  await setLocale(code);
  localizeDocument();
  localeSelect.value = currentLocale();
  // The three JS-built surfaces — the account row and the two selectors —
  // hold rendered text, so they are rebuilt rather than left in the old
  // language beside a header that changed.
  const region = regionSelect.value;
  const delivery = deliverySelect.value;
  buildRegionSelect();
  buildDeliverySelect();
  regionSelect.value = region;
  deliverySelect.value = delivery;
  if (!accountRow.hidden) renderAccount();
  renderStatus();
}

async function init() {
  await initLocale();
  localizeDocument();
  buildLocaleSelect();
  // The switcher is wired before the core is loaded on purpose: the
  // "extension has not been built" path returns early, and a user who lands
  // there must still be able to read the sentence in their own language.
  localeSelect.addEventListener('change', async () => {
    const code = localeSelect.value;
    if (!isSupportedLocale(code) || code === currentLocale()) return;
    await persistLocale(code);
    await applyLocale(code);
  });
  chrome.storage.onChanged.addListener((changes, area) => {
    if (area !== 'local' || !changes[LOCALE_KEY]) return;
    const code = changes[LOCALE_KEY].newValue;
    if (!isSupportedLocale(code) || code === currentLocale()) return;
    applyLocale(code);
  });

  // Nothing else in the popup may run before the core is up: every handler
  // below reaches into it. A checkout whose `wasm/` has not been built lands
  // here, so the failure has to be an explicit, actionable message rather
  // than dead buttons.
  try {
    await loadCore();
  } catch (error) {
    setBusy(true);
    setStatus(
      msg('errorCoreMissing', [String(error && error.message ? error.message : error)]),
      'error',
    );
    return;
  }

  const stored = await chrome.storage.local.get(ENDPOINT_KEY);
  const endpoint = stored[ENDPOINT_KEY] || defaultEndpoint();
  endpointInput.value = endpoint;
  endpointSummary.textContent = endpoint;

  if (!(await showPendingWorkerResult())) setStatus(msg('statusIdle'), 'idle');

  captureButton.addEventListener('click', onCapture);
  designButton.addEventListener('click', onDesignMd);
  pickButton.addEventListener('click', onPick);
  downloadButton.addEventListener('click', onDownload);
  endpointInput.addEventListener('change', async () => {
    if (await commitEndpoint()) setStatus(msg('statusIdle'), 'idle');
  });

  buildRegionSelect();
  buildDeliverySelect();
  regionSelect.addEventListener('change', async () => {
    await persistRegion(regionSelect.value);
    // The two regions are separate deployments with separate accounts, so
    // the cached row from the old one is not a session in the new one.
    await clearCachedAccount();
    account = null;
    csrfToken = null;
    await refreshAccount();
  });
  deliverySelect.addEventListener('change', async () => {
    deliverySelect.value = await persistDeliveryTarget(deliverySelect.value, account !== null);
  });

  // Last, and deliberately not awaited: the account row is optional, and a
  // hub that is slow or unreachable must not delay the capture buttons by so
  // much as a frame. `refreshAccount` already absorbs probe failures; this
  // catch covers the storage and core paths, so nothing about the account can
  // take the popup down — it just leaves the header slot as it found it.
  refreshAccount().catch(() => undefined);
}

init();
