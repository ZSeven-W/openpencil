import { describe, it, expect, beforeEach } from 'vitest';
import {
  registerSSEClient,
  unregisterSSEClient,
  setSyncDocument,
  setSyncSelection,
  markClientActive,
  getLastActiveClientId,
  isClientConnected,
  sendToClient,
  clearSyncState,
} from '../utils/mcp-sync-state';
import type { PenDocument } from '../../src/types/pen';

describe('mcp-sync-state: active client tracking', () => {
  beforeEach(() => {
    clearSyncState();
  });

  it('getLastActiveClientId returns null initially', () => {
    expect(getLastActiveClientId()).toBeNull();
  });

  it('setSyncDocument updates lastActiveClientId when sourceClientId provided', () => {
    registerSSEClient('client-a', { push: () => {} });
    setSyncDocument({ version: '1.0.0', children: [] } as PenDocument, 'client-a');
    expect(getLastActiveClientId()).toBe('client-a');
  });

  it('setSyncSelection updates lastActiveClientId', () => {
    registerSSEClient('client-b', { push: () => {} });
    setSyncSelection(['node-1'], 'page-1', 'client-b');
    expect(getLastActiveClientId()).toBe('client-b');
  });

  it('markClientActive updates lastActiveClientId only for connected clients', () => {
    registerSSEClient('client-c', { push: () => {} });
    markClientActive('client-c');
    expect(getLastActiveClientId()).toBe('client-c');
    markClientActive('client-nonexistent');
    expect(getLastActiveClientId()).toBe('client-c');
  });

  it('isClientConnected reflects registration state', () => {
    registerSSEClient('client-d', { push: () => {} });
    expect(isClientConnected('client-d')).toBe(true);
    unregisterSSEClient('client-d');
    expect(isClientConnected('client-d')).toBe(false);
  });

  it('sendToClient dispatches payload to a specific client', () => {
    const received: string[] = [];
    registerSSEClient('client-e', { push: (data: string) => received.push(data) });
    const ok = sendToClient('client-e', { type: 'screenshot:request' });
    expect(ok).toBe(true);
    expect(received).toHaveLength(1);
    expect(JSON.parse(received[0]).type).toBe('screenshot:request');
  });

  it('sendToClient returns false for unknown client', () => {
    expect(sendToClient('nope', { type: 'x' })).toBe(false);
  });
});
