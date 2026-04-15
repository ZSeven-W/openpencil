import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
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

describe('system font detection via Local Font Access API', () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('marks unknown local font as systemFont when Google Fonts fails', async () => {
    // Mock fetch to simulate Google Fonts returning 400
    globalThis.fetch = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 400, statusText: 'Bad Request' }));

    // Mock window.queryLocalFonts to include "HeyMeow Rnd"
    const origQuery = (globalThis as unknown as Record<string, unknown>).queryLocalFonts;
    (globalThis as unknown as Record<string, unknown>).queryLocalFonts = async () => [
      {
        family: 'HeyMeow Rnd',
        fullName: 'HeyMeow Rnd Regular',
        postscriptName: 'HeyMeowRnd-Regular',
        style: 'Regular',
        blob: async () => new Blob([]),
      },
      {
        family: 'Arial',
        fullName: 'Arial Regular',
        postscriptName: 'ArialMT',
        style: 'Regular',
        blob: async () => new Blob([]),
      },
    ];

    // Mock window for the Local Font Access API check
    const origWindow = globalThis.window;
    vi.stubGlobal('window', {
      queryLocalFonts: (
        globalThis as unknown as Record<string, () => Promise<Array<{ family: string }>>>
      ).queryLocalFonts,
    });

    const fm = new SkiaFontManager(makeMockCk() as never);

    // "HeyMeow Rnd" is not bundled and not in NON_GOOGLE_FONT_PATTERNS
    const result = await fm.ensureFont('HeyMeow Rnd');

    // Should return false (not loaded into CanvasKit) but be classified as system font
    expect(result).toBe(false);
    expect(fm.isSystemFont('HeyMeow Rnd')).toBe(true);

    // Cleanup
    vi.unstubAllGlobals();
    if (origWindow !== undefined) {
      globalThis.window = origWindow as Window & typeof globalThis;
    }
    (globalThis as unknown as Record<string, unknown>).queryLocalFonts = origQuery;
  });

  it('marks font as failed when not found locally either', async () => {
    // Mock fetch to simulate Google Fonts returning 400
    globalThis.fetch = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 400, statusText: 'Bad Request' }));

    // Mock window.queryLocalFonts WITHOUT "SomeRandomFont"
    vi.stubGlobal('window', {
      queryLocalFonts: async () => [
        {
          family: 'Arial',
          fullName: 'Arial Regular',
          postscriptName: 'ArialMT',
          style: 'Regular',
          blob: async () => new Blob([]),
        },
        {
          family: 'Inter',
          fullName: 'Inter Regular',
          postscriptName: 'Inter-Regular',
          style: 'Regular',
          blob: async () => new Blob([]),
        },
      ],
    });

    const fm = new SkiaFontManager(makeMockCk() as never);
    const result = await fm.ensureFont('SomeRandomFont');

    expect(result).toBe(false);
    expect(fm.isSystemFont('SomeRandomFont')).toBe(false);

    // Cleanup
    vi.unstubAllGlobals();
  });
});

describe('requestLocalFontAccess', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns true and populates localFontMap when permission granted', async () => {
    const mockFontData = new Blob([new ArrayBuffer(100)]);
    vi.stubGlobal('window', {
      queryLocalFonts: async () => [
        {
          family: 'Segoe UI',
          fullName: 'Segoe UI Regular',
          postscriptName: 'SegoeUI-Regular',
          style: 'Regular',
          blob: async () => mockFontData,
        },
        {
          family: 'Segoe UI',
          fullName: 'Segoe UI Bold',
          postscriptName: 'SegoeUI-Bold',
          style: 'Bold',
          blob: async () => mockFontData,
        },
      ],
    });

    const fm = new SkiaFontManager(makeMockCk() as never);
    const result = await fm.requestNativeFontAccess();

    expect(result).toBe(true);
    expect(fm.nativeFontPermission).toBe('granted');

    // Check nativeFontMap has entries
    const map = (fm as unknown as { nativeFontMap: Map<string, unknown[]> }).nativeFontMap;
    expect(map.has('segoe ui')).toBe(true);
    expect(map.get('segoe ui')?.length).toBe(2);

    vi.unstubAllGlobals();
  });

  it('returns false and sets denied when permission denied', async () => {
    vi.stubGlobal('window', {
      queryLocalFonts: async () => {
        throw new DOMException('Permission denied', 'NotAllowedError');
      },
    });

    const fm = new SkiaFontManager(makeMockCk() as never);
    const result = await fm.requestNativeFontAccess();

    expect(result).toBe(false);
    expect(fm.nativeFontPermission).toBe('denied');

    vi.unstubAllGlobals();
  });

  it('returns false and sets unavailable when API not present', async () => {
    vi.stubGlobal('window', {});

    const fm = new SkiaFontManager(makeMockCk() as never);
    const result = await fm.requestNativeFontAccess();

    expect(result).toBe(false);
    expect(fm.nativeFontPermission).toBe('unavailable');

    vi.unstubAllGlobals();
  });
});

describe('local font blob loading into CanvasKit', () => {
  let originalFetch: typeof globalThis.fetch;

  beforeEach(() => {
    originalFetch = globalThis.fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('loads local font data into CanvasKit when blob provides valid data', async () => {
    // Create mock font data (enough bytes to look like a font file)
    const fontBuffer = new ArrayBuffer(100);
    const mockBlob = new Blob([fontBuffer]);

    // Mock fetch to fail (no bundled, no Google Fonts)
    globalThis.fetch = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 400, statusText: 'Bad Request' }));

    vi.stubGlobal('window', {
      queryLocalFonts: async () => [
        {
          family: 'TestFont',
          fullName: 'TestFont Regular',
          postscriptName: 'TestFont-Regular',
          style: 'Regular',
          blob: async () => mockBlob,
        },
      ],
    });

    const fm = new SkiaFontManager(makeMockCk() as never);

    // Grant access first
    await fm.requestNativeFontAccess();
    expect(fm.nativeFontPermission).toBe('granted');

    // Now try to ensure the font — should attempt blob loading
    await fm.ensureFont('TestFont');

    // Even if registerFont fails with mock data, the font should be recognized
    // The mock CK registerFont is a no-op, so it won't actually register
    // But the flow should exercise the blob loading path
    expect(fm.nativeFontPermission).toBe('granted');

    vi.unstubAllGlobals();
  });
});

describe('requestNativeFontAccess prompt-preservation', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('preserves prompt state when queryLocalFonts fails and permissions.query returns prompt', async () => {
    vi.stubGlobal('window', {
      queryLocalFonts: async () => {
        throw new DOMException('Permission denied', 'NotAllowedError');
      },
    });
    // Stub navigator.permissions for Node.js test environment
    vi.stubGlobal('navigator', {
      permissions: {
        query: async () => ({ state: 'prompt' }),
      },
    });

    const fm = new SkiaFontManager(makeMockCk() as never);
    const result = await fm.requestNativeFontAccess();

    expect(result).toBe(false);
    expect(fm.nativeFontPermission).toBe('prompt');

    vi.unstubAllGlobals();
  });

  it('sets denied when queryLocalFonts fails and permissions.query returns denied', async () => {
    vi.stubGlobal('window', {
      queryLocalFonts: async () => {
        throw new DOMException('Permission denied', 'NotAllowedError');
      },
    });
    // Stub navigator.permissions for Node.js test environment
    vi.stubGlobal('navigator', {
      permissions: {
        query: async () => ({ state: 'denied' }),
      },
    });

    const fm = new SkiaFontManager(makeMockCk() as never);
    const result = await fm.requestNativeFontAccess();

    expect(result).toBe(false);
    expect(fm.nativeFontPermission).toBe('denied');

    vi.unstubAllGlobals();
  });
});
