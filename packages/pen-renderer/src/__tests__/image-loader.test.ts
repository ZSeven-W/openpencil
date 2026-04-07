import { describe, it, expect } from 'vitest';
import { SkiaImageLoader } from '../image-loader';

function makeMockCk(): unknown {
  // SkiaImageLoader constructor only stores `ck`; the load path uses
  // ck.MakeImage which we won't reach in these unit tests.
  return {};
}

describe('SkiaImageLoader.pendingCount / flushPending', () => {
  it('starts with pendingCount = 0', () => {
    const il = new SkiaImageLoader(makeMockCk() as never);
    expect(il.pendingCount()).toBe(0);
  });

  it('flushPending resolves immediately when nothing is pending', async () => {
    const il = new SkiaImageLoader(makeMockCk() as never);
    let resolved = false;
    await il.flushPending().then(() => {
      resolved = true;
    });
    expect(resolved).toBe(true);
  });

  it('tracks in-flight promises injected via the pendingPromises set', async () => {
    const il = new SkiaImageLoader(makeMockCk() as never);
    let release: () => void = () => {};
    const p = new Promise<void>((resolve) => {
      release = () => resolve();
    });
    (il as unknown as { pendingPromises: Set<Promise<unknown>> }).pendingPromises.add(p);
    expect(il.pendingCount()).toBe(1);

    let flushResolved = false;
    const flushed = il.flushPending().then(() => {
      flushResolved = true;
    });
    await new Promise((r) => setTimeout(r, 10));
    expect(flushResolved).toBe(false);

    release();
    await p;
    await flushed;
    expect(flushResolved).toBe(true);
  });
});
