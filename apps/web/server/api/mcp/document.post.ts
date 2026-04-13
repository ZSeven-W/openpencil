import { defineEventHandler, readBody, createError, getRequestHeader, setResponseStatus } from 'h3';
import { setSyncDocument } from '../../utils/mcp-sync-state';
import { serverLog } from '../../utils/server-logger';
import type { PenDocument } from '../../../src/types/pen';

interface PostBody {
  document: PenDocument;
  sourceClientId?: string;
}

interface DocumentStats {
  nodeCount: number;
  imageCount: number;
  dataUrlImageCount: number;
  dataUrlChars: number;
}

function collectDocumentStats(doc: PenDocument): DocumentStats {
  const stats: DocumentStats = {
    nodeCount: 0,
    imageCount: 0,
    dataUrlImageCount: 0,
    dataUrlChars: 0,
  };

  const visit = (nodes?: unknown): void => {
    if (!Array.isArray(nodes)) return;
    for (const node of nodes) {
      if (!node || typeof node !== 'object') continue;
      stats.nodeCount++;

      const typedNode = node as {
        type?: string;
        src?: string;
        children?: unknown;
      };

      if (typedNode.type === 'image') {
        stats.imageCount++;
        if (typeof typedNode.src === 'string' && typedNode.src.startsWith('data:')) {
          stats.dataUrlImageCount++;
          stats.dataUrlChars += typedNode.src.length;
        }
      }

      visit(typedNode.children);
    }
  };

  visit(doc.children);
  if (Array.isArray(doc.pages)) {
    for (const page of doc.pages) {
      visit(page?.children);
    }
  }

  return stats;
}

function formatBytes(bytes: number | null): string {
  if (bytes == null || Number.isNaN(bytes)) return 'unknown';
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)}MiB`;
}

function isConnectionClosedError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false;
  const maybeError = error as { name?: string; message?: string; cause?: unknown };
  const message = maybeError.message ?? '';
  const causeMessage =
    typeof maybeError.cause === 'object' && maybeError.cause
      ? String((maybeError.cause as { message?: string }).message ?? '')
      : '';

  return (
    maybeError.name === 'AbortError' ||
    /connection was closed/i.test(message) ||
    /connection was closed/i.test(causeMessage) ||
    /abort/i.test(message)
  );
}

/** POST /api/mcp/document — Receives document update from MCP or renderer, triggers SSE broadcast. */
export default defineEventHandler(async (event) => {
  const startedAt = Date.now();
  const contentLengthHeader = getRequestHeader(event, 'content-length');
  const bodyBytesHeader = getRequestHeader(event, 'x-openpencil-body-bytes');
  const contentLength = contentLengthHeader
    ? Number.parseInt(contentLengthHeader, 10)
    : bodyBytesHeader
      ? Number.parseInt(bodyBytesHeader, 10)
      : null;
  const sourceClientIdHeader = getRequestHeader(event, 'x-openpencil-client-id') ?? 'unknown';
  let phase = 'readBody';

  try {
    const body = await readBody<PostBody>(event);
    if (!body?.document) {
      throw createError({ statusCode: 400, statusMessage: 'Missing document in request body' });
    }
    const doc = body.document;
    if (!doc.version || (!Array.isArray(doc.children) && !Array.isArray(doc.pages))) {
      throw createError({ statusCode: 400, statusMessage: 'Invalid document format' });
    }

    const stats = collectDocumentStats(doc);
    phase = 'setSyncDocument';
    const version = setSyncDocument(doc, body.sourceClientId);
    const elapsedMs = Date.now() - startedAt;

    serverLog.info(
      `[mcp-document] ok version=${version} sourceClientId=${body.sourceClientId ?? sourceClientIdHeader} ` +
        `contentLength=${formatBytes(contentLength)} nodes=${stats.nodeCount} images=${stats.imageCount} ` +
        `dataUrlImages=${stats.dataUrlImageCount} dataUrlChars=${stats.dataUrlChars} elapsedMs=${elapsedMs}`,
    );

    return { ok: true, version };
  } catch (error) {
    const elapsedMs = Date.now() - startedAt;
    const message = error instanceof Error ? error.message : String(error);

    if (isConnectionClosedError(error)) {
      serverLog.warn(
        `[mcp-document] connection-closed phase=${phase} contentLength=${formatBytes(contentLength)} ` +
          `sourceClientId=${sourceClientIdHeader} elapsedMs=${elapsedMs} message=${message}`,
      );

      // The client already closed the request while Nitro was still reading it.
      // Returning a soft status keeps expected sync churn out of the 500 logs.
      setResponseStatus(event, 202, 'Client closed request during MCP document sync');
      return {
        ok: false,
        aborted: true,
        phase,
      };
    }

    serverLog.error(
      `[mcp-document] failed phase=${phase} contentLength=${formatBytes(contentLength)} ` +
        `sourceClientId=${sourceClientIdHeader} elapsedMs=${elapsedMs} message=${message}`,
    );
    throw error;
  }
});
