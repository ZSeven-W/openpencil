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
  const coverageCache = new Map();
  const browserTextCanvas = document.createElement('canvas');
  const browserTextCtx = browserTextCanvas.getContext('2d', { willReadFrequently: true });
  const browserTextCache = new Map();
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
    '"SF Hebrew"',
    '"SFHebrew"',
    '"SF Georgian"',
    '"SFGeorgian"',
    '"Thonburi"',
    '"Sukhumvit Set"',
    '"SukhumvitSet"',
    '"Noto Sans Thai"',
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
  const fillPaint = (r, g, b, a) => { const p = new CK.Paint(); p.setColor(col(r, g, b, a)); p.setAntiAlias(true); p.setStyle(CK.PaintStyle.Fill); return p; };
  const strokePaint = (r, g, b, a, w) => { const p = new CK.Paint(); p.setColor(col(r, g, b, a)); p.setAntiAlias(true); p.setStyle(CK.PaintStyle.Stroke); p.setStrokeWidth(w); p.setStrokeCap(CK.StrokeCap.Round); p.setStrokeJoin(CK.StrokeJoin.Round); return p; };
  const browserTextFont = (sz, weight, italic) => `${italic ? 'italic ' : ''}${Math.max(100, Math.min(900, Math.round(weight || 400)))} ${Math.max(1, sz)}px ${browserTextFontStack}`;
  const shouldUseBrowserTextFallback = (_t, _emojiRun) => Boolean(browserTextCtx);
  const browserTextMeasure = (t, sz, weight = 400, italic = false) => {
    if (!browserTextCtx) return 0;
    browserTextCtx.font = browserTextFont(sz, weight, italic);
    return browserTextCtx.measureText(t).width;
  };
  const browserTextImage = (t, sz, weight, italic, r, g, b, a) => {
    if (!browserTextCtx) return null;
    const key = [t, sz, weight, italic ? 1 : 0, r, g, b, a].join('\n');
    if (browserTextCache.has(key)) return browserTextCache.get(key);
    const font = browserTextFont(sz, weight, italic);
    browserTextCtx.font = font;
    const metrics = browserTextCtx.measureText(t);
    const width = Math.max(1, Math.ceil(metrics.width + 4));
    const ascent = Math.ceil(metrics.actualBoundingBoxAscent || sz * 0.8);
    const descent = Math.ceil(metrics.actualBoundingBoxDescent || sz * 0.25);
    const baseline = ascent + 2;
    const height = Math.max(1, baseline + descent + 2);
    browserTextCanvas.width = width;
    browserTextCanvas.height = height;
    browserTextCtx.clearRect(0, 0, width, height);
    browserTextCtx.font = font;
    browserTextCtx.textBaseline = 'alphabetic';
    browserTextCtx.fillStyle = `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`;
    browserTextCtx.fillText(t, 2, baseline);
    const image = CK.MakeImageFromCanvasImageSource(browserTextCanvas);
    if (!image) return null;
    const entry = { image, width: metrics.width, baseline };
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
    canvas.drawImage(entry.image, x - 2, y - entry.baseline);
    return entry.width;
  };
  const shaderPaint = (shader) => { const p = new CK.Paint(); p.setAntiAlias(true); p.setStyle(CK.PaintStyle.Fill); p.setShader(shader); return p; };
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
      const path = CK.Path.MakeFromSVGString(d); if (!path) return;
      const m = CK.Matrix.multiply(CK.Matrix.translated(tx, ty), CK.Matrix.scaled(scale, scale));
      path.transform(m);
      const p = strokePaint(r, g, b, a, sw); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    fillSvgPath(d, tx, ty, scale, evenOdd, r, g, b, a) {
      const path = CK.Path.MakeFromSVGString(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      const m = CK.Matrix.multiply(CK.Matrix.translated(tx, ty), CK.Matrix.scaled(scale, scale));
      path.transform(m);
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    fillSvgPathInRect(d, x, y, w, h, evenOdd, r, g, b, a) {
      const path = CK.Path.MakeFromSVGString(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      const p = fillPaint(r, g, b, a); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    strokeSvgPathInRect(d, x, y, w, h, r, g, b, a, sw) {
      const path = CK.Path.MakeFromSVGString(d); if (!path) return;
      fitPathToRect(path, x, y, w, h);
      const p = strokePaint(r, g, b, a, sw); canvas.drawPath(path, p); p.delete(); path.delete();
    },
    fillSvgPathInRectLinearGradient(d, x, y, w, h, evenOdd, stops, angleDeg, opacity) {
      const path = CK.Path.MakeFromSVGString(d); if (!path) return;
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
      const path = CK.Path.MakeFromSVGString(d); if (!path) return;
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
      const path = CK.Path.MakeFromSVGString(d); if (!path) return;
      if (evenOdd) path.setFillType(CK.FillType.EvenOdd);
      fitPathToRect(path, x, y, w, h);
      const offsetPath = CK.Path.MakeFromSVGString(d);
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

    drawText(t, x, y, sz, weight, italic, r, g, b, a) {
      const p = fillPaint(r, g, b, a);
      if (weight >= 600 && isPaintStyle(CK.PaintStyle.StrokeAndFill)) {
        p.setStyle(CK.PaintStyle.StrokeAndFill);
        p.setStrokeWidth(sz * 0.06);
      }
      let cx = x;
      for (const seg of segments(t)) {
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
    },
    measureText(t, sz) {
      let w = 0;
      for (const seg of segments(t)) {
        if (shouldUseBrowserTextFallback(seg.text, seg.emoji)) {
          w += browserTextMeasure(seg.text, sz);
          continue;
        }
        const f = new CK.Font(tfFor(seg.text, seg.emoji), sz);
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
  };
}
