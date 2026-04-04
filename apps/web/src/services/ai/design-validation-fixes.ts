/**
 * Validation fix application — applies property and structural fixes
 * returned by the vision validator to the document store.
 */

import { nanoid } from 'nanoid';
import { useDocumentStore } from '@/stores/document-store';
import { lookupIconByName } from './icon-resolver';
import type { PenNode } from '@/types/pen';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ValidationFix {
  nodeId: string;
  property: string;
  value: number | string | number[];
}

export interface StructuralAddChildFix {
  action: 'addChild';
  parentId: string;
  index?: number;
  node: {
    type: 'frame' | 'text' | 'path' | 'rectangle' | 'ellipse';
    name?: string;
    width?: number | string;
    height?: number | string;
    fillColor?: string;
    content?: string;
    fontSize?: number;
    fontWeight?: number;
    layout?: string;
    gap?: number;
    padding?: number | number[];
    cornerRadius?: number;
    alignItems?: string;
    justifyContent?: string;
  };
}

export interface StructuralRemoveNodeFix {
  action: 'removeNode';
  nodeId: string;
}

export type StructuralFix = StructuralAddChildFix | StructuralRemoveNodeFix;

export interface ValidationResult {
  issues: string[];
  fixes: ValidationFix[];
  structuralFixes: StructuralFix[];
  qualityScore: number;
  skipped?: boolean;
}

// ---------------------------------------------------------------------------
// Safe fix constants
// ---------------------------------------------------------------------------

export const SAFE_FIX_PROPERTIES: Record<
  string,
  | 'number'
  | 'sizing'
  | 'number_or_array'
  | 'enum_align'
  | 'enum_justify'
  | 'enum_text_align'
  | 'color'
  | 'font_weight'
  | 'enum_text_growth'
> = {
  width: 'sizing',
  height: 'sizing',
  padding: 'number_or_array',
  gap: 'number',
  fontSize: 'number',
  fontWeight: 'font_weight',
  letterSpacing: 'number',
  lineHeight: 'number',
  cornerRadius: 'number',
  opacity: 'number',
  fillColor: 'color',
  strokeColor: 'color',
  strokeWidth: 'number',
  textAlign: 'enum_text_align',
  textGrowth: 'enum_text_growth',
  alignItems: 'enum_align',
  justifyContent: 'enum_justify',
};

const VALID_SIZING_STRINGS = new Set(['fill_container', 'fit_content']);
const VALID_ALIGN = new Set(['start', 'center', 'end']);
const VALID_JUSTIFY = new Set(['start', 'center', 'end', 'space_between', 'space_around']);
const VALID_HEX_COLOR = /^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;
const VALID_FONT_WEIGHTS = new Set([100, 200, 300, 400, 500, 600, 700, 800, 900]);
const VALID_TEXT_ALIGN = new Set(['left', 'center', 'right']);
const VALID_TEXT_GROWTH = new Set(['auto', 'fixed-width', 'fixed-width-height']);

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

export function isValidFixValue(property: string, value: unknown): boolean {
  const type = SAFE_FIX_PROPERTIES[property];
  if (!type) return false;

  switch (type) {
    case 'number':
      return typeof value === 'number';
    case 'sizing':
      return (
        typeof value === 'number' || (typeof value === 'string' && VALID_SIZING_STRINGS.has(value))
      );
    case 'number_or_array':
      return (
        typeof value === 'number' ||
        (Array.isArray(value) && value.every((v) => typeof v === 'number'))
      );
    case 'enum_align':
      return typeof value === 'string' && VALID_ALIGN.has(value);
    case 'enum_justify':
      return typeof value === 'string' && VALID_JUSTIFY.has(value);
    case 'color':
      return typeof value === 'string' && VALID_HEX_COLOR.test(value);
    case 'font_weight':
      return typeof value === 'number' && VALID_FONT_WEIGHTS.has(value);
    case 'enum_text_align':
      return typeof value === 'string' && VALID_TEXT_ALIGN.has(value);
    case 'enum_text_growth':
      return typeof value === 'string' && VALID_TEXT_GROWTH.has(value);
    default:
      return false;
  }
}

export function isValidStructuralFix(fix: unknown): fix is StructuralFix {
  if (!fix || typeof fix !== 'object') return false;
  const f = fix as Record<string, unknown>;
  if (f.action === 'addChild') {
    if (typeof f.parentId !== 'string' || !f.parentId) return false;
    if (!f.node || typeof f.node !== 'object') return false;
    const node = f.node as Record<string, unknown>;
    const validTypes = new Set(['frame', 'text', 'path', 'rectangle', 'ellipse']);
    return typeof node.type === 'string' && validTypes.has(node.type);
  }
  if (f.action === 'removeNode') {
    return typeof f.nodeId === 'string' && !!f.nodeId;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Apply fixes
// ---------------------------------------------------------------------------

export async function applyValidationFixes(result: ValidationResult): Promise<number> {
  const hasFixes = result.fixes.length > 0;
  const hasStructural = result.structuralFixes.length > 0;
  if (!hasFixes && !hasStructural) return 0;

  const store = useDocumentStore.getState();
  let applied = 0;
  const skipped: string[] = [];

  // --- Property fixes ---
  for (const fix of result.fixes) {
    const node = store.getNodeById(fix.nodeId);
    if (!node) {
      skipped.push(`${fix.nodeId} (not found)`);
      continue;
    }
    if (!(fix.property in SAFE_FIX_PROPERTIES)) {
      skipped.push(`${fix.nodeId}.${fix.property} (unsupported property)`);
      continue;
    }
    if (!isValidFixValue(fix.property, fix.value)) {
      skipped.push(`${fix.nodeId}.${fix.property}=${JSON.stringify(fix.value)} (invalid value)`);
      continue;
    }

    const oldValue = (node as unknown as Record<string, unknown>)[fix.property];

    // Safety guard: never change fit_content → fixed pixel on layout containers.
    if (
      (fix.property === 'height' || fix.property === 'width') &&
      typeof fix.value === 'number' &&
      oldValue === 'fit_content' &&
      'layout' in node &&
      node.layout &&
      'children' in node &&
      Array.isArray(node.children) &&
      node.children.length > 1
    ) {
      skipped.push(
        `${fix.nodeId}.${fix.property} (refusing fit_content→${fix.value}px on layout container)`,
      );
      console.warn(
        `[Validation] Blocked: ${fix.nodeId}.${fix.property} fit_content→${fix.value}px (layout container with ${node.children.length} children)`,
      );
      continue;
    }

    // fillColor is a virtual property — translate to PenFill array
    if (fix.property === 'fillColor' && typeof fix.value === 'string') {
      store.updateNode(fix.nodeId, { fill: [{ type: 'solid', color: fix.value }] });
      console.log(`[Validation Fix] ${fix.nodeId}: fill → ${fix.value}`);
      applied++;
      continue;
    }

    // strokeColor — translate to PenStroke
    if (fix.property === 'strokeColor' && typeof fix.value === 'string') {
      const existingNode = store.getNodeById(fix.nodeId);
      const existingStroke =
        existingNode && 'stroke' in existingNode ? existingNode.stroke : undefined;
      const thickness =
        existingStroke && 'thickness' in existingStroke
          ? ((existingStroke as { thickness?: number }).thickness ?? 1)
          : 1;
      store.updateNode(fix.nodeId, {
        stroke: { thickness, fill: [{ type: 'solid', color: fix.value }] },
      });
      console.log(`[Validation Fix] ${fix.nodeId}: strokeColor → ${fix.value}`);
      applied++;
      continue;
    }

    // strokeWidth — update thickness in existing stroke or create new stroke
    if (fix.property === 'strokeWidth' && typeof fix.value === 'number') {
      const existingNode = store.getNodeById(fix.nodeId);
      const existingStroke =
        existingNode && 'stroke' in existingNode ? existingNode.stroke : undefined;
      const color =
        existingStroke && 'fill' in (existingStroke as object)
          ? ((existingStroke as { fill?: Array<{ color?: string }> }).fill?.[0]?.color ?? '#CBD5E1')
          : '#CBD5E1';
      store.updateNode(fix.nodeId, {
        stroke: { thickness: fix.value, fill: [{ type: 'solid', color }] },
      });
      console.log(`[Validation Fix] ${fix.nodeId}: strokeWidth → ${fix.value}`);
      applied++;
      continue;
    }

    store.updateNode(fix.nodeId, { [fix.property]: fix.value });
    console.log(
      `[Validation Fix] ${fix.nodeId}: ${fix.property} ${JSON.stringify(oldValue)} → ${JSON.stringify(fix.value)}`,
    );
    applied++;
  }

  // --- Structural fixes ---
  for (const sf of result.structuralFixes) {
    if (sf.action === 'addChild') {
      const parent = store.getNodeById(sf.parentId);
      if (!parent) {
        skipped.push(`addChild: parent ${sf.parentId} not found`);
        continue;
      }
      const newNode = await buildNodeFromSpec(sf.node);
      if (!newNode) {
        skipped.push(`addChild: could not build node for ${sf.node.type}:${sf.node.name ?? '?'}`);
        continue;
      }
      store.addNode(sf.parentId, newNode, sf.index);
      console.log(
        `[Validation Fix] addChild: ${newNode.type}:${newNode.name ?? newNode.id} → parent ${sf.parentId} at index ${sf.index ?? 0}`,
      );
      applied++;

      autoFixParentLayoutAfterAddChild(store, sf.parentId, parent);
    } else if (sf.action === 'removeNode') {
      const node = store.getNodeById(sf.nodeId);
      if (!node) {
        skipped.push(`removeNode: ${sf.nodeId} not found`);
        continue;
      }
      // Never remove pre-injected chrome (e.g. iPhone status bar)
      if ('role' in node && (node as { role?: string }).role === 'status-bar') {
        skipped.push(`removeNode: ${sf.nodeId} is a protected status-bar node`);
        continue;
      }
      store.removeNode(sf.nodeId);
      console.log(`[Validation Fix] removeNode: ${sf.nodeId}`);
      applied++;
    }
  }

  if (skipped.length > 0) {
    console.warn(`[Validation] Skipped fixes:`, skipped);
  }

  return applied;
}

// ---------------------------------------------------------------------------
// Build node from structural fix spec
// ---------------------------------------------------------------------------

async function buildNodeFromSpec(spec: StructuralAddChildFix['node']): Promise<PenNode | null> {
  const id = nanoid(8);
  const node: Record<string, unknown> = {
    id,
    type: spec.type,
    name: spec.name,
  };

  if (spec.width != null) node.width = spec.width;
  if (spec.height != null) node.height = spec.height;
  if (spec.cornerRadius != null) node.cornerRadius = spec.cornerRadius;
  if (spec.layout) node.layout = spec.layout;
  if (spec.gap != null) node.gap = spec.gap;
  if (spec.padding != null) node.padding = spec.padding;
  if (spec.alignItems) node.alignItems = spec.alignItems;
  if (spec.justifyContent) node.justifyContent = spec.justifyContent;

  if (spec.fillColor && VALID_HEX_COLOR.test(spec.fillColor)) {
    node.fill = [{ type: 'solid', color: spec.fillColor }];
  }

  if (spec.type === 'text') {
    if (spec.content) node.content = spec.content;
    if (spec.fontSize) node.fontSize = spec.fontSize;
    if (spec.fontWeight) node.fontWeight = spec.fontWeight;
  }

  if (spec.type === 'path' && spec.name) {
    const color =
      spec.fillColor && VALID_HEX_COLOR.test(spec.fillColor) ? spec.fillColor : '#64748B';
    const icon = lookupIconByName(spec.name);
    if (icon) {
      node.d = icon.d;
      node.iconId = icon.iconId;
      if (icon.style === 'stroke') {
        node.stroke = { thickness: 2, fill: [{ type: 'solid', color }] };
        node.fill = [];
      } else {
        node.fill = [{ type: 'solid', color }];
      }
    } else {
      try {
        const res = await fetch(`/api/ai/icon?name=${encodeURIComponent(spec.name)}`);
        if (res.ok) {
          const data = await res.json();
          if (data.icon) {
            node.d = data.icon.d;
            node.iconId = data.icon.iconId;
            if (data.icon.style === 'stroke') {
              node.stroke = { thickness: 2, fill: [{ type: 'solid', color }] };
              node.fill = [];
            } else {
              node.fill = [{ type: 'solid', color }];
            }
          }
        }
      } catch {
        console.warn(`[Validation] Icon resolution failed for ${spec.name}`);
      }
    }
    if (!spec.width) node.width = 18;
    if (!spec.height) node.height = 18;
  }

  return node as unknown as PenNode;
}

// ---------------------------------------------------------------------------
// Auto-fix parent layout after adding a child
// ---------------------------------------------------------------------------

function autoFixParentLayoutAfterAddChild(
  store: ReturnType<typeof useDocumentStore.getState>,
  parentId: string,
  parentBeforeAdd: PenNode,
): void {
  const parentNode = parentBeforeAdd as unknown as Record<string, unknown>;
  const justify = parentNode.justifyContent as string | undefined;
  if (!justify || justify === 'start') return;

  const flatNodes = store.getFlatNodes();
  const parentNameBase = extractNameBase(parentBeforeAdd.name ?? '');

  const currentParent = store.getNodeById(parentId);
  const currentChildCount =
    currentParent && 'children' in currentParent ? (currentParent.children?.length ?? 0) : 0;

  for (const candidate of flatNodes) {
    if (candidate.id === parentId) continue;
    if (candidate.type !== parentBeforeAdd.type) continue;

    const cand = candidate as unknown as Record<string, unknown>;
    if (cand.layout !== parentNode.layout) continue;

    const candNameBase = extractNameBase(candidate.name ?? '');
    if (!parentNameBase || !candNameBase || parentNameBase !== candNameBase) continue;

    const candChildCount =
      'children' in candidate ? ((candidate as { children?: unknown[] }).children?.length ?? 0) : 0;
    if (currentChildCount > candChildCount) {
      console.log(
        `[Validation Fix] autoFixParentLayout: skipped ${parentId} — has ${currentChildCount} children vs candidate ${candidate.id} with ${candChildCount}`,
      );
      return;
    }

    const candJustify = cand.justifyContent as string | undefined;
    const candGap = cand.gap as number | undefined;

    const updates: Record<string, unknown> = {};
    if ((candJustify ?? 'start') !== justify) {
      updates.justifyContent = candJustify ?? 'start';
    }
    if (candGap != null && candGap !== parentNode.gap) {
      updates.gap = candGap;
    }

    if (Object.keys(updates).length > 0) {
      store.updateNode(parentId, updates);
      console.log(
        `[Validation Fix] autoFixParentLayout: ${parentId} matched ${candidate.id} →`,
        updates,
      );
    }
    return;
  }
}

function extractNameBase(name: string): string {
  const words = name.trim().toLowerCase().split(/\s+/);
  return words.length > 0 ? words[words.length - 1] : '';
}
