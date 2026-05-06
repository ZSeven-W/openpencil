// packages/pen-renderer/src/render-node-thumbnail.ts
//
// Offscreen 个人 PenNodes 的缩略图助手。 Used 因 git 冲突
// UI 渲染并排 ours/theirs 预览，而无需安装完整的
// PenRenderer 实例。
//
// Design 目标：
//   - Accept 完整文档上下文，以便正确解析引用类型节点。
//   - Return 成功时为 data-URL 字符串，失败时为 null（优雅）。
//   - Never throw — 所有错误均被捕获并转换为 null。
//   - CanvasKit 在测试/SSR 中不可用：检测并回退到 null。
//   - The 输出形状（数据 URL | null）对于测试断言来说是稳定的。

import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import { getAllChildren, getDefaultTheme, resolveNodeForCanvas } from '@zseven-w/pen-core';
import { flattenToRenderNodes, resolveRefs } from './document-flattener.js';

export interface ThumbnailContext {
  /** Full 用于引用节点解析的文档。 */
  document: PenDocument;
  /** Page id 用于在多页文档中定位节点（今天未使用；为将来的每页上下文保留）。 */
  pageId: string | null;
  /** Output 画布大小，以逻辑像素为单位（方形，默认值：128）。 */
  size?: number;
}

/**
 * Render 将单个
 *
 * PenNode 按请求的大小转换为数据 URL。 Returns 渲染成功时的数据 URL 字符串，或 `null`，当：
 * - CanvasKit
 * / OffscreenCanvas 不可用（Node.js / 测试环境） - The 节点或文档无效 - Any 渲染步骤抛出
 * Callers MUST 处理 `null` 情况并显示 placeholder.
 *
 *
 */
export async function renderNodeThumbnail(
  node: PenNode,
  ctx: ThumbnailContext,
): Promise<string | null> {
  try {
    // Guard：需要一个有效的节点对象。
    if (!node || typeof node !== 'object') return null;

    const size = ctx.size ?? 128;
    if (!Number.isFinite(size) || size <= 0) return null;

    // Resolve ref 节点使用根文档树作为组件注册表。 resolveRefs 遍历节点树并用其原始组件替换 `ref` 节点。 For
    // 非引用节点它是浅身份传递。 getAllChildren 处理单页 (document.children) 和多页
    // (document.pages[i].children) 布局 - refs 可以跨页面，所以我们需要全部。
    const rootNodes: PenNode[] = ctx.document ? getAllChildren(ctx.document) : [];
    let resolvedNodes: PenNode[];
    try {
      resolvedNodes = resolveRefs([node], rootNodes);
    } catch {
      // If 引用解析失败（例如循环引用），回退到原始节点。
      resolvedNodes = [node];
    }

    const resolvedNode = resolvedNodes[0] ?? node;

    // Resolve 设计 $variable 引用，以便填充颜色、描边宽度等使用其具体值而不是原始的“$color-primary”字符串进行渲染
    // 。 Mirrors 与 renderer.ts 中的步骤相同（第 279 行）。
    const variables = ctx.document?.variables ?? {};
    const themes = ctx.document?.themes;
    const activeTheme = getDefaultTheme(themes);
    const variableResolved = resolveNodeForCanvas(resolvedNode, variables, activeTheme);

    // Flatten 到 RenderNode 数组，因此我们拥有所有后代的绝对坐标、自动布局位置和文本预先测量。
    let renderNodes;
    try {
      renderNodes = flattenToRenderNodes([variableResolved]);
    } catch {
      return null;
    }

    if (!renderNodes || renderNodes.length === 0) return null;

    // Detect CanvasKit 可用性 — 在测试或 SSR 中不可用。
    let ck: import('canvaskit-wasm').CanvasKit | null = null;
    try {
      const { getCanvasKit } = await import('./init.js');
      ck = getCanvasKit();
    } catch {
      return null;
    }

    if (!ck) {
      // CanvasKit 尚未初始化。
      return null;
    }

    // OffscreenCanvas 防护 — 在 Node.js 中不可用。
    if (typeof OffscreenCanvas === 'undefined') return null;

    // Determine 缩放：使节点的边界框适合请求的大小。
    const rootRenderNode = renderNodes[0];
    const nodeW = rootRenderNode.absW > 0 ? rootRenderNode.absW : size;
    const nodeH = rootRenderNode.absH > 0 ? rootRenderNode.absH : size;
    const scale = Math.min(size / nodeW, size / nodeH);

    const canvasW = Math.max(1, Math.round(nodeW * scale));
    const canvasH = Math.max(1, Math.round(nodeH * scale));

    // Software SkSurface（不需要 WebGL — 可以在屏幕外安全使用）。
    const skSurface = ck.MakeSurface(canvasW, canvasH);
    if (!skSurface) return null;

    try {
      const skCanvas = skSurface.getCanvas();
      skCanvas.clear(ck.TRANSPARENT);
      skCanvas.scale(scale, scale);

      const { SkiaNodeRenderer } = await import('./node-renderer.js');
      const renderer = new SkiaNodeRenderer(ck);
      for (const rn of renderNodes) {
        renderer.drawNode(skCanvas, rn);
      }

      skSurface.flush();
      const imgSnapshot = skSurface.makeImageSnapshot();
      if (!imgSnapshot) return null;

      const pngBytes = imgSnapshot.encodeToBytes();
      if (!pngBytes) return null;

      // Convert 通过 Blob + FileReader 将原始 PNG 字节转换为数据 URL。
      const blob = new Blob([pngBytes as Uint8Array<ArrayBuffer>], { type: 'image/png' });
      const dataUrl = await blobToDataUrl(blob);
      return dataUrl;
    } finally {
      skSurface.delete();
    }
  } catch {
    // Any 意外错误 → 优雅的 null
    return null;
  }
}

/** 使用 FileReader 将 Convert a Blob 转换为数据 URL。 */
async function blobToDataUrl(blob: Blob): Promise<string | null> {
  return new Promise<string | null>((resolve) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      resolve(typeof result === 'string' ? result : null);
    };
    reader.onerror = () => resolve(null);
    reader.readAsDataURL(blob);
  });
}
