import type { PenNode } from '@/types/pen';
import type { PenEffect, PenFill, PenStroke, StyledTextSegment } from '@/types/styles';
import type { CodegenAssetHint } from './codegen-assets';

export type CodegenSemanticKind =
  | 'button'
  | 'input'
  | 'card'
  | 'navbar'
  | 'tabbar'
  | 'table'
  | 'form'
  | 'list_item'
  | 'modal'
  | 'image'
  | 'avatar'
  | 'text'
  | 'icon'
  | 'container';

export interface CodegenSemanticHint {
  kind: CodegenSemanticKind;
  confidence: number;
  reason: string;
}

export interface CodegenDesignIRNode {
  id: string;
  name?: string;
  type: PenNode['type'];
  role?: string;
  semanticHints: CodegenSemanticHint[];
  bounds: {
    x?: number;
    y?: number;
    width?: number | string;
    height?: number | string;
  };
  layout?: {
    mode?: string;
    gap?: number | string;
    padding?: unknown;
    justifyContent?: string;
    alignItems?: string;
  };
  text?: {
    content: string;
    fontFamily?: string;
    fontSize?: number;
    fontWeight?: number | string;
    lineHeight?: number;
    textAlign?: string;
    fill?: string;
  };
  appearance?: {
    fills?: unknown[];
    stroke?: unknown;
    effects?: unknown[];
    cornerRadius?: unknown;
    opacity?: number | string;
  };
  assetRefs: string[];
  children?: CodegenDesignIRNode[];
}

export interface CodegenDesignIR {
  version: 1;
  target: {
    width?: number | string;
    height?: number | string;
    platformHint: 'desktop' | 'mobile' | 'unknown';
  };
  summary: {
    nodeCount: number;
    textCount: number;
    imageCount: number;
    assetCount: number;
    semanticKinds: Record<string, number>;
    textContent: string[];
  };
  assets: CodegenAssetHint[];
  nodes: CodegenDesignIRNode[];
}

const MAX_TEXT_CONTENT = 80;

function numberLike(value: unknown): number | string | undefined {
  if (typeof value === 'number') return Number.isFinite(value) ? value : undefined;
  if (typeof value === 'string' && value.trim()) return value;
  return undefined;
}

function readNodeSize(node: PenNode): { width?: number | string; height?: number | string } {
  const sized = node as PenNode & { width?: unknown; height?: unknown };
  return {
    width: numberLike(sized.width),
    height: numberLike(sized.height),
  };
}

function readTextContent(content: unknown): string {
  if (typeof content === 'string') return content;
  if (Array.isArray(content)) {
    return content
      .map((segment: StyledTextSegment) => segment.text)
      .filter(Boolean)
      .join('');
  }
  return '';
}

function getTextFromNode(node: PenNode): string {
  if (node.type === 'text') return readTextContent(node.content);
  return '';
}

function collectDescendantText(node: PenNode): string {
  const parts: string[] = [];
  const visit = (current: PenNode) => {
    const text = getTextFromNode(current);
    if (text) parts.push(text);
    if ('children' in current && Array.isArray(current.children)) {
      for (const child of current.children) visit(child);
    }
  };
  visit(node);
  return parts.join(' ').trim();
}

function lowerText(value: string | undefined): string {
  return (value ?? '').toLowerCase();
}

function hasAny(value: string, words: string[]): boolean {
  return words.some((word) => value.includes(word));
}

function hasImageAsset(node: PenNode): boolean {
  if (node.type === 'image') return true;
  if ('fill' in node && Array.isArray(node.fill)) {
    return node.fill.some((fill) => fill.type === 'image');
  }
  return false;
}

function hasChildren(node: PenNode): boolean {
  return 'children' in node && Array.isArray(node.children) && node.children.length > 0;
}

function isLikelyAvatar(node: PenNode): boolean {
  const { width, height } = readNodeSize(node);
  return (
    hasImageAsset(node) &&
    typeof width === 'number' &&
    typeof height === 'number' &&
    Math.abs(width - height) <= 4 &&
    width <= 160
  );
}

function detectSemanticHints(node: PenNode): CodegenSemanticHint[] {
  const hints: CodegenSemanticHint[] = [];
  const name = lowerText(node.name);
  const role = lowerText(node.role);
  const text = lowerText(collectDescendantText(node));
  const combined = [name, role, text].join(' ');
  const layout = 'layout' in node ? node.layout : undefined;

  const push = (kind: CodegenSemanticKind, confidence: number, reason: string) => {
    hints.push({ kind, confidence: Number(confidence.toFixed(2)), reason });
  };

  if (role) {
    if (hasAny(role, ['button', '按钮'])) push('button', 0.9, 'role');
    if (hasAny(role, ['card', '卡片'])) push('card', 0.86, 'role');
    if (hasAny(role, ['input', 'field', '输入'])) push('input', 0.84, 'role');
    if (hasAny(role, ['nav', '导航'])) push('navbar', 0.84, 'role');
  }

  if (hasAny(combined, ['button', 'btn', 'cta', '按钮', '立即', '提交', '登录', '注册'])) {
    push('button', 0.72, 'name or text');
  }
  if (hasAny(combined, ['input', 'search', 'placeholder', 'email', 'password', '输入', '搜索'])) {
    push('input', 0.7, 'name or text');
  }
  if (hasAny(combined, ['card', 'item', 'tile', '卡片', '列表项'])) {
    push('card', 0.68, 'name');
  }
  if (hasAny(combined, ['navbar', 'nav', 'header', 'menu', '导航', '顶部'])) {
    push('navbar', 0.72, 'name');
  }
  if (hasAny(combined, ['tabbar', 'tabs', 'bottom bar', '底部导航'])) {
    push('tabbar', 0.76, 'name');
  }
  if (hasAny(combined, ['table', 'grid', '表格'])) {
    push('table', 0.72, 'name');
  }
  if (hasAny(combined, ['form', '表单'])) {
    push('form', 0.72, 'name');
  }
  if (hasAny(combined, ['modal', 'dialog', 'drawer', '弹窗', '抽屉'])) {
    push('modal', 0.72, 'name');
  }
  if (isLikelyAvatar(node)) push('avatar', 0.78, 'square image');
  if (hasImageAsset(node)) push('image', 0.82, 'image asset');
  if (node.type === 'text') push('text', 0.86, 'text node');
  if (node.type === 'path' || node.type === 'icon_font') push('icon', 0.78, 'icon node');
  if ((node.type === 'frame' || node.type === 'group') && hasChildren(node)) {
    push('container', layout && layout !== 'none' ? 0.74 : 0.58, 'container node');
  }

  const deduped = new Map<CodegenSemanticKind, CodegenSemanticHint>();
  for (const hint of hints) {
    const existing = deduped.get(hint.kind);
    if (!existing || hint.confidence > existing.confidence) deduped.set(hint.kind, hint);
  }
  return [...deduped.values()].sort((a, b) => b.confidence - a.confidence);
}

function serializeFills(fills: PenFill[] | undefined): unknown[] | undefined {
  if (!Array.isArray(fills) || fills.length === 0) return undefined;
  return fills.map((fill) => {
    if (fill.type === 'solid') {
      return { type: fill.type, color: fill.color, opacity: fill.opacity };
    }
    if (fill.type === 'image') {
      return { type: fill.type, url: fill.url, mode: fill.mode, opacity: fill.opacity };
    }
    return fill;
  });
}

function serializeStroke(stroke: PenStroke | undefined): unknown {
  if (!stroke) return undefined;
  return {
    thickness: stroke.thickness,
    align: stroke.align,
    fill: serializeFills(stroke.fill),
  };
}

function serializeEffects(effects: PenEffect[] | undefined): unknown[] | undefined {
  if (!Array.isArray(effects) || effects.length === 0) return undefined;
  return effects;
}

function collectAssetRefs(node: PenNode): string[] {
  const refs: string[] = [];
  if (node.type === 'image' && typeof node.src === 'string') refs.push(node.src);
  if ('fill' in node && Array.isArray(node.fill)) {
    for (const fill of node.fill) {
      if (fill.type === 'image' && typeof fill.url === 'string') refs.push(fill.url);
    }
  }
  return refs.filter((ref) => ref.startsWith('./assets/') || ref.startsWith('assets/'));
}

function buildIRNode(node: PenNode): CodegenDesignIRNode {
  const { width, height } = readNodeSize(node);
  const container = node as PenNode & {
    layout?: string;
    gap?: number | string;
    padding?: unknown;
    justifyContent?: string;
    alignItems?: string;
    cornerRadius?: unknown;
    fill?: PenFill[];
    stroke?: PenStroke;
    effects?: PenEffect[];
  };
  const textContent = getTextFromNode(node);
  const textNode = node.type === 'text' ? node : null;
  const children =
    'children' in node && Array.isArray(node.children) ? node.children.map(buildIRNode) : undefined;

  return {
    id: node.id,
    name: node.name,
    type: node.type,
    role: node.role,
    semanticHints: detectSemanticHints(node),
    bounds: {
      x: node.x,
      y: node.y,
      width,
      height,
    },
    layout:
      container.layout || container.gap !== undefined || container.padding !== undefined
        ? {
            mode: container.layout,
            gap: container.gap,
            padding: container.padding,
            justifyContent: container.justifyContent,
            alignItems: container.alignItems,
          }
        : undefined,
    text: textContent
      ? {
          content: textContent,
          fontFamily: textNode?.fontFamily,
          fontSize: textNode?.fontSize,
          fontWeight: textNode?.fontWeight,
          lineHeight: textNode?.lineHeight,
          textAlign: textNode?.textAlign,
          fill: serializeFills(textNode?.fill)?.[0]
            ? JSON.stringify(serializeFills(textNode?.fill)?.[0])
            : undefined,
        }
      : undefined,
    appearance: {
      fills: serializeFills(container.fill),
      stroke: serializeStroke(container.stroke),
      effects: serializeEffects(container.effects),
      cornerRadius: container.cornerRadius,
      opacity: node.opacity,
    },
    assetRefs: collectAssetRefs(node),
    children,
  };
}

function walkIR(nodes: CodegenDesignIRNode[], visit: (node: CodegenDesignIRNode) => void) {
  for (const node of nodes) {
    visit(node);
    if (node.children) walkIR(node.children, visit);
  }
}

export function collectDesignIRText(ir: CodegenDesignIR): string[] {
  return ir.summary.textContent;
}

export function buildCodegenDesignIR(
  nodes: PenNode[],
  assets: CodegenAssetHint[] = [],
): CodegenDesignIR {
  const irNodes = nodes.map(buildIRNode);
  const firstSize = nodes[0] ? readNodeSize(nodes[0]) : {};
  const summary = {
    nodeCount: 0,
    textCount: 0,
    imageCount: 0,
    assetCount: assets.length,
    semanticKinds: {} as Record<string, number>,
    textContent: [] as string[],
  };

  walkIR(irNodes, (node) => {
    summary.nodeCount += 1;
    if (node.text?.content) {
      summary.textCount += 1;
      if (summary.textContent.length < MAX_TEXT_CONTENT) {
        summary.textContent.push(node.text.content);
      }
    }
    if (node.assetRefs.length > 0 || node.type === 'image') summary.imageCount += 1;
    for (const hint of node.semanticHints) {
      summary.semanticKinds[hint.kind] = (summary.semanticKinds[hint.kind] ?? 0) + 1;
    }
  });

  const width = firstSize.width;
  const height = firstSize.height;
  const platformHint =
    typeof width === 'number' && typeof height === 'number' && width <= 480 && height >= width
      ? 'mobile'
      : typeof width === 'number' && width >= 900
        ? 'desktop'
        : 'unknown';

  return {
    version: 1,
    target: {
      width,
      height,
      platformHint,
    },
    summary,
    assets,
    nodes: irNodes,
  };
}
