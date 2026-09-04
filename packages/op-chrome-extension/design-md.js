/** End-to-end design.md action used by the MV3 service worker. */

import { captureDesignEvidence } from './design-capture.js';
import { generateDesignMd } from './client.js';
import { getCore } from './core-registry.js';
import { designEvidenceForNetwork } from './design-evidence.js';

/** Pure, language-free reason stored for the popup to render later. */
export function designFallbackReason(error, endpoint) {
  const code = String((error && error.code) || '');
  if (code === 'offline') return { key: 'designFallbackOffline', args: [String(endpoint || '')] };
  if (code === 'extensionNotPaired') return { key: 'designFallbackPairing', args: [] };
  if (code === 'unsupported') return { key: 'designFallbackUnsupported', args: [] };
  if (code === 'noModel') return { key: 'designFallbackModel', args: [] };
  const detail = String((error && (error.detail || error.message)) || error || 'unknown error');
  return { key: 'designFallbackError', args: [detail] };
}

/** Pure classification of the service worker's immediate acknowledgement. */
export function workerActionResponseError(response) {
  if (!response || response.ok !== false || !response.busy) return null;
  const error = new Error('another capture or design.md action is already running');
  error.code = 'actionBusy';
  return error;
}

/** Whether a stored worker result still owns the shared toolbar badge. */
export function hasFreshWorkerResult(records, now) {
  return records.some(
    (record) => record && Number.isFinite(record.expiresAt) && record.expiresAt > now,
  );
}

function markdownDataUrl(markdown) {
  const bytes = new TextEncoder().encode(markdown);
  let binary = '';
  for (let index = 0; index < bytes.length; index += 0x8000) {
    binary += String.fromCharCode.apply(null, bytes.subarray(index, index + 0x8000));
  }
  return `data:text/markdown;charset=utf-8;base64,${btoa(binary)}`;
}

async function downloadMarkdown(markdown) {
  try {
    await chrome.downloads.download({
      url: markdownDataUrl(markdown),
      filename: getCore().designMdFilename(),
      saveAs: false,
    });
  } catch (cause) {
    const error = new Error(String((cause && cause.message) || cause));
    error.code = 'designDownload';
    throw error;
  }
}

/**
 * Capture, try the local intelligent route, deterministically fall back, then
 * download the fixed `design.md` filename.
 */
export async function createDesignMd(tabId, endpoint) {
  const evidence = await captureDesignEvidence(tabId);
  let markdown;
  let intelligent = false;
  let reason = null;
  try {
    const generated = await generateDesignMd(endpoint, designEvidenceForNetwork(evidence));
    markdown = generated.markdown;
    intelligent = true;
  } catch (error) {
    reason = designFallbackReason(error, endpoint);
    const fallback = JSON.parse(getCore().evidenceToDesignMd(JSON.stringify(evidence)));
    if (!fallback.ok || typeof fallback.markdown !== 'string' || fallback.markdown.trim() === '') {
      const fallbackError = new Error(String(fallback.error || 'empty deterministic design.md'));
      fallbackError.code = 'designFallback';
      fallbackError.smartCause = error;
      throw fallbackError;
    }
    markdown = fallback.markdown;
  }
  if (typeof markdown !== 'string' || markdown.trim() === '') {
    const error = new Error('OpenPencil returned an empty design.md');
    error.code = 'designEmpty';
    throw error;
  }
  await downloadMarkdown(markdown);
  return {
    intelligent,
    reason,
    elementCount: evidence.elementCount,
    truncated: evidence.truncated,
  };
}
