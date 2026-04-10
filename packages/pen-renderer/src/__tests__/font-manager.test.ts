import { describe, it, expect } from 'vitest';
import { SkiaFontManager } from '../font-manager';

// Minimal mock CanvasKit shim — only the bits SkiaFontManager constructor touches.
function makeMockCk(): unknown {
  return {
    TypefaceFontProvider: {
      Make: () => ({ registerFont: () => {} }),
    },
  };
}

describe('SkiaFontManager.pendingCount / flushPending', () => {
  it('starts with pendingCount = 0', () => {
    const fm = new SkiaFontManager(makeMockCk() as never);
    expect(fm.pendingCount()).toBe(0);
  });

  it('flushPending resolves immediately when nothing is pending', async () => {
    const fm = new SkiaFontManager(makeMockCk() as never);
    let resolved = false;
    await fm.flushPending().then(() => {
      resolved = true;
    });
    expect(resolved).toBe(true);
  });

  it('tracks in-flight promises injected via the pendingFetches map', async () => {
    const fm = new SkiaFontManager(makeMockCk() as never);
    // Use private access via cast — testing internals to verify the new
    // public methods read the map correctly without coupling tests to
    // network/font-loading machinery.
    let releaseA: () => void = () => {};
    let releaseB: () => void = () => {};
    const pA = new Promise<boolean>((resolve) => {
      releaseA = () => resolve(true);
    });
    const pB = new Promise<boolean>((resolve) => {
      releaseB = () => resolve(true);
    });
    (fm as unknown as { pendingFetches: Map<string, Promise<boolean>> }).pendingFetches.set(
      'a',
      pA,
    );
    (fm as unknown as { pendingFetches: Map<string, Promise<boolean>> }).pendingFetches.set(
      'b',
      pB,
    );
    expect(fm.pendingCount()).toBe(2);

    let flushResolved = false;
    const flushed = fm.flushPending().then(() => {
      flushResolved = true;
    });
    await new Promise((r) => setTimeout(r, 10));
    expect(flushResolved).toBe(false);

    releaseA();
    releaseB();
    await pA;
    await pB;
    await flushed;
    expect(flushResolved).toBe(true);
  });
});
