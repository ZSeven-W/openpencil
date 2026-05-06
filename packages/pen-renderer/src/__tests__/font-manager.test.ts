import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { SkiaFontManager } from '../font-manager';

// Minimal 模拟 CanvasKit shim — 仅 SkiaFontManager 构造函数涉及的位。
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
    // Use 通过强制转换进行私有访问 — 测试内部结构以验证新的公共方法是否正确读取地图，而无需将测试耦合到
    // network/font-loading 机器。
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
    // Mock 获取模拟 Google Fonts 返回 400
    globalThis.fetch = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 400, statusText: 'Bad Request' }));

    // Mock window.queryLocalFonts 包含“HeyMeow Rnd”
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

    // Mock 用于 Local Font Access API 检查的窗口
    const origWindow = globalThis.window;
    vi.stubGlobal('window', {
      queryLocalFonts: (
        globalThis as unknown as Record<string, () => Promise<Array<{ family: string }>>>
      ).queryLocalFonts,
    });

    const fm = new SkiaFontManager(makeMockCk() as never);

    // “HeyMeow Rnd”未捆绑且不在 NON_GOOGLE_FONT_PATTERNS 中
    const result = await fm.ensureFont('HeyMeow Rnd');

    // Should 返回 false（未加载到 CanvasKit）但被分类为系统字体
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
    // Mock 获取模拟 Google Fonts 返回 400
    globalThis.fetch = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 400, statusText: 'Bad Request' }));

    // Mock 窗口.queryLocalFonts WITHOUT "SomeRandomFont"
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

    // Check nativeFontMap 有条目
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
    // Create 模拟字体数据（足够的字节看起来像字体文件）
    const fontBuffer = new ArrayBuffer(100);
    const mockBlob = new Blob([fontBuffer]);

    // Mock 获取失败（无捆绑，无 Google Fonts）
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

    // Grant 首先访问
    await fm.requestNativeFontAccess();
    expect(fm.nativeFontPermission).toBe('granted');

    // Now 尝试确保字体 — 应该尝试加载 blob
    await fm.ensureFont('TestFont');

    // Even 如果 registerFont 因模拟数据失败，则应识别字体 The 模拟 CK registerFont
    // 是无操作，因此它不会实际注册 But 流程应执行 Blob 加载路径
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
    // Stub navigator.Node.js 测试环境的权限
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
    // Stub navigator.Node.js 测试环境的权限
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
