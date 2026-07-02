import { describe, expect, it } from 'vitest';

import { SkiaEngine } from '../skia-engine';

function createCanvasStub() {
  return {
    clientWidth: 1200,
    clientHeight: 800,
    width: 1200,
    height: 800,
  } as HTMLCanvasElement;
}

function createCanvasOps() {
  return {
    clear() {},
    save() {},
    scale() {},
    concat() {},
    restore() {},
  };
}

describe('SkiaEngine surface recovery', () => {
  it('recreates the surface instead of throwing when the current surface is invalid', () => {
    const healthyCanvas = createCanvasOps();
    const healthySurface = {
      getCanvas() {
        return healthyCanvas;
      },
      flush() {},
      delete() {},
    };

    let recreatedSurfaces = 0;
    const ck = {
      Color4f(r: number, g: number, b: number, a: number) {
        return Float32Array.of(r, g, b, a);
      },
      TypefaceFontProvider: {
        Make() {
          return {
            registerFont() {},
            delete() {},
          };
        },
      },
      MakeWebGLCanvasSurface() {
        recreatedSurfaces += 1;
        return healthySurface;
      },
      MakeSWCanvasSurface() {
        return null;
      },
    };

    const engine = new SkiaEngine(ck as any);
    (engine as any).canvasEl = createCanvasStub();
    (engine as any).renderNodes = [];
    engine.surface = {
      getCanvas() {
        throw new Error('Surface instance already deleted');
      },
      flush() {},
      delete() {},
    } as any;

    expect(() => {
      (engine as any).render();
    }).not.toThrow();

    expect(recreatedSurfaces).toBe(1);
    expect(engine.surface).toBe(healthySurface as any);
  });
});

describe('SkiaEngine resize flash prevention', () => {
  it('renders synchronously after recreating the surface on resize', () => {
    const canvasOps = createCanvasOps();
    let flushCount = 0;
    const surface = {
      getCanvas() {
        return canvasOps;
      },
      flush() {
        flushCount += 1;
      },
      delete() {},
    };

    const ck = {
      Color4f(r: number, g: number, b: number, a: number) {
        return Float32Array.of(r, g, b, a);
      },
      TypefaceFontProvider: {
        Make() {
          return {
            registerFont() {},
            delete() {},
          };
        },
      },
      MakeWebGLCanvasSurface() {
        return surface;
      },
      MakeSWCanvasSurface() {
        return null;
      },
    };

    const engine = new SkiaEngine(ck as any);
    const canvasEl = createCanvasStub();
    (engine as any).canvasEl = canvasEl;
    (engine as any).renderNodes = [];
    engine.surface = surface as any;
    (engine as any).dirty = true;

    const originalWindow = (globalThis as any).window;
    (globalThis as any).window = { devicePixelRatio: 1 };
    try {
      engine.resize(800, 600);
    } finally {
      (globalThis as any).window = originalWindow;
    }

    // Synchronous render must have run: dirty cleared, surface flushed once.
    // Without this, canvas.width = ... leaves the element transparent until
    // the next RAF, causing a one-frame white flash on layout changes.
    expect((engine as any).dirty).toBe(false);
    expect(flushCount).toBe(1);
  });
});
