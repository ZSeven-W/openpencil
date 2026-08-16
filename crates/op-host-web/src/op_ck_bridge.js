// Thin CanvasKit FFI bridge for the Rust web shell.
//
// FFI glue only — flat, scalar-argument drawing functions the Rust
// `CanvasKitBackend` calls through wasm-bindgen. All drawing / layout / widget
// logic lives in Rust; this file just maps those calls onto CanvasKit
// `SkCanvas` ops. Depends on the compiled CanvasKit artifact
// (`/canvaskit/canvaskit.{js,wasm}`), not on any TS source.

function loadScript(src) {
  return new Promise((res, rej) => {
    if (window.CanvasKitInit) return res();
    const s = document.createElement('script');
    s.src = src;
    s.onload = res;
    s.onerror = () => rej(new Error('failed to load ' + src));
    document.head.appendChild(s);
  });
}

function copyBytes(u8) {
  return u8.buffer.slice(u8.byteOffset, u8.byteOffset + u8.byteLength);
}

let createWebImageCaches = null;

const TEXT_SCALE_STEPS_PER_OCTAVE = 4;

// Round UP to quarter-octave buckets (about 19% apart). This keeps cached
// browser-text bitmaps at or above the requested resolution while allowing
// adjacent continuous-zoom frames to reuse the same raster.
export function opCkQuantizeTextRasterScale(scale) {
  const finiteScale = Number.isFinite(scale) && scale > 1 ? scale : 1;
  const bucket = Math.ceil(
    Math.log2(finiteScale) * TEXT_SCALE_STEPS_PER_OCTAVE - 1e-9,
  );
  let quantized = 2 ** (bucket / TEXT_SCALE_STEPS_PER_OCTAVE);
  if (quantized < finiteScale) {
    quantized *= 2 ** (1 / TEXT_SCALE_STEPS_PER_OCTAVE);
  }
  return quantized;
}

export function setImageCacheFactory(factory) {
  createWebImageCaches = factory;
}

const CSS_GENERIC_FONT_FAMILIES = new Set([
  'serif',
  'sans-serif',
  'monospace',
  'cursive',
  'fantasy',
  'system-ui',
  'ui-serif',
  'ui-sans-serif',
  'ui-monospace',
  'ui-rounded',
  'math',
  'emoji',
  'fangsong',
  '-apple-system',
  'blinkmacsystemfont',
]);

// Family matching is ASCII-case-insensitive. Deliberately avoid locale-aware
// case conversion: font registry keys must be stable in every browser locale.
export function opCkNormalizeFontFamilyName(family) {
  return String(family || '').trim().replace(/[A-Z]/g, (ch) => String.fromCharCode(ch.charCodeAt(0) + 32));
}

// Parse a CSS font-family list without splitting commas contained in quoted
// family names. A backslash quotes the following character, which is enough
// for the escaped quotes and commas emitted by our HTML/Figma importers.
export function opCkParseFontFamilyStack(family) {
  const candidates = [];
  let current = '';
  let quote = '';
  let escaped = false;
  const push = () => {
    const candidate = current.trim();
    if (candidate) candidates.push(candidate);
    current = '';
  };
  for (const ch of String(family || '')) {
    if (escaped) {
      current += ch;
      escaped = false;
    } else if (ch === '\\') {
      escaped = true;
    } else if (quote) {
      if (ch === quote) quote = '';
      else current += ch;
    } else if (ch === '"' || ch === "'") {
      quote = ch;
    } else if (ch === ',') {
      push();
    } else {
      current += ch;
    }
  }
  if (escaped) current += '\\';
  push();
  return candidates;
}

// Resolve registered families in authored CSS order. For one candidate an
// explicit browser import wins over the system face with the same name. A
// generic family is already available through the browser canvas path, so it
// terminates named-face lookup instead of skipping ahead in the stack.
export function opCkResolveRegisteredTypeface(family, importedTypefaces, systemTypefacesByFamily) {
  for (const candidate of opCkParseFontFamilyStack(family)) {
    const familyKey = opCkNormalizeFontFamilyName(candidate);
    if (!familyKey) continue;
    if (CSS_GENERIC_FONT_FAMILIES.has(familyKey)) return null;
    const imported = importedTypefaces.get(familyKey);
    if (imported) {
      return { key: `imported:${familyKey}`, familyKey, source: 'imported', tf: imported.tf };
    }
    const system = systemTypefacesByFamily.get(familyKey);
    if (system) {
      return { key: `system:${familyKey}`, familyKey, source: 'system', tf: system.tf };
    }
  }
  return null;
}

const opCkClampUnit = (value, fallback = 0) => {
  const number = Number(value);
  return Number.isFinite(number) ? Math.max(0, Math.min(1, number)) : fallback;
};

// Build the row-major vertex lattice consumed by CanvasKit.MakeVertices.
// Kept pure + exported so the geometry/index contract can run under Node
// without booting a WebGL surface. CanvasKit indices are u16, so grids larger
// than 65,535 vertices take the same visible first-colour fallback as malformed
// grids instead of wrapping indices and drawing corrupt triangles.
export function opCkBuildMeshGradientData(x, y, w, h, rows, cols, colors) {
  rows = Math.trunc(Number(rows));
  cols = Math.trunc(Number(cols));
  const vertexCount = rows * cols;
  if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(w) || !Number.isFinite(h)
      || rows < 2 || cols < 2 || !Number.isSafeInteger(vertexCount)
      || vertexCount > 0xffff || colors.length !== vertexCount * 4) {
    return null;
  }

  const positions = new Float32Array(vertexCount * 2);
  const rgba = new Float32Array(vertexCount * 4);
  for (let row = 0; row < rows; row++) {
    for (let colIndex = 0; colIndex < cols; colIndex++) {
      const vertex = row * cols + colIndex;
      positions[vertex * 2] = x + (colIndex / (cols - 1)) * w;
      positions[vertex * 2 + 1] = y + (row / (rows - 1)) * h;
      const colorOffset = vertex * 4;
      rgba[colorOffset] = opCkClampUnit(colors[colorOffset]);
      rgba[colorOffset + 1] = opCkClampUnit(colors[colorOffset + 1]);
      rgba[colorOffset + 2] = opCkClampUnit(colors[colorOffset + 2]);
      rgba[colorOffset + 3] = opCkClampUnit(colors[colorOffset + 3]);
    }
  }

  const indices = new Uint16Array((rows - 1) * (cols - 1) * 6);
  let index = 0;
  for (let row = 0; row < rows - 1; row++) {
    for (let colIndex = 0; colIndex < cols - 1; colIndex++) {
      const topLeft = row * cols + colIndex;
      const topRight = topLeft + 1;
      const bottomLeft = (row + 1) * cols + colIndex;
      const bottomRight = bottomLeft + 1;
      indices.set([topLeft, topRight, bottomLeft, topRight, bottomRight, bottomLeft], index);
      index += 6;
    }
  }
  return { positions, rgba, indices };
}

// Initialise CanvasKit on `canvasId`. Returns a bridge object the Rust backend
// drives. Text is rasterized with browser/system fonts by default; the Rust
// side can additionally register Local Font Access faces through
// registerSystemFont().
export async function opCkInit(canvasId) {
  await loadScript('/canvaskit/canvaskit.js');
  const CK = await CanvasKitInit({ locateFile: (f) => '/canvaskit/' + f });
  if (typeof createWebImageCaches !== 'function') {
    throw new Error('CanvasKit image cache factory was not configured');
  }
  let surface = CK.MakeWebGLCanvasSurface(canvasId);
  if (!surface) throw new Error('CanvasKit: MakeWebGLCanvasSurface returned null');
  let canvas = surface.getCanvas();
  const el = document.getElementById(canvasId);

  const systemTypefaces = [];
  const systemTypefaceKeys = new Set();
  const systemTypefacesByFamily = new Map();
  // User-imported font faces, keyed by normalized family name -> { tf, family }.
  // A named family is a deliberate single-typeface choice (mirrors how native
  // resolves a named family via FontMgr before any script fallback), so
  // imported text shapes the WHOLE run with one typeface instead of
  // re-segmenting by script.
  const importedTypefaces = new Map();
  const coverageCache = new Map();
  // Per-CHARACTER coverage segments for explicitly selected family text,
  // keyed on `(registry + family key, size, text)`. This used to
  // rebuild a throwaway CK.Font + call getGlyphIDs over the whole string on
  // EVERY drawText/measureTextFamilyStyled call, every frame — bounded FIFO
  // cache like `browserTextCache` below (values are plain JS arrays, no
  // wasm handles, so eviction is a plain `.delete()` on the Map).
  const REGISTERED_COVERAGE_CACHE_CAP = 512;
  const registeredCoverageCache = new Map();
  // Per-(registry + family key, size) CK.Font instances shared by the
  // family-aware draw + measure paths (`registeredFamilyFont`). Bounded,
  // full-wipe eviction mirroring `svgPathCache` below (these ARE wasm
  // handles and must be `.delete()`d before the Map entry is dropped).
  const REGISTERED_FONT_CACHE_CAP = 512;
  const registeredFamilyFontCache = new Map();
  // These caches key off the selected registry identity. Any imported or
  // system registry change invalidates them so later exact-name matches use
  // the newly registered face.
  const clearRegisteredFontCaches = () => {
    registeredCoverageCache.clear();
    for (const f of registeredFamilyFontCache.values()) f.delete();
    registeredFamilyFontCache.clear();
  };
  const browserTextCanvas = document.createElement('canvas');
  const browserTextCtx = browserTextCanvas.getContext('2d', { willReadFrequently: true });
  const browserTextCache = new Map();
  const clearBrowserTextCache = () => {
    for (const entry of browserTextCache.values()) {
      if (entry && entry.image && entry.image.delete) entry.image.delete();
    }
    browserTextCache.clear();
  };
  // Retain DPR so display changes can immediately invalidate stale handles;
  // per-draw supersampling comes from the current CanvasKit transform below.
  let textDpr = 1;
  const browserTextFontStack = [
    '-apple-system',
    'BlinkMacSystemFont',
    '"Segoe UI"',
    '"Helvetica Neue"',
    'Arial',
    '"Apple Color Emoji"',
    '"Segoe UI Emoji"',
    '"Noto Color Emoji"',
    '"PingFang SC"',
    '"PingFang TC"',
    '"Hiragino Sans"',
    '"Hiragino Kaku Gothic ProN"',
    '"Yu Gothic"',
    '"Yu Mincho"',
    '"Meiryo"',
    '"Apple SD Gothic Neo"',
    '"Malgun Gothic"',
    '"Noto Sans CJK SC"',
    '"Noto Sans CJK TC"',
    '"Noto Sans CJK JP"',
    '"Noto Sans CJK KR"',
    '"Kohinoor Devanagari"',
    '"Devanagari Sangam MN"',
    '"ITF Devanagari"',
    '"ITFDevanagari"',
    '"Mukta Mahee"',
    '"MuktaMahee"',
    '"Noto Sans Devanagari"',
    '"Nirmala UI"',
    '"Mangal"',
    '"SF Hebrew"',
    '"SFHebrew"',
    '"Arial Hebrew"',
    '"Geeza Pro"',
    '"Al Nile"',
    '"SF Georgian"',
    '"SFGeorgian"',
    '"Thonburi"',
    '"Sukhumvit Set"',
    '"SukhumvitSet"',
    '"Noto Sans Thai"',
    '"Noto Sans Thai UI"',
    '"Leelawadee UI"',
    '"Arial Unicode MS"',
    '"Noto Sans"',
    'sans-serif',
  ].join(', ');

  // CJK range check (Han / Hiragana / Katakana / Hangul / fullwidth).
  const hasCjk = (t) => { for (const ch of t) { const c = ch.codePointAt(0); if ((c >= 0x2e80 && c <= 0x9fff) || (c >= 0xac00 && c <= 0xd7a3) || (c >= 0xff00 && c <= 0xffef) || (c >= 0x3000 && c <= 0x30ff)) return true; } return false; };
  const hasSystemFallbackText = (t) => {
    for (const ch of t) {
      const c = ch.codePointAt(0);
      if (c > 0x7f) return true;
    }
    return false;
  };
  const normalizedFamily = opCkNormalizeFontFamilyName;
  const familyIncludes = (family, parts) => {
    const f = normalizedFamily(family);
    return parts.some((part) => f.includes(part.toLowerCase()));
  };
  const isEmojiFamily = (family) => {
    const f = normalizedFamily(family);
    return f.includes('emoji') || f.includes('color symbol');
  };
  const isCjkFamily = (family) => {
    return familyIncludes(family, ['PingFang', 'Hiragino Sans', 'Hiragino Kaku Gothic', 'Heiti', 'STHeiti', 'Songti', 'Noto Sans CJK', 'Noto Sans SC', 'Noto Sans TC', 'Noto Sans JP', 'Noto Sans KR', 'Source Han Sans', 'Microsoft YaHei', 'Microsoft JhengHei', 'SimHei', 'SimSun', 'Yu Gothic', 'Yu Mincho', 'Meiryo', 'Malgun Gothic', 'AppleGothic', 'Nanum Gothic', 'Apple SD Gothic Neo', 'Arial Unicode MS']);
  };
  const isTextFallbackFamily = (family) => {
    return isEmojiFamily(family) || isCjkFamily(family) || familyIncludes(family, ['Arial', 'Arial Unicode MS', 'Helvetica Neue', 'SF Pro', '.SF NS', 'Segoe UI', 'Segoe UI Historic', 'Segoe UI Symbol', 'Apple Symbols', 'Noto Sans', 'Kohinoor Devanagari', 'Devanagari Sangam MN', 'ITFDevanagari', 'ITF Devanagari', 'MuktaMahee', 'Mukta Mahee', 'Noto Sans Devanagari', 'Nirmala UI', 'Mangal', 'SFGeorgian', 'SF Georgian', 'SFHebrew', 'SF Hebrew', 'Thonburi', 'Sukhumvit', 'Noto Sans Thai', 'Leelawadee UI']);
  };
  // Emoji codepoint (pictographs / symbols / dingbats / regional) + attaching
  // modifiers (variation selector, ZWJ, keycap, skin tone) that extend a run.
  const isEmojiCp = (c) => (c >= 0x1f000 && c <= 0x1faff) || (c >= 0x2600 && c <= 0x27bf) || (c >= 0x2b00 && c <= 0x2bff) || (c >= 0x1f1e6 && c <= 0x1f1ff);
  const isEmojiMod = (c) => (c >= 0xfe00 && c <= 0xfe0f) || c === 0x200d || c === 0x20e3 || (c >= 0x1f3fb && c <= 0x1f3ff);
  // Split text into consecutive {text, emoji} runs so each draws with the right
  // typeface (CanvasKit drawText is single-typeface, no per-glyph fallback).
  const segments = (t) => {
    const out = []; let cur = '', curEmoji = null;
    for (const ch of t) {
      const c = ch.codePointAt(0);
      const e = isEmojiMod(c) ? (curEmoji === null ? false : curEmoji) : isEmojiCp(c);
      if (curEmoji === null) { curEmoji = e; cur = ch; }
      else if (e === curEmoji) { cur += ch; }
      else { out.push({ text: cur, emoji: curEmoji }); cur = ch; curEmoji = e; }
    }
    if (cur) out.push({ text: cur, emoji: curEmoji });
    return out;
  };

  const col = (r, g, b, a) => CK.Color4f(r, g, b, a);
  const isPaintStyle = (style) => Boolean(style && typeof style.value !== 'undefined');
  const setPaintStyle = (paint, style) => { if (isPaintStyle(style)) paint.setStyle(style); };
  // Long-lived fill/stroke paints reused across every plain-color primitive
  // draw (fillRect, fillRRect, ovals, lines, polygons, SVG path fills) —
  // a full-editor repaint issued hundreds of `new CK.Paint()` + `.delete()`
  // pairs per frame otherwise. Every `fillPaint`/`strokePaint` call resets
  // ALL mutable properties (color, antialias, style, stroke width/cap/join)
  // it ever sets, so no state leaks between unrelated call sites even though
  // a couple of callers (drawText) further mutate the returned paint (e.g.
  // StrokeAndFill for bold synth) after fetching it — the next fetch wipes
  // that back to a clean Fill/Stroke paint. Callers must NOT `.delete()`
  // these two objects. One-off-effect paints (shaders, blend modes, mask
  // filters/blur) are excluded from this cache by design — see
  // `shaderPaint` and `fillInnerShadowSvgPath`'s `cut` paint.
  const cachedFillPaint = new CK.Paint();
  const cachedStrokePaint = new CK.Paint();
  const fillPaint = (r, g, b, a) => { const p = cachedFillPaint; p.setColor(col(r, g, b, a)); p.setAntiAlias(true); setPaintStyle(p, CK.PaintStyle.Fill); return p; };
  const strokePaint = (r, g, b, a, w) => { const p = cachedStrokePaint; p.setColor(col(r, g, b, a)); p.setAntiAlias(true); setPaintStyle(p, CK.PaintStyle.Stroke); p.setStrokeWidth(w); p.setStrokeCap(CK.StrokeCap.Round); p.setStrokeJoin(CK.StrokeJoin.Round); return p; };
  // Dedicated (allocated + `.delete()`d) fill paint for the one call site
  // that needs a SECOND live fill paint while `drawText`'s cached `p` (see
  // above) is already in scope: `drawScriptRun` is invoked both standalone
  // AND nested inside `drawText`'s per-segment loop, so it cannot safely
  // share the single cached fill paint without the outer and inner draws
  // fighting over the same object mid-loop. Kept as a plain allocation
  // rather than a second cache since text draws are far less frequent than
  // the primitive-shape hot path this task targets.
  const allocFillPaint = (r, g, b, a) => { const p = new CK.Paint(); p.setColor(col(r, g, b, a)); p.setAntiAlias(true); setPaintStyle(p, CK.PaintStyle.Fill); return p; };
  // Quote the authored family so names with spaces stay one CSS token, and
  // strip quote/backslash so a family name can't break out of the shorthand.
  // Only families the BROWSER knows resolve here; faces registered into
  // CanvasKit as raw bytes are invisible to CSS and fall through the stack.
  const cssFamilyToken = (family) => {
    const name = String(family || '').trim().replace(/["\\]/g, '');
    return name ? `"${name}", ` : '';
  };
  const browserTextFont = (sz, weight, italic, family) => `${italic ? 'italic ' : ''}${Math.max(100, Math.min(900, Math.round(weight || 400)))} ${Math.max(1, sz)}px ${cssFamilyToken(family)}${browserTextFontStack}`;
  const shouldUseBrowserTextFallback = (_t, _emojiRun) => Boolean(browserTextCtx);
  // Complex scripts (Arabic and friends — see `op_editor_core::text_script`,
  // which owns the predicate for BOTH hosts). CanvasKit's `canvas.drawText`
  // is a 1:1 cmap lookup: no bidi reordering, no contextual joining, so it
  // paints Arabic in storage order with isolated letterforms. The browser's
  // own 2D text engine does both correctly and `drawBrowserText` already
  // wraps it, so complex runs are sent there instead.
  //
  // They must go WHOLE. `drawScriptRun` splits a run into script segments and
  // advances left to right, so it would resolve bidi within each segment and
  // then lay the segments out in storage order — a mixed digit/Arabic run
  // like "50 ك.م" would still come out wrong.
  const shapedTextUnavailable = () => !browserTextCtx;
  const allSegmentsUseBrowserTextFallback = (segs) => segs.length > 0 && segs.every((seg) => shouldUseBrowserTextFallback(seg.text, seg.emoji));
  const browserTextMeasure = (t, sz, weight = 400, italic = false, family = '') => {
    if (!browserTextCtx) return 0;
    browserTextCtx.font = browserTextFont(sz, weight, italic, family);
    return browserTextCtx.measureText(t).width;
  };
  const effectiveTextScale = () => {
    const m = canvas.getTotalMatrix();
    const xScale = Math.hypot(m[0], m[3]);
    const yScale = Math.hypot(m[1], m[4]);
    return Math.max(1, xScale, yScale);
  };
  // Continuous zoom used to put the exact floating-point transform in every
  // browser-text cache key. Quantization lets adjacent frames reuse bitmaps.
  const browserTextRasterScale = () =>
    opCkQuantizeTextRasterScale(effectiveTextScale());
  // Text uses a white coverage mask keyed without colour/alpha, then receives
  // a SrcIn tint at draw time so differently coloured runs share a bitmap.
  // Emoji retain their legacy RGBA-keyed, untinted raster path because colour
  // glyphs can ignore fillStyle. Cache-hit reinsertion keeps eviction LRU.
  const browserTextImage = (t, sz, weight, italic, emoji, r, g, b, a, family = '') => {
    if (!browserTextCtx) return null;
    const ss = browserTextRasterScale();
    // `family` participates in the key: the same string shaped in two
    // different families is two different bitmaps.
    const key = emoji
      ? ['e', t, sz, weight, italic ? 1 : 0, r, g, b, a, ss, family].join('\n')
      : [t, sz, weight, italic ? 1 : 0, ss, family].join('\n');
    const hit = browserTextCache.get(key);
    if (hit) {
      browserTextCache.delete(key);
      browserTextCache.set(key, hit);
      return hit;
    }
    const font = browserTextFont(sz, weight, italic, family);
    browserTextCtx.font = font;
    const metrics = browserTextCtx.measureText(t);
    // Logical (CSS-px) box the glyphs occupy; positioning stays in CSS units.
    const width = Math.max(1, Math.ceil(metrics.width + 4));
    const ascent = Math.ceil(metrics.actualBoundingBoxAscent || sz * 0.8);
    const descent = Math.ceil(metrics.actualBoundingBoxDescent || sz * 0.25);
    const baseline = ascent + 2;
    const height = Math.max(1, baseline + descent + 2);
    // Back the offscreen canvas with `ss`x pixels and scale drawing so the
    // rasterized glyphs carry device-resolution detail. Setting canvas.width/
    // height resets the 2D context, so (re)apply font/baseline/fill + the
    // supersample transform here.
    browserTextCanvas.width = Math.max(1, Math.ceil(width * ss));
    browserTextCanvas.height = Math.max(1, Math.ceil(height * ss));
    browserTextCtx.setTransform(ss, 0, 0, ss, 0, 0);
    browserTextCtx.clearRect(0, 0, width, height);
    browserTextCtx.font = font;
    browserTextCtx.textBaseline = 'alphabetic';
    // Emoji bake their colour (legacy path); text rasters a WHITE mask that is
    // tinted at draw time.
    browserTextCtx.fillStyle = emoji
      ? `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`
      : 'rgba(255, 255, 255, 1)';
    browserTextCtx.fillText(t, 2, baseline);
    browserTextCtx.setTransform(1, 0, 0, 1, 0, 0);
    const image = CK.MakeImageFromCanvasImageSource(browserTextCanvas);
    if (!image) return null;
    const entry = { image, width: metrics.width, baseline, ss, emoji: Boolean(emoji) };
    browserTextCache.set(key, entry);
    if (browserTextCache.size > 512) {
      const firstKey = browserTextCache.keys().next().value;
      const old = browserTextCache.get(firstKey);
      if (old && old.image && old.image.delete) old.image.delete();
      browserTextCache.delete(firstKey);
    }
    return entry;
  };
  // Bounded rgb→ColorFilter cache for tinting the WHITE text raster at draw.
  // `MakeBlend(color, SrcIn)` recolours the mask to `color × coverage`; alpha
  // is NOT part of the filter (it rides the paint via setAlphaf), so the key is
  // rgb only and one filter serves every opacity of a colour. Filters are wasm
  // handles — LRU with a hard cap, `.delete()`ing the evicted handle (mirrors
  // `svgPathCache`'s wasm-object hygiene). Cap is small: distinct text colours
  // on screen are few, so this never thrashes the way the old RGBA raster key
  // did. An evicted filter was already consumed by its draw call (immediate
  // mode), and `cachedTintPaint` re-`setColorFilter`s before every draw, so a
  // deleted handle is never dereferenced.
  const TINT_FILTER_CACHE_CAP = 64;
  const tintFilterCache = new Map();
  const tintColorFilter = (r, g, b) => {
    if (!(CK.ColorFilter && CK.ColorFilter.MakeBlend)) return null;
    const key = r + '\n' + g + '\n' + b;
    const hit = tintFilterCache.get(key);
    if (hit) {
      tintFilterCache.delete(key);
      tintFilterCache.set(key, hit);
      return hit;
    }
    const cf = CK.ColorFilter.MakeBlend(col(r, g, b, 1), CK.BlendMode.SrcIn);
    if (!cf) return null;
    tintFilterCache.set(key, cf);
    if (tintFilterCache.size > TINT_FILTER_CACHE_CAP) {
      const firstKey = tintFilterCache.keys().next().value;
      const old = tintFilterCache.get(firstKey);
      if (old && old.delete) old.delete();
      tintFilterCache.delete(firstKey);
    }
    return cf;
  };
  // Dedicated long-lived paint that tints the white text raster. Kept SEPARATE
  // from cachedFillPaint / cachedStrokePaint because drawBrowserText runs while
  // drawText's / drawScriptRun's fill paint `p` is already live in an outer
  // loop — sharing would corrupt that paint mid-run (the same reason
  // allocFillPaint exists). antiAlias stays at the CK.Paint default (false) so
  // image sampling matches the prior no-paint drawImage exactly.
  const cachedTintPaint = new CK.Paint();
  const drawBrowserText = (t, x, y, sz, weight, italic, r, g, b, a, emoji, family = '') => {
    const entry = browserTextImage(t, sz, weight, italic, emoji, r, g, b, a, family);
    if (!entry) return 0;
    // Emoji rasters bake their own colour and draw UNTINTED (legacy path);
    // text rasters are white masks tinted here. For text: tint to (r,g,b) via a
    // cached SrcIn ColorFilter with opacity `a` riding the paint — SrcIn keeps
    // alpha = coverage × a and rgb = colour, pixel-identical to the old
    // baked-colour raster.
    let paint = null;
    if (!entry.emoji) {
      paint = cachedTintPaint;
      paint.setColorFilter(tintColorFilter(r, g, b) || null);
      paint.setAlphaf(a < 0 ? 0 : a > 1 ? 1 : a);
    }
    const ss = entry.ss || 1;
    if (ss !== 1) {
      // The bitmap is `ss`x oversampled; place it in logical space at
      // (x-2, y-baseline) then scale down by `ss` so its device footprint
      // matches the intended CSS box at native resolution.
      canvas.save();
      canvas.translate(x - 2, y - entry.baseline);
      canvas.scale(1 / ss, 1 / ss);
      if (paint) canvas.drawImage(entry.image, 0, 0, paint);
      else canvas.drawImage(entry.image, 0, 0);
      canvas.restore();
    } else if (paint) {
      canvas.drawImage(entry.image, x - 2, y - entry.baseline, paint);
    } else {
      canvas.drawImage(entry.image, x - 2, y - entry.baseline);
    }
    return entry.width;
  };
  const shaderPaint = (shader) => { const p = new CK.Paint(); p.setAntiAlias(true); setPaintStyle(p, CK.PaintStyle.Fill); p.setShader(shader); return p; };
  const firstStopColor = (stops, opacity) => stops.length >= 5
    ? col(
      opCkClampUnit(stops[1]),
      opCkClampUnit(stops[2]),
      opCkClampUnit(stops[3]),
      opCkClampUnit(stops[4]) * opCkClampUnit(opacity),
    )
    : col(0, 0, 0, 0);
  const gradientStops = (stops, opacity) => {
    const colors = [];
    const offsets = [];
    const alpha = opCkClampUnit(opacity);
    for (let i = 0; i + 4 < stops.length; i += 5) {
      offsets.push(opCkClampUnit(stops[i]));
      colors.push(col(
        opCkClampUnit(stops[i + 1]),
        opCkClampUnit(stops[i + 2]),
        opCkClampUnit(stops[i + 3]),
        opCkClampUnit(stops[i + 4]) * alpha,
      ));
    }
    return { colors, offsets };
  };
  const linearGradientPoints = (x, y, w, h, angleDeg) => {
    const rad = (angleDeg - 90) * Math.PI / 180;
    const cx = x + w / 2;
    const cy = y + h / 2;
    const dx = Math.cos(rad) * w / 2;
    const dy = Math.sin(rad) * h / 2;
    return { start: [cx - dx, cy - dy], end: [cx + dx, cy + dy] };
  };
  const uniformRRect = (x, y, w, h, radius) => CK.RRectXY(
    CK.LTRBRect(x, y, x + w, y + h),
    Math.max(0, radius),
    Math.max(0, radius),
  );
  const perCornerRRect = (x, y, w, h, topLeft, topRight, bottomRight, bottomLeft) => Float32Array.of(
    x, y, x + w, y + h,
    Math.max(0, topLeft), Math.max(0, topLeft),
    Math.max(0, topRight), Math.max(0, topRight),
    Math.max(0, bottomRight), Math.max(0, bottomRight),
    Math.max(0, bottomLeft), Math.max(0, bottomLeft),
  );
  const drawGradientRRect = (rrect, shader, fallbackColor) => {
    if (!shader) {
      canvas.drawRRect(rrect, fillPaint(...fallbackColor));
      return;
    }
    const paint = shaderPaint(shader);
    try {
      canvas.drawRRect(rrect, paint);
    } finally {
      paint.delete();
      if (shader.delete) shader.delete();
    }
  };
  const makeLinearGradient = (x, y, w, h, stops, angleDeg, opacity) => {
    const gradient = gradientStops(stops, opacity);
    if (!gradient.colors.length) return null;
    const points = linearGradientPoints(x, y, w, h, angleDeg);
    try {
      // Flag 1 requests interpolation in premultiplied colour space, matching
      // the native Skia backend and avoiding halos around transparent stops.
      return CK.Shader.MakeLinearGradient(
        points.start,
        points.end,
        gradient.colors,
        gradient.offsets,
        CK.TileMode.Clamp,
        null,
        1,
      );
    } catch (_error) {
      return null;
    }
  };
  const makeRadialGradient = (x, y, w, h, stops, cxFrac, cyFrac, radiusFrac, opacity) => {
    const gradient = gradientStops(stops, opacity);
    if (!gradient.colors.length) return null;
    const center = [
      x + w * opCkClampUnit(cxFrac),
      y + h * opCkClampUnit(cyFrac),
    ];
    const radius = Math.max(0.01, Math.max(w, h) * opCkClampUnit(radiusFrac));
    try {
      return CK.Shader.MakeRadialGradient(
        center,
        radius,
        gradient.colors,
        gradient.offsets,
        CK.TileMode.Clamp,
        null,
        1,
      );
    } catch (_error) {
      return null;
    }
  };
  const drawMeshGradientRRect = (rrect, x, y, w, h, rows, cols, colors, opacity) => {
    const alpha = opCkClampUnit(opacity);
    const fallback = colors.length >= 4
      ? [
        opCkClampUnit(colors[0]),
        opCkClampUnit(colors[1]),
        opCkClampUnit(colors[2]),
        opCkClampUnit(colors[3]) * alpha,
      ]
      : [0, 0, 0, 0];
    const mesh = opCkBuildMeshGradientData(x, y, w, h, rows, cols, colors);
    if (!mesh || typeof CK.MakeVertices !== 'function') {
      canvas.drawRRect(rrect, fillPaint(...fallback));
      return;
    }

    let vertices = null;
    try {
      vertices = CK.MakeVertices(
        CK.VertexMode.Triangles,
        mesh.positions,
        null,
        mesh.rgba,
        mesh.indices,
        true,
      );
    } catch (_error) {
      vertices = null;
    }
    if (!vertices) {
      canvas.drawRRect(rrect, fillPaint(...fallback));
      return;
    }

    const paint = allocFillPaint(1, 1, 1, alpha);
    canvas.save();
    try {
      canvas.clipRRect(rrect, CK.ClipOp.Intersect, true);
      canvas.drawVertices(vertices, CK.BlendMode.Modulate, paint);
    } finally {
      canvas.restore();
      paint.delete();
      vertices.delete();
    }
  };
  const fontCovers = (entry, text) => {
    if (!entry || !entry.tf || !text) return false;
    const cacheKey = entry.key + '\n' + text;
    if (coverageCache.has(cacheKey)) return coverageCache.get(cacheKey);
    const f = new CK.Font(entry.tf, 16);
    const ids = f.getGlyphIDs(text);
    const ok = ids.length > 0 && ids.every((id) => id !== 0);
    f.delete();
    coverageCache.set(cacheKey, ok);
    return ok;
  };
  const systemTypefaceFor = (t, emojiRun) => {
    const preferred = emojiRun
      ? systemTypefaces.filter((entry) => entry.emoji)
      : hasCjk(t)
        ? systemTypefaces.filter((entry) => entry.cjk)
        : hasSystemFallbackText(t)
          ? systemTypefaces.filter((entry) => entry.textFallback)
          : [];
    for (const entry of preferred) {
      if (fontCovers(entry, t)) return entry.tf;
    }
    if (!emojiRun && hasSystemFallbackText(t)) {
      for (const entry of systemTypefaces) {
        if (fontCovers(entry, t)) return entry.tf;
      }
    }
    return null;
  };
  const tfFor = (t, emojiRun) => systemTypefaceFor(t, emojiRun) || CK.Typeface.GetDefault();
  const runWidth = (f, s) => { const ids = f.getGlyphIDs(s); return f.getGlyphWidths(ids).reduce((a, v) => a + v, 0); };
  // Normalize the primary family for imported-font registry mutations. Text
  // lookup itself uses every authored candidate via `familyTypefaceEntry`.
  const primaryFamilyKey = (family) => {
    const first = opCkParseFontFamilyStack(family)[0] || '';
    if (!first) return '';
    const key = normalizedFamily(first);
    return CSS_GENERIC_FONT_FAMILIES.has(key) ? '' : key;
  };
  // Resolve an explicit imported/system face in authored stack order. Generic
  // candidates intentionally return null and continue through browser text.
  const familyTypefaceEntry = (family) => {
    return opCkResolveRegisteredTypeface(family, importedTypefaces, systemTypefacesByFamily);
  };
  // Shared "typeface + font for (family, sz)" so the family-aware draw and
  // measure paths build the SAME CK.Font and agree to sub-pixel. Cached per
  // `(registry + family key, size)` — this used to build + discard a
  // fresh CK.Font on every selected segment of every draw/measure call.
  // Iterations that use this font always fully consume it (draw or measure)
  // before the next one is fetched, so a single shared instance per key is
  // safe to reuse across loop iterations and across calls. Skew (italic) is
  // reset unconditionally on every fetch since the same cached Font may be
  // reused for an italic run and then a later upright run. Callers must NOT
  // `.delete()` the returned Font; eviction (mirroring `svgPathCache`) frees
  // every cached wasm Font before wiping the map. Invalidated wholesale by
  // `clearRegisteredFontCaches` whenever either font registry changes.
  const registeredFamilyFont = (key, tf, sz, italic) => {
    const cacheKey = key + '\n' + sz;
    let f = registeredFamilyFontCache.get(cacheKey);
    if (!f) {
      f = new CK.Font(tf, sz);
      if (registeredFamilyFontCache.size >= REGISTERED_FONT_CACHE_CAP) {
        for (const v of registeredFamilyFontCache.values()) v.delete();
        registeredFamilyFontCache.clear();
      }
      registeredFamilyFontCache.set(cacheKey, f);
    }
    f.setSkewX(italic ? -0.25 : 0);
    return f;
  };
  // Split a run into maximal {text, registered} segments by whether the named
  // typeface has a glyph for each char (glyph id 0 = .notdef = uncovered). This
  // is per-CHARACTER, so a mixed run keeps the selected face for the chars it
  // covers and falls back (system/CJK/emoji) only for the rest — matching how
  // native resolves a named family per character, instead of dropping the
  // imported family for the whole run. `getGlyphIDs` returns one id per
  // codepoint, so it aligns with the codepoint iteration; if the counts don't
  // line up we conservatively treat the whole run as uncovered.
  const registeredCoverageSegments = (key, tf, sz, t) => {
    const cacheKey = key + '\n' + sz + '\n' + t;
    const cached = registeredCoverageCache.get(cacheKey);
    if (cached) return cached;
    const cps = Array.from(t);
    let segs;
    if (cps.length === 0) {
      segs = [];
    } else {
      const f = new CK.Font(tf, sz);
      let ids = null;
      try {
        ids = f.getGlyphIDs(t);
      } catch (e) {
        ids = null;
      }
      f.delete();
      if (!ids || ids.length !== cps.length) {
        segs = [{ text: t, registered: false }];
      } else {
        const out = [];
        let cur = '';
        let curRegistered = null;
        for (let i = 0; i < cps.length; i++) {
          const registered = ids[i] !== 0;
          if (curRegistered === null) {
            curRegistered = registered;
            cur = cps[i];
          } else if (registered === curRegistered) {
            cur += cps[i];
          } else {
            out.push({ text: cur, registered: curRegistered });
            cur = cps[i];
            curRegistered = registered;
          }
        }
        if (cur) out.push({ text: cur, registered: curRegistered });
        segs = out;
      }
    }
    registeredCoverageCache.set(cacheKey, segs);
    if (registeredCoverageCache.size > REGISTERED_COVERAGE_CACHE_CAP) {
      const firstKey = registeredCoverageCache.keys().next().value;
      registeredCoverageCache.delete(firstKey);
    }
    return segs;
  };
  // Draw a run via the script-segmented fallback (system / CJK / emoji /
  // browser-canvas), returning the advance consumed. Shared by the
  // family-blind path and the uncovered segments of a family-aware run, so the
  // two stay identical. Mirrors `measureTextStyled` advance-for-advance.
  const drawScriptRun = (t, x, y, sz, weight, italic, r, g, b, a) => {
    const segs = segments(t);
    if (segs.length === 0) return 0;
    if (allSegmentsUseBrowserTextFallback(segs)) {
      let cx = x;
      for (const seg of segs) cx += drawBrowserText(seg.text, cx, y, sz, weight, italic, r, g, b, a, seg.emoji);
      return cx - x;
    }
    const p = allocFillPaint(r, g, b, a);
    if (weight >= 600 && isPaintStyle(CK.PaintStyle.StrokeAndFill)) {
      setPaintStyle(p, CK.PaintStyle.StrokeAndFill);
      p.setStrokeWidth(sz * 0.06);
    }
    let cx = x;
    for (const seg of segs) {
      if (shouldUseBrowserTextFallback(seg.text, seg.emoji)) {
        cx += drawBrowserText(seg.text, cx, y, sz, weight, italic, r, g, b, a, seg.emoji);
        continue;
      }
      const f = new CK.Font(tfFor(seg.text, seg.emoji), sz);
      if (italic && !seg.emoji) f.setSkewX(-0.25);
      canvas.drawText(seg.text, cx, y, p, f);
      cx += runWidth(f, seg.text);
      f.delete();
    }
    p.delete();
    return cx - x;
  };
  const pathIsFinite = (bounds) => bounds && bounds.length >= 4 && bounds.every((v) => Number.isFinite(v));
  const fitPathToRect = (path, x, y, w, h) => {
    if (!Number.isFinite(w) || !Number.isFinite(h) || w <= 0 || h <= 0) {
      path.transform(CK.Matrix.translated(x, y));
      return path;
    }
    // Match native Skia's `compute_tight_bounds`: control points may sit far
    // outside the actual Bezier curve, so loose bounds can visibly shrink or
    // offset a fitted path. Older CanvasKit builds lack the tight API; retain
    // `getBounds` as a compatibility fallback.
    let bounds = null;
    if (typeof path.computeTightBounds === 'function') {
      try { bounds = path.computeTightBounds(); } catch (_error) { bounds = null; }
    }
    if (!pathIsFinite(bounds)) {
      try { bounds = path.getBounds(); } catch (_error) { bounds = null; }
    }
    if (!pathIsFinite(bounds)) {
      path.transform(CK.Matrix.translated(x, y));
      return path;
    }
    const nativeW = bounds[2] - bounds[0];
    const nativeH = bounds[3] - bounds[1];
    const sx = Math.abs(nativeW) > 0.01 ? w / nativeW : 1;
    const sy = Math.abs(nativeH) > 0.01 ? h / nativeH : 1;
    const tx = x - bounds[0] * sx;
    const ty = y - bounds[1] * sy;
    path.transform(CK.Matrix.multiply(CK.Matrix.translated(tx, ty), CK.Matrix.scaled(sx, sy)));
    return path;
  };

  // Parsed-SVG-path cache. `CK.Path.MakeFromSVGString` is the dominant per-icon
  // cost, and every chrome icon / lucide glyph / brand logo / vector node
  // re-parsed its `d` string on EVERY frame (mirrors what the native backend
  // avoids via `svg_path_cache`). Cache the parsed, untransformed path keyed on
  // `d`; callers draw a `.copy()` they transform + delete, leaving the cached
  // original pristine. Bounded — on overflow drop all (a full refill is one
  // frame) and delete the wasm-heap paths so they don't leak.
  const SVG_PATH_CACHE_CAP = 1024;
  const svgPathCache = new Map();
  const cachedSvgPath = (d) => {
    let base = svgPathCache.get(d);
    if (!base) {
      base = CK.Path.MakeFromSVGString(d);
      if (!base) return null;
      if (svgPathCache.size >= SVG_PATH_CACHE_CAP) {
        for (const v of svgPathCache.values()) v.delete();
        svgPathCache.clear();
      }
      svgPathCache.set(d, base);
    }
    return base.copy();
  };

  const imageCaches = createWebImageCaches(CK);

  // Figma maps node-normalized coordinates to normalized image UV. Image
  // shaders consume the inverse, mapping image pixels into the destination
  // rect: node_rect * inverse(figma) * inverse(image_dimensions).
  const figmaImageLocalMatrix = (x, y, w, h, imageW, imageH, transform) => {
    if (transform.length !== 6 || !(w > 0) || !(h > 0) || !(imageW > 0) || !(imageH > 0)) return null;
    const [a, b, tx, c, d, ty] = transform;
    const det = a * d - b * c;
    if (!Number.isFinite(det) || Math.abs(det) <= Number.EPSILON) return null;
    const invDet = 1 / det;
    const ia = d * invDet;
    const ib = -b * invDet;
    const ic = -c * invDet;
    const id = a * invDet;
    const itx = (b * ty - d * tx) * invDet;
    const ity = (c * tx - a * ty) * invDet;
    return Float32Array.of(
      w * ia / imageW, w * ib / imageH, x + w * itx,
      h * ic / imageW, h * id / imageH, y + h * ity,
      0, 0, 1,
    );
  };

  const imageAdjustmentMatrix = (values) => {
    if (values.length !== 7 || values.every((v) => v === 0)) return null;
    const exp = values[0] / 100;
    const con = values[1] / 100;
    const sat = values[2] / 100;
    const temp = values[3] / 100;
    const tint = values[4] / 100;
    const hi = values[5] / 100;
    const sh = values[6] / 100;
    const e = 1 + exp * 1.5;
    const contrast = 1 + con;
    const contrastOffset = 0.5 * (1 - contrast);
    const saturation = 1 + sat;
    const [lr, lg, lb] = [0.2126, 0.7152, 0.0722];
    const [sr, sg, sb] = [(1 - saturation) * lr, (1 - saturation) * lg, (1 - saturation) * lb];
    const f = contrast * e;
    const common = (hi + sh * 0.5) * 0.1;
    return Float32Array.of(
      f * (sr + saturation), f * sg, f * sb, 0, contrastOffset + temp * 0.15 + common,
      f * sr, f * (sg + saturation), f * sb, 0, contrastOffset + tint * 0.15 + common,
      f * sr, f * sg, f * (sb + saturation), 0, contrastOffset - temp * 0.15 + common,
      0, 0, 0, 1, 0,
    );
  };

  const blendModeForCode = (blendMode) => [
    CK.BlendMode.SrcOver,
    CK.BlendMode.Darken,
    CK.BlendMode.Multiply,
    CK.BlendMode.Screen,
    CK.BlendMode.Overlay,
    CK.BlendMode.Lighten,
    CK.BlendMode.Difference,
    CK.BlendMode.Hue,
    CK.BlendMode.Saturation,
    CK.BlendMode.Color,
    CK.BlendMode.Luminosity,
    CK.BlendMode.SoftLight,
    CK.BlendMode.ColorDodge,
    CK.BlendMode.ColorBurn,
    CK.BlendMode.HardLight,
    CK.BlendMode.Exclusion,
  ][blendMode] || CK.BlendMode.SrcOver;

  const drawImageRectLinear = (image, src, dst, paint) => {
    if (canvas.drawImageRectOptions) {
      canvas.drawImageRectOptions(image, src, dst, CK.FilterMode.Linear, CK.MipmapMode.None, paint);
    } else {
      canvas.drawImageRect(image, src, dst, paint, false);
    }
  };

  return {
    beginFrame() {
      // Discard any leaked save / clip / matrix state from a prior (possibly
      // unbalanced) paint pass, then open a clean frame baseline. This mirrors
      // the native backend's per-frame `reset_matrix()`: without it a single
      // unbalanced save/clip in the widget composition would accumulate across
      // repaints and corrupt z-order / clipping (e.g. a frame fill bleeding
      // over later layers).
      while (canvas.getSaveCount() > 1) canvas.restore();
      canvas.save();
    },
    endFrame() {
      while (canvas.getSaveCount() > 1) canvas.restore();
      surface.flush();
    },
    clear(r, g, b, a) { canvas.clear(col(r, g, b, a)); },

    fillRect(x, y, w, h, r, g, b, a) { const p = fillPaint(r, g, b, a); canvas.drawRect(CK.LTRBRect(x, y, x + w, y + h), p); },
    strokeRect(x, y, w, h, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawRect(CK.LTRBRect(x, y, x + w, y + h), p); },
    fillRoundRect(x, y, w, h, rad, r, g, b, a) { const p = fillPaint(r, g, b, a); canvas.drawRRect(CK.RRectXY(CK.LTRBRect(x, y, x + w, y + h), rad, rad), p); },
    fillRoundRectPerCorner(x, y, w, h, tl, tr, br, bl, r, g, b, a) {
      const rr = Float32Array.of(x, y, x + w, y + h, tl, tl, tr, tr, br, br, bl, bl);
      const p = fillPaint(r, g, b, a); canvas.drawRRect(rr, p);
    },
    fillRoundRectLinearGradient(x, y, w, h, radius, stops, angleDeg, opacity) {
      if (stops.length < 5) return;
      drawGradientRRect(
        uniformRRect(x, y, w, h, radius),
        makeLinearGradient(x, y, w, h, stops, angleDeg, opacity),
        firstStopColor(stops, opacity),
      );
    },
    fillRoundRectLinearGradientPerCorner(x, y, w, h, tl, tr, br, bl, stops, angleDeg, opacity) {
      if (stops.length < 5) return;
      drawGradientRRect(
        perCornerRRect(x, y, w, h, tl, tr, br, bl),
        makeLinearGradient(x, y, w, h, stops, angleDeg, opacity),
        firstStopColor(stops, opacity),
      );
    },
    fillRoundRectRadialGradient(x, y, w, h, radius, stops, cxFrac, cyFrac, radiusFrac, opacity) {
      if (stops.length < 5) return;
      drawGradientRRect(
        uniformRRect(x, y, w, h, radius),
        makeRadialGradient(x, y, w, h, stops, cxFrac, cyFrac, radiusFrac, opacity),
        firstStopColor(stops, opacity),
      );
    },
    fillRoundRectRadialGradientPerCorner(x, y, w, h, tl, tr, br, bl, stops, cxFrac, cyFrac, radiusFrac, opacity) {
      if (stops.length < 5) return;
      drawGradientRRect(
        perCornerRRect(x, y, w, h, tl, tr, br, bl),
        makeRadialGradient(x, y, w, h, stops, cxFrac, cyFrac, radiusFrac, opacity),
        firstStopColor(stops, opacity),
      );
    },
    fillRoundRectMeshGradient(x, y, w, h, radius, rows, cols, colors, opacity) {
      if (colors.length < 4) return;
      drawMeshGradientRRect(
        uniformRRect(x, y, w, h, radius),
        x, y, w, h, rows, cols, colors, opacity,
      );
    },
    fillRoundRectMeshGradientPerCorner(x, y, w, h, tl, tr, br, bl, rows, cols, colors, opacity) {
      if (colors.length < 4) return;
      drawMeshGradientRRect(
        perCornerRRect(x, y, w, h, tl, tr, br, bl),
        x, y, w, h, rows, cols, colors, opacity,
      );
    },
    strokeRoundRect(x, y, w, h, rad, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawRRect(CK.RRectXY(CK.LTRBRect(x, y, x + w, y + h), rad, rad), p); },
    strokeRoundRectPerCorner(x, y, w, h, tl, tr, br, bl, r, g, b, a, sw) {
      const rr = Float32Array.of(x, y, x + w, y + h, tl, tl, tr, tr, br, br, bl, bl);
      const p = strokePaint(r, g, b, a, sw); canvas.drawRRect(rr, p);
    },
    fillOval(x, y, w, h, r, g, b, a) { const p = fillPaint(r, g, b, a); canvas.drawOval(CK.LTRBRect(x, y, x + w, y + h), p); },
    strokeOval(x, y, w, h, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawOval(CK.LTRBRect(x, y, x + w, y + h), p); },
    strokeLine(x1, y1, x2, y2, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawLine(x1, y1, x2, y2, p); },

    fillPolygon(pts, r, g, b, a) {
      const path = new CK.Path(); path.moveTo(pts[0], pts[1]);
      for (let i = 2; i < pts.length; i += 2) path.lineTo(pts[i], pts[i + 1]);
      path.close();
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); path.delete();
    },
    // SVG path d-string scaled by `size/viewbox` and translated to (tx,ty).
    strokeSvgPath(d, tx, ty, scale, r, g, b, a, sw) {
      const path = cachedSvgPath(d); if (!path) return;
      const m = CK.Matrix.multiply(CK.Matrix.translated(tx, ty), CK.Matrix.scaled(scale, scale));
      path.transform(m);
      const p = strokePaint(r, g, b, a, sw); canvas.drawPath(path, p); path.delete();
    },
    fillSvgPath(d, tx, ty, scale, evenOdd, r, g, b, a) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      const m = CK.Matrix.multiply(CK.Matrix.translated(tx, ty), CK.Matrix.scaled(scale, scale));
      path.transform(m);
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); path.delete();
    },
    fillSvgPathInRect(d, x, y, w, h, evenOdd, r, g, b, a) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); path.delete();
    },
    strokeSvgPathInRect(d, x, y, w, h, r, g, b, a, sw) {
      const path = cachedSvgPath(d); if (!path) return;
      fitPathToRect(path, x, y, w, h);
      const p = strokePaint(r, g, b, a, sw); canvas.drawPath(path, p); path.delete();
    },
    fillSvgPathInRectLinearGradient(d, x, y, w, h, evenOdd, stops, angleDeg, opacity) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      if (stops.length < 5) { path.delete(); return; }
      const shader = makeLinearGradient(x, y, w, h, stops, angleDeg, opacity);
      if (shader) {
        // shaderPaint always allocates fresh (one-off effect, excluded from
        // the shared cache) — this instance owns its own delete.
        const p = shaderPaint(shader);
        canvas.drawPath(path, p);
        p.delete();
        if (shader.delete) shader.delete();
      } else {
        const p = fillPaint(...firstStopColor(stops, opacity));
        canvas.drawPath(path, p);
      }
      path.delete();
    },
    fillSvgPathInRectRadialGradient(d, x, y, w, h, evenOdd, stops, cxFrac, cyFrac, radiusFrac, opacity) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      if (stops.length < 5) { path.delete(); return; }
      const shader = makeRadialGradient(x, y, w, h, stops, cxFrac, cyFrac, radiusFrac, opacity);
      if (shader) {
        // shaderPaint always allocates fresh (one-off effect, excluded from
        // the shared cache) — this instance owns its own delete.
        const p = shaderPaint(shader);
        canvas.drawPath(path, p);
        p.delete();
        if (shader.delete) shader.delete();
      } else {
        const p = fillPaint(...firstStopColor(stops, opacity));
        canvas.drawPath(path, p);
      }
      path.delete();
    },
    fillInnerShadowSvgPath(d, x, y, w, h, evenOdd, offsetX, offsetY, blur, r, g, b, a) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      const offsetPath = cachedSvgPath(d);
      if (!offsetPath) { path.delete(); return; }
      if (evenOdd) offsetPath.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(offsetPath, x, y, w, h);
      offsetPath.transform(CK.Matrix.translated(offsetX, offsetY));

      canvas.save();
      canvas.clipPath(path, CK.ClipOp.Intersect, true);
      canvas.saveLayer(null, CK.LTRBRect(x, y, x + w, y + h));

      const fill = fillPaint(r, g, b, a);
      canvas.drawPath(path, fill);

      // Dedicated (allocated + deleted) paint: `cut` carries a one-off blend
      // mode + blur mask filter, so it is excluded from the shared fill-
      // paint cache by design — reusing the cache here would leak DstOut /
      // the blur mask into every later `fillPaint()` caller.
      const cut = allocFillPaint(0, 0, 0, 1);
      cut.setBlendMode(CK.BlendMode.DstOut);
      let mask = null;
      const sigma = blur * 0.5;
      if (sigma > 0 && CK.MaskFilter && CK.MaskFilter.MakeBlur) {
        mask = CK.MaskFilter.MakeBlur(CK.BlurStyle.Normal, sigma, false);
        if (mask) cut.setMaskFilter(mask);
      }
      canvas.drawPath(offsetPath, cut);

      cut.delete(); if (mask && mask.delete) mask.delete();
      canvas.restore();
      canvas.restore();
      offsetPath.delete(); path.delete();
    },

    imageDecoded(imageIdLo, imageIdHi, maxEdgePx) {
      return imageCaches.hasFullImage(imageIdLo, imageIdHi, maxEdgePx);
    },

    decodeImage(imageIdLo, imageIdHi, encoded, maxEdgePx) {
      return imageCaches.installFullImage(imageIdLo, imageIdHi, encoded, maxEdgePx);
    },

    drawImageThumb(imageIdLo, imageIdHi, x, y, w, h, jpeg) {
      imageCaches.drawThumbnailCover(canvas, imageIdLo, imageIdHi, jpeg, x, y, w, h);
    },

    drawImageWithOptions(imageIdLo, imageIdHi, x, y, w, h, mode, transform, adjustments, opacity, cornerRadius, blendMode, originalWidth, originalHeight, tileScale) {
      const image = imageCaches.fullImage(imageIdLo, imageIdHi);
      if (!image || !(w > 0) || !(h > 0)) return;
      const imageW = image.width();
      const imageH = image.height();
      if (!(imageW > 0) || !(imageH > 0)) return;
      const dst = CK.LTRBRect(x, y, x + w, y + h);
      const src = CK.LTRBRect(0, 0, imageW, imageH);
      const paint = new CK.Paint();
      paint.setAntiAlias(true);
      paint.setAlphaf(Math.max(0, Math.min(1, opacity)));
      paint.setBlendMode(blendModeForCode(blendMode));
      const matrix = imageAdjustmentMatrix(adjustments);
      const colorFilter = matrix && CK.ColorFilter && CK.ColorFilter.MakeMatrix
        ? CK.ColorFilter.MakeMatrix(matrix)
        : null;
      if (colorFilter) paint.setColorFilter(colorFilter);

      canvas.save();
      if (cornerRadius > 0.5) {
        canvas.clipRRect(CK.RRectXY(dst, cornerRadius, cornerRadius), CK.ClipOp.Intersect, true);
      }
      let shader = null;
      const local = mode === 3 ? null : figmaImageLocalMatrix(x, y, w, h, imageW, imageH, transform);
      if (local) {
        canvas.clipRect(dst, CK.ClipOp.Intersect, true);
        const tileMode = typeof CK.TileMode.Decal !== 'undefined' ? CK.TileMode.Decal : CK.TileMode.Clamp;
        shader = image.makeShaderOptions(tileMode, tileMode, CK.FilterMode.Linear, CK.MipmapMode.None, local);
        if (shader) {
          paint.setShader(shader);
          canvas.drawRect(dst, paint);
        }
      }
      if (!shader) {
        if (mode === 1) {
          const scale = Math.min(w / imageW, h / imageH);
          const dw = imageW * scale;
          const dh = imageH * scale;
          drawImageRectLinear(image, src, CK.LTRBRect(x + (w - dw) / 2, y + (h - dh) / 2, x + (w + dw) / 2, y + (h + dh) / 2), paint);
        } else if (mode === 3) {
          canvas.clipRect(dst, CK.ClipOp.Intersect, true);
          const sourceW = Number.isFinite(originalWidth) && originalWidth > 0 ? originalWidth : imageW;
          const sourceH = Number.isFinite(originalHeight) && originalHeight > 0 ? originalHeight : imageH;
          const authoredTileScale = Number.isFinite(tileScale) && tileScale > 0 ? tileScale : 1.0;
          const maxRepeatsPerAxis = 128;
          const safeTileScale = Math.max(
            authoredTileScale,
            1 / sourceW,
            1 / sourceH,
            Math.abs(w) / (sourceW * maxRepeatsPerAxis),
            Math.abs(h) / (sourceH * maxRepeatsPerAxis),
          );
          const tileW = sourceW * safeTileScale;
          const tileH = sourceH * safeTileScale;
          const centeredX = x + (w - tileW) / 2;
          const centeredY = y + (h - tileH) / 2;
          const startX = centeredX - Math.max(0, Math.ceil((centeredX - x) / tileW)) * tileW;
          const startY = centeredY - Math.max(0, Math.ceil((centeredY - y) / tileH)) * tileH;
          let iy = startY;
          while (iy < y + h) {
            let ix = startX;
            while (ix < x + w) {
              drawImageRectLinear(image, src, CK.LTRBRect(ix, iy, ix + tileW, iy + tileH), paint);
              const nextX = ix + tileW;
              if (!(nextX > ix)) break;
              ix = nextX;
            }
            const nextY = iy + tileH;
            if (!(nextY > iy)) break;
            iy = nextY;
          }
        } else if (mode === 0 || mode === 2) {
          const scale = Math.max(w / imageW, h / imageH);
          const dw = imageW * scale;
          const dh = imageH * scale;
          canvas.clipRect(dst, CK.ClipOp.Intersect, true);
          drawImageRectLinear(image, src, CK.LTRBRect(x + (w - dw) / 2, y + (h - dh) / 2, x + (w + dw) / 2, y + (h + dh) / 2), paint);
        } else {
          drawImageRectLinear(image, src, dst, paint);
        }
      }
      canvas.restore();
      paint.delete();
      if (shader && shader.delete) shader.delete();
      if (colorFilter && colorFilter.delete) colorFilter.delete();
    },

    drawText(t, family, x, y, sz, weight, italic, r, g, b, a) {
      if (!t) return;
      // Per-CHARACTER family resolution: chars the selected named face covers draw
      // with it; the rest fall to the script-segmented path — so a mixed run
      // keeps the named family where it applies and never renders tofu,
      // matching native. Draw + measure split on the SAME registeredCoverage
      // segments and share drawScriptRun/registeredFamilyFont, so advances agree.
      const familyEntry = familyTypefaceEntry(family);
      const covSegs = familyEntry ? registeredCoverageSegments(familyEntry.key, familyEntry.tf, sz, t) : null;
      if (!covSegs || (covSegs.length === 1 && !covSegs[0].registered)) {
        drawScriptRun(t, x, y, sz, weight, italic, r, g, b, a);
        return;
      }
      const p = fillPaint(r, g, b, a);
      if (weight >= 600 && isPaintStyle(CK.PaintStyle.StrokeAndFill)) {
        setPaintStyle(p, CK.PaintStyle.StrokeAndFill);
        p.setStrokeWidth(sz * 0.06);
      }
      let cx = x;
      for (const seg of covSegs) {
        if (seg.registered) {
          const f = registeredFamilyFont(familyEntry.key, familyEntry.tf, sz, italic);
          canvas.drawText(seg.text, cx, y, p, f);
          cx += runWidth(f, seg.text);
        } else {
          cx += drawScriptRun(seg.text, cx, y, sz, weight, italic, r, g, b, a);
        }
      }
    },
    drawShapedText(t, family, x, y, sz, weight, italic, r, g, b, a) {
      if (!t) return;
      if (shapedTextUnavailable()) {
        // No 2D context (headless embed / hostile sandbox). The segmented
        // path at least paints glyphs rather than nothing, though it cannot
        // reorder or join them.
        drawScriptRun(t, x, y, sz, weight, italic, r, g, b, a);
        return;
      }
      drawBrowserText(t, x, y, sz, weight, italic, r, g, b, a, false, family);
    },
    measureShapedText(t, family, sz, weight, italic) {
      if (!t) return 0;
      // Must mirror `drawShapedText` exactly — measuring on one engine and
      // painting on another is what desyncs wrapping and caret geometry.
      if (shapedTextUnavailable()) {
        return this.measureTextFamilyStyled(t, family, sz, weight, italic);
      }
      return browserTextMeasure(t, sz, weight, italic, family);
    },
    measureText(t, sz) {
      return this.measureTextStyled(t, sz, 400, false);
    },
    textAscent(family, sz, weight) {
      const familyEntry = familyTypefaceEntry(family);
      const font = new CK.Font(familyEntry ? familyEntry.tf : tfFor('M', false), sz);
      let ascent = sz * 0.8;
      if (font.getMetrics) {
        const metrics = font.getMetrics();
        const candidate = metrics && Number.isFinite(metrics.ascent) ? -metrics.ascent : NaN;
        if (Number.isFinite(candidate) && candidate > 0) ascent = candidate;
      }
      font.delete();
      return ascent;
    },
    textFirstBaseline(t, family, sz, weight, italic, lineHeight) {
      const firstBreak = String(t || '').search(/[\r\n]/);
      const line = firstBreak < 0 ? String(t || '') : String(t || '').slice(0, firstBreak);
      const sample = line || 'M';
      const familyEntry = familyTypefaceEntry(family);
      const covSegs = familyEntry
        ? registeredCoverageSegments(familyEntry.key, familyEntry.tf, sz, sample)
        : null;
      let ascent = 0;
      let descent = 0;
      const includeBrowserMetrics = (text) => {
        if (!browserTextCtx) return false;
        browserTextCtx.font = browserTextFont(sz, weight, italic);
        const metrics = browserTextCtx.measureText(text);
        const fontAscent = Number(metrics.fontBoundingBoxAscent);
        const fontDescent = Number(metrics.fontBoundingBoxDescent);
        const inkAscent = Number(metrics.actualBoundingBoxAscent);
        const inkDescent = Number(metrics.actualBoundingBoxDescent);
        ascent = Math.max(ascent,
          Number.isFinite(fontAscent) ? fontAscent : 0,
          Number.isFinite(inkAscent) ? inkAscent : 0);
        descent = Math.max(descent,
          Number.isFinite(fontDescent) ? fontDescent : 0,
          Number.isFinite(inkDescent) ? inkDescent : 0);
        return true;
      };
      const includeFontMetrics = (font) => {
        if (!font || !font.getMetrics) return;
        const metrics = font.getMetrics();
        if (!metrics) return;
        const candidateAscent = -Number(metrics.ascent);
        const candidateDescent = Number(metrics.descent);
        if (Number.isFinite(candidateAscent)) ascent = Math.max(ascent, candidateAscent);
        if (Number.isFinite(candidateDescent)) descent = Math.max(descent, candidateDescent);
      };
      const includeFallback = (text) => {
        for (const seg of segments(text)) {
          if (includeBrowserMetrics(seg.text)) continue;
          const font = new CK.Font(tfFor(seg.text, seg.emoji), sz);
          includeFontMetrics(font);
          font.delete();
        }
      };
      if (covSegs && covSegs.some((seg) => seg.registered)) {
        for (const seg of covSegs) {
          if (seg.registered) {
            includeFontMetrics(registeredFamilyFont(familyEntry.key, familyEntry.tf, sz, italic));
          } else {
            includeFallback(seg.text);
          }
        }
      } else {
        includeFallback(sample);
      }
      if (!(ascent > 0)) ascent = sz * 0.8;
      if (!(descent > 0)) descent = sz * 0.2;
      const naturalHeight = ascent + descent;
      const targetHeight = lineHeight > 0 ? sz * lineHeight : naturalHeight;
      const baseline = ascent + (targetHeight - naturalHeight) / 2;
      return Number.isFinite(baseline) && baseline > 0 ? baseline : sz * 0.8;
    },
    measureTextFamilyStyled(t, family, sz, weight, italic) {
      const familyEntry = familyTypefaceEntry(family);
      const covSegs = familyEntry ? registeredCoverageSegments(familyEntry.key, familyEntry.tf, sz, t) : null;
      if (!covSegs || (covSegs.length === 1 && !covSegs[0].registered)) {
        // No registered family (or it covers nothing): the family-blind
        // script-segmented measure — the SAME path drawText falls to.
        return this.measureTextStyled(t, sz, weight, italic);
      }
      let w = 0;
      for (const seg of covSegs) {
        if (seg.registered) {
          const f = registeredFamilyFont(familyEntry.key, familyEntry.tf, sz, italic);
          w += runWidth(f, seg.text);
        } else {
          w += this.measureTextStyled(seg.text, sz, weight, italic);
        }
      }
      return w;
    },
    measureTextStyled(t, sz, weight, italic) {
      let w = 0;
      for (const seg of segments(t)) {
        if (shouldUseBrowserTextFallback(seg.text, seg.emoji)) {
          w += browserTextMeasure(seg.text, sz, weight, italic);
          continue;
        }
        const f = new CK.Font(tfFor(seg.text, seg.emoji), sz);
        if (italic && !seg.emoji) f.setSkewX(-0.25);
        w += runWidth(f, seg.text);
        f.delete();
      }
      return w;
    },
    registerSystemFont(family, bytes) {
      const key = normalizedFamily(family);
      if (!key || systemTypefaceKeys.has(key)) return Boolean(key);
      const tf = CK.Typeface.MakeFreeTypeFaceFromData(copyBytes(bytes));
      if (!tf) return false;
      const entry = {
        key,
        family: String(family || ''),
        tf,
        cjk: isCjkFamily(family),
        emoji: isEmojiFamily(family),
        textFallback: isTextFallbackFamily(family),
      };
      systemTypefaceKeys.add(key);
      systemTypefaces.push(entry);
      systemTypefacesByFamily.set(key, entry);
      coverageCache.clear();
      clearRegisteredFontCaches();
      return true;
    },
    registerImportedFont(family, bytes) {
      const key = primaryFamilyKey(family);
      if (!key) return false;
      const tf = CK.Typeface.MakeFreeTypeFaceFromData(copyBytes(bytes));
      if (!tf) return false;
      // Replace any prior face under the same key, freeing its wasm-heap tf.
      const prev = importedTypefaces.get(key);
      if (prev && prev.tf && prev.tf.delete) prev.tf.delete();
      importedTypefaces.set(key, { tf, family: String(family || '') });
      coverageCache.clear();
      clearRegisteredFontCaches();
      return true;
    },
    // (Fresh browser imports parse the family name in Rust via `ttf-parser` —
    // the vendored CanvasKit build exposes no family-name API — then register
    // through `registerImportedFont` above with the known family.)
    // The display names of every registered imported family, so the Rust snapshot
    // (which doesn't own the registry on web) can mirror the picker's Imported
    // group after every add / remove and at mount.
    importedFamilyList() {
      const out = [];
      for (const entry of importedTypefaces.values()) {
        if (entry && entry.family) out.push(entry.family);
      }
      return out;
    },
    removeImportedFont(family) {
      const key = primaryFamilyKey(family);
      if (!key) return;
      const entry = importedTypefaces.get(key);
      if (!entry) return;
      if (entry.tf && entry.tf.delete) entry.tf.delete();
      importedTypefaces.delete(key);
      clearRegisteredFontCaches();
    },

    clipRect(x, y, w, h) { canvas.clipRect(CK.LTRBRect(x, y, x + w, y + h), CK.ClipOp.Intersect, true); },
    clipRoundRect(x, y, w, h, rad) { canvas.clipRRect(CK.RRectXY(CK.LTRBRect(x, y, x + w, y + h), rad, rad), CK.ClipOp.Intersect, true); },
    clipRoundRectPerCorner(x, y, w, h, tl, tr, br, bl) {
      const rr = Float32Array.of(x, y, x + w, y + h, tl, tl, tr, tr, br, br, bl, bl);
      canvas.clipRRect(rr, CK.ClipOp.Intersect, true);
    },
    clipOval(x, y, w, h) {
      const path = new CK.Path();
      path.addOval(CK.LTRBRect(x, y, x + w, y + h));
      canvas.clipPath(path, CK.ClipOp.Intersect, true);
      path.delete();
    },
    clipPolygon(pts) {
      if (pts.length < 6) {
        canvas.clipRect(CK.LTRBRect(0, 0, 0, 0), CK.ClipOp.Intersect, true);
        return;
      }
      const path = new CK.Path(); path.moveTo(pts[0], pts[1]);
      for (let i = 2; i < pts.length; i += 2) path.lineTo(pts[i], pts[i + 1]);
      path.close();
      canvas.clipPath(path, CK.ClipOp.Intersect, true);
      path.delete();
    },
    clipSvgPathInRect(d, x, y, w, h, evenOdd) {
      const path = cachedSvgPath(d);
      if (!path) {
        canvas.clipRect(CK.LTRBRect(0, 0, 0, 0), CK.ClipOp.Intersect, true);
        return;
      }
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      canvas.clipPath(path, CK.ClipOp.Intersect, true);
      path.delete();
    },
    save() { canvas.save(); },
    pushCompositeLayer(x, y, w, h, opacity, blendMode) {
      const paint = new CK.Paint();
      paint.setAlphaf(Math.max(0, Math.min(1, opacity)));
      paint.setBlendMode(blendModeForCode(blendMode));
      canvas.saveLayer(paint, CK.LTRBRect(x, y, x + w, y + h));
      paint.delete();
    },
    pushMaskSourceLayer(luminance) {
      const paint = new CK.Paint();
      paint.setBlendMode(CK.BlendMode.DstIn);
      const luma = luminance && CK.ColorFilter && CK.ColorFilter.MakeLuma
        ? CK.ColorFilter.MakeLuma()
        : null;
      if (luma) paint.setColorFilter(luma);
      canvas.saveLayer(paint);
      paint.delete();
      if (luma && luma.delete) luma.delete();
    },
    pushBlendLayer(blendMode) {
      if (!blendMode) {
        canvas.save();
        return;
      }
      const paint = new CK.Paint();
      paint.setBlendMode(blendModeForCode(blendMode));
      canvas.saveLayer(paint);
      paint.delete();
    },
    pushBackdropBlurLayer(sigma) {
      if (!(sigma > 0) || !CK.ImageFilter || !CK.ImageFilter.MakeBlur) {
        canvas.save();
        return;
      }
      const filter = CK.ImageFilter.MakeBlur(sigma, sigma, CK.TileMode.Clamp, null);
      if (!filter) {
        canvas.save();
        return;
      }
      canvas.saveLayer(null, null, filter, 0, CK.TileMode.Clamp);
      if (filter.delete) filter.delete();
    },
    restore() { canvas.restore(); },
    translate(x, y) { canvas.translate(x, y); },
    scale(sx, sy) { canvas.scale(sx, sy); },
    rotate(deg, px, py) { canvas.rotate(deg, px, py); },

    resize(w, h) {
      el.width = w; el.height = h;
      try { surface.delete(); } catch (e) {}
      surface = CK.MakeWebGLCanvasSurface(canvasId);
      canvas = surface.getCanvas();
    },
    // Set the device-pixel-ratio used to supersample the offscreen text raster.
    // Called from Rust on mount + every display resize so glyph bitmaps stay
    // crisp on HiDPI screens.
    setDpr(v) {
      const next = Number.isFinite(v) && v > 0 ? Math.max(1, v) : 1;
      if (next === textDpr) return;
      textDpr = next;
      clearBrowserTextCache();
    },
  };
}
