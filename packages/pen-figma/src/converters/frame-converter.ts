import type {
  FigmaNodeChange,
  FigmaSymbolOverride,
  FigmaDerivedSymbolDataEntry,
  FigmaGUID,
} from '../figma-types.js';
import type { PenNode } from '@zseven-w/pen-types';
import { mapFigmaFills } from '../figma-fill-mapper.js';
import { mapFigmaStroke } from '../figma-stroke-mapper.js';
import { mapFigmaEffects } from '../figma-effect-mapper.js';
import { mapFigmaLayout } from '../figma-layout-mapper.js';
import type { TreeNode } from '../figma-tree-builder.js';
import { guidToString } from '../figma-tree-builder.js';
import {
  commonProps,
  resolveWidth,
  resolveHeight,
  mapCornerRadius,
  scaleTreeChildren,
  type ConversionContext,
} from './common.js';

// Forward 声明 — convertChildren 在 index.ts 中定义并注入此处以避免循环导入。
let _convertChildren: (parent: TreeNode, ctx: ConversionContext) => PenNode[];

export function setConvertChildren(fn: (parent: TreeNode, ctx: ConversionContext) => PenNode[]) {
  _convertChildren = fn;
}

export function convertFrame(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const id = ctx.generateId();
  const children = _convertChildren(treeNode, ctx);

  // In 保留模式，仅对实际设置了 stackMode 的帧应用自动布局属性。 没有 stackMode 的 Frames 使用绝对 x,y 定位。 For
  // 自动布局框架，子级顺序必须颠倒，因为树构建器按降序排序（对于 z 堆叠），但布局需要升序（流顺序）。
  const hasAutoLayout = figma.stackMode && figma.stackMode !== 'NONE';
  const layout =
    ctx.layoutMode === 'preserve'
      ? hasAutoLayout
        ? mapFigmaLayout(figma)
        : figma.frameMaskDisabled !== true
          ? { clipContent: true }
          : {}
      : mapFigmaLayout(figma);

  // Reverse 保留模式下自动布局框架的子级顺序：树构建器按位置降序排序（z 堆叠），但自动布局需要升序（第一个子级 =
  // 布局流的开始）。
  const orderedChildren =
    hasAutoLayout && ctx.layoutMode === 'preserve' && children.length > 1
      ? [...children].reverse()
      : children;

  return {
    type: 'frame',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    ...layout,
    cornerRadius: mapCornerRadius(figma),
    fill: mapFigmaFills(figma.fillPaints) ?? mapFigmaFills(figma.backgroundPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
    children: orderedChildren.length > 0 ? orderedChildren : undefined,
  };
}

export function convertGroup(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const id = ctx.generateId();
  const children = _convertChildren(treeNode, ctx);

  return {
    type: 'group',
    ...commonProps(figma, id),
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    children: children.length > 0 ? children : undefined,
  };
}

export function convertComponent(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const figmaId = figma.guid ? guidToString(figma.guid) : '';
  const id = ctx.componentMap.get(figmaId) ?? ctx.generateId();
  const children = _convertChildren(treeNode, ctx);

  const hasAutoLayout = figma.stackMode && figma.stackMode !== 'NONE';
  const layout =
    ctx.layoutMode === 'preserve'
      ? hasAutoLayout
        ? mapFigmaLayout(figma)
        : figma.frameMaskDisabled !== true
          ? { clipContent: true }
          : {}
      : mapFigmaLayout(figma);

  const orderedChildren =
    hasAutoLayout && ctx.layoutMode === 'preserve' && children.length > 1
      ? [...children].reverse()
      : children;

  return {
    type: 'frame',
    ...commonProps(figma, id),
    reusable: true,
    width: resolveWidth(figma, parentStackMode, ctx),
    height: resolveHeight(figma, parentStackMode, ctx),
    ...layout,
    cornerRadius: mapCornerRadius(figma),
    fill: mapFigmaFills(figma.fillPaints) ?? mapFigmaFills(figma.backgroundPaints),
    stroke: mapFigmaStroke(figma),
    effects: mapFigmaEffects(figma.effects),
    children: orderedChildren.length > 0 ? orderedChildren : undefined,
  };
}

export function convertInstance(
  treeNode: TreeNode,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): PenNode {
  const figma = treeNode.figma;
  const componentGuid = figma.overriddenSymbolID ?? figma.symbolData?.symbolID;

  // Check 如果此实例具有必须内联的视觉覆盖（填充、描边、arcData、文本）
  const hasVisualOverrides =
    figma.symbolData?.symbolOverrides?.some(
      (ov: any) =>
        ov.fillPaints?.length > 0 ||
        ov.strokePaints?.length > 0 ||
        ov.arcData ||
        ov.textData ||
        ov.fontSize !== undefined,
    ) ?? false;

  // Try 内联主 SYMBOL 的子级并应用覆盖。当实例没有本地子级（意味着它需要渲染符号的内容）或具有视觉覆盖时，Always
  // 内联。
  if (componentGuid && (treeNode.children.length === 0 || hasVisualOverrides)) {
    const symbolNode = ctx.symbolTree.get(guidToString(componentGuid));
    if (symbolNode && symbolNode.children.length > 0) {
      const children = applyInstanceOverrides(
        symbolNode,
        figma.symbolData?.symbolOverrides,
        figma.derivedSymbolData,
        figma.size,
        ctx.symbolTree,
      );
      // Merge 符号的布局和视觉属性到实例中。 Instances 从其主节点继承，但剪贴板数据可能不包括实例节点本身的继承属性
      // 。
      const mergedFigma = mergeSymbolProps(treeNode.figma, symbolNode.figma);
      return convertFrame({ figma: mergedFigma, children }, parentStackMode, ctx);
    }
  }

  const componentPenId = componentGuid
    ? ctx.componentMap.get(guidToString(componentGuid))
    : undefined;

  if (componentPenId) {
    const id = ctx.generateId();
    return {
      type: 'ref',
      ...commonProps(figma, id),
      ref: componentPenId,
    };
  }

  return convertFrame(treeNode, parentStackMode, ctx);
}

/**
 * Merge 符号的属性到
 * 实例节点中。 Instances 从其主组件继承布局和视觉属性，但剪贴板数据可能不包含实例上的这
 * 些继承值。 Instance 自己的属性优先（它们是显式覆盖）。
 *
 */
function mergeSymbolProps(instance: FigmaNodeChange, symbol: FigmaNodeChange): FigmaNodeChange {
  const merged = { ...instance };

  // Layout 属性 — 自动布局检测和布局生成所需
  const layoutKeys: (keyof FigmaNodeChange)[] = [
    'stackMode',
    'stackSpacing',
    'stackPadding',
    'stackHorizontalPadding',
    'stackVerticalPadding',
    'stackPaddingRight',
    'stackPaddingBottom',
    'stackPrimaryAlignItems',
    'stackCounterAlignItems',
    'stackPrimarySizing',
    'stackCounterSizing',
    'stackChildPrimaryGrow',
    'stackChildAlignSelf',
    'frameMaskDisabled',
  ];

  // Visual 属性 — fills/strokes 为框架本身
  const visualKeys: (keyof FigmaNodeChange)[] = [
    'fillPaints',
    'strokePaints',
    'strokeWeight',
    'strokeAlign',
    'cornerRadius',
    'rectangleCornerRadiiIndependent',
    'rectangleTopLeftCornerRadius',
    'rectangleTopRightCornerRadius',
    'rectangleBottomLeftCornerRadius',
    'rectangleBottomRightCornerRadius',
  ];

  for (const key of [...layoutKeys, ...visualKeys]) {
    if ((merged as any)[key] === undefined && (symbol as any)[key] !== undefined) {
      (merged as any)[key] = (symbol as any)[key];
    }
  }

  return merged;
}

function guidPathKey(guids: FigmaGUID[]): string {
  return guids.map((g) => guidToString(g)).join('/');
}

/**
 * 当将 SYMBOL
 * 子级内联到实例中时，Apply INSTANCE 会覆盖（填充、arcData）和派生数据（大小、转换）到 SYMBOL 子级。
 */
function applyInstanceOverrides(
  symbolNode: TreeNode,
  overrides: FigmaSymbolOverride[] | undefined,
  derived: FigmaDerivedSymbolDataEntry[] | undefined,
  instanceSize: { x: number; y: number } | undefined,
  _symbolTree: Map<string, TreeNode>,
): TreeNode[] {
  // If 没有派生数据，也没有覆盖，退回到简单缩放
  if ((!derived || derived.length === 0) && (!overrides || overrides.length === 0)) {
    if (instanceSize && symbolNode.figma.size) {
      const sx = instanceSize.x / symbolNode.figma.size.x;
      const sy = instanceSize.y / symbolNode.figma.size.y;
      return scaleTreeChildren(symbolNode.children, sx, sy);
    }
    return symbolNode.children;
  }

  // Build 覆盖由 guidPath 字符串键入的映射
  const overrideMap = new Map<string, FigmaSymbolOverride>();
  if (overrides) {
    for (const ov of overrides) {
      if (ov.guidPath?.guids?.length) {
        overrideMap.set(guidPathKey(ov.guidPath.guids), ov);
      }
    }
  }

  // Build 派生映射，由 guidPath 字符串键入
  const derivedMap = new Map<string, FigmaDerivedSymbolDataEntry>();
  const safeDerived = derived ?? [];
  for (const d of safeDerived) {
    if (d.guidPath?.guids?.length) {
      derivedMap.set(guidPathKey(d.guidPath.guids), d);
    }
  }

  // Flatten SYMBOL 树处于预序 DFS 中，子级按升序排序 GUID localID
  const flatSymbol: TreeNode[] = [];
  function flattenDFS(node: TreeNode) {
    flatSymbol.push(node);
    const sorted = [...node.children].sort((a, b) => {
      const aId = a.figma.guid?.localID ?? 0;
      const bId = b.figma.guid?.localID ?? 0;
      return aId - bId;
    });
    for (const c of sorted) flattenDFS(c);
  }
  flattenDFS(symbolNode);

  // Filter 仅派生为 length-1 guidPaths（不包括嵌套实例条目）
  const len1Derived = safeDerived.filter((d) => d.guidPath?.guids?.length === 1);

  // Extract 来自第一个派生条目的 session/localID 基础
  const firstGuids = len1Derived[0]?.guidPath?.guids;
  const sessionID = firstGuids?.[0]?.sessionID;
  const firstLocalID = firstGuids?.[0]?.localID;

  // Resolve 覆盖并派生数据到实际符号树节点。
  const nodeOverride = new Map<string, FigmaSymbolOverride>();
  const nodeDerived = new Map<string, FigmaDerivedSymbolDataEntry>();
  const pkToNodeGuid = new Map<string, string>();

  /** Resolve a pathKey 的 override/derived 条目到目标节点 GUID。 */
  function resolveToNode(pathKey: string, nodeGuid: string) {
    const d = derivedMap.get(pathKey);
    if (d) nodeDerived.set(nodeGuid, d);
    const ov = overrideMap.get(pathKey);
    if (ov) nodeOverride.set(nodeGuid, ov);
  }

  // Build GUID→nodeGuid 直接查找映射
  const guidToNodeMap = new Map<string, string>();
  for (const node of flatSymbol) {
    if (node.figma.guid)
      guidToNodeMap.set(guidToString(node.figma.guid), guidToString(node.figma.guid));
  }

  // Strategy 0: Direct GUID 匹配
  let directMatches = 0;
  for (const d of len1Derived) {
    const pk = d.guidPath?.guids?.[0];
    if (pk && guidToNodeMap.has(guidToString(pk))) directMatches++;
  }

  if (directMatches > len1Derived.length * 0.5 || len1Derived.length === 0) {
    for (const d of len1Derived) {
      const pk = d.guidPath?.guids?.[0];
      if (!pk) continue;
      const pkStr = guidToString(pk);
      if (guidToNodeMap.has(pkStr)) {
        resolveToNode(pkStr, pkStr);
        pkToNodeGuid.set(pkStr, pkStr);
      }
    }
    // Also 解析使用实际 GUIDs 的覆盖
    for (const [pk] of overrideMap) {
      if (pk.includes('/')) continue;
      if (guidToNodeMap.has(pk)) {
        const ov = overrideMap.get(pk);
        if (ov) nodeOverride.set(pk, ov);
      }
    }
  } else if (len1Derived.length === flatSymbol.length) {
    // Strategy 1：精确计数匹配 — 索引映射（对于虚拟 GUIDs）
    for (let i = 0; i < flatSymbol.length; i++) {
      const node = flatSymbol[i];
      const d = len1Derived[i];
      if (node.figma.guid && d.guidPath?.guids?.length) {
        const actualGuid = guidToString(node.figma.guid);
        resolveToNode(guidPathKey(d.guidPath.guids), actualGuid);
        pkToNodeGuid.set(guidToString(d.guidPath.guids[0]), actualGuid);
      }
    }
  } else if (firstLocalID !== undefined && sessionID !== undefined) {
    // Strategy 2：完整的 DFS + 扩展的 DFS。
    const childSorted = [...symbolNode.children].sort(
      (a, b) => (a.figma.guid?.localID ?? 0) - (b.figma.guid?.localID ?? 0),
    );

    const fullPkToNode = new Map<string, string>();
    let fullIdx = 0;
    function walkFull(node: TreeNode) {
      if (node.figma.guid) {
        fullPkToNode.set(`${sessionID}:${firstLocalID! + fullIdx}`, guidToString(node.figma.guid));
      }
      fullIdx++;
      const sorted = [...node.children].sort(
        (a, b) => (a.figma.guid?.localID ?? 0) - (b.figma.guid?.localID ?? 0),
      );
      for (const c of sorted) walkFull(c);
    }
    for (const c of childSorted) walkFull(c);

    const rootGuid = symbolNode.figma.guid ? guidToString(symbolNode.figma.guid) : '';
    const rootPkToNode = new Map<string, string>();
    let rootIdx = 0;
    function walkRoot(node: TreeNode) {
      if (node.figma.guid) {
        rootPkToNode.set(`${sessionID}:${firstLocalID! + rootIdx}`, guidToString(node.figma.guid));
      }
      rootIdx++;
      const sorted = [...node.children].sort(
        (a, b) => (a.figma.guid?.localID ?? 0) - (b.figma.guid?.localID ?? 0),
      );
      for (const c of sorted) walkRoot(c);
    }
    walkRoot(symbolNode);

    for (const [pk, ng] of fullPkToNode) {
      pkToNodeGuid.set(pk, ng);
    }

    for (const [pk, d] of derivedMap) {
      if (pk.includes('/')) continue;
      const ng = fullPkToNode.get(pk);
      if (ng) nodeDerived.set(ng, d);
    }
    for (const [pk, ov] of overrideMap) {
      if (pk.includes('/')) continue;
      if (rootPkToNode.get(pk) === rootGuid) continue;
      const ng = fullPkToNode.get(pk);
      if (ng) nodeOverride.set(ng, ov);
    }
  } else {
    // Fallback: direct index mapping with all derived
    for (let i = 0; i < Math.min(flatSymbol.length, safeDerived.length); i++) {
      const node = flatSymbol[i];
      const d = safeDerived[i];
      if (node.figma.guid && d.guidPath?.guids?.length) {
        const actualGuid = guidToString(node.figma.guid);
        resolveToNode(guidPathKey(d.guidPath.guids), actualGuid);
        if (d.guidPath.guids.length === 1) {
          pkToNodeGuid.set(guidToString(d.guidPath.guids[0]), actualGuid);
        }
      }
    }
  }

  // Build nested maps for multi-guid entries
  const nestedOverrideMap = new Map<string, FigmaSymbolOverride[]>();
  const nestedDerivedMap = new Map<string, FigmaDerivedSymbolDataEntry[]>();

  for (const [pk, ov] of overrideMap) {
    if (!pk.includes('/')) continue;
    const parts = pk.split('/');
    const instanceGuid = pkToNodeGuid.get(parts[0]) ?? parts[0];
    const childGuids = ov.guidPath?.guids?.slice(1);
    if (childGuids?.length) {
      const childOv = { ...ov, guidPath: { guids: childGuids } };
      const existing = nestedOverrideMap.get(instanceGuid) ?? [];
      existing.push(childOv);
      nestedOverrideMap.set(instanceGuid, existing);
    }
  }

  for (const [pk, d] of derivedMap) {
    if (!pk.includes('/')) continue;
    const parts = pk.split('/');
    const instanceGuid = pkToNodeGuid.get(parts[0]) ?? parts[0];
    const childGuids = d.guidPath?.guids?.slice(1);
    if (childGuids?.length) {
      const childD = { ...d, guidPath: { guids: childGuids } };
      const existing = nestedDerivedMap.get(instanceGuid) ?? [];
      existing.push(childD);
      nestedDerivedMap.set(instanceGuid, existing);
    }
  }

  // Recursively apply resolved overrides and derived data to each node
  function applyToNode(node: TreeNode): TreeNode {
    const nodeKey = node.figma.guid ? guidToString(node.figma.guid) : '';
    const d = nodeDerived.get(nodeKey);
    const ov = nodeOverride.get(nodeKey);
    const nestedOvs = nestedOverrideMap.get(nodeKey);
    const nestedDer = nestedDerivedMap.get(nodeKey);

    if (!d && !ov && !nestedOvs && !nestedDer) {
      return { figma: { ...node.figma }, children: node.children.map(applyToNode) };
    }

    const figma = { ...node.figma };

    // Apply derived data (pre-computed sizes and transforms for this instance)
    if (d) {
      if (d.size && node.figma.size && figma.strokeWeight !== undefined) {
        const sx = d.size.x / node.figma.size.x;
        const sy = d.size.y / node.figma.size.y;
        const strokeScale = Math.min(sx, sy);
        if (strokeScale < 0.99) {
          figma.strokeWeight = Math.round(figma.strokeWeight * strokeScale * 100) / 100;
        }
      }
      if (d.size) figma.size = d.size;
      if (d.transform) figma.transform = d.transform;
      if (d.fontSize !== undefined) figma.fontSize = d.fontSize;
      if (d.derivedTextData?.characters !== undefined) figma.textData = d.derivedTextData;
    }

    // Apply all override properties from the symbolOverride entry
    if (ov) {
      const skipKeys = new Set([
        'guidPath',
        'guid',
        'parentIndex',
        'type',
        'phase',
        'symbolData',
        'derivedSymbolData',
        'componentKey',
        'variableConsumptionMap',
        'parameterConsumptionMap',
        'prototypeInteractions',
        'styleIdForFill',
        'styleIdForStrokeFill',
        'styleIdForText',
        'overrideLevel',
        'componentPropAssignments',
        'proportionsConstrained',
        'fontVersion',
      ]);
      for (const key of Object.keys(ov)) {
        if (skipKeys.has(key)) continue;
        const value = (ov as Record<string, unknown>)[key];
        if (value !== undefined) {
          (figma as Record<string, unknown>)[key] = value;
        }
      }
    }

    // Propagate multi-guid overrides and derived data to nested INSTANCE nodes.
    if ((nestedOvs || nestedDer) && (figma.type === 'INSTANCE' || figma.symbolData)) {
      if (nestedOvs) {
        const existingOverrides = figma.symbolData?.symbolOverrides ?? [];
        figma.symbolData = {
          ...figma.symbolData,
          symbolOverrides: [...existingOverrides, ...nestedOvs],
        };
      }
      if (nestedDer) {
        figma.derivedSymbolData = nestedDer;
      }
    }

    return { figma, children: node.children.map(applyToNode) };
  }

  return symbolNode.children.map(applyToNode);
}
