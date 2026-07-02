import { afterEach, describe, expect, it, vi } from 'vitest';
import { startSSEKeepAlive } from '../utils/sse-keepalive';
import { touchSession } from '../utils/agent-sessions';

describe('startSSEKeepAlive', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('emits immediately and keeps emitting until cleared', () => {
    vi.useFakeTimers();

    const send = vi.fn();
    const timer = startSSEKeepAlive(send, 3_000);

    expect(send).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(9_000);
    expect(send).toHaveBeenCalledTimes(4);

    clearInterval(timer);
    vi.advanceTimersByTime(9_000);
    expect(send).toHaveBeenCalledTimes(4);
  });

  it('can keep an agent session active while the stream is only sending pings', () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000);

    const session = { lastActivity: 0 };
    const timer = startSSEKeepAlive(() => touchSession(session), 5_000);

    expect(session.lastActivity).toBe(1_000);

    vi.advanceTimersByTime(5_000);

    expect(session.lastActivity).toBe(6_000);

    clearInterval(timer);
  });
});
