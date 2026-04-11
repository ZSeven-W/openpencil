import { describe, it, expect } from 'vitest';
import {
  allocateRequestId,
  registerPending,
  resolvePending,
  rejectPending,
} from '../utils/mcp-screenshot-rpc';

describe('mcp-screenshot-rpc', () => {
  it('allocates unique request ids', () => {
    const a = allocateRequestId();
    const b = allocateRequestId();
    expect(a).not.toBe(b);
    expect(a).toMatch(/^[0-9a-f-]{36}$/);
  });

  it('resolvePending completes a registered request', async () => {
    const id = allocateRequestId();
    const promise = registerPending(id, 5000);
    const response = {
      requestId: id,
      success: true,
      pngBase64: 'aGVsbG8=',
    };
    const ok = resolvePending(response);
    expect(ok).toBe(true);
    const out = await promise;
    expect(out.pngBase64).toBe('aGVsbG8=');
  });

  it('resolvePending returns false for unknown request id', () => {
    expect(resolvePending({ requestId: 'nope', success: true })).toBe(false);
  });

  it('rejectPending surfaces the error to the pending promise', async () => {
    const id = allocateRequestId();
    const promise = registerPending(id, 5000);
    rejectPending(id, new Error('client disconnected'));
    await expect(promise).rejects.toThrow('client disconnected');
  });

  it('timeouts after the specified duration', async () => {
    const id = allocateRequestId();
    const promise = registerPending(id, 50);
    await expect(promise).rejects.toThrow(/timed out/i);
  });
});
