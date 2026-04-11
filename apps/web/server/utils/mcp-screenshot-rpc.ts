import { randomUUID } from 'node:crypto';

export interface ScreenshotBounds {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface ScreenshotRequestBody {
  bounds?: ScreenshotBounds | 'root';
  nodeId?: string;
  opts?: { dpr?: number; padding?: number };
  timeoutMs?: number;
}

export interface ScreenshotResponse {
  requestId: string;
  success: boolean;
  pngBase64?: string;
  actualBounds?: ScreenshotBounds;
  error?: string;
}

interface PendingEntry {
  resolve: (data: ScreenshotResponse) => void;
  reject: (err: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

const pendingRequests = new Map<string, PendingEntry>();

/** Allocate a new request id. */
export function allocateRequestId(): string {
  return randomUUID();
}

/**
 * Register a pending screenshot request and return a promise that resolves
 * when the renderer posts its response or rejects on timeout.
 */
export function registerPending(requestId: string, timeoutMs: number): Promise<ScreenshotResponse> {
  return new Promise<ScreenshotResponse>((resolve, reject) => {
    const timer = setTimeout(() => {
      pendingRequests.delete(requestId);
      reject(new Error(`Screenshot request ${requestId} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    pendingRequests.set(requestId, { resolve, reject, timer });
  });
}

/** Called from the renderer-response endpoint. Returns true if matched. */
export function resolvePending(response: ScreenshotResponse): boolean {
  const entry = pendingRequests.get(response.requestId);
  if (!entry) return false;
  clearTimeout(entry.timer);
  pendingRequests.delete(response.requestId);
  entry.resolve(response);
  return true;
}

/** Reject a pending request without waiting for timeout. */
export function rejectPending(requestId: string, err: Error): void {
  const entry = pendingRequests.get(requestId);
  if (!entry) return;
  clearTimeout(entry.timer);
  pendingRequests.delete(requestId);
  entry.reject(err);
}
