import { openDocument, saveDocument, resolveDocPath } from '../document-manager';
import {
  findNodeInTree,
  flattenNodes,
  getDocChildren,
  setDocChildren,
} from '../utils/node-operations';
import type { PenNode } from '@zseven-w/pen-types';
import type { PenFill, PenStroke } from '@zseven-w/pen-types';

// ---------------------------------------------------------------------------
// Property helpers
// ---------------------------------------------------------------------------

type PropertyName =
  | 'fillColor'
  | 'textColor'
  | 'strokeColor'
  | 'strokeThickness'
  | 'cornerRadius'
  | 'padding'
  | 'gap'
  | 'fontSize'
  | 'fontFamily'
  | 'fontWeight';

const PROPERTY_ENUM: PropertyName[] = [
  'fillColor',
  'textColor',
  'strokeColor',
  'strokeThickness',
  'cornerRadius',
  'padding',
  'gap',
  'fontSize',
  'fontFamily',
  'fontWeight',
];

/** Extract the first solid fill color from a fill value (PenFill[] or string). */
function extractFillColor(fill: unknown): string | undefined {
  if (typeof fill === 'string') return fill;
  if (Array.isArray(fill)) {
    for (const f of fill as PenFill[]) {
      if (f.type === 'solid') return f.color;
    }
  }
  return undefined;
}

/** Get the value of a property from a node. Returns undefined if not present. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function getPropertyValue(node: PenNode, prop: PropertyName): any {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const n = node as any;
  switch (prop) {
    case 'fillColor':
      // For text nodes, fillColor and textColor are the same; skip text here
      if (node.type === 'text') return undefined;
      return extractFillColor(n.fill);
    case 'textColor':
      if (node.type !== 'text') return undefined;
      return extractFillColor(n.fill);
    case 'strokeColor': {
      const stroke = n.stroke as PenStroke | undefined;
      if (!stroke?.fill) return undefined;
      return extractFillColor(stroke.fill);
    }
    case 'strokeThickness': {
      const stroke = n.stroke as PenStroke | undefined;
      return stroke?.thickness;
    }
    case 'cornerRadius':
      return n.cornerRadius;
    case 'padding':
      return n.padding;
    case 'gap':
      return n.gap;
    case 'fontSize':
      return n.fontSize;
    case 'fontFamily':
      return n.fontFamily;
    case 'fontWeight':
      return n.fontWeight;
    default:
      return undefined;
  }
}

/**
 * Compare two values for equality. Handles arrays (cornerRadius, padding)
 * and primitives.
 */
function valuesEqual(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    return a.every((v, i) => v === b[i]);
  }
  return false;
}

/**
 * Serialize a value for use as a unique key in a Set/Map.
 * Arrays are JSON-stringified so [1,2] and [1,2] are treated as equal.
 */
function serializeValue(v: unknown): string {
  if (v === undefined || v === null) return String(v);
  if (Array.isArray(v)) return JSON.stringify(v);
  return String(v);
}

// ---------------------------------------------------------------------------
// Replace helpers
// ---------------------------------------------------------------------------

/**
 * Replace fill color in a fill value. Mutates the fill array in-place if the
 * first solid fill matches `from`.
 */
function replaceFillColor(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  node: any,
  fieldName: 'fill' | 'stroke.fill',
  from: string,
  to: string,
): boolean {
  const fillHolder = fieldName === 'fill' ? node : node.stroke;
  if (!fillHolder) return false;
  const fill = fillHolder.fill;
  if (typeof fill === 'string') {
    if (fill === from) {
      fillHolder.fill = to;
      return true;
    }
    return false;
  }
  if (Array.isArray(fill)) {
    let replaced = false;
    for (const f of fill) {
      if (f.type === 'solid' && f.color === from) {
        f.color = to;
        replaced = true;
      }
    }
    return replaced;
  }
  return false;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type ReplacementRule = { from: any; to: any };

/**
 * Apply a single property replacement on a node. Returns true if replaced.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function applyReplacement(node: PenNode, prop: PropertyName, rule: ReplacementRule): boolean {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const n = node as any;
  switch (prop) {
    case 'fillColor':
      if (node.type === 'text') return false;
      return replaceFillColor(n, 'fill', rule.from, rule.to);
    case 'textColor':
      if (node.type !== 'text') return false;
      return replaceFillColor(n, 'fill', rule.from, rule.to);
    case 'strokeColor':
      if (!n.stroke) return false;
      return replaceFillColor(n, 'stroke.fill', rule.from, rule.to);
    case 'strokeThickness': {
      if (!n.stroke) return false;
      if (valuesEqual(n.stroke.thickness, rule.from)) {
        n.stroke.thickness = rule.to;
        return true;
      }
      return false;
    }
    case 'cornerRadius':
      if (valuesEqual(n.cornerRadius, rule.from)) {
        n.cornerRadius = rule.to;
        return true;
      }
      return false;
    case 'padding':
      if (valuesEqual(n.padding, rule.from)) {
        n.padding = rule.to;
        return true;
      }
      return false;
    case 'gap':
      if (valuesEqual(n.gap, rule.from)) {
        n.gap = rule.to;
        return true;
      }
      return false;
    case 'fontSize':
      if (n.fontSize === rule.from) {
        n.fontSize = rule.to;
        return true;
      }
      return false;
    case 'fontFamily':
      if (n.fontFamily === rule.from) {
        n.fontFamily = rule.to;
        return true;
      }
      return false;
    case 'fontWeight':
      // fontWeight can be number or string; compare loosely
      if (String(n.fontWeight) === String(rule.from)) {
        n.fontWeight = rule.to;
        return true;
      }
      return false;
    default:
      return false;
  }
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

export const STYLE_OPS_TOOL_DEFINITIONS = [
  {
    name: 'search_all_unique_properties',
    description:
      'Recursively search for all unique property values on the node tree under provided parent IDs.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        parents: {
          type: 'array',
          items: { type: 'string' },
          description: 'IDs of parent nodes to search',
        },
        properties: {
          type: 'array',
          items: {
            type: 'string',
            enum: PROPERTY_ENUM,
          },
          description:
            'List of properties to search. Values: fillColor, textColor, strokeColor, strokeThickness, cornerRadius, padding, gap, fontSize, fontFamily, fontWeight',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['parents', 'properties'],
    },
  },
  {
    name: 'replace_all_matching_properties',
    description:
      'Recursively replace all matching property values on the node tree under provided parent IDs.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        parents: {
          type: 'array',
          items: { type: 'string' },
          description: 'IDs of parent nodes to search',
        },
        properties: {
          type: 'object',
          description:
            'Replacement rules per property. Each key maps to an array of {from, to} objects.',
          properties: {
            fillColor: {
              type: 'array',
              items: {
                type: 'object',
                properties: { from: { type: 'string' }, to: { type: 'string' } },
                required: ['from', 'to'],
              },
            },
            textColor: {
              type: 'array',
              items: {
                type: 'object',
                properties: { from: { type: 'string' }, to: { type: 'string' } },
                required: ['from', 'to'],
              },
            },
            strokeColor: {
              type: 'array',
              items: {
                type: 'object',
                properties: { from: { type: 'string' }, to: { type: 'string' } },
                required: ['from', 'to'],
              },
            },
            fontFamily: {
              type: 'array',
              items: {
                type: 'object',
                properties: { from: { type: 'string' }, to: { type: 'string' } },
                required: ['from', 'to'],
              },
            },
            fontSize: {
              type: 'array',
              items: {
                type: 'object',
                properties: { from: { type: 'number' }, to: { type: 'number' } },
                required: ['from', 'to'],
              },
            },
            fontWeight: {
              type: 'array',
              items: {
                type: 'object',
                properties: { from: { type: 'string' }, to: { type: 'string' } },
                required: ['from', 'to'],
              },
            },
            cornerRadius: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  from: {
                    oneOf: [{ type: 'number' }, { type: 'array', items: { type: 'number' } }],
                  },
                  to: { oneOf: [{ type: 'number' }, { type: 'array', items: { type: 'number' } }] },
                },
                required: ['from', 'to'],
              },
            },
            gap: {
              type: 'array',
              items: {
                type: 'object',
                properties: { from: { type: 'number' }, to: { type: 'number' } },
                required: ['from', 'to'],
              },
            },
            padding: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  from: {
                    oneOf: [{ type: 'number' }, { type: 'array', items: { type: 'number' } }],
                  },
                  to: { oneOf: [{ type: 'number' }, { type: 'array', items: { type: 'number' } }] },
                },
                required: ['from', 'to'],
              },
            },
            strokeThickness: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  from: {
                    oneOf: [{ type: 'number' }, { type: 'array', items: { type: 'number' } }],
                  },
                  to: { oneOf: [{ type: 'number' }, { type: 'array', items: { type: 'number' } }] },
                },
                required: ['from', 'to'],
              },
            },
          },
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['parents', 'properties'],
    },
  },
];

export const STYLE_OPS_TOOL_NAMES = new Set([
  'search_all_unique_properties',
  'replace_all_matching_properties',
]);

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/**
 * Collect all descendant nodes (including the parent itself) for the given
 * parent IDs.
 */
function collectDescendants(children: PenNode[], parentIds: string[]): PenNode[] {
  const result: PenNode[] = [];
  for (const pid of parentIds) {
    const parent = findNodeInTree(children, pid);
    if (!parent) continue;
    // Include parent and all its descendants
    result.push(...flattenNodes([parent]));
  }
  return result;
}

export async function handleStyleOpsToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const a = args as any;

  switch (name) {
    case 'search_all_unique_properties': {
      const filePath = resolveDocPath(a.filePath);
      const doc = await openDocument(filePath);
      const children = getDocChildren(doc, a.pageId);
      const nodes = collectDescendants(children, a.parents as string[]);
      const properties = a.properties as PropertyName[];

      const result: Record<string, unknown[]> = {};
      for (const prop of properties) {
        const seen = new Set<string>();
        const values: unknown[] = [];
        for (const node of nodes) {
          const val = getPropertyValue(node, prop);
          if (val === undefined || val === null) continue;
          const key = serializeValue(val);
          if (!seen.has(key)) {
            seen.add(key);
            values.push(val);
          }
        }
        result[prop] = values;
      }
      return JSON.stringify(result, null, 2);
    }

    case 'replace_all_matching_properties': {
      const filePath = resolveDocPath(a.filePath);
      const doc = await openDocument(filePath);
      const children = getDocChildren(doc, a.pageId);
      const nodes = collectDescendants(children, a.parents as string[]);
      const propRules = a.properties as Record<string, ReplacementRule[]>;

      let replacedCount = 0;
      for (const node of nodes) {
        for (const [prop, rules] of Object.entries(propRules)) {
          if (!Array.isArray(rules)) continue;
          for (const rule of rules) {
            if (applyReplacement(node, prop as PropertyName, rule)) {
              replacedCount++;
            }
          }
        }
      }

      // Persist changes
      setDocChildren(doc, children, a.pageId);
      await saveDocument(filePath, doc);

      return JSON.stringify({ replacedCount }, null, 2);
    }

    default:
      return '';
  }
}
