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

// Initialise CanvasKit on `canvasId`. `roboto` is the Latin UI font; `cjk` the
// CJK fallback (may be empty). Returns a bridge object the Rust backend drives.
export async function opCkInit(canvasId, roboto, cjk, emoji) {
  await loadScript('/canvaskit/canvaskit.js');
  const CK = await CanvasKitInit({ locateFile: (f) => '/canvaskit/' + f });
  let surface = CK.MakeWebGLCanvasSurface(canvasId);
  if (!surface) throw new Error('CanvasKit: MakeWebGLCanvasSurface returned null');
  let canvas = surface.getCanvas();
  const el = document.getElementById(canvasId);

  const tfLatin = CK.Typeface.MakeFreeTypeFaceFromData(copyBytes(roboto));
  const tfCjk = cjk && cjk.byteLength > 0 ? CK.Typeface.MakeFreeTypeFaceFromData(copyBytes(cjk)) : null;
  const tfEmoji = emoji && emoji.byteLength > 0 ? CK.Typeface.MakeFreeTypeFaceFromData(copyBytes(emoji)) : null;

  // CJK range check (Han / Hiragana / Katakana / Hangul / fullwidth).
  const hasCjk = (t) => { for (const ch of t) { const c = ch.codePointAt(0); if ((c >= 0x2e80 && c <= 0x9fff) || (c >= 0xac00 && c <= 0xd7a3) || (c >= 0xff00 && c <= 0xffef) || (c >= 0x3000 && c <= 0x30ff)) return true; } return false; };
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
  const fillPaint = (r, g, b, a) => { const p = new CK.Paint(); p.setColor(col(r, g, b, a)); p.setAntiAlias(true); p.setStyle(CK.PaintStyle.Fill); return p; };
  const strokePaint = (r, g, b, a, w) => { const p = new CK.Paint(); p.setColor(col(r, g, b, a)); p.setAntiAlias(true); p.setStyle(CK.PaintStyle.Stroke); p.setStrokeWidth(w); p.setStrokeCap(CK.StrokeCap.Round); p.setStrokeJoin(CK.StrokeJoin.Round); return p; };
  const tfFor = (t, emojiRun) => (emojiRun && tfEmoji) ? tfEmoji : (tfCjk && hasCjk(t) ? tfCjk : tfLatin);
  const fontFor = (t, sz) => new CK.Font(tfCjk && hasCjk(t) ? tfCjk : tfLatin, sz);
  const runWidth = (f, s) => { const ids = f.getGlyphIDs(s); return f.getGlyphWidths(ids).reduce((a, v) => a + v, 0); };

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

    drawText(t, x, y, sz, weight, italic, r, g, b, a) {
      const p = fillPaint(r, g, b, a);
      if (weight >= 600) { p.setStyle(CK.PaintStyle.StrokeAndFill); p.setStrokeWidth(sz * 0.06); }
      let cx = x;
      for (const seg of segments(t)) {
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
        const f = new CK.Font(tfFor(seg.text, seg.emoji), sz);
        w += runWidth(f, seg.text);
        f.delete();
      }
      return w;
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
