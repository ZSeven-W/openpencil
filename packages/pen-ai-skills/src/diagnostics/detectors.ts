import type { PenNode, PenDocument } from '@zseven-w/pen-types';
import type { Issue } from './types';

/** Extract 节点中的第一个填充颜色（原始，包括变量引用） */
function getFirstFillColor(node: PenNode): string | null {
  if (!('fill' in node) || !Array.isArray(node.fill) || node.fill.length === 0) return null;
  const first = node.fill[0];
  if (first && 'color' in first && first.color) return first.color;
  return null;
}

/**
 * Compare 通过
 * WCAG 相对亮度对比度计算两个颜色字符串。 Returns 1.0 对于相同的颜色，随着它们的发散而向 21.0 增长，或者
 * Infinity 如果任一颜色无法解析（例如变量引用）。 Why WCAG 对比度而不是最大 RGB
 *
 * 通道差异：人眼对深色背景上的微小色调差异比浅色背景更敏感（Weber–Fechner / 暗适应）。 9 单元 RGB diff
 * 在不同的亮度下意味着非常
 * 不同的东西： #FAFAFA vs #F1F1F1 (light, RGB diff 9)：对比度 ≈ 1.07
 * → 不可见 #111111 vs #1a1a1a (dark, RGB diff 9)：对比度 ≈ 1.18 → 可区分
 *
 * Channel-diff 对它们一视同仁，并在黑暗主题卡上产生误报。 Contrast
 * 比率是基于亮度的，感知上均匀，并且在两种情况下都给出了正确的答案。 It 还与度量 WCAG
 *
 * 和设计工具（Stark、
 * Figma）使用相匹配。 Used by detectInvisibleContainers 来捕获填充颜色在视觉上几乎相同但不严格相等的情况。
 *
 *
 *
 *
 *
 */
function colorContrast(a: string, b: string): number {
  if (a === b) return 1;
  const pa = parseHexColor(a);
  const pb = parseHexColor(b);
  if (!pa || !pb) return Infinity;
  const lumA = relativeLuminance(pa);
  const lumB = relativeLuminance(pb);
  const lighter = Math.max(lumA, lumB);
  const darker = Math.min(lumA, lumB);
  return (lighter + 0.05) / (darker + 0.05);
}

/** WCAG sRGB 2.x 相对亮度。 Returns 0.0–1.0。 */
function relativeLuminance(c: { r: number; g: number; b: number }): number {
  const lin = (v: number): number => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
}

/** Parse #rgb / #rrggbb / #rrggbbaa 到 {r,g,b}。 Returns 解析失败时为 null。 */
function parseHexColor(s: string): { r: number; g: number; b: number } | null {
  if (typeof s !== 'string') return null;
  const m = s.trim().match(/^#([0-9a-fA-F]{3,8})$/);
  if (!m) return null;
  let hex = m[1];
  if (hex.length === 3) {
    hex = hex
      .split('')
      .map((c) => c + c)
      .join('');
  }
  if (hex.length !== 6 && hex.length !== 8) return null;
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) return null;
  return { r, g, b };
}

/** Check 如果节点已经有可见的笔画 */
function hasStroke(node: PenNode): boolean {
  if (!('stroke' in node)) return false;
  const s = node.stroke as { thickness?: number } | undefined;
  return s != null && (s.thickness ?? 0) > 0;
}

interface PenDocumentLike {
  variables?: Record<string, unknown>;
}

/** Resolve 边框颜色：首选 $color-border 变量（如果存在），否则为中性灰色 */
function getBorderStroke(doc: PenDocumentLike): {
  thickness: number;
  fill: Array<{ type: 'solid'; color: string }>;
} {
  const hasBorderVar = doc.variables && 'color-border' in doc.variables;
  const color = hasBorderVar ? '$color-border' : '#E2E8F0';
  return { thickness: 1, fill: [{ type: 'solid', color }] };
}

export interface DetectInvisibleContainersOptions {
  /**
   * “感知”使用 WCAG
   * 相对亮度对比度和 `contrastRatioThreshold`。 'strict' 使用 === 相等（原始行为）。
   * Default：“感知”。
   */
  colorMatchMode?: 'strict' | 'perceptual';
  /**
   * Maximum WCAG
   * 对比度 (1.0–21.0)，低于该对比度的两种颜色在感知模式下被视为“相同”。 Default：1.10。 Reference
   *
   * 分： 1.00 = 相同 1.10 = 在浅色或深色背景下基本无法区分 1.50 = 微妙可见 3.00 =
   * 清晰可见（WCAG AA
   * UI 元素） 4.50 =
   * WCAG AA 文本对比度
   * Higher = 更积极的检测（将标记更多对为“看不见”）。 Lower = 更严格（误报更少，遗漏更多）。
   *
   *
   *
   *
   */
  contrastRatioThreshold?: number;
}

/**
 * Detect 帧的填充与
 * 其父级的填充相匹配（呈现“不可见”）并且包含可见内容。 Suggests 添加微妙的笔划，使容器变得可区分。
 *
 */
export function detectInvisibleContainers(
  root: PenNode,
  doc: PenDocument,
  opts: DetectInvisibleContainersOptions = {},
): Issue[] {
  const mode = opts.colorMatchMode ?? 'perceptual';
  const threshold = opts.contrastRatioThreshold ?? 1.1;
  const issues: Issue[] = [];
  walk(root, null);
  return issues;

  function walk(node: PenNode, parentFillColor: string | null): void {
    const nodeFill = getFirstFillColor(node);

    if (
      parentFillColor &&
      nodeFill &&
      !hasStroke(node) &&
      node.type === 'frame' &&
      'layout' in node &&
      (node as { layout?: unknown }).layout &&
      'children' in node &&
      node.children &&
      node.children.length > 0
    ) {
      const ratio = colorContrast(nodeFill, parentFillColor);
      const same = mode === 'strict' ? nodeFill === parentFillColor : ratio <= threshold;
      if (same) {
        // Theme 感知严重性：在深色背景上，建议的浅灰色边框 (#E2E8F0) 会严重损坏设计。
        // Downgrade 暗对暗情况为“信息”（仅检测）；预验证会跳过信息严重性，因此 user/agent
        // 可以手动决定是否添加适合深色的边框。 Light-on-light 情况保持“警告”并保持自动修复。
        const parsedParent = parseHexColor(parentFillColor);
        const isDarkOnDark = parsedParent != null && relativeLuminance(parsedParent) < 0.1;
        issues.push({
          nodeId: node.id,
          category: 'invisible-container',
          severity: isDarkOnDark ? 'info' : 'warning',
          property: 'stroke',
          currentValue: (node as { stroke?: unknown }).stroke ?? null,
          suggestedValue: getBorderStroke(doc),
          reason: `same fill as parent (${nodeFill} ≈ ${parentFillColor}, contrast=${ratio.toFixed(2)})`,
        });
      }
    }

    if ('children' in node && node.children) {
      for (const child of node.children) {
        walk(child, nodeFill ?? parentFillColor);
      }
    }
  }
}

/**
 * Detect path nodes without geometry data (empty `d` property).
 * These render as invisible empty rectangles on canvas.
 */
export function detectEmptyPaths(root: PenNode): Issue[] {
  const issues: Issue[] = [];
  walk(root);
  return issues;

  function walk(node: PenNode): void {
    if (node.type === 'path') {
      const hasD = 'd' in node && (node as unknown as Record<string, unknown>).d;
      if (!hasD) {
        issues.push({
          nodeId: node.id,
          category: 'empty-path',
          severity: 'warning',
          property: '__remove',
          currentValue: null,
          suggestedValue: true,
          reason: 'path node without geometry (renders invisible)',
        });
      }
    }
    if ('children' in node && node.children) {
      for (const child of node.children) walk(child);
    }
  }
}

/**
 * Detect text nodes with explicit pixel heights. Explicit heights on text
 * always cause clipping or overlap; the layout engine should auto-calculate
 * height from content + fontSize + lineHeight instead.
 */
export function detectTextExplicitHeights(root: PenNode): Issue[] {
  const issues: Issue[] = [];
  walk(root);
  return issues;

  function walk(node: PenNode): void {
    if (node.type === 'text') {
      const textNode = node as PenNode & { height?: unknown; textGrowth?: string };
      if (typeof textNode.height === 'number' && textNode.textGrowth !== 'fixed-width-height') {
        issues.push({
          nodeId: node.id,
          category: 'text-explicit-height',
          severity: 'warning',
          property: 'height',
          currentValue: textNode.height,
          suggestedValue: 'fit_content',
          reason: `text node has explicit height=${textNode.height}px — causes clipping`,
        });
      }
    }
    if ('children' in node && node.children) {
      for (const child of node.children) walk(child);
    }
  }
}

/**
 * Property classification for sibling consistency checks.
 *
 * STRICT props are intentionally role-dependent — their value SHOULD vary
 * by what the sibling represents (hero is tall, footer is short, tab-bar
 * is fixed-height). They are only compared among same-type, same-role
 * siblings.
 *
 * LOOSE props are design-system tokens that should be uniform across
 * structurally-similar siblings regardless of semantic role (typically
 * cornerRadius). They additionally get a cross-role check among same-type
 * siblings, catching outliers in singleton-role compositions (e.g. a web
 * landing page where every section has a unique role and the strict pass
 * would otherwise skip every group as <3 members).
 *
 * The strict/loose split is what lets this detector handle BOTH:
 *   - Mobile chrome shells without false positives (chrome's fixed height
 *     is in STRICT → never compared against sections cross-role)
 *   - Web landing pages without false negatives (cornerRadius is in LOOSE
 *     → compared across hero/features/cta/footer even though all roles
 *     are singletons)
 */
const FRAME_STRICT_PROPS = ['height'] as const;
const FRAME_LOOSE_PROPS = ['cornerRadius'] as const;
const TEXT_STRICT_PROPS = ['fontSize'] as const;

/**
 * Detect property inconsistencies among siblings (>= 3 siblings,
 * >= 2/3 majority rule). Outliers get an Issue to align with the majority.
 *
 * Two parallel passes:
 *
 *   1. STRICT pass: groups siblings by `${type}:${role || '__none__'}`
 *      and checks STRICT + LOOSE properties. Same-role siblings get the
 *      tightest consistency check. Divider/spacer roles are skipped
 *      entirely (intentionally tiny layout primitives whose dimensions
 *      shouldn't match anything).
 *
 *   2. LOOSE pass: groups siblings by `${type}` only and checks LOOSE
 *      properties. This catches outliers in compositions where every
 *      sibling has a unique role (web landing pages: hero/features/
 *      cta/footer) which would otherwise all be skipped as singleton
 *      strict groups.
 *
 * Issues that overlap between the two passes are deduplicated by nodeId
 * + property; the strict-pass version wins (it iterates first).
 *
 * Why a strict/loose split rather than a hand-curated chrome list:
 *   - Same role names mean different things in different design types:
 *     `navbar` is chrome on mobile but a content section on web; `footer`
 *     is a content section on web but rare on mobile. Any hand-curated
 *     chrome list will be wrong in one of those contexts.
 *   - Splitting by property semantics ("which props are role-dependent?")
 *     is invariant across design contexts and doesn't require curating
 *     a list of structurally-distinct roles.
 */
export function detectSiblingInconsistencies(root: PenNode): Issue[] {
  const raw: Issue[] = [];
  walk(root);

  // Dedupe: strict and loose passes can both emit on the same
  // (nodeId, property) pair (e.g. a same-role cornerRadius outlier
  // is caught by both). Keep first occurrence — strict iterates first.
  const seen = new Set<string>();
  const unique: Issue[] = [];
  for (const issue of raw) {
    const key = `${issue.nodeId}:${issue.property}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(issue);
  }
  return unique;

  function walk(node: PenNode): void {
    if (!('children' in node) || !node.children) return;
    if (node.children.length >= 3) {
      const strictGroups = new Map<string, PenNode[]>();
      const typeGroups = new Map<string, PenNode[]>();
      for (const child of node.children) {
        const childRole = ((child as { role?: string }).role ?? '').toLowerCase() || '__none__';
        // Skip divider/spacer — visual layout primitives whose dimensions
        // are intentionally tiny and don't share structure with siblings;
        // reporting them as outliers is always noise.
        if (childRole === 'divider' || childRole === 'spacer') continue;

        const strictKey = `${child.type}:${childRole}`;
        if (!strictGroups.has(strictKey)) strictGroups.set(strictKey, []);
        strictGroups.get(strictKey)!.push(child);

        if (!typeGroups.has(child.type)) typeGroups.set(child.type, []);
        typeGroups.get(child.type)!.push(child);
      }

      // Strict pass: same-type, same-role siblings get checked on every
      // applicable property (STRICT + LOOSE). Severity 'warning' — these
      // are confidently auto-fixable because the comparison group shares
      // a semantic role.
      for (const [strictKey, siblings] of strictGroups) {
        if (siblings.length < 3) continue;
        const type = strictKey.split(':', 1)[0];
        const props =
          type === 'text'
            ? (TEXT_STRICT_PROPS as readonly string[])
            : ([...FRAME_STRICT_PROPS, ...FRAME_LOOSE_PROPS] as readonly string[]);
        for (const prop of props) {
          checkConsistency(siblings, prop, raw, 'warning');
        }
      }

      // Loose pass: same-type-only siblings get checked on role-independent
      // properties only. Catches singleton-role outliers (web landing page
      // sections) without re-introducing the chrome height false positive
      // (height is in STRICT, never checked here).
      //
      // Severity 'info' — these are DETECT-ONLY. The pre-validation pipeline
      // skips info-severity issues for auto-fix, because cross-role
      // comparison can match a structurally distinct sibling (e.g. a
      // rounded tab-bar among square sections) and silently rewriting it
      // would damage intentional design choices. The issue still appears
      // in debug reports so the user/agent can review it manually.
      for (const [type, siblings] of typeGroups) {
        if (siblings.length < 3) continue;
        if (type === 'text') continue; // text has no role-independent props
        for (const prop of FRAME_LOOSE_PROPS) {
          checkConsistency(siblings, prop, raw, 'info');
        }
      }
    }
    for (const child of node.children) walk(child);
  }
}

/**
 * Run all 4 detectors and return the deduplicated combined issue list.
 * Dedup key: `${nodeId}:${property}` (matches runPreValidationFixes).
 * On collision, the first issue wins (detector execution order below).
 */
export function detectAllIssues(root: PenNode, doc: PenDocument): Issue[] {
  const combined: Issue[] = [
    ...detectInvisibleContainers(root, doc),
    ...detectEmptyPaths(root),
    ...detectTextExplicitHeights(root),
    ...detectSiblingInconsistencies(root),
  ];
  const seen = new Set<string>();
  const unique: Issue[] = [];
  for (const issue of combined) {
    const key = `${issue.nodeId}:${issue.property}`;
    if (seen.has(key)) continue;
    seen.add(key);
    unique.push(issue);
  }
  return unique;
}

function checkConsistency(
  siblings: PenNode[],
  property: string,
  issues: Issue[],
  severity: 'warning' | 'info',
): void {
  const values = new Map<string, { value: unknown; nodes: PenNode[] }>();
  for (const node of siblings) {
    const raw = (node as unknown as Record<string, unknown>)[property];
    if (raw == null) continue;
    const key = JSON.stringify(raw);
    if (!values.has(key)) values.set(key, { value: raw, nodes: [] });
    values.get(key)!.nodes.push(node);
  }
  if (values.size < 2) return;

  let majority: { value: unknown; nodes: PenNode[] } | null = null;
  for (const entry of values.values()) {
    if (!majority || entry.nodes.length > majority.nodes.length) majority = entry;
  }
  if (!majority) return;

  const totalWithProp = Array.from(values.values()).reduce((s, e) => s + e.nodes.length, 0);
  if (majority.nodes.length < (totalWithProp * 2) / 3) return;

  for (const entry of values.values()) {
    if (entry === majority) continue;
    for (const node of entry.nodes) {
      issues.push({
        nodeId: node.id,
        category: 'sibling-inconsistency',
        severity,
        property,
        currentValue: (node as unknown as Record<string, unknown>)[property],
        suggestedValue: majority.value,
        reason: `inconsistent with ${majority.nodes.length} siblings`,
      });
    }
  }
}
