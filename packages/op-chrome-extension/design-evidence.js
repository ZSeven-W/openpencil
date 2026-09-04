/** Strict, pure reconstruction of page-produced design evidence. */

/* oxlint-disable unicorn/no-array-sort -- these arrays are fresh bounded copies, sorted in place deliberately. */

export const MAX_DESIGN_EVIDENCE_BYTES = 256 * 1024;

const COLOR = /^#[0-9a-f]{6}(?:[0-9a-f]{2})?$/;
const USAGES = new Set(['text', 'background', 'border', 'shadow', 'gradient']);
const ROLES = new Set(['display', 'heading', 'body', 'label', 'control', 'code']);
const SPACING = new Set(['margin', 'padding', 'gap']);
const VARIABLE_KINDS = new Set(['color', 'length', 'font']);
const PROMPT_DIRECTIVES = [
  'ignore previous',
  'ignore prior',
  'disregard',
  'forget previous',
  'system prompt',
  'developer message',
  'follow these instructions',
  'new instructions',
  'override instructions',
  'you are now',
  'act as',
  'assistant:',
  'developer:',
];

function codePointPrefix(value, limit) {
  let result = '';
  let count = 0;
  for (const character of String(value ?? '')) {
    if (count >= limit) break;
    const point = character.codePointAt(0);
    result += point >= 0xd800 && point <= 0xdfff ? '\uFFFD' : character;
    count += 1;
  }
  return result;
}

function text(value, limit) {
  const normalized = codePointPrefix(value, Math.max(512, limit * 8))
    // oxlint-disable-next-line no-control-regex -- remove C0/C1, DEL and JS line separators.
    .replaceAll(/[\u0000-\u001f\u007f-\u009f\u2028\u2029]+/g, ' ')
    .replaceAll(/\s+/g, ' ')
    .trim();
  return codePointPrefix(normalized, limit);
}

function number(value, minimum, maximum, fallback) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? Math.max(minimum, Math.min(maximum, parsed)) : fallback;
}

function integer(value, minimum, maximum, fallback = minimum) {
  return Math.round(number(value, minimum, maximum, fallback));
}

function color(value) {
  const normalized = text(value, 9).toLowerCase();
  return COLOR.test(normalized) ? normalized : null;
}

function safeCss(value, limit) {
  const normalized = text(value, limit);
  return normalized &&
    !/[<>`[\]]|\/\//.test(normalized) &&
    !/url\s*\(|(?:https?|file|chrome-extension|chrome|data|blob|javascript|vbscript|ftp|mailto):/i.test(
      normalized,
    )
    ? normalized
    : null;
}

function networkStringAllowed(value) {
  const lower = String(value).toLowerCase();
  return !PROMPT_DIRECTIVES.some((directive) => lower.includes(directive));
}

function networkRecordAllowed(record) {
  return Object.values(record).every(
    (value) => typeof value !== 'string' || networkStringAllowed(value),
  );
}

function counted(items, limit, convert) {
  if (!Array.isArray(items)) return [];
  return items
    .slice(0, limit)
    .map((item) => convert(item && typeof item === 'object' ? item : {}))
    .filter(Boolean)
    .sort(
      (left, right) =>
        right.count - left.count || JSON.stringify(left).localeCompare(JSON.stringify(right)),
    );
}

function sample(raw) {
  const result = {};
  const background = color(raw.background);
  const foreground = color(raw.color);
  const fontFamily = safeCss(raw.fontFamily, 96);
  const padding = safeCss(raw.padding, 64);
  const border = safeCss(raw.border, 96);
  const shadow = safeCss(raw.shadow, 160);
  if (background) result.background = background;
  if (foreground) result.color = foreground;
  if (fontFamily) result.fontFamily = fontFamily;
  if (Number(raw.fontSize) >= 1) result.fontSize = number(raw.fontSize, 1, 512, 1);
  result.fontWeight = integer(raw.fontWeight, 1, 1000, 400);
  result.lineHeight = Number(raw.lineHeight) >= 1 ? number(raw.lineHeight, 1, 2048, 1) : null;
  if (padding) result.padding = padding;
  if (Number.isFinite(Number(raw.gap))) result.gap = number(raw.gap, 0, 16_384, 0);
  if (Number.isFinite(Number(raw.radius))) result.radius = integer(raw.radius, 0, 16_384);
  if (border) result.border = border;
  if (shadow) result.shadow = shadow;
  result.width = integer(raw.width, 1, 16_384, 1);
  result.height = integer(raw.height, 1, 16_384, 1);
  return result;
}

/**
 * Only named schema fields are copied. Unknown keys such as text, href, src,
 * class, id or html cannot cross this boundary even if page code interferes.
 */
export function sanitizeDesignEvidence(raw) {
  const viewport = raw.viewport && typeof raw.viewport === 'object' ? raw.viewport : {};
  const evidence = {
    version: 1,
    title: safeCss(raw.title, 120) || '',
    viewport: {
      width: integer(viewport.width, 1, 100_000, 1),
      height: integer(viewport.height, 1, 100_000, 1),
      dpr: number(viewport.dpr, 0.1, 8, 1),
    },
    pageBackground: color(raw.pageBackground),
    colorScheme: ['dark', 'light'].includes(text(raw.colorScheme, 16))
      ? text(raw.colorScheme, 16)
      : null,
    colors: counted(raw.colors, 64, (item) => {
      const value = color(item.value);
      const usage = text(item.usage, 12);
      return value && USAGES.has(usage)
        ? { value, usage, count: integer(item.count, 1, 12_000, 1) }
        : null;
    }),
    typography: counted(raw.typography, 64, (item) => {
      const role = text(item.role, 12);
      const family = safeCss(item.family, 96);
      if (!ROLES.has(role) || !family || Number(item.size) < 1) return null;
      return {
        role,
        family,
        size: number(item.size, 1, 512, 1),
        weight: integer(item.weight, 1, 1000, 400),
        lineHeight: Number(item.lineHeight) >= 1 ? number(item.lineHeight, 1, 2048, 1) : null,
        count: integer(item.count, 1, 12_000, 1),
      };
    }),
    spacing: counted(raw.spacing, 64, (item) => {
      const property = text(item.property, 12);
      return SPACING.has(property)
        ? {
            property,
            value: number(item.value, 0, 16_384, 0),
            count: integer(item.count, 1, 48_000, 1),
          }
        : null;
    }),
    radii: counted(raw.radii, 64, (item) => ({
      value: integer(item.value, 0, 16_384),
      count: integer(item.count, 1, 48_000, 1),
    })),
    shadows: counted(raw.shadows, 32, (item) => {
      const value = safeCss(item.value, 160);
      return value ? { value, count: integer(item.count, 1, 12_000, 1) } : null;
    }),
    components: Array.isArray(raw.components)
      ? raw.components
          .slice(0, 24)
          .map((item) => {
            const kind = text(item && item.kind, 32).toLowerCase();
            if (!/^[a-z][a-z0-9-]{0,31}$/.test(kind)) return null;
            return {
              kind,
              count: integer(item.count, 1, 12_000, 1),
              samples: Array.isArray(item.samples)
                ? item.samples
                    .slice(0, 4)
                    .map(sample)
                    .sort((left, right) =>
                      JSON.stringify(left).localeCompare(JSON.stringify(right)),
                    )
                : [],
            };
          })
          .filter(Boolean)
          .sort((left, right) => right.count - left.count || left.kind.localeCompare(right.kind))
      : [],
    gradients: counted(raw.gradients, 32, (item) => {
      const value = safeCss(item.value, 200);
      return value && /^(?:(?:repeating-)?(?:linear|radial|conic)-gradient)\(/i.test(value)
        ? { value, count: integer(item.count, 1, 12_000, 1) }
        : null;
    }),
    mediaQueries: Array.isArray(raw.mediaQueries)
      ? [...new Set(raw.mediaQueries.map((value) => safeCss(value, 160)).filter(Boolean))]
          .sort()
          .slice(0, 32)
      : [],
    cssVariables: Array.isArray(raw.cssVariables)
      ? raw.cssVariables
          .slice(0, 64)
          .map((item) => {
            const name = text(item && item.name, 64);
            const value = safeCss(item && item.value, 120);
            const kind = text(item && item.kind, 8);
            if (
              !/^--[A-Za-z][A-Za-z0-9_-]{0,61}$/.test(name) ||
              !value ||
              !VARIABLE_KINDS.has(kind)
            ) {
              return null;
            }
            return { name, value, kind };
          })
          .filter(Boolean)
          .sort(
            (left, right) =>
              left.name.localeCompare(right.name) || left.value.localeCompare(right.value),
          )
      : [],
    elementCount: integer(raw.elementCount, 0, 12_000),
    truncated: Boolean(raw.truncated),
  };

  const byteLength = () => new TextEncoder().encode(JSON.stringify(evidence)).byteLength;
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
  while (byteLength() > MAX_DESIGN_EVIDENCE_BYTES) {
    const target = trimOrder.find((items) => items.length > 0);
    if (!target) throw new Error('design evidence exceeds its hard byte limit');
    target.pop();
    evidence.truncated = true;
  }
  return evidence;
}

/**
 * Network evidence never carries the page title. The collector retains its
 * bounded title only so the wasm fallback can name a guide without inventing
 * one; OpenPencil and the configured provider receive no page text at all.
 */
export function designEvidenceForNetwork(evidence) {
  const network = { ...sanitizeDesignEvidence(evidence), title: '' };
  network.typography = network.typography.filter(networkRecordAllowed);
  network.shadows = network.shadows.filter(networkRecordAllowed);
  network.gradients = network.gradients.filter(networkRecordAllowed);
  network.mediaQueries = network.mediaQueries.filter(networkStringAllowed);
  network.cssVariables = network.cssVariables.filter(networkRecordAllowed);
  network.components = network.components.map((component) => ({
    ...component,
    samples: component.samples.filter(networkRecordAllowed),
  }));
  return network;
}
