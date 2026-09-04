/**
 * Collect bounded, de-identified design-system evidence from the active page.
 *
 * This is deliberately separate from the snapshot extractor. A snapshot is a
 * lossless import payload and may contain page text, URLs and image bytes;
 * design evidence is a small statistical summary intended for an LLM. The
 * injected collector never reads textContent/innerHTML, href/src, class or id.
 */

/* oxlint-disable unicorn/consistent-function-scoping, unicorn/no-array-sort -- injected helpers must stay inside the serialized function; each sorted array is a fresh local copy. */

import { sanitizeDesignEvidence } from './design-evidence.js';
import { restoreScroll, runInTab, SETTLE_BUDGET_MS, settlePageForCapture } from './capture.js';

/**
 * Run inside Chrome's ISOLATED world. Keep every helper self-contained:
 * executeScript serializes this function and cannot carry module closures.
 */
function collectDesignEvidenceInPage() {
  const MAX_VISIBLE = 12_000;
  const MAX_SCANNED = 60_000;
  const MAX_CSS_RULES = 10_000;
  const MAX_CSS_DECLARATIONS = 50_000;
  const MAX_CSS_DEPTH = 12;
  const MAX_DISTINCT = 1024;
  const MAX_BYTES = 256 * 1024;
  const startedAt = performance.now();
  const cssDeadline = startedAt + 1000;
  let domDeadline = Number.POSITIVE_INFINITY;
  const LIMITS = {
    colors: 64,
    typography: 64,
    spacing: 64,
    radii: 64,
    shadows: 32,
    gradients: 32,
    componentKinds: 24,
    componentSamples: 4,
    mediaQueries: 32,
    cssVariables: 64,
  };

  const counts = () => new Map();
  const colors = counts();
  const typography = counts();
  const spacing = counts();
  const radii = counts();
  const shadows = counts();
  const gradients = counts();
  const components = new Map();
  const mediaQueries = new Set();
  const cssVariables = new Map();
  let pageBackground = null;
  let visibleCount = 0;
  let scannedCount = 0;
  let cssRuleCount = 0;
  let cssDeclarationCount = 0;
  let truncated = false;
  let colorScheme = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';

  const codePointPrefix = (value, limit) => {
    let result = '';
    let count = 0;
    for (const character of String(value ?? '')) {
      if (count >= limit) break;
      const point = character.codePointAt(0);
      result += point >= 0xd800 && point <= 0xdfff ? '\uFFFD' : character;
      count += 1;
    }
    return result;
  };

  const clampText = (value, limit) => {
    const normalized = codePointPrefix(value, Math.max(512, limit * 8))
      // oxlint-disable-next-line no-control-regex -- remove C0/C1, DEL and JS line separators.
      .replaceAll(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]+/g, ' ')
      .replaceAll(/\s+/g, ' ')
      .trim();
    return codePointPrefix(normalized, limit);
  };

  const bump = (map, key) => {
    if (!key) return;
    if (map.has(key)) {
      map.set(key, map.get(key) + 1);
    } else if (map.size < MAX_DISTINCT) {
      map.set(key, 1);
    } else {
      truncated = true;
    }
  };

  const roundQuarter = (value) => Math.round(value * 4) / 4;
  const finitePx = (value) => {
    const match = /^(-?(?:\d+\.?\d*|\.\d+))px$/i.exec(String(value || '').trim());
    if (!match) return null;
    const parsed = Number(match[1]);
    return Number.isFinite(parsed) && Math.abs(parsed) <= 16_384 ? roundQuarter(parsed) : null;
  };

  const byteHex = (value) => Math.max(0, Math.min(255, value)).toString(16).padStart(2, '0');

  /** Only computed rgb(a) and literal hex are accepted. */
  const toHex = (value) => {
    const input = String(value || '')
      .trim()
      .toLowerCase();
    if (!input || input === 'transparent') return null;
    const literal = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/.exec(input);
    if (literal) {
      const body = literal[1];
      if (body.length === 3 || body.length === 4) {
        const expanded = [...body].map((part) => part + part).join('');
        return expanded.endsWith('00') ? null : `#${expanded}`;
      }
      return body.length === 8 && body.endsWith('00') ? null : `#${body}`;
    }
    if (!input.startsWith('rgb')) return null;
    const values = input.match(/(?:\d+\.?\d*|\.\d+)%?/g);
    if (!values || values.length < 3) return null;
    const channel = (part) => {
      const number = Number.parseFloat(part);
      return part.endsWith('%') ? Math.round((number / 100) * 255) : Math.round(number);
    };
    const red = channel(values[0]);
    const green = channel(values[1]);
    const blue = channel(values[2]);
    if (![red, green, blue].every((part) => Number.isFinite(part) && part >= 0 && part <= 255)) {
      return null;
    }
    let alpha = 255;
    if (values[3] !== undefined) {
      const parsed = Number.parseFloat(values[3]);
      alpha = Math.round((values[3].endsWith('%') ? parsed / 100 : parsed) * 255);
      if (!Number.isFinite(alpha) || alpha <= 0) return null;
      alpha = Math.min(255, alpha);
    }
    return `#${byteHex(red)}${byteHex(green)}${byteHex(blue)}${alpha < 255 ? byteHex(alpha) : ''}`;
  };

  const recordColor = (value, usage) => {
    const hex = toHex(value);
    if (hex) bump(colors, `${usage}\u0000${hex}`);
    return hex;
  };

  const colorsInside = (value, usage) => {
    const input = String(value || '');
    for (const match of input.matchAll(/rgba?\([^)]*\)/gi)) recordColor(match[0], usage);
  };

  const safeCss = (value, limit) => {
    const text = clampText(value, limit);
    if (
      !text ||
      /[<>`[\]]|\/\//.test(text) ||
      /url\s*\(|(?:https?|file|chrome-extension|chrome|data|blob|javascript|vbscript|ftp|mailto):/i.test(
        text,
      )
    ) {
      return null;
    }
    return text;
  };

  const family = (value) => {
    const text = safeCss(value, 96);
    return text && /^[\p{L}\p{N}\s,'"._-]+$/u.test(text) ? text : null;
  };

  const fontWeight = (value) => {
    const parsed = Number.parseInt(value, 10);
    if (Number.isFinite(parsed)) return Math.max(1, Math.min(1000, parsed));
    return String(value).toLowerCase() === 'bold' ? 700 : 400;
  };

  const roleFor = (element) => {
    const tag = element.localName;
    const role = String(element.getAttribute('role') || '').toLowerCase();
    if (tag === 'h1') return 'display';
    if (/^h[2-6]$/.test(tag) || role === 'heading') return 'heading';
    if (['code', 'pre', 'kbd', 'samp'].includes(tag)) return 'code';
    if (
      ['button', 'input', 'select', 'textarea', 'option'].includes(tag) ||
      ['button', 'textbox', 'combobox', 'checkbox', 'radio', 'switch', 'slider'].includes(role)
    ) {
      return 'control';
    }
    if (['label', 'legend', 'small', 'dt', 'th', 'caption'].includes(tag)) return 'label';
    return 'body';
  };

  const componentKind = (element) => {
    const tag = element.localName;
    const role = String(element.getAttribute('role') || '').toLowerCase();
    const roleKinds = new Set([
      'alert',
      'button',
      'checkbox',
      'dialog',
      'list',
      'listbox',
      'menu',
      'navigation',
      'radio',
      'search',
      'slider',
      'switch',
      'tab',
      'table',
      'textbox',
      'toolbar',
    ]);
    if (roleKinds.has(role)) return role;
    if (tag === 'input') {
      const type = String(element.getAttribute('type') || 'text').toLowerCase();
      const allowed = new Set([
        'button',
        'checkbox',
        'color',
        'date',
        'email',
        'file',
        'number',
        'password',
        'radio',
        'range',
        'search',
        'submit',
        'tel',
        'text',
        'time',
        'url',
      ]);
      return `input-${allowed.has(type) ? type : 'other'}`;
    }
    const kinds = {
      a: 'link',
      article: 'article',
      aside: 'aside',
      button: 'button',
      dialog: 'dialog',
      fieldset: 'fieldset',
      footer: 'footer',
      form: 'form',
      header: 'header',
      img: 'image',
      nav: 'navigation',
      progress: 'progress',
      section: 'section',
      select: 'select',
      table: 'table',
      textarea: 'textarea',
    };
    return kinds[tag] || null;
  };

  /** Recognize a common de-identified card without consulting class or id. */
  const inferredCardKind = (element, style, rect) => {
    if (
      !['div', 'li', 'main'].includes(element.localName) ||
      element.childElementCount === 0 ||
      rect.width < 80 ||
      rect.height < 48 ||
      rect.width > 1600 ||
      rect.height > 1200 ||
      !toHex(style.backgroundColor)
    ) {
      return null;
    }
    const bordered = ['Top', 'Right', 'Bottom', 'Left'].some(
      (side) =>
        style[`border${side}Style`] !== 'none' && (finitePx(style[`border${side}Width`]) || 0) > 0,
    );
    const rounded = ['TopLeft', 'TopRight', 'BottomRight', 'BottomLeft'].some(
      (corner) => (finitePx(String(style[`border${corner}Radius`]).split(/[\s/]/, 1)[0]) || 0) > 0,
    );
    const elevated = style.boxShadow !== 'none' && Boolean(safeCss(style.boxShadow, 160));
    return bordered || rounded || elevated ? 'card' : null;
  };

  const isVisible = (style, rect) =>
    style.display !== 'none' &&
    style.visibility !== 'hidden' &&
    style.visibility !== 'collapse' &&
    Number.parseFloat(style.opacity || '1') > 0 &&
    rect.width > 0 &&
    rect.height > 0;

  const addSpacing = (property, raw) => {
    const value = finitePx(raw);
    if (value !== null && value > 0) bump(spacing, `${property}\u0000${value}`);
  };

  const addRadius = (raw, rect, allowZero) => {
    const first = String(raw || '').split(/[\s/]/, 1)[0];
    const value = finitePx(first);
    if (value === null) return;
    const rounded = Math.max(0, Math.round(value));
    if (rounded === 0) {
      if (allowZero) bump(radii, '0');
      return;
    }
    // Fully rounded avatars and pill controls are component geometry, not a
    // reusable corner-radius token. Their sample still retains the radius.
    const shortSide = Math.min(rect.width, rect.height);
    if (shortSide > 0 && value >= shortSide * 0.45) return;
    bump(radii, String(rounded));
  };

  const componentSample = (style, rect) => {
    const sample = {};
    const background = toHex(style.backgroundColor);
    const color = toHex(style.color);
    const font = family(style.fontFamily);
    const size = finitePx(style.fontSize);
    const lineHeight = finitePx(style.lineHeight);
    const padding = safeCss(
      [style.paddingTop, style.paddingRight, style.paddingBottom, style.paddingLeft].join(' '),
      64,
    );
    const gap = finitePx(style.gap) ?? finitePx(style.columnGap) ?? finitePx(style.rowGap);
    const radius = finitePx(String(style.borderTopLeftRadius || '').split(/[\s/]/, 1)[0]);
    const border = safeCss(
      `${style.borderTopWidth} ${style.borderTopStyle} ${toHex(style.borderTopColor) || ''}`,
      96,
    );
    const shadow = style.boxShadow === 'none' ? null : safeCss(style.boxShadow, 160);
    if (background) sample.background = background;
    if (color) sample.color = color;
    if (font) sample.fontFamily = font;
    if (size !== null) sample.fontSize = size;
    sample.fontWeight = fontWeight(style.fontWeight);
    sample.lineHeight = lineHeight;
    if (padding) sample.padding = padding;
    if (gap !== null && gap >= 0) sample.gap = gap;
    if (radius !== null && radius > 0) sample.radius = Math.round(radius);
    if (border && style.borderTopStyle !== 'none' && style.borderTopWidth !== '0px') {
      sample.border = border;
    }
    if (shadow) sample.shadow = shadow;
    sample.width = Math.min(16_384, Math.max(1, Math.round(rect.width)));
    sample.height = Math.min(16_384, Math.max(1, Math.round(rect.height)));
    return sample;
  };

  const recordComponent = (kind, style, rect) => {
    if (!kind) return;
    let entry = components.get(kind);
    if (!entry) {
      entry = { count: 0, samples: new Map() };
      components.set(kind, entry);
    }
    entry.count += 1;
    const sample = componentSample(style, rect);
    const key = JSON.stringify(sample);
    if (!entry.samples.has(key) && entry.samples.size < LIMITS.componentSamples) {
      entry.samples.set(key, sample);
    }
  };

  /** Read only same-origin/readable rule lists. Cross-origin sheets throw. */
  const walkRules = (rules, depth) => {
    if (!rules) return;
    if (depth > MAX_CSS_DEPTH) {
      truncated = true;
      return;
    }
    for (const rule of rules) {
      if (cssRuleCount >= MAX_CSS_RULES || performance.now() >= cssDeadline) {
        truncated = true;
        return;
      }
      cssRuleCount += 1;
      const mediaText = rule.media && safeCss(rule.media.mediaText, 160);
      if (mediaText) {
        if (mediaQueries.size < 256 || mediaQueries.has(mediaText)) mediaQueries.add(mediaText);
        else truncated = true;
      }
      const declaration = rule.style;
      if (declaration) {
        for (let index = 0; index < declaration.length; index += 1) {
          if (cssDeclarationCount >= MAX_CSS_DECLARATIONS || performance.now() >= cssDeadline) {
            truncated = true;
            return;
          }
          cssDeclarationCount += 1;
          const name = declaration[index];
          if (!/^--[A-Za-z][A-Za-z0-9_-]{0,61}$/.test(name)) continue;
          const raw = clampText(declaration.getPropertyValue(name), 120);
          if (
            !raw ||
            /[<>`[\]]|\/\//.test(raw) ||
            /url\s*\(|(?:https?|file|chrome-extension|chrome|data|blob|javascript|vbscript|ftp|mailto):/i.test(
              raw,
            )
          ) {
            continue;
          }
          const color = toHex(raw);
          if (color) {
            const key = `${name}\u0000${color}`;
            if (cssVariables.size < 512 || cssVariables.has(key)) {
              cssVariables.set(key, { name, value: color, kind: 'color' });
            } else truncated = true;
            continue;
          }
          if (
            /^-?(?:\d+\.?\d*|\.\d+)(?:px|rem|em|%|vw|vh|vmin|vmax|ch|ex)(?:\s+-?(?:\d+\.?\d*|\.\d+)(?:px|rem|em|%|vw|vh|vmin|vmax|ch|ex)){0,3}$/i.test(
              raw,
            )
          ) {
            const key = `${name}\u0000${raw}`;
            if (cssVariables.size < 512 || cssVariables.has(key)) {
              cssVariables.set(key, { name, value: raw, kind: 'length' });
            } else truncated = true;
            continue;
          }
          if (
            /(?:font|type|family)/i.test(name) &&
            /^[\p{L}\p{N}\s,'"._-]+$/u.test(raw) &&
            /\p{L}/u.test(raw)
          ) {
            const key = `${name}\u0000${raw}`;
            if (cssVariables.size < 512 || cssVariables.has(key)) {
              cssVariables.set(key, { name, value: raw, kind: 'font' });
            } else truncated = true;
          }
        }
      }
      try {
        if (rule.cssRules) {
          walkRules(rule.cssRules, depth + 1);
        }
      } catch {
        // Nested imported sheets can be unreadable even when the parent is.
      }
    }
  };

  for (const sheet of document.styleSheets) {
    if (cssRuleCount >= MAX_CSS_RULES || performance.now() >= cssDeadline) {
      truncated = true;
      break;
    }
    try {
      walkRules(sheet.cssRules, 0);
    } catch {
      // Cross-origin stylesheets are intentionally skipped; no new host
      // permission is requested to inspect them.
    }
  }

  domDeadline = performance.now() + 5000;

  const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_ELEMENT);
  let element = walker.currentNode;
  while (
    element &&
    scannedCount < MAX_SCANNED &&
    visibleCount < MAX_VISIBLE &&
    performance.now() < domDeadline
  ) {
    scannedCount += 1;
    let style;
    let rect;
    try {
      style = getComputedStyle(element);
      rect = element.getBoundingClientRect();
    } catch {
      element = walker.nextNode();
      continue;
    }
    if (isVisible(style, rect)) {
      visibleCount += 1;
      const kind = componentKind(element) || inferredCardKind(element, style, rect);
      const textRole = roleFor(element);
      const mayCarryTypography =
        textRole !== 'body' ||
        (element.childElementCount === 0 &&
          !['audio', 'canvas', 'img', 'svg', 'video'].includes(element.localName));
      const background = recordColor(style.backgroundColor, 'background');
      if (mayCarryTypography) recordColor(style.color, 'text');
      for (const side of ['Top', 'Right', 'Bottom', 'Left']) {
        if (
          style[`border${side}Style`] !== 'none' &&
          (finitePx(style[`border${side}Width`]) || 0) > 0
        ) {
          recordColor(style[`border${side}Color`], 'border');
        }
        addSpacing('margin', style[`margin${side}`]);
        addSpacing('padding', style[`padding${side}`]);
      }
      for (const corner of ['TopLeft', 'TopRight', 'BottomRight', 'BottomLeft']) {
        addRadius(style[`border${corner}Radius`], rect, Boolean(kind && kind !== 'image'));
      }
      addSpacing('gap', style.rowGap);
      addSpacing('gap', style.columnGap);

      const shadow = style.boxShadow === 'none' ? null : safeCss(style.boxShadow, 160);
      if (shadow) {
        bump(shadows, shadow);
        colorsInside(shadow, 'shadow');
      }
      const backgroundImage = safeCss(style.backgroundImage, 200);
      if (
        backgroundImage &&
        /^(?:(?:repeating-)?(?:linear|radial|conic)-gradient)\(/i.test(backgroundImage)
      ) {
        bump(gradients, backgroundImage);
        colorsInside(backgroundImage, 'gradient');
      }

      const font = family(style.fontFamily);
      const size = finitePx(style.fontSize);
      if (font && size !== null && size >= 1 && mayCarryTypography) {
        const measuredLineHeight = finitePx(style.lineHeight);
        const lineHeight =
          measuredLineHeight !== null && measuredLineHeight >= 1 ? measuredLineHeight : null;
        const record = {
          role: textRole,
          family: font,
          size,
          weight: fontWeight(style.fontWeight),
          lineHeight,
        };
        bump(typography, JSON.stringify(record));
      }

      recordComponent(kind, style, rect);
      if (element === document.documentElement) {
        if (background) pageBackground = background;
        if (style.colorScheme === 'dark') colorScheme = 'dark';
        if (style.colorScheme === 'light') colorScheme = 'light';
      }
      if (element === document.body && !pageBackground && background) pageBackground = background;
    }
    element = walker.nextNode();
  }
  if (
    element ||
    scannedCount >= MAX_SCANNED ||
    visibleCount >= MAX_VISIBLE ||
    performance.now() >= domDeadline
  ) {
    truncated = true;
  }

  const counted = (map, decode, limit) =>
    [...map]
      .map(([key, count]) => ({ ...decode(key), count }))
      .sort(
        (left, right) =>
          right.count - left.count || JSON.stringify(left).localeCompare(JSON.stringify(right)),
      )
      .slice(0, limit);

  const evidence = {
    version: 1,
    title: safeCss(document.title, 120) || '',
    viewport: {
      width: Math.max(0, Math.round(window.innerWidth)),
      height: Math.max(0, Math.round(window.innerHeight)),
      dpr: Math.max(0.1, Math.min(8, roundQuarter(window.devicePixelRatio || 1))),
    },
    pageBackground,
    colorScheme,
    colors: counted(
      colors,
      (key) => {
        const [usage, value] = key.split('\u0000');
        return { value, usage };
      },
      LIMITS.colors,
    ),
    typography: counted(typography, (key) => JSON.parse(key), LIMITS.typography),
    spacing: counted(
      spacing,
      (key) => {
        const [property, value] = key.split('\u0000');
        return { property, value: Number(value) };
      },
      LIMITS.spacing,
    ),
    radii: counted(radii, (key) => ({ value: Number(key) }), LIMITS.radii),
    shadows: counted(shadows, (value) => ({ value }), LIMITS.shadows),
    components: [...components]
      .map(([kind, entry]) => ({
        kind,
        count: entry.count,
        samples: [...entry.samples.values()].sort((left, right) =>
          JSON.stringify(left).localeCompare(JSON.stringify(right)),
        ),
      }))
      .sort((left, right) => right.count - left.count || left.kind.localeCompare(right.kind))
      .slice(0, LIMITS.componentKinds),
    gradients: counted(gradients, (value) => ({ value }), LIMITS.gradients),
    mediaQueries: [...mediaQueries].sort().slice(0, LIMITS.mediaQueries),
    cssVariables: [...cssVariables.values()]
      .sort(
        (left, right) =>
          left.name.localeCompare(right.name) || left.value.localeCompare(right.value),
      )
      .slice(0, LIMITS.cssVariables),
    elementCount: visibleCount,
    truncated,
  };

  // The static caps above keep ordinary evidence far below 256 KiB. This is
  // the non-negotiable final gate for pathological font/shadow/token sets.
  const size = () => new TextEncoder().encode(JSON.stringify(evidence)).byteLength;
  const trimOrder = [
    evidence.cssVariables,
    evidence.mediaQueries,
    evidence.components,
    evidence.gradients,
    evidence.shadows,
    evidence.typography,
    evidence.spacing,
    evidence.colors,
    evidence.radii,
  ];
  while (size() > MAX_BYTES) {
    const target = trimOrder.find((items) => items.length > 0);
    if (!target) break;
    target.pop();
    evidence.truncated = true;
  }
  return evidence;
}

/** Remove an abandoned picker before design statistics inspect page styles. */
function clearPickerForDesign() {
  const pick = globalThis.openpencilPick;
  if (pick && typeof pick.teardown === 'function') pick.teardown();
  delete globalThis.openpencilSnapshotOptions;
  return true;
}

/**
 * Collect evidence from the top frame and reconstruct it through the strict
 * transport schema before it can leave the tab process.
 */
export async function captureDesignEvidence(tabId) {
  await runInTab(tabId, { func: clearPickerForDesign }).catch(() => undefined);
  const scrollBack = await runInTab(tabId, {
    func: settlePageForCapture,
    args: [SETTLE_BUDGET_MS],
  }).catch(() => null);
  try {
    const raw = await runInTab(tabId, { func: collectDesignEvidenceInPage });
    if (!raw || typeof raw !== 'object') {
      const error = new Error('design evidence was not returned');
      error.code = 'designCapture';
      throw error;
    }
    return sanitizeDesignEvidence(raw);
  } finally {
    if (scrollBack) {
      await runInTab(tabId, { func: restoreScroll, args: [scrollBack] }).catch(() => undefined);
    }
  }
}
