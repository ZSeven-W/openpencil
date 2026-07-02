// apps/web/src/utils/__tests__/document-events.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { documentEvents } from '@/utils/document-events';
import { createEmptyDocument } from '@/stores/document-tree-utils';

describe('documentEvents', () => {
  beforeEach(() => {
    documentEvents._clear();
  });

  it('delivers a saved event to a subscriber', () => {
    const handler = vi.fn();
    documentEvents.on('saved', handler);
    documentEvents.emit('saved', {
      filePath: '/tmp/foo.op',
      fileName: 'foo.op',
      document: createEmptyDocument(),
    });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler.mock.calls[0][0].fileName).toBe('foo.op');
  });

  it('returns an unsubscribe function that prevents further deliveries', () => {
    const handler = vi.fn();
    const unsubscribe = documentEvents.on('saved', handler);
    documentEvents.emit('saved', {
      filePath: null,
      fileName: 'a.op',
      document: createEmptyDocument(),
    });
    unsubscribe();
    documentEvents.emit('saved', {
      filePath: null,
      fileName: 'b.op',
      document: createEmptyDocument(),
    });
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('delivers to multiple subscribers', () => {
    const a = vi.fn();
    const b = vi.fn();
    documentEvents.on('saved', a);
    documentEvents.on('saved', b);
    documentEvents.emit('saved', {
      filePath: null,
      fileName: 'x.op',
      document: createEmptyDocument(),
    });
    expect(a).toHaveBeenCalledTimes(1);
    expect(b).toHaveBeenCalledTimes(1);
  });

  it('does not throw when emitting with no subscribers', () => {
    expect(() => {
      documentEvents.emit('saved', {
        filePath: null,
        fileName: 'x.op',
        document: createEmptyDocument(),
      });
    }).not.toThrow();
  });

  it('isolates a handler that throws so other handlers still receive the event', () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    const bad = vi.fn(() => {
      throw new Error('boom');
    });
    const good = vi.fn();
    documentEvents.on('saved', bad);
    documentEvents.on('saved', good);
    documentEvents.emit('saved', {
      filePath: null,
      fileName: 'x.op',
      document: createEmptyDocument(),
    });
    expect(bad).toHaveBeenCalledTimes(1);
    expect(good).toHaveBeenCalledTimes(1);
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });
});
