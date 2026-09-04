/** Popup-side handoff and result rendering for service-worker-owned actions. */

import { msg, setStatus } from './popup-status.js';
import { workerActionResponseError } from './design-md.js';

export const PICK_RESULT_KEY = 'pickResult';
export const DESIGN_RESULT_KEY = 'designResult';

/** Hand one long-lived action to the MV3 worker. */
export async function startWorkerAction(type, tabId) {
  try {
    const response = await chrome.runtime.sendMessage({ type, tabId });
    const responseError = workerActionResponseError(response);
    if (responseError) throw responseError;
  } catch (cause) {
    if (cause && cause.code === 'actionBusy') throw cause;
    const error = new Error(String((cause && cause.message) || cause));
    error.code = 'backgroundUnavailable';
    throw error;
  }
}

/** Render the newest worker result; an older independent result stays queued. */
export async function showPendingWorkerResult() {
  const stored = await chrome.storage.local.get([PICK_RESULT_KEY, DESIGN_RESULT_KEY]);
  let newest = null;
  for (const storageKey of [PICK_RESULT_KEY, DESIGN_RESULT_KEY]) {
    const result = stored[storageKey];
    if (!result || typeof result.key !== 'string') continue;
    if (!newest || Number(result.at || 0) > Number(newest.result.at || 0)) {
      newest = { storageKey, result };
    }
  }
  if (!newest) return false;
  const { storageKey, result } = newest;
  await chrome.storage.local.remove(storageKey);
  await chrome.action.setBadgeText({ text: '' });
  const args = Array.isArray(result.args) ? result.args : [];
  const detail =
    result.detail && typeof result.detail.key === 'string' ? [result.detail] : undefined;
  setStatus(msg(result.key, args), result.tone === 'error' ? 'error' : 'ok', { detail });
  return true;
}
