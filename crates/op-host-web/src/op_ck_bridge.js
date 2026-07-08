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

// Initialise CanvasKit on `canvasId`. Returns a bridge object the Rust backend
// drives. Text is rasterized with browser/system fonts by default; the Rust
// side can additionally register Local Font Access faces through
// registerSystemFont().
export async function opCkInit(canvasId) {
  await loadScript('/canvaskit/canvaskit.js');
  const CK = await CanvasKitInit({ locateFile: (f) => '/canvaskit/' + f });
  let surface = CK.MakeWebGLCanvasSurface(canvasId);
  if (!surface) throw new Error('CanvasKit: MakeWebGLCanvasSurface returned null');
  let canvas = surface.getCanvas();
  const el = document.getElementById(canvasId);

  const systemTypefaces = [];
  const systemTypefaceKeys = new Set();
  // User-imported font faces, keyed by normalized family name -> { tf, family }.
  // A named family is a deliberate single-typeface choice (mirrors how native
  // resolves a named family via FontMgr before any script fallback), so
  // imported text shapes the WHOLE run with one typeface instead of
  // re-segmenting by script.
  const importedTypefaces = new Map();
  const coverageCache = new Map();
  const browserTextCanvas = document.createElement('canvas');
  const browserTextCtx = browserTextCanvas.getContext('2d', { willReadFrequently: true });
  const browserTextCache = new Map();
  // Device-pixel-ratio for the offscreen text raster. Glyph bitmaps are drawn
  // onto the CanvasKit surface while a `scale(dpr, dpr)` transform is active, so
  // a 1x raster would be magnified `dpr`x and read blurry on HiDPI displays. We
  // supersample the offscreen canvas by `textDpr` and composite it back at
  // `1 / textDpr` so glyphs land at native device resolution. Driven from Rust
  // via `setDpr` on mount + every display resize.
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
  const normalizedFamily = (family) => String(family || '').trim().toLowerCase();
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
  const fillPaint = (r, g, b, a) => { const p = new CK.Paint(); p.setColor(col(r, g, b, a)); p.setAntiAlias(true); setPaintStyle(p, CK.PaintStyle.Fill); return p; };
  const strokePaint = (r, g, b, a, w) => { const p = new CK.Paint(); p.setColor(col(r, g, b, a)); p.setAntiAlias(true); setPaintStyle(p, CK.PaintStyle.Stroke); p.setStrokeWidth(w); p.setStrokeCap(CK.StrokeCap.Round); p.setStrokeJoin(CK.StrokeJoin.Round); return p; };
  const browserTextFont = (sz, weight, italic) => `${italic ? 'italic ' : ''}${Math.max(100, Math.min(900, Math.round(weight || 400)))} ${Math.max(1, sz)}px ${browserTextFontStack}`;
  const shouldUseBrowserTextFallback = (_t, _emojiRun) => Boolean(browserTextCtx);
  const allSegmentsUseBrowserTextFallback = (segs) => segs.length > 0 && segs.every((seg) => shouldUseBrowserTextFallback(seg.text, seg.emoji));
  const browserTextMeasure = (t, sz, weight = 400, italic = false) => {
    if (!browserTextCtx) return 0;
    browserTextCtx.font = browserTextFont(sz, weight, italic);
    return browserTextCtx.measureText(t).width;
  };
  const browserTextImage = (t, sz, weight, italic, r, g, b, a) => {
    if (!browserTextCtx) return null;
    const ss = Math.max(1, textDpr);
    const key = [t, sz, weight, italic ? 1 : 0, r, g, b, a, ss].join('\n');
    if (browserTextCache.has(key)) return browserTextCache.get(key);
    const font = browserTextFont(sz, weight, italic);
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
    browserTextCtx.fillStyle = `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`;
    browserTextCtx.fillText(t, 2, baseline);
    browserTextCtx.setTransform(1, 0, 0, 1, 0, 0);
    const image = CK.MakeImageFromCanvasImageSource(browserTextCanvas);
    if (!image) return null;
    const entry = { image, width: metrics.width, baseline, ss };
    browserTextCache.set(key, entry);
    if (browserTextCache.size > 512) {
      const firstKey = browserTextCache.keys().next().value;
      const old = browserTextCache.get(firstKey);
      if (old && old.image && old.image.delete) old.image.delete();
      browserTextCache.delete(firstKey);
    }
    return entry;
  };
  const drawBrowserText = (t, x, y, sz, weight, italic, r, g, b, a) => {
    const entry = browserTextImage(t, sz, weight, italic, r, g, b, a);
    if (!entry) return 0;
    const ss = entry.ss || 1;
    if (ss !== 1) {
      // The bitmap is `ss`x oversampled; place it in logical space at
      // (x-2, y-baseline) then scale down by `ss` so its device footprint
      // matches the intended CSS box at native resolution.
      canvas.save();
      canvas.translate(x - 2, y - entry.baseline);
      canvas.scale(1 / ss, 1 / ss);
      canvas.drawImage(entry.image, 0, 0);
      canvas.restore();
    } else {
      canvas.drawImage(entry.image, x - 2, y - entry.baseline);
    }
    return entry.width;
  };
  const shaderPaint = (shader) => { const p = new CK.Paint(); p.setAntiAlias(true); setPaintStyle(p, CK.PaintStyle.Fill); p.setShader(shader); return p; };
  const firstStopColor = (stops, opacity) => stops.length >= 5 ? col(stops[1], stops[2], stops[3], stops[4] * opacity) : col(0, 0, 0, 0);
  const gradientStops = (stops, opacity) => {
    const colors = [];
    const offsets = [];
    for (let i = 0; i + 4 < stops.length; i += 5) {
      offsets.push(stops[i]);
      colors.push(col(stops[i + 1], stops[i + 2], stops[i + 3], stops[i + 4] * opacity));
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
  // Normalize a CSS font stack to an imported-family key, mirroring jian-skia
  // `primary_font_family`: first family before a comma, quotes stripped;
  // empty / generic keywords resolve to no imported family.
  const GENERIC_FAMILIES = new Set(['system-ui', 'sans-serif', 'serif', 'monospace', '-apple-system']);
  const primaryFamilyKey = (family) => {
    const first = String(family || '').split(',')[0].trim().replace(/^["']|["']$/g, '').trim();
    if (!first) return '';
    const key = first.toLowerCase();
    return GENERIC_FAMILIES.has(key) ? '' : key;
  };
  // Resolve the imported typeface for a family stack (null when unregistered).
  const familyTypeface = (family) => {
    const key = primaryFamilyKey(family);
    if (!key) return null;
    const entry = importedTypefaces.get(key);
    return entry ? entry.tf : null;
  };
  // Shared "typeface + font for (family, sz)" so the family-aware draw and
  // measure paths build the SAME CK.Font and agree to sub-pixel. Caller deletes.
  const importedFamilyFont = (tf, sz, italic) => {
    const f = new CK.Font(tf, sz);
    if (italic) f.setSkewX(-0.25);
    return f;
  };
  // Split a run into maximal {text, imported} segments by whether the imported
  // typeface has a glyph for each char (glyph id 0 = .notdef = uncovered). This
  // is per-CHARACTER, so a mixed run keeps the imported face for the chars it
  // covers and falls back (system/CJK/emoji) only for the rest — matching how
  // native resolves a named family per character, instead of dropping the
  // imported family for the whole run. `getGlyphIDs` returns one id per
  // codepoint, so it aligns with the codepoint iteration; if the counts don't
  // line up we conservatively treat the whole run as uncovered.
  const importedCoverageSegments = (tf, sz, t) => {
    const cps = Array.from(t);
    if (cps.length === 0) return [];
    const f = new CK.Font(tf, sz);
    let ids = null;
    try {
      ids = f.getGlyphIDs(t);
    } catch (e) {
      ids = null;
    }
    f.delete();
    if (!ids || ids.length !== cps.length) return [{ text: t, imported: false }];
    const out = [];
    let cur = '';
    let curImported = null;
    for (let i = 0; i < cps.length; i++) {
      const imp = ids[i] !== 0;
      if (curImported === null) {
        curImported = imp;
        cur = cps[i];
      } else if (imp === curImported) {
        cur += cps[i];
      } else {
        out.push({ text: cur, imported: curImported });
        cur = cps[i];
        curImported = imp;
      }
    }
    if (cur) out.push({ text: cur, imported: curImported });
    return out;
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
      for (const seg of segs) cx += drawBrowserText(seg.text, cx, y, sz, weight, italic, r, g, b, a);
      return cx - x;
    }
    const p = fillPaint(r, g, b, a);
    if (weight >= 600 && isPaintStyle(CK.PaintStyle.StrokeAndFill)) {
      setPaintStyle(p, CK.PaintStyle.StrokeAndFill);
      p.setStrokeWidth(sz * 0.06);
    }
    let cx = x;
    for (const seg of segs) {
      if (shouldUseBrowserTextFallback(seg.text, seg.emoji)) {
        cx += drawBrowserText(seg.text, cx, y, sz, weight, italic, r, g, b, a);
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
    const bounds = path.getBounds();
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

    fillRect(x, y, w, h, r, g, b, a) { const p = fillPaint(r, g, b, a); canvas.drawRect(CK.LTRBRect(x, y, x + w, y + h), p); p.delete(); },
    strokeRect(x, y, w, h, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawRect(CK.LTRBRect(x, y, x + w, y + h), p); p.delete(); },
    fillRoundRect(x, y, w, h, rad, r, g, b, a) { const p = fillPaint(r, g, b, a); canvas.drawRRect(CK.RRectXY(CK.LTRBRect(x, y, x + w, y + h), rad, rad), p); p.delete(); },
    strokeRoundRect(x, y, w, h, rad, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawRRect(CK.RRectXY(CK.LTRBRect(x, y, x + w, y + h), rad, rad), p); p.delete(); },
    fillOval(x, y, w, h, r, g, b, a) { const p = fillPaint(r, g, b, a); canvas.drawOval(CK.LTRBRect(x, y, x + w, y + h), p); p.delete(); },
    strokeOval(x, y, w, h, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawOval(CK.LTRBRect(x, y, x + w, y + h), p); p.delete(); },
    strokeLine(x1, y1, x2, y2, r, g, b, a, sw) { const p = strokePaint(r, g, b, a, sw); canvas.drawLine(x1, y1, x2, y2, p); p.delete(); },

    fillPolygon(pts, r, g, b, a) {
      const path = new CK.Path(); path.moveTo(pts[0], pts[1]);
      for (let i = 2; i < pts.length; i += 2) path.lineTo(pts[i], pts[i + 1]);
      path.close();
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    // SVG path d-string scaled by `size/viewbox` and translated to (tx,ty).
    strokeSvgPath(d, tx, ty, scale, r, g, b, a, sw) {
      const path = cachedSvgPath(d); if (!path) return;
      const m = CK.Matrix.multiply(CK.Matrix.translated(tx, ty), CK.Matrix.scaled(scale, scale));
      path.transform(m);
      const p = strokePaint(r, g, b, a, sw); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    fillSvgPath(d, tx, ty, scale, evenOdd, r, g, b, a) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      const m = CK.Matrix.multiply(CK.Matrix.translated(tx, ty), CK.Matrix.scaled(scale, scale));
      path.transform(m);
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    fillSvgPathInRect(d, x, y, w, h, evenOdd, r, g, b, a) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    strokeSvgPathInRect(d, x, y, w, h, r, g, b, a, sw) {
      const path = cachedSvgPath(d); if (!path) return;
      fitPathToRect(path, x, y, w, h);
      const p = strokePaint(r, g, b, a, sw); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    fillSvgPathInRectLinearGradient(d, x, y, w, h, evenOdd, stops, angleDeg, opacity) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      const gs = gradientStops(stops, opacity);
      if (!gs.colors.length) { path.delete(); return; }
      const points = linearGradientPoints(x, y, w, h, angleDeg);
      const shader = CK.Shader.MakeLinearGradient(points.start, points.end, gs.colors, gs.offsets, CK.TileMode.Clamp);
      const p = shader ? shaderPaint(shader) : fillPaint(...firstStopColor(stops, opacity));
      canvas.drawPath(path, p);
      p.delete(); if (shader && shader.delete) shader.delete(); path.delete();
    },
    fillSvgPathInRectRadialGradient(d, x, y, w, h, evenOdd, stops, cxFrac, cyFrac, radiusFrac, opacity) {
      const path = cachedSvgPath(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      const gs = gradientStops(stops, opacity);
      if (!gs.colors.length) { path.delete(); return; }
      const center = [x + w * Math.max(0, Math.min(1, cxFrac)), y + h * Math.max(0, Math.min(1, cyFrac))];
      const radius = Math.max(0.01, Math.max(w, h) * Math.max(0, Math.min(1, radiusFrac)));
      const shader = CK.Shader.MakeRadialGradient(center, radius, gs.colors, gs.offsets, CK.TileMode.Clamp);
      const p = shader ? shaderPaint(shader) : fillPaint(...firstStopColor(stops, opacity));
      canvas.drawPath(path, p);
      p.delete(); if (shader && shader.delete) shader.delete(); path.delete();
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

      const cut = fillPaint(0, 0, 0, 1);
      cut.setBlendMode(CK.BlendMode.DstOut);
      let mask = null;
      const sigma = blur * 0.5;
      if (sigma > 0 && CK.MaskFilter && CK.MaskFilter.MakeBlur) {
        mask = CK.MaskFilter.MakeBlur(CK.BlurStyle.Normal, sigma, false);
        if (mask) cut.setMaskFilter(mask);
      }
      canvas.drawPath(offsetPath, cut);

      cut.delete(); if (mask && mask.delete) mask.delete(); fill.delete();
      canvas.restore();
      canvas.restore();
      offsetPath.delete(); path.delete();
    },

    drawText(t, family, x, y, sz, weight, italic, r, g, b, a) {
      if (!t) return;
      // Per-CHARACTER family resolution: chars the imported face covers draw
      // with it; the rest fall to the script-segmented path — so a mixed run
      // keeps the imported family where it applies and never renders tofu,
      // matching native. Draw + measure split on the SAME importedCoverage
      // segments and share drawScriptRun/importedFamilyFont, so advances agree.
      const importedTf = familyTypeface(family);
      const covSegs = importedTf ? importedCoverageSegments(importedTf, sz, t) : null;
      if (!covSegs || (covSegs.length === 1 && !covSegs[0].imported)) {
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
        if (seg.imported) {
          const f = importedFamilyFont(importedTf, sz, italic);
          canvas.drawText(seg.text, cx, y, p, f);
          cx += runWidth(f, seg.text);
          f.delete();
        } else {
          cx += drawScriptRun(seg.text, cx, y, sz, weight, italic, r, g, b, a);
        }
      }
      p.delete();
    },
    measureText(t, sz) {
      return this.measureTextStyled(t, sz, 400, false);
    },
    measureTextFamilyStyled(t, family, sz, weight, italic) {
      const importedTf = familyTypeface(family);
      const covSegs = importedTf ? importedCoverageSegments(importedTf, sz, t) : null;
      if (!covSegs || (covSegs.length === 1 && !covSegs[0].imported)) {
        // No imported family (or it covers nothing): the family-blind
        // script-segmented measure — the SAME path drawText falls to.
        return this.measureTextStyled(t, sz, weight, italic);
      }
      let w = 0;
      for (const seg of covSegs) {
        if (seg.imported) {
          const f = importedFamilyFont(importedTf, sz, italic);
          w += runWidth(f, seg.text);
          f.delete();
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
      systemTypefaceKeys.add(key);
      systemTypefaces.push({
        key,
        family: String(family || ''),
        tf,
        cjk: isCjkFamily(family),
        emoji: isEmojiFamily(family),
        textFallback: isTextFallbackFamily(family),
      });
      coverageCache.clear();
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
    },

    clipRect(x, y, w, h) { canvas.clipRect(CK.LTRBRect(x, y, x + w, y + h), CK.ClipOp.Intersect, true); },
    clipRoundRect(x, y, w, h, rad) { canvas.clipRRect(CK.RRectXY(CK.LTRBRect(x, y, x + w, y + h), rad, rad), CK.ClipOp.Intersect, true); },
    save() { canvas.save(); },
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
      // Prior bitmaps were baked at the old ratio; drop them so they re-raster.
      for (const entry of browserTextCache.values()) {
        if (entry && entry.image && entry.image.delete) entry.image.delete();
      }
      browserTextCache.clear();
    },
  };
}
